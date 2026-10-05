//! FM Towns icon files (ICN): `ICNFILE`, `CRI-FJ2 ` and `CRI-FUJI`, shown as
//! a sheet of all their icons.
//!
//! Sources:
//! - Layouts (`ICNFILE`: 32-byte header with the table count at 12, 32-byte
//!   tables from offset 32 with icon count at +2 and icon-header offset at
//!   +8, 48-byte icon headers with width, height, colour count, size and
//!   pixel offset; `CRI-FJ2 ` little-endian and `CRI-FUJI` big-endian: 16-byte
//!   header with bits per pixel, width, height and icon count, then icons of a
//!   16-bit id and 32x32 4-bit pixels; 1-bit icons are rows of whole bytes
//!   with a set bit black, 4-bit rows are padded to 32 bits; the low nibble is
//!   the left pixel; the 16-colour palette): Deark's `modules/misc2.c`,
//!   `fmtowns_icn` (<https://github.com/jsummers/deark>, MIT licence, notice
//!   below). Signatures: Just Solve the Computer, ICN (FM Towns),
//!   <http://justsolve.archiveteam.org/wiki/ICN_(FM_Towns)>.
//! - The sheet is this crate's own choice, in `icon_sheet.rs`. No FM Towns
//!   icon sample was available, so the layouts are checked only by unit tests
//!   built from the documented structure.

// Parts of this file follow Deark's modules/misc2.c
// (Deark, https://github.com/jsummers/deark):
//
// Copyright (C) 2016-2026 Jason Summers
// <jason1@pobox.com>
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

use alloc::vec::Vec;

use crate::bytes::{be16, le16, le32};
use crate::icon_sheet::{self, Icon};
use crate::{DecodeError, Image};

const PALETTE: [u32; 16] = [
    0x222222, 0x2277bb, 0xcc6644, 0xffccaa, 0x999999, 0x00cc77, 0xcccccc, 0x777777, 0x000000,
    0x88bbee, 0xdd0000, 0x0000aa, 0x555555, 0x00ffff, 0xffdd00, 0xffffff,
];

/// Most icons shown, and the widest or tallest one accepted.
const MAX_ICONS: usize = 256;
/// Icon headers visited across all tables, skipped ones included.
const MAX_HEADERS: usize = 4096;
const MAX_SIDE: usize = 512;
/// Pixels of a `width` x `height` icon at `at`: 1-bit (set is black) or
/// 4-bit (low nibble first) rows, 4-bit rows padded to 32 bits.
fn read_icon(data: &[u8], at: usize, width: usize, height: usize, bits: usize) -> Option<Icon> {
    if width == 0 || height == 0 || width > MAX_SIDE || height > MAX_SIDE {
        return None;
    }
    let row_len = if bits == 1 {
        width.div_ceil(8)
    } else {
        (width * 4).div_ceil(32) * 4
    };
    let bytes = data.get(at..at.checked_add(row_len * height)?)?;
    let mut pixels = Vec::with_capacity(width * height);
    for row in bytes.chunks_exact(row_len) {
        for x in 0..width {
            pixels.push(if bits == 1 {
                if row[x / 8] & (0x80 >> (x % 8)) != 0 {
                    0x000000
                } else {
                    0xffffff
                }
            } else {
                let byte = row[x / 2];
                PALETTE[usize::from(if x % 2 == 0 { byte & 15 } else { byte >> 4 })]
            });
        }
    }
    Some(Icon {
        width,
        height,
        pixels,
    })
}

/// `CRI-FJ2 ` (little-endian) and `CRI-FUJI` (big-endian): 32x32 4-bit icons.
fn fixed_icons(data: &[u8], little: bool) -> Option<Vec<Icon>> {
    let word = |at| {
        if little {
            le16(data, at)
        } else {
            be16(data, at)
        }
    };
    let (bits, width, height, count) = (word(8)?, word(10)?, word(12)?, word(14)?);
    let size = 2 + 16 * 32;
    if (bits, width, height) != (4, 32, 32) || !(data.len() - 16).is_multiple_of(size) {
        return None;
    }
    (0..usize::from(count)
        .min((data.len() - 16) / size)
        .min(MAX_ICONS))
        .map(|i| read_icon(data, 16 + i * size + 2, 32, 32, 4))
        .collect()
}

/// `ICNFILE`: tables of icons, each icon with its own size and depth.
fn table_icons(data: &[u8]) -> Option<Vec<Icon>> {
    let mut icons = Vec::new();
    let mut headers = 0;
    for table in 0..usize::from(le16(data, 12)?) {
        let at = 32 + 32 * table;
        let count = usize::from(le16(data, at + 2)?);
        let first = le32(data, at + 8)? as usize;
        if first <= at {
            return None;
        }
        for i in 0..count {
            if icons.len() == MAX_ICONS || headers == MAX_HEADERS {
                return Some(icons);
            }
            headers += 1;
            let header = first.checked_add(48 * i)?;
            let width = usize::from(le16(data, header + 6)?);
            let height = usize::from(le16(data, header + 8)?);
            let colors = le32(data, header + 10)?;
            let pixels_at = le32(data, header + 20)? as usize;
            if pixels_at <= header {
                return None;
            }
            let bits = match colors {
                2 => 1,
                16 => 4,
                _ => continue,
            };
            icons.push(read_icon(data, pixels_at, width, height, bits)?);
        }
    }
    Some(icons)
}

pub(super) fn decode_icn(data: &[u8]) -> Result<Image, DecodeError> {
    let icons = if data.starts_with(b"ICNFILE\0\0\x1a") {
        table_icons(data)
    } else if data.starts_with(b"CRI-FJ2 ") {
        fixed_icons(data, true)
    } else if data.starts_with(b"CRI-FUJI") {
        fixed_icons(data, false)
    } else {
        None
    };
    match icons {
        Some(icons) if !icons.is_empty() => icon_sheet::sheet(&icons),
        _ => Err(DecodeError::Unrecognized),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn fixed(magic: &[u8; 8], little: bool, count: u16) -> Vec<u8> {
        let word = |v: u16| {
            if little {
                v.to_le_bytes()
            } else {
                v.to_be_bytes()
            }
        };
        let mut data = magic.to_vec();
        for v in [4, 32, 32, count] {
            data.extend_from_slice(&word(v));
        }
        for _ in 0..count {
            data.extend_from_slice(&[0, 0]);
            data.extend_from_slice(&[0x21; 512]);
        }
        data
    }

    #[test]
    fn fixed_icons_low_nibble_is_left_pixel() {
        for (magic, little) in [(b"CRI-FJ2 ", true), (b"CRI-FUJI", false)] {
            let image = decode_icn(&fixed(magic, little, 2)).unwrap();
            // Left pixel = palette[1], right = palette[2]; icon at (4, 4).
            let at = (4 * image.width() as usize + 4) * 3;
            assert_eq!(&image.rgb()[at..at + 3], &[0x22, 0x77, 0xbb]);
            assert_eq!(&image.rgb()[at + 3..at + 6], &[0xcc, 0x66, 0x44]);
        }
    }

    #[test]
    fn rejects_truncated_and_odd_sizes() {
        let mut data = fixed(b"CRI-FJ2 ", true, 1);
        data.pop();
        assert!(decode_icn(&data).is_err());
        assert!(decode_icn(b"ICNFILE\0\0\x1a").is_err());
    }

    #[test]
    fn icnfile_mono_icon() {
        let mut data = vec![0u8; 32 + 32 + 48 + 8];
        data[..10].copy_from_slice(b"ICNFILE\0\0\x1a");
        data[12] = 1; // one table
        data[32 + 2] = 1; // one icon
        data[32 + 8] = 64; // icon headers at 64
        let h = 64;
        data[h + 6] = 8; // width
        data[h + 8] = 8; // height
        data[h + 10] = 2; // colours
        data[h + 14] = 8; // size
        data[h + 20] = 112; // pixels at 112
        data[112] = 0x80;
        let image = decode_icn(&data).unwrap();
        let at = (4 * image.width() as usize + 4) * 3;
        assert_eq!(&image.rgb()[at..at + 3], &[0, 0, 0]);
        assert_eq!(&image.rgb()[at + 3..at + 6], &[255, 255, 255]);
    }
}
