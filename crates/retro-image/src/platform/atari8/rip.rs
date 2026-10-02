//! RIP (Rocky Interlace Picture): HIP-like interlaced pictures with colour
//! registers, raw or packed with the "PCK" packer.
//!
//! Sources:
//! - Just Solve "Rocky Interlace Picture"
//!   (<http://fileformats.archiveteam.org/wiki/Rocky_Interlace_Picture>) for the
//!   name and that it extends HIP with colour; atari-owner.com "Atari Software
//!   Graphic Modes" (<https://atari-owner.com/club/articles/atari-software-graphic-modes.17/>)
//!   for the display modes. Neither documents the layout or the packer.
//! - Everything else was reverse engineered from the corpus samples (AWORL,
//!   COYOTE, TAQUART, GOSTBUST, C640002, MOONSET, PRAGNIEN, MADMAN, C640095,
//!   C640096) and by black-box probing of `recoil2png`: raw files built from
//!   a hand-made header with uniform frames and one colour at a time, size and
//!   header scans, and bit flips in packed streams. The packer was solved by
//!   decoding GOSTBUST's stream and comparing it with the 7680 bytes
//!   `recoil2png` shows.
//!
//! Header: `RIP`, four bytes of version text, a mode byte, then big-endian
//! 16-bit fields: 0 or 1 (packed; the data is packed when it starts with
//! `PCK`), the header length (not read), the width in 4-pixel units (even,
//! 2-80), the height (1-239) and the title length; then `T:`, the title, a
//! tab, `CM:` and 9 colour bytes (the registers 704-712). The data follows.
//! Bytes per line are width / 2. Modes (the byte after the version):
//! - `0e`: one Graphics 15 frame; 0 is the background (register 8), 1-3 are
//!   registers 4-6.
//! - `1e`: two such frames, averaged.
//! - `10`: two Graphics 15 frames averaged. On even lines the first uses
//!   registers 4-7 for the values 0-3 and the second registers 0-3; on odd
//!   lines they swap.
//! - `20`: a GTIA mode 10 frame (registers 0-8, as in HIP) then a mode 9
//!   frame, shown like HIP.
//! - `30`: like `20`, but mode 10 colours come from a table after both
//!   frames: 8 bytes per pair of lines, for the values 1-8 (the registers
//!   0-7 by value - 1; 9-11 use the last, 12-15 the entries 3-6). Value 0 is
//!   black. The colour bytes of the header are not read.
//!
//! The packer ("PCK", an LZ77 scheme with Huffman codes): 13 bytes that are
//! not read, then three canonical Huffman code length tables of 4-bit lengths
//! (high nibble first): 64 symbols for match lengths, 256 for match
//! distances, 256 for literals. The codes are assigned by length and then
//! by symbol, and read most significant bit first. Then a bit stream of
//! tokens: a 0 bit and a literal, or a 1 bit, a distance symbol (distance
//! symbol + 2 bytes back, bytes before the start count as 0) and a length
//! symbol (length symbol + 2 bytes). A stream that ends early or holds an
//! unused code leaves the rest of the picture 0, as `recoil2png` does.

use super::antic::Bitmap;
use super::hip::{half_pixel_pair, nibble};
use super::palette::{register_rgb, rgb};
use super::screen::gtia10_register;
use crate::bytes::be16;
use crate::{DecodeError, Image};
use alloc::vec::Vec;

const MAX_CODE_LENGTH: usize = 15;

pub(super) fn decode_rip(data: &[u8]) -> Result<Image, DecodeError> {
    let bad = DecodeError::Unrecognized;
    if data.len() < 18 || !data.starts_with(b"RIP") {
        return Err(bad);
    }
    let mode = data[7];
    let field = |n: usize| {
        be16(data, 8 + 2 * n)
            .map(usize::from)
            .ok_or(DecodeError::Unrecognized)
    };
    let (packed, width, height, title) = (field(0)?, field(2)?, field(3)?, field(4)?);
    let bytes_per_line = width / 2;
    if packed & 0xff > 1
        || width % 2 != 0
        || !(2..=80).contains(&width)
        || !(1..=239).contains(&height)
    {
        return Err(bad);
    }
    let rest = data.get(18..).ok_or(DecodeError::Unrecognized)?;
    let rest = rest.strip_prefix(b"T:").ok_or(DecodeError::Unrecognized)?;
    let rest = rest.get(title..).ok_or(DecodeError::Unrecognized)?;
    let rest = rest.strip_prefix(b"\t").ok_or(DecodeError::Unrecognized)?;
    let rest = rest.strip_prefix(b"CM:").ok_or(DecodeError::Unrecognized)?;
    let (registers, body) = rest.split_at_checked(9).ok_or(DecodeError::Unrecognized)?;

    let frame = height * bytes_per_line;
    let size = match mode {
        0x0e => frame,
        0x1e | 0x10 | 0x20 => 2 * frame,
        0x30 => 2 * frame + 8 * height.div_ceil(2),
        _ => return Err(bad),
    };
    let unpacked;
    let screen = match body.strip_prefix(b"PCK") {
        Some(payload) => {
            unpacked = unpack(payload, size);
            &unpacked[..]
        }
        None if body.len() >= size => body,
        None => return Err(bad),
    };

    let gr15 = |data: &[u8], colors: &dyn Fn(usize) -> [u8; 4]| {
        let bitmap = Bitmap {
            data,
            bytes_per_line,
            lines: height,
            bits: 2,
        };
        bitmap.render(2, 1, |y, value| register_rgb(colors(y)[usize::from(value)]))
    };
    let out_width = 4 * width;
    let (first, second) = (
        &screen[..frame.min(screen.len())],
        screen.get(frame..2 * frame),
    );
    Ok(match mode {
        0x0e => gr15(first, &|_| {
            [registers[8], registers[4], registers[5], registers[6]]
        }),
        0x1e => {
            let colors = |_| [registers[8], registers[4], registers[5], registers[6]];
            Image::blend(&[&gr15(first, &colors), &gr15(second.ok_or(bad)?, &colors)])
        }
        0x10 => {
            // The two sets of four registers swap between the frames on
            // every line.
            let set = |n: usize| {
                [
                    registers[4 * n],
                    registers[4 * n + 1],
                    registers[4 * n + 2],
                    registers[4 * n + 3],
                ]
            };
            Image::blend(&[
                &gr15(first, &|y| set(1 - y % 2)),
                &gr15(second.ok_or(bad)?, &|y| set(y % 2)),
            ])
        }
        _ => {
            let second = second.ok_or(bad)?;
            let tail = &screen[2 * frame..];
            let mode10 = |y: usize, x: usize| {
                let value = nibble(&first[y * bytes_per_line..], x);
                if mode == 0x20 {
                    register_rgb(registers[gtia10_register(value)])
                } else {
                    tail_color(tail, y, value)
                }
            };
            let mode9 = |y: usize, x: usize| rgb(nibble(&second[y * bytes_per_line..], x));
            half_pixel_pair(out_width, height, mode9, mode10)
        }
    })
}

/// Colour of a mode `30` value on line `y`: black for 0, else the line pair's
/// table entry.
fn tail_color(tail: &[u8], y: usize, value: u8) -> u32 {
    let entry = match value {
        0 => return 0,
        1..=8 => value - 1,
        9..=11 => 7,
        _ => value - 9,
    };
    register_rgb(tail[8 * (y / 2) + usize::from(entry)])
}

/// A canonical Huffman code: the symbols sorted by code length and symbol,
/// and for each length the first code and the index of its first symbol.
struct Code {
    symbols: Vec<u16>,
    count: [u16; MAX_CODE_LENGTH + 1],
}

impl Code {
    fn new(lengths: impl Iterator<Item = u8>) -> Self {
        let lengths: Vec<u8> = lengths.collect();
        let mut count = [0u16; MAX_CODE_LENGTH + 1];
        for &length in &lengths {
            if length != 0 {
                count[usize::from(length)] += 1;
            }
        }
        let mut symbols = Vec::new();
        for length in 1..=MAX_CODE_LENGTH {
            for (symbol, &l) in lengths.iter().enumerate() {
                if usize::from(l) == length {
                    symbols.push(symbol as u16);
                }
            }
        }
        Self { symbols, count }
    }

    /// The next symbol, or `None` at the end of the data or for an unused code.
    fn read(&self, bits: &mut Bits) -> Option<u16> {
        let (mut code, mut first, mut index) = (0u32, 0u32, 0usize);
        for length in 1..=MAX_CODE_LENGTH {
            code = code << 1 | bits.next()?;
            let count = u32::from(self.count[length]);
            if code < first + count {
                return self.symbols.get(index + (code - first) as usize).copied();
            }
            index += count as usize;
            first = (first + count) << 1;
        }
        None
    }
}

/// Bits of a byte slice, most significant first.
struct Bits<'a> {
    data: &'a [u8],
    position: usize,
}

impl Bits<'_> {
    fn next(&mut self) -> Option<u32> {
        let byte = self.data.get(self.position / 8)?;
        let bit = byte >> (7 - self.position % 8) & 1;
        self.position += 1;
        Some(u32::from(bit))
    }
}

/// Unpacks `size` bytes; whatever the stream does not fill stays 0.
fn unpack(payload: &[u8], size: usize) -> Vec<u8> {
    let tables = payload.get(13..).unwrap_or(&[]);
    let nibbles = |from: usize, count: usize| {
        (from..from + count).map(|n| {
            tables
                .get(n / 2)
                .map_or(0, |&byte| if n % 2 == 0 { byte >> 4 } else { byte & 0x0f })
        })
    };
    let (lengths, distances, literals) = (
        Code::new(nibbles(0, 64)),
        Code::new(nibbles(64, 256)),
        Code::new(nibbles(320, 256)),
    );
    let mut bits = Bits {
        data: tables.get(288..).unwrap_or(&[]),
        position: 0,
    };
    let mut out = Vec::with_capacity(size);
    while out.len() < size {
        let Some(flag) = bits.next() else { break };
        if flag == 0 {
            let Some(literal) = literals.read(&mut bits) else {
                break;
            };
            out.push(literal as u8);
        } else {
            let (Some(distance), Some(length)) =
                (distances.read(&mut bits), lengths.read(&mut bits))
            else {
                break;
            };
            let distance = usize::from(distance) + 2;
            for _ in 0..(usize::from(length) + 2).min(size - out.len()) {
                let byte = out.len().checked_sub(distance).map_or(0, |i| out[i]);
                out.push(byte);
            }
        }
    }
    out.resize(size, 0);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;
    use alloc::vec;

    #[test]
    fn huffman_codes_are_canonical() {
        // Lengths 1, 2, 3, 3: codes 0, 10, 110, 111.
        let code = Code::new([1u8, 2, 3, 3].into_iter());
        let mut bits = Bits {
            data: &[0b0101_1011, 0b1000_0000],
            position: 0,
        };
        let symbols: Vec<u16> = (0..4).filter_map(|_| code.read(&mut bits)).collect();
        assert_eq!(symbols, [0, 1, 2, 3]);
    }

    #[test]
    fn unpacks_literals_and_matches() {
        // Literal code 8 bits wide (symbols 0-255), others 6 and 8 bits wide.
        let mut payload = vec![0u8; 13];
        payload.extend(vec![0x66; 32]);
        payload.extend(vec![0x88; 128]);
        payload.extend(vec![0x88; 128]);
        // Literal 0x12, literal 0x34, then a match of distance 2 + 0 and
        // length 2 + 3.
        let bits = "0".to_string() + "00010010" + "0" + "00110100" + "1" + "00000000" + "000011";
        for chunk in bits.as_bytes().chunks(8) {
            let mut byte = 0u8;
            for (i, &c) in chunk.iter().enumerate() {
                byte |= u8::from(c == b'1') << (7 - i);
            }
            payload.push(byte);
        }
        assert_eq!(
            unpack(&payload, 9),
            [0x12, 0x34, 0x12, 0x34, 0x12, 0x34, 0x12, 0, 0]
        );
    }

    #[test]
    fn empty_packed_stream_is_blank() {
        let mut data = b"RIP1.6 \x0e\0\x01\0\x21\0\x02\0\x01\0\0T:\tCM:".to_vec();
        data.extend_from_slice(&[0; 9]);
        data.extend_from_slice(b"PCK");
        let image = decode_rip(&data).unwrap();
        assert_eq!((image.width(), image.height()), (8, 1));
    }
}
