//! Yanagisawa PIC, shared by the Sharp X68000, FM Towns, NEC PC-88 VA and MSX
//! platforms.
//!
//! Sources:
//! - Header, machine types and colour formats, the length code, the colour cache
//!   (128 entries, least recently used replaced) and the chain code that carries a
//!   change point down to following lines: Yanagisawa, "PICのフォーマットについて"
//!   (`pic_fmt.txt` in PIC_FMT, <https://www.vector.co.jp/soft/data/art/se003198.html>;
//!   "転載、引用すべて自由"). The sample loader in that archive was not read.
//! - Observed from `recoil2png` output (samples and synthesized files):
//!   - accepted type bytes: 0x00 and 0x1F (X68000), 0x02 and 0xC2 (FM Towns),
//!     0x11 and 0x21 (PC-88 VA, 16 bits only); types 0x02 and 0x1F carry the
//!     6-byte extended header (save position and aspect);
//!   - decoding starts before the first pixel, the first new colour goes to
//!     cache entry 1 and entry 0 is the last to be replaced;
//!   - palettes are X68000 colour words (FM Towns: 5 bits per component, no
//!     intensity); a comment starting with `/MM/` (MSX) keeps 3 bits;
//!   - PC-88 VA type 0x21 packs two 256-colour `GGGRRRBB` pixels per word and
//!     two lines side by side, so the height doubles; blue levels step by 0x55.

use alloc::vec;
use alloc::vec::Vec;

use super::super::nec_pc::Machine;
use crate::{DecodeError, Image};

/// Largest picture accepted, in pixels.
const MAX_PIXELS: usize = 1 << 22;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Colour {
    /// Palette indices of 4 or 8 bits.
    Indexed(u32),
    /// X68000 `GGGGGRRRRRBBBBB`, intensity 0.
    X68000Rgb15,
    /// X68000 `GGGGGRRRRRBBBBBI`.
    X68000Rgb16,
    /// PC-88 VA `GGGGGGRRRRRBBBBB`.
    Va16,
    /// PC-88 VA: two `GGGRRRBB` pixels per word.
    VaTiled,
}

impl Colour {
    fn bits(self) -> u32 {
        match self {
            Self::Indexed(bits) => bits,
            Self::X68000Rgb15 => 15,
            _ => 16,
        }
    }
}

struct Header<'a> {
    machine: Machine,
    colour: Colour,
    width: usize,
    height: usize,
    palette: Vec<u32>,
    stream: &'a [u8],
}

fn be16(data: &[u8], at: usize) -> Option<usize> {
    Some(u16::from_be_bytes([*data.get(at)?, *data.get(at + 1)?]) as usize)
}

const fn level(v: u32, bits: u32) -> u32 {
    let v = v << (8 - bits);
    (v | v >> bits | v >> (2 * bits)) & 0xff
}

/// X68000 colour word `GGGGGRRRRRBBBBBI`: 5 bits per component plus intensity.
fn x68000(word: u32) -> u32 {
    let i = word & 1;
    let six = |shift: u32| level((word >> shift & 31) << 1 | i, 6);
    six(6) << 16 | six(11) << 8 | six(1)
}

impl<'a> Header<'a> {
    fn parse(data: &'a [u8]) -> Option<Self> {
        if !data.starts_with(b"PIC") {
            return None;
        }
        let msx = data[3..].starts_with(b"/MM/");
        let eof = 3 + data[3..].iter().position(|&b| b == 0x1a)?;
        let comment_end = eof + 1 + data[eof + 1..].iter().position(|&b| b == 0)?;
        let fixed = comment_end + 2;
        let kind = *data.get(fixed)?;
        let bits = be16(data, fixed + 1)?;
        let width = be16(data, fixed + 3)?;
        let height = be16(data, fixed + 5)?;
        let mut at = fixed + 7;
        let (mut machine, extended, towns) = match kind {
            0x00 => (Machine::X68000, false, false),
            0x1f => (Machine::X68000, true, false),
            0x02 => (Machine::FmTowns, true, true),
            0xc2 => (Machine::FmTowns, false, true),
            0x11 | 0x21 if bits == 16 => (Machine::Pc88Va, false, false),
            _ => return None,
        };
        if extended {
            at += 6;
        }
        let colour = match (kind, bits) {
            (0x11, _) => Colour::Va16,
            (0x21, _) => Colour::VaTiled,
            (_, 4 | 8) => Colour::Indexed(bits as u32),
            (_, 15) => Colour::X68000Rgb15,
            (_, 16) => Colour::X68000Rgb16,
            _ => return None,
        };
        let mut palette = Vec::new();
        if let Colour::Indexed(bits) = colour {
            for i in 0..1usize << bits {
                let word = be16(data, at + 2 * i)? as u32;
                palette.push(if msx {
                    let three = |shift: u32| level(word >> (shift + 2) & 7, 3);
                    three(6) << 16 | three(11) << 8 | three(1)
                } else if towns {
                    let five = |shift: u32| level(word >> shift & 31, 5);
                    five(6) << 16 | five(11) << 8 | five(1)
                } else {
                    x68000(word)
                });
            }
            at += 2 << bits;
        }
        if msx {
            machine = Machine::Msx;
        }
        Some(Self {
            machine,
            colour,
            width,
            height,
            palette,
            stream: data.get(at..)?,
        })
    }
}

/// MSB-first bit reader; `None` past the end.
struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
}

impl Bits<'_> {
    fn bit(&mut self) -> Option<u32> {
        let byte = *self.data.get(self.pos / 8)?;
        let bit = (byte >> (7 - self.pos % 8)) & 1;
        self.pos += 1;
        Some(bit as u32)
    }

    fn bits(&mut self, count: u32) -> Option<u32> {
        (0..count).try_fold(0, |acc, _| Some(acc << 1 | self.bit()?))
    }

    /// Distance to the next change point: `k` ones, a zero, then `k + 1` bits.
    fn length(&mut self) -> Option<usize> {
        let mut k = 0;
        while self.bit()? == 1 {
            k += 1;
            if k > 28 {
                return None;
            }
        }
        Some(self.bits(k + 1)? as usize + (1 << (k + 1)) - 1)
    }

    /// Chain step to the next line: `01` left, `10` down, `11` right,
    /// `000` end, `0010` two left, `0011` two right.
    fn chain_step(&mut self) -> Option<Option<isize>> {
        Some(match self.bits(2)? {
            0 if self.bit()? == 0 => None,
            0 if self.bit()? == 0 => Some(-2),
            0 => Some(2),
            code => Some(code as isize - 2),
        })
    }
}

/// 128 recently used colours; `order` runs from the next to be replaced to the
/// most recently used.
struct Cache {
    colours: [u32; 128],
    order: Vec<u8>,
}

impl Cache {
    fn new() -> Self {
        Self {
            colours: [0; 128],
            order: (1..128).chain([0]).collect(),
        }
    }

    fn touch(&mut self, slot: u8) {
        if let Some(i) = self.order.iter().position(|&s| s == slot) {
            self.order.remove(i);
        }
        self.order.push(slot);
    }

    fn get(&mut self, slot: u8) -> u32 {
        self.touch(slot);
        self.colours[slot as usize]
    }

    fn insert(&mut self, colour: u32) {
        let slot = self.order[0];
        self.colours[slot as usize] = colour;
        self.touch(slot);
    }
}

const NO_MARK: u32 = u32::MAX;

/// Decodes the change-point stream into one colour value per pixel.
fn unpack(header: &Header) -> Option<Vec<u32>> {
    let (width, total) = (header.width, header.width * header.height);
    let mut out = vec![0u32; total];
    let mut marks = vec![NO_MARK; total];
    let mut bits = Bits {
        data: header.stream,
        pos: 0,
    };
    let colour_bits = header.colour.bits();
    let mut cache = (!matches!(header.colour, Colour::Indexed(_))).then(Cache::new);
    let mut colour = 0;
    let mut next = 0;
    loop {
        let length = bits.length()?;
        let point = next + length - 1;
        for i in next..point.min(total) {
            if marks[i] != NO_MARK {
                colour = marks[i];
                marks[i] = NO_MARK;
            }
            out[i] = colour;
        }
        if point >= total {
            return Some(out);
        }
        colour = match cache.as_mut() {
            Some(cache) if bits.bit()? == 1 => cache.get(bits.bits(7)? as u8),
            Some(cache) => {
                let colour = bits.bits(colour_bits)?;
                cache.insert(colour);
                colour
            }
            None => bits.bits(colour_bits)?,
        };
        out[point] = colour;
        marks[point] = NO_MARK;
        next = point + 1;
        if bits.bit()? == 1 {
            let (mut x, mut y) = ((point % width) as isize, point / width);
            while let Some(dx) = bits.chain_step()? {
                x += dx;
                y += 1;
                if (0..width as isize).contains(&x) && y < header.height {
                    marks[y * width + x as usize] = colour;
                }
            }
        }
    }
}

/// Decodes a PIC picture if it was saved on `machine`.
pub(in crate::platform) fn decode_pic(data: &[u8], machine: Machine) -> Result<Image, DecodeError> {
    let header = Header::parse(data).ok_or(DecodeError::Unrecognized)?;
    let (width, height) = (header.width, header.height);
    if header.machine != machine || width == 0 || height == 0 || width * height > MAX_PIXELS {
        return Err(DecodeError::Unrecognized);
    }
    let values = unpack(&header).ok_or(DecodeError::Unrecognized)?;
    if header.colour == Colour::VaTiled {
        // Word (x, y) holds pixels 2x and 2x + 1 of a line made of output lines
        // 2y and 2y + 1 side by side; the low byte is the left pixel.
        let mut image = Image::new(width as u32, (height * 2) as u32);
        for (i, &word) in values.iter().enumerate() {
            let (x, y) = (i % width * 2, i / width * 2);
            for (half, byte) in [word & 0xff, word >> 8].into_iter().enumerate() {
                let joined = x + half;
                let (column, line) = (joined % width, y + joined / width);
                let colour =
                    level(byte >> 2 & 7, 3) << 16 | level(byte >> 5, 3) << 8 | ((byte & 3) * 0x55);
                image.set(column as u32, line as u32, colour);
            }
        }
        return Ok(image);
    }
    let mut image = Image::new(width as u32, height as u32);
    for (i, &value) in values.iter().enumerate() {
        let colour = match header.colour {
            Colour::Indexed(_) => header.palette.get(value as usize).copied().unwrap_or(0),
            Colour::X68000Rgb15 => x68000(value << 1),
            Colour::X68000Rgb16 => x68000(value),
            _ => {
                level(value >> 5 & 31, 5) << 16 | level(value >> 10, 6) << 8 | level(value & 31, 5)
            }
        };
        image.set((i % width) as u32, (i / width) as u32, colour);
    }
    Ok(image)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels() {
        assert_eq!(level(31, 5), 0xff);
        assert_eq!(level(17, 5), 0x8c);
        assert_eq!(level(62, 6), 0xfb);
        assert_eq!(level(4, 3), 0x92);
        assert_eq!(x68000(0x8c62), 0x8a8a8a);
    }

    #[test]
    fn cache_fills_from_entry_1() {
        let mut cache = Cache::new();
        cache.insert(5);
        cache.insert(6);
        assert_eq!(cache.get(1), 5);
        assert_eq!(cache.get(2), 6);
    }

    #[test]
    fn single_colour_picture() {
        // Length 1, new colour 0x7c00 (green), no chain, then the rest (length 4).
        let mut data = b"PICx\x1a\0\0\0\0\x0f\0\x02\0\x02".to_vec();
        data.extend([0b0001_1111, 0, 0b0001_0010, 0]);
        let image = decode_pic(&data, Machine::X68000).unwrap();
        assert_eq!((image.width(), image.height()), (2, 2));
        assert_eq!(&image.rgb()[..3], &[0, 0xfb, 0]);
        assert_eq!(&image.rgb()[9..], &[0, 0xfb, 0]);
        assert!(decode_pic(&data[..data.len() - 2], Machine::X68000).is_err());
    }
}
