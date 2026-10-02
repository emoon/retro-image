//! Multi Palette Picture (`MPP`) by Zerkman / Sector One.
//!
//! Sources:
//! - <http://zerkman.sector1.fr/index.php?post/2012/10/08/Atari-ST-Multipalette-Picture-file-format>
//! - Reference converter `mpp2bmp.c` (WTFPL, a permissive licence):
//!   <https://codeberg.org/zerkman/mpp> - header layout, the four display
//!   modes and their palette change positions.
//! - Files in the wild hold 199 (273 in overscan) lines, not the 200 (276)
//!   of the current converter: derived from sample file sizes.
//! - Observed from `recoil2png` output: components are scaled by bit
//!   replication; double frames are averaged.

use alloc::vec::Vec;

use super::common::be32;
use crate::{DecodeError, Image};

struct Mode {
    /// Palette entries per line, including fixed ones.
    colors: usize,
    /// Entries shared by all lines (not stored per line).
    fixed: usize,
    /// Whether entries 0 and `(colors - 1) & !15` are forced to black.
    border0: bool,
    /// X position of the first palette change.
    x0: usize,
    /// Distance to the next palette change after changing entry `c`.
    step: fn(usize) -> usize,
    width: usize,
    height: usize,
}

const MODES: [Mode; 4] = [
    Mode {
        colors: 54,
        fixed: 0,
        border0: true,
        x0: 33,
        step: |c| match c {
            15 => 88,
            31 => 12,
            37 => 100,
            _ => 4,
        },
        width: 320,
        height: 199,
    },
    Mode {
        colors: 48,
        fixed: 0,
        border0: true,
        x0: 9,
        step: |c| if c & 1 != 0 { 16 } else { 4 },
        width: 320,
        height: 199,
    },
    Mode {
        colors: 56,
        fixed: 0,
        border0: true,
        x0: 5,
        step: |_| 8,
        width: 320,
        height: 199,
    },
    Mode {
        colors: 54,
        fixed: 6,
        border0: false,
        x0: 69,
        step: |c| match c {
            15 => 112,
            31 => 12,
            37 => 100,
            _ => 4,
        },
        width: 416,
        height: 273,
    },
];

pub(super) fn decode_mpp(data: &[u8]) -> Result<Image, DecodeError> {
    decode(data).ok_or(DecodeError::Unrecognized)
}

fn decode(data: &[u8]) -> Option<Image> {
    if data.get(..3)? != b"MPP" {
        return None;
    }
    let mode = MODES.get(usize::from(*data.get(3)?))?;
    let flags = *data.get(4)?;
    let bits = 3 * (3 + usize::from(flags & 1) + usize::from(flags >> 1 & 1));
    let skip = be32(data, 8)? as usize;
    let mut pos = 12usize.checked_add(skip)?;
    let first = frame(data, &mut pos, mode, bits)?;
    if flags & 4 == 0 {
        return Some(first);
    }
    let second = frame(data, &mut pos, mode, bits)?;
    Some(Image::blend(&[&first, &second]))
}

/// Converts a packed palette entry of `bits` bits to `0xRRGGBB`.
fn color(c: u32, bits: usize) -> u32 {
    let (r, g, b) = match bits {
        9 => (c >> 6 & 7, c >> 3 & 7, c & 7),
        _ => {
            // STE order: each nibble holds its LSB above the three MSBs.
            let ste = |n: u32| (n & 7) << 1 | n >> 3 & 1;
            let (r, g, b) = (ste(c >> 8 & 0xf), ste(c >> 4 & 0xf), ste(c & 0xf));
            if bits == 15 {
                (
                    r << 1 | c >> 14 & 1,
                    g << 1 | c >> 13 & 1,
                    b << 1 | c >> 12 & 1,
                )
            } else {
                (r, g, b)
            }
        }
    };
    let depth = bits as u32 / 3;
    let scale = |v: u32| {
        let mut out = 0;
        let mut shift = 8i32 - depth as i32;
        while shift > -(depth as i32) {
            out |= if shift >= 0 { v << shift } else { v >> -shift };
            shift -= depth as i32;
        }
        out & 0xff
    };
    scale(r) << 16 | scale(g) << 8 | scale(b)
}

/// Decodes one frame (palettes then bitmap) starting at `pos`.
fn frame(data: &[u8], pos: &mut usize, mode: &Mode, bits: usize) -> Option<Image> {
    let per_line = mode.colors - mode.fixed;
    let stored = per_line - if mode.border0 { 2 } else { 0 };
    let packed_len = (bits * stored * mode.height)
        .div_ceil(8)
        .next_multiple_of(2);
    let packed = data.get(*pos..*pos + packed_len)?;
    *pos += packed_len;
    let bitmap_len = mode.width / 2 * mode.height;
    let bitmap = data.get(*pos..*pos + bitmap_len)?;
    *pos += bitmap_len;

    let mut reader = Bits {
        data: packed,
        pos: 0,
    };
    let border = (mode.colors - 1) & !15;
    let mut palettes = Vec::with_capacity(per_line * mode.height);
    for i in 0..per_line * mode.height {
        let x = i % mode.colors;
        if mode.border0 && (x == 0 || x == border) {
            palettes.push(0);
        } else {
            palettes.push(color(reader.read(bits)?, bits));
        }
    }

    let mut image = Image::new(mode.width as u32, mode.height as u32);
    let mut palette = [0u32; 16];
    let line_len = mode.width / 2;
    for (y, line) in bitmap.chunks_exact(line_len).enumerate() {
        let mut entries = palettes[y * per_line..(y + 1) * per_line].iter();
        for slot in &mut palette[mode.fixed..] {
            *slot = *entries.next()?;
        }
        let mut next_x = mode.x0;
        let mut next_c = 0;
        for x in 0..mode.width {
            if x == next_x {
                palette[next_c & 15] = *entries.next()?;
                next_x += (mode.step)(next_c);
                next_c += 1;
            }
            let index = super::common::interleaved_index(line, x as u32, 4);
            image.set(x as u32, y as u32, palette[index]);
        }
    }
    Some(image)
}

struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
}

impl Bits<'_> {
    fn read(&mut self, count: usize) -> Option<u32> {
        let mut value = 0;
        for _ in 0..count {
            let bit = self.data.get(self.pos / 8)? >> (7 - self.pos % 8) & 1;
            value = value << 1 | u32::from(bit);
            self.pos += 1;
        }
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_depths() {
        assert_eq!(color(0o777, 9), 0xffffff);
        assert_eq!(color(0x0f00, 12), 0xff0000);
        assert_eq!(color(0x0800, 12), 0x110000);
        assert_eq!(color(0x0800, 15), 0x100000);
        assert_eq!(color(0x4000, 15), 0x080000);
        assert_eq!(color(0x7fff, 15), 0xffffff);
    }

    #[test]
    fn rejects_short_file() {
        assert_eq!(
            decode_mpp(b"MPP\x01\x00\0\0\0\0\0\0\0"),
            Err(DecodeError::Unrecognized)
        );
    }
}
