//! MIF pictures (MSX Interchange Format, MIF package by Louthrax).
//!
//! Source: reverse engineered from the 14 RECOIL sample files (no MIF
//! documentation exists; the MIF package sources were not read), checked
//! against `recoil2png` output and black-box probes of it:
//! - a mode byte: bits 0-3 select Screen 5, 6, 7, 8, 10/11, 12 (0-5), 2 (8)
//!   or 3 (9), bit 4 marks an interlaced picture (Screens 5-12, 424 lines);
//!   any other value is rejected. The next byte is ignored.
//! - Unless the mode is Screen 8 or 12, 16 palette entries in the V9938
//!   register format follow.
//! - Then an LZW stream (see [`unpack`]) of the VRAM bytes: 212 lines (424
//!   in display order when interlaced) for the bitmap modes, the first 0x3800 bytes of VRAM for Screen 2
//!   and the 0x600-byte pattern table for Screen 3 (shown with BASIC's name
//!   table). The stream must end in the file's last byte, with exactly that
//!   many bytes.

use alloc::vec::Vec;

use super::screen::{self, Bitmap, Tiled};
use super::vdp::{self, Palette, Vram};
use crate::{DecodeError, Image};

/// Most LZW dictionary entries before the encoder restarts it.
const MAX_ENTRIES: usize = 0x1000;

enum Mode {
    Bitmap { mode: Bitmap, interlaced: bool },
    Tiled(Tiled),
}

impl Mode {
    fn from_byte(byte: u8) -> Option<Self> {
        let bitmap = match byte & 0x0f {
            0 => Bitmap::Graphic4,
            1 => Bitmap::Graphic5,
            2 => Bitmap::Graphic6,
            3 => Bitmap::Graphic7,
            4 => Bitmap::Yae,
            5 => Bitmap::Yjk,
            8 if byte == 8 => return Some(Self::Tiled(Tiled::Graphic2)),
            9 if byte == 9 => return Some(Self::Tiled(Tiled::Multicolour)),
            _ => return None,
        };
        match byte & 0xf0 {
            0x00 | 0x10 => Some(Self::Bitmap {
                mode: bitmap,
                interlaced: byte & 0x10 != 0,
            }),
            _ => None,
        }
    }

    fn has_palette(&self) -> bool {
        !matches!(
            self,
            Self::Bitmap {
                mode: Bitmap::Graphic7 | Bitmap::Yjk,
                ..
            }
        )
    }

    fn unpacked_size(&self) -> usize {
        match self {
            Self::Bitmap { mode, interlaced } => {
                212 * mode.bytes_per_line() * if *interlaced { 2 } else { 1 }
            }
            Self::Tiled(Tiled::Multicolour) => 0x600,
            Self::Tiled(_) => 0x3800,
        }
    }
}

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    decode_inner(data).ok_or(DecodeError::Unrecognized)
}

fn decode_inner(data: &[u8]) -> Option<Image> {
    let mode = Mode::from_byte(*data.first()?)?;
    let (palette, packed) = if mode.has_palette() {
        let table = data.get(2..34)?;
        (Some(vdp::palette_table(table)), &data[34..])
    } else {
        (None, data.get(2..)?)
    };
    let size = mode.unpacked_size();
    let vram = unpack(packed, size)?;
    match mode {
        Mode::Bitmap { mode, interlaced } => {
            let palette: Palette = palette.unwrap_or_else(|| mode.default_palette());
            if !interlaced {
                return screen::render_bitmap(mode, &vram, None, &palette).ok();
            }
            // Interlaced pictures hold their 424 lines in display order.
            let lines = vram.chunks_exact(mode.bytes_per_line());
            let even: Vec<u8> = lines.clone().step_by(2).flatten().copied().collect();
            let odd: Vec<u8> = lines.skip(1).step_by(2).flatten().copied().collect();
            screen::render_bitmap(mode, &even, Some(&odd), &palette).ok()
        }
        Mode::Tiled(mode) => {
            let mut vram = Vram::new(&vram);
            if mode == Tiled::Multicolour {
                screen::set_basic_multicolour_names(&mut vram);
            }
            let palette = palette.unwrap_or(vdp::MSX1_PALETTE);
            Some(screen::render_tiled(mode, &vram, &palette, false).ok()?)
        }
    }
}

/// MSB-first bit reader.
struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
}

impl Bits<'_> {
    fn read(&mut self, count: u32) -> Option<usize> {
        let mut value = 0;
        for _ in 0..count {
            let byte = *self.data.get(self.pos / 8)?;
            value = value << 1 | (byte >> (7 - self.pos % 8) & 1) as usize;
            self.pos += 1;
        }
        Some(value)
    }
}

/// The LZW variant of MIF, unpacking to exactly `size` bytes. Each token is
/// a 1 bit and a literal byte, or a 0 bit and a code as wide as needed for
/// the number of dictionary entries plus 2. Codes below that number are
/// entries, the number itself is the previous string plus its first byte,
/// one more restarts the dictionary and the next ends the stream. Every
/// token after the first since a restart adds the previous string plus the
/// first byte of the current one.
fn unpack(data: &[u8], size: usize) -> Option<Vec<u8>> {
    let mut bits = Bits { data, pos: 0 };
    let mut out = Vec::with_capacity(size);
    // Entries as (start, length) in `out`.
    let mut entries: Vec<(usize, usize)> = Vec::new();
    let mut previous: Option<(usize, usize)> = None;
    loop {
        let start = out.len();
        let length = if bits.read(1)? == 1 {
            out.push(bits.read(8)? as u8);
            1
        } else {
            let count = entries.len();
            let code = bits.read(usize::BITS - (count + 2).leading_zeros())?;
            if code < count {
                let (from, length) = entries[code];
                out.extend_from_within(from..from + length);
                length
            } else if code == count {
                let (from, length) = previous?;
                out.extend_from_within(from..from + length);
                out.push(out[from]);
                length + 1
            } else if code == count + 1 {
                entries.clear();
                previous = None;
                continue;
            } else if code == count + 2 {
                // The end code must be in the last byte.
                let exact = out.len() == size && bits.pos.div_ceil(8) == data.len();
                return exact.then_some(out);
            } else {
                return None;
            }
        };
        if out.len() > size {
            return None;
        }
        if let Some((from, previous_length)) = previous {
            if entries.len() >= MAX_ENTRIES {
                return None;
            }
            entries.push((from, previous_length + 1));
        }
        previous = Some((start, length));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    /// Packs `(value, width)` pairs MSB first.
    fn pack(fields: &[(usize, u32)]) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut used = 0;
        for &(value, width) in fields {
            for bit in (0..width).rev() {
                if used % 8 == 0 {
                    bytes.push(0);
                }
                *bytes.last_mut().unwrap() |= ((value >> bit & 1) as u8) << (7 - used % 8);
                used += 1;
            }
        }
        bytes
    }

    #[test]
    fn lzw_literals_entries_and_end() {
        // Literal 0x12; code 0 with 0 entries (2 bits) repeats it plus its
        // first byte; code 0 (now 1 entry: 12 12) again; end code 2 + 2 = 4
        // (3 bits).
        let data = pack(&[
            (1, 1),
            (0x12, 8),
            (0, 1),
            (0, 2),
            (0, 1),
            (0, 2),
            (0, 1),
            (4, 3),
        ]);
        assert_eq!(unpack(&data, 5), Some(vec![0x12; 5]));
        assert_eq!(unpack(&data, 6), None);
    }

    #[test]
    fn lzw_restart_and_trailing_bytes() {
        // Literal, restart (code 0 + 1 in 2 bits), literal, end.
        let data = pack(&[
            (1, 1),
            (7, 8),
            (0, 1),
            (1, 2),
            (1, 1),
            (8, 8),
            (0, 1),
            (3, 2),
        ]);
        // After the restart the second literal adds no entry: end code is 2.
        assert_eq!(unpack(&data, 2), None);
        let data = pack(&[
            (1, 1),
            (7, 8),
            (0, 1),
            (1, 2),
            (1, 1),
            (8, 8),
            (0, 1),
            (2, 2),
        ]);
        assert_eq!(unpack(&data, 2), Some(vec![7, 8]));
        let mut padded = data.clone();
        padded.push(0);
        assert_eq!(unpack(&padded, 2), None);
    }

    #[test]
    fn mode_byte() {
        assert!(Mode::from_byte(0x15).is_some());
        assert!(Mode::from_byte(0x18).is_none());
        assert!(Mode::from_byte(0x06).is_none());
        assert!(Mode::from_byte(0x20).is_none());
    }
}
