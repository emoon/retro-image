//! Pi (Yanagisawa), shared by the NEC PC-88 VA/PC-98, MSX and Sharp X68000
//! platforms.
//!
//! Sources:
//! - Header, delta table (per-colour move-to-front), variable-length delta and
//!   length codes, the five repeat locations: Kirinn Bunnylin, "Pi and PIC
//!   graphics formats" (<https://mooncore.eu/bunny/txt/pi-pic.htm>).
//! - Observed from `recoil2png` output (samples and synthesized files): a repeat
//!   sequence's first location is always used; a source before the start of the
//!   picture restarts at byte 0 or 1 for each pair; aspect 2:1 doubles the
//!   height except for `X68K`; palette precision by saver model (`X68K` the
//!   X68000 colour word, `MSX2` 3 bits, `PCVA` 4 bits at 2:1 and R5 G6 B5
//!   otherwise, else 4 bits for 16 colours and 8 bits for 256); the mode byte
//!   must be 0, the depth 4 or 8, and the stream must not end early.

use alloc::vec;
use alloc::vec::Vec;

use super::Machine;
use super::precision::Precision;
use crate::{DecodeError, Image};

/// Largest picture accepted, in pixels.
const MAX_PIXELS: usize = 1 << 22;

struct Header<'a> {
    model: &'a [u8],
    tall: bool,
    colours: usize,
    width: usize,
    height: usize,
    palette: &'a [u8],
    stream: &'a [u8],
}

impl<'a> Header<'a> {
    fn parse(data: &'a [u8]) -> Option<Self> {
        if !data.starts_with(b"Pi") {
            return None;
        }
        let comment_end = 2 + data[2..].iter().position(|&b| b == 0x1a)?;
        let h = comment_end + data[comment_end..].iter().position(|&b| b == 0)?;
        let fixed = data.get(h..h + 11)?;
        let colours = match (fixed[1], fixed[4]) {
            (0, 4) => 16,
            (0, 8) => 256,
            _ => return None,
        };
        let extra = u16::from_be_bytes([fixed[9], fixed[10]]) as usize;
        let size_at = h + 11 + extra;
        let size = data.get(size_at..size_at + 4)?;
        let width = u16::from_be_bytes([size[0], size[1]]) as usize;
        let height = u16::from_be_bytes([size[2], size[3]]) as usize;
        let palette_at = size_at + 4;
        let stream_at = palette_at + colours * 3;
        Some(Self {
            model: &fixed[5..9],
            tall: (fixed[2], fixed[3]) == (2, 1),
            colours,
            width,
            height,
            palette: data.get(palette_at..stream_at)?,
            stream: &data[stream_at..],
        })
    }

    fn machine(&self) -> Machine {
        match self.model {
            b"X68K" => Machine::X68000,
            b"MSX2" | b"MSX " => Machine::Msx,
            b"PCVA" => Machine::Pc88Va,
            b"PC88" | b"88SR" => Machine::Pc88,
            _ => Machine::Pc98,
        }
    }

    fn precision(&self) -> Precision {
        match self.model {
            b"X68K" => Precision::X68000,
            b"MSX2" => Precision::Bits(3),
            b"PCVA" if self.tall => Precision::Bits(4),
            b"PCVA" => Precision::Rgb565,
            _ if self.colours == 16 => Precision::Bits(4),
            _ => Precision::Bits(8),
        }
    }

    fn doubles_height(&self) -> bool {
        self.tall && self.model != b"X68K"
    }
}

/// MSB-first bit reader; `None` past the end.
struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
}

impl Bits<'_> {
    fn bit(&mut self) -> Option<usize> {
        let byte = *self.data.get(self.pos / 8)?;
        let bit = (byte >> (7 - self.pos % 8)) & 1;
        self.pos += 1;
        Some(bit as usize)
    }

    fn bits(&mut self, count: usize) -> Option<usize> {
        (0..count).try_fold(0, |acc, _| Some(acc << 1 | self.bit()?))
    }

    /// Delta code: `1x`, `00x`, then `01` + unary + binary, capped by the
    /// number of colours.
    fn delta(&mut self, colours: usize) -> Option<usize> {
        if self.bit()? == 1 {
            return self.bit();
        }
        if self.bit()? == 0 {
            return Some(2 + self.bit()?);
        }
        let mut k = 2;
        while (1 << (k + 1)) < colours && self.bit()? == 1 {
            k += 1;
        }
        Some((1 << k) + self.bits(k)?)
    }

    /// Repeat length in byte pairs: unary count `k` of ones, a zero, `k` bits.
    fn length(&mut self) -> Option<usize> {
        let mut k = 0;
        while self.bit()? == 1 {
            k += 1;
            if k > 24 {
                return None;
            }
        }
        Some((1 << k) + self.bits(k)?)
    }

    /// Repeat location 0-4 (`00`, `01`, `10`, `110`, `111`).
    fn location(&mut self) -> Option<u8> {
        match self.bits(2)? {
            3 => Some(3 + self.bit()? as u8),
            loc => Some(loc as u8),
        }
    }
}

struct Decoder<'a> {
    bits: Bits<'a>,
    out: Vec<u8>,
    pos: usize,
    width: usize,
    colours: usize,
    /// `table[a * colours + i]`: i-th most recent colour to follow colour `a`.
    table: Vec<u8>,
}

impl Decoder<'_> {
    fn full(&self) -> bool {
        self.pos >= self.out.len()
    }

    fn push(&mut self, value: u8) {
        if !self.full() {
            self.out[self.pos] = value;
        }
        self.pos += 1;
    }

    fn delta(&mut self) -> Option<()> {
        let previous = if self.pos == 0 {
            0
        } else {
            self.out[self.pos - 1] as usize
        };
        let index = self.bits.delta(self.colours)?;
        let row = &mut self.table[previous * self.colours..(previous + 1) * self.colours];
        let colour = row[index];
        row.copy_within(0..index, 1);
        row[0] = colour;
        self.push(colour);
        Some(())
    }

    /// Copies `pairs` byte pairs from `location`.
    fn repeat(&mut self, location: u8, pairs: usize) {
        let offset = match location {
            0 if self.pos < 4 || self.out[self.pos - 1] == self.out[self.pos - 2] => 2,
            0 => 4,
            1 => self.width,
            2 => 2 * self.width,
            3 => self.width - 1,
            _ => self.width + 1,
        };
        for _ in 0..pairs {
            if self.full() {
                return;
            }
            let mut source = self.pos as isize - offset as isize;
            if source < 0 {
                source = source.rem_euclid(2);
            }
            for i in 0..2 {
                let value = self.out[source as usize + i];
                self.push(value);
            }
        }
    }

    /// A sequence of repeats; ends when a location equals the previous one.
    fn repeats(&mut self, first_in_picture: bool) -> Option<()> {
        let mut previous = None;
        let mut shorten = first_in_picture;
        while !self.full() {
            let location = self.bits.location()?;
            if previous == Some(location) {
                break;
            }
            previous = Some(location);
            let pairs = self.bits.length()? - shorten as usize;
            shorten = false;
            self.repeat(location, pairs);
        }
        Some(())
    }

    fn run(&mut self) -> Option<()> {
        self.delta()?;
        self.delta()?;
        self.repeats(true)?;
        while !self.full() {
            self.delta()?;
            self.delta()?;
            if self.full() {
                break;
            }
            if self.bits.bit()? == 0 {
                self.repeats(false)?;
            }
        }
        Some(())
    }
}

/// Decodes a Pi picture if it was saved on `machine`.
pub(in crate::platform) fn decode_pi(data: &[u8], machine: Machine) -> Result<Image, DecodeError> {
    let header = Header::parse(data).ok_or(DecodeError::Unrecognized)?;
    let pixels = header.width * header.height;
    // Pairs are copied from the line above, so lines need at least 2 bytes.
    if header.machine() != machine || header.width < 2 || pixels == 0 || pixels > MAX_PIXELS {
        return Err(DecodeError::Unrecognized);
    }
    let colours = header.colours;
    let mut decoder = Decoder {
        bits: Bits {
            data: header.stream,
            pos: 0,
        },
        out: vec![0; pixels],
        pos: 0,
        width: header.width,
        colours,
        table: (0..colours * colours)
            .map(|i| ((colours + i / colours - i % colours) % colours) as u8)
            .collect(),
    };
    decoder.run().ok_or(DecodeError::Unrecognized)?;

    let precision = header.precision();
    let palette: Vec<u32> = header
        .palette
        .chunks_exact(3)
        .map(|c| precision.rgb(c[0], c[1], c[2]))
        .collect();
    let scale = 1 + header.doubles_height() as usize;
    let mut image = Image::new(header.width as u32, (header.height * scale) as u32);
    for y in 0..header.height * scale {
        for x in 0..header.width {
            let index = decoder.out[y / scale * header.width + x];
            image.set(x as u32, y as u32, palette[index as usize]);
        }
    }
    Ok(image)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delta_codes() {
        // 1x, 00x, 010xx, 011xxx for 16 colours.
        let data = [0b1100_1010, 0b1101_1111];
        let mut bits = Bits {
            data: &data,
            pos: 0,
        };
        assert_eq!(bits.delta(16), Some(1));
        assert_eq!(bits.delta(16), Some(3));
        assert_eq!(bits.delta(16), Some(7));
        assert_eq!(bits.delta(16), Some(15));
        assert_eq!(bits.delta(16), None);
    }

    #[test]
    fn length_codes() {
        let data = [0b0101_1101, 0];
        let mut bits = Bits {
            data: &data,
            pos: 0,
        };
        assert_eq!(bits.length(), Some(1));
        assert_eq!(bits.length(), Some(3));
        assert_eq!(bits.length(), Some(6));
    }

    #[test]
    fn rejects_truncated_stream() {
        let mut data = b"Pi\x1a\0\0\0\0\x04PC98\0\0\0\x04\0\x01".to_vec();
        data.extend([0; 48]);
        assert_eq!(
            decode_pi(&data, Machine::Pc98),
            Err(DecodeError::Unrecognized)
        );
        // Two deltas (colour 15, 15), a first repeat of 1 pair (shortened to 0),
        // then an end marker.
        data.extend([0b1110_0000, 0b0000_0000]);
        let image = decode_pi(&data, Machine::Pc98).unwrap();
        assert_eq!((image.width(), image.height()), (4, 1));
    }
}
