//! OS/2 icons and pointers: `IC` and `PT` (two-color), `CI` and `CP` (color),
//! alone or inside a `BA` bitmap array.
//!
//! Sources:
//! - Deark `os2bmp.c` (<https://github.com/jsummers/deark>, MIT license): the
//!   layout and the meaning of the masks, also used as the oracle.
//! - Just Solve the File Format Problem, "OS/2 Icon", "OS/2 Pointer" and "OS/2
//!   Bitmap Array": <http://justsolve.archiveteam.org/wiki/OS/2_Icon> (CC0;
//!   the markers, the BA container, and the OS/2 Presentation Manager
//!   Programming Reference as the specification).
//! - Reverse engineered from 22 sample files (Sembiance's `icoOS2` and
//!   `os2Pointer` folders) and 15 icons from the Hobbes OS/2 archive CD
//!   (<http://cd.textfiles.com/hobbesos29804/disk1/ICONS/>).
//!
//! A bitmap record is a 14-byte file header (marker, size, hotspot, and the
//! offset of the pixels from the start of the file), a bitmap header (12-byte
//! OS/2 1.x core header or a 40 or 64-byte one) and the palette (3-byte
//! entries after a core header, 4-byte ones after the others). Rows are
//! padded to 4 bytes and stored bottom-up. `IC` and `PT` hold one two-color
//! bitmap of double height; `CI` and `CP` hold that bitmap as the mask, then
//! the color bitmap. In a mask the upper half is the AND mask and the lower
//! half the XOR mask. A pixel with AND clear shows its color (black or white
//! for `IC` and `PT`: XOR clear is black); with AND set, XOR clear leaves the
//! screen alone (transparent) and XOR set inverts it.
//!
//! Transparent pixels keep alpha 0. An inverting pixel inverts whatever is
//! behind it, so it has no color of its own; it is drawn as the fixed color
//! `3f3f3f`, the inverse of the gray `c0c0c0`. A `BA` array shows its largest
//! picture (the first of equal size); an array entry that is a plain OS/2
//! bitmap (`BM`) is not read. Hotspots are ignored.
//!
//! Verification: no RECOIL oracle for this format; output matches Deark's PNG
//! output (its alpha composited onto the gray `c0c0c0`) on the sample files.

// Parts of this file follow Deark's modules/os2bmp.c
// (Deark, https://github.com/jsummers/deark):
//
// Copyright (C) 2016 Jason Summers
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

use crate::bytes::{le16, le32};
use crate::image::{CLEAR, check_size};
use crate::{DecodeError, Image};

const FAIL: DecodeError = DecodeError::Unrecognized;
const FILE_HEADER_LEN: usize = 14;
/// `BA` entries followed in one array: far more than any icon file holds.
const MAX_ENTRIES: usize = 64;
const BLACK: u32 = 0x000000;
const WHITE: u32 = 0xffffff;
const INVERTED_FILL: u32 = 0x3f_3f3f;

/// One bitmap of a record: its geometry, palette and pixel rows.
struct Bitmap<'a> {
    width: usize,
    height: usize,
    bits_per_pixel: usize,
    palette: Vec<u32>,
    rows: &'a [u8],
    row_len: usize,
    /// Where the next record starts, after this one's headers and palette.
    end: usize,
}

impl Bitmap<'_> {
    /// Pixel `x` of row `row` of the file (the bottom row first): a palette
    /// index, or the color itself for 24-bit pixels.
    fn value(&self, x: usize, row: usize) -> u32 {
        let line = &self.rows[row * self.row_len..][..self.row_len];
        match self.bits_per_pixel {
            24 => u32::from_be_bytes([0, line[x * 3 + 2], line[x * 3 + 1], line[x * 3]]),
            bpp => {
                let shift = 8 - bpp - (x * bpp) % 8;
                u32::from(line[x * bpp / 8] >> shift) & ((1 << bpp) - 1)
            }
        }
    }

    /// The color of a pixel.
    fn color(&self, x: usize, row: usize) -> u32 {
        let value = self.value(x, row);
        match self.bits_per_pixel {
            24 => value,
            _ => self.palette.get(value as usize).copied().unwrap_or(BLACK),
        }
    }
}

/// Parses the record at `at`, which must start with `marker`.
fn parse_bitmap<'a>(data: &'a [u8], at: usize, marker: [u8; 2]) -> Result<Bitmap<'a>, DecodeError> {
    if !data.get(at..).is_some_and(|rest| rest.starts_with(&marker)) {
        return Err(FAIL);
    }
    let pixels_at = le32(data, at + 10).ok_or(FAIL)? as usize;
    let info = at + FILE_HEADER_LEN;
    let header_len = le32(data, info).ok_or(FAIL)? as usize;
    let word = |offset: usize| le16(data, info + offset).map(usize::from).ok_or(FAIL);
    let long = |offset: usize| le32(data, info + offset).map(|v| v as usize).ok_or(FAIL);
    let (width, height, planes, bits_per_pixel, entry_len, colors_used) = match header_len {
        12 => (word(4)?, word(6)?, word(8)?, word(10)?, 3, 0),
        40 | 64 => {
            // Uncompressed only.
            if long(16)? != 0 {
                return Err(FAIL);
            }
            (long(4)?, long(8)?, word(12)?, word(14)?, 4, long(32)?)
        }
        _ => return Err(FAIL),
    };
    if planes != 1 || !matches!(bits_per_pixel, 1 | 4 | 8 | 24) {
        return Err(FAIL);
    }
    check_size(width, height)?;
    let palette_len = match (bits_per_pixel, colors_used) {
        (24, _) => 0,
        (bpp, 0) => 1 << bpp,
        (bpp, used) => used.min(1 << bpp),
    };
    let palette_at = info + header_len;
    let end = palette_at + palette_len * entry_len;
    let palette = data
        .get(palette_at..end)
        .ok_or(FAIL)?
        .chunks_exact(entry_len)
        .map(|e| u32::from_be_bytes([0, e[2], e[1], e[0]]))
        .collect();
    let row_len = (width * bits_per_pixel).div_ceil(32) * 4;
    let rows = pixels_at
        .checked_add(row_len * height)
        .and_then(|end| data.get(pixels_at..end))
        .ok_or(FAIL)?;
    Ok(Bitmap {
        width,
        height,
        bits_per_pixel,
        palette,
        rows,
        row_len,
        end,
    })
}

/// A parsed `IC`, `CI`, `PT` or `CP` record: the mask and, for the color
/// kinds, the color bitmap. Nothing is drawn until [`Record::render`].
struct Record<'a> {
    mask: Bitmap<'a>,
    picture: Option<Bitmap<'a>>,
}

impl<'a> Record<'a> {
    /// Parses the record starting at `at`.
    fn parse(data: &'a [u8], at: usize) -> Result<Self, DecodeError> {
        let (marker, colored) = match data.get(at..at + 2) {
            Some(b"IC") => (*b"IC", false),
            Some(b"PT") => (*b"PT", false),
            Some(b"CI") => (*b"CI", true),
            Some(b"CP") => (*b"CP", true),
            _ => return Err(FAIL),
        };
        let mask = parse_bitmap(data, at, marker)?;
        if mask.bits_per_pixel != 1 || mask.height % 2 != 0 {
            return Err(FAIL);
        }
        let picture = if colored {
            let picture = parse_bitmap(data, mask.end, marker)?;
            if (picture.width, picture.height) != (mask.width, mask.height / 2) {
                return Err(FAIL);
            }
            Some(picture)
        } else {
            None
        };
        Ok(Self { mask, picture })
    }

    fn width(&self) -> usize {
        self.mask.width
    }

    /// The mask holds the AND and the XOR half, one above the other.
    fn height(&self) -> usize {
        self.mask.height / 2
    }

    fn render(&self) -> Result<Image, DecodeError> {
        let (mask, picture) = (&self.mask, &self.picture);
        let (width, height) = (self.width(), self.height());
        let color = (0..height).flat_map(|y| (0..width).map(move |x| (x, y)));
        Image::from_argb(
            width as u32,
            height as u32,
            color.map(|(x, y)| {
                // Rows are stored bottom-up; the AND mask is the upper half
                // of the mask picture, so it comes last in the file.
                let and = mask.value(x, 2 * height - 1 - y) != 0;
                let xor = mask.value(x, height - 1 - y) != 0;
                let opaque = |color: u32| 0xff00_0000 | color;
                match (and, xor, picture) {
                    (false, _, Some(picture)) => opaque(picture.color(x, height - 1 - y)),
                    (false, xor, None) => opaque([BLACK, WHITE][usize::from(xor)]),
                    (true, false, _) => CLEAR,
                    (true, true, _) => opaque(INVERTED_FILL),
                }
            }),
        )
    }
}

/// The largest record of an array (the first of equal size). Only headers
/// are read, so the entries that lose are never drawn.
fn largest_in_array(data: &[u8]) -> Result<Record<'_>, DecodeError> {
    // A chain of array headers, each followed by one record; the offset of
    // the next header is absolute and zero in the last one.
    let mut best: Option<Record> = None;
    let mut at = 0;
    for _ in 0..MAX_ENTRIES {
        // A chain that runs off into something else ends the array.
        if !data.get(at..).is_some_and(|rest| rest.starts_with(b"BA")) {
            break;
        }
        if let Ok(record) = Record::parse(data, at + 14) {
            let area = |r: &Record| r.width() * r.height();
            if best.as_ref().is_none_or(|b| area(&record) > area(b)) {
                best = Some(record);
            }
        }
        let next = le32(data, at + 6).ok_or(FAIL)? as usize;
        if next <= at {
            break;
        }
        at = next;
    }
    best.ok_or(FAIL)
}

pub(super) fn decode_os2_icon(data: &[u8]) -> Result<Image, DecodeError> {
    let record = if data.starts_with(b"BA") {
        largest_in_array(data)?
    } else {
        Record::parse(data, 0)?
    };
    record.render()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 2x1 `IC` record: the mask is 2x2 bits in 4-byte rows, XOR row first.
    fn icon(xor: u8, and: u8) -> Vec<u8> {
        let mut file = b"IC".to_vec();
        file.extend_from_slice(&[0; 8]);
        file.extend_from_slice(&32u32.to_le_bytes()); // pixels at 32
        file.extend_from_slice(&12u32.to_le_bytes());
        file.extend_from_slice(&[2, 0, 2, 0, 1, 0, 1, 0]);
        file.extend_from_slice(&[0, 0, 0, 255, 255, 255]);
        file.extend_from_slice(&[xor, 0, 0, 0, and, 0, 0, 0]);
        file
    }

    #[test]
    fn mask_halves_choose_black_white_transparent_and_inverse() {
        // Left pixel: AND 0, XOR 1 is white. Right: AND 1, XOR 0 is transparent.
        let image = decode_os2_icon(&icon(0b1000_0000, 0b0100_0000)).unwrap();
        assert_eq!((image.width(), image.height()), (2, 1));
        assert_eq!(
            [image.get_argb(0, 0), image.get_argb(1, 0)],
            [0xff00_0000 | WHITE, CLEAR]
        );
        let inverted = decode_os2_icon(&icon(0b0100_0000, 0b0100_0000)).unwrap();
        assert_eq!(inverted.get(1, 0), INVERTED_FILL);
        assert_eq!(inverted.get(0, 0), BLACK);
    }

    #[test]
    fn an_array_wraps_a_record_and_a_broken_chain_ends_it() {
        let mut array = b"BA".to_vec();
        array.extend_from_slice(&40u32.to_le_bytes());
        array.extend_from_slice(&[0; 8]);
        // Pixel offsets are absolute: the record moves by 14 bytes.
        let mut record = icon(0, 0);
        record[10..14].copy_from_slice(&(32u32 + 14).to_le_bytes());
        array.extend_from_slice(&record);
        assert_eq!(decode_os2_icon(&array).unwrap().width(), 2);
        // The next array header points at something else: the first record stays.
        array[6..10].copy_from_slice(&20u32.to_le_bytes());
        assert_eq!(decode_os2_icon(&array).unwrap().width(), 2);
        // A first record that is no icon leaves nothing to show.
        array[14] = b'X';
        assert!(decode_os2_icon(&array).is_err());
    }

    /// An `IC` record header of `width` x `height` pixels (a mask of twice
    /// the height) whose mask rows start at the file offset `pixels_at`.
    fn ic_header(width: u16, height: u16, pixels_at: usize) -> Vec<u8> {
        let mut record = b"IC".to_vec();
        record.extend_from_slice(&[0; 8]);
        record.extend_from_slice(&(pixels_at as u32).to_le_bytes());
        record.extend_from_slice(&12u32.to_le_bytes());
        record.extend_from_slice(&width.to_le_bytes());
        record.extend_from_slice(&(2 * height).to_le_bytes());
        record.extend_from_slice(&[1, 0, 1, 0]);
        record.extend_from_slice(&[0, 0, 0, 255, 255, 255]);
        record
    }

    #[test]
    fn an_array_of_many_entries_shows_the_first_largest() {
        // 64 entries of 32x32, two of them 96x64 (the first of those wins),
        // all sharing one mask area big enough for the largest.
        let entry_len = 14 + 32;
        let pixels_at = 64 * entry_len;
        let mut array = Vec::new();
        for i in 0..64usize {
            let (width, height) = if i == 40 || i == 50 {
                (96, 64)
            } else {
                (32, 32)
            };
            array.extend_from_slice(b"BA");
            array.extend_from_slice(&40u32.to_le_bytes());
            let next = if i == 63 { 0 } else { (i + 1) * entry_len };
            array.extend_from_slice(&(next as u32).to_le_bytes());
            array.extend_from_slice(&[0; 4]);
            array.extend_from_slice(&ic_header(width, height, pixels_at));
        }
        array.resize(pixels_at + 12 * 2 * 64, 0);
        let image = decode_os2_icon(&array).unwrap();
        assert_eq!((image.width(), image.height()), (96, 64));
    }

    /// A 2x1 `CI` record: the mask, then a 4-bit color bitmap whose palette
    /// entry `i` is gray `i`.
    fn color_icon(xor: u8, and: u8, pixels: u8) -> Vec<u8> {
        let mut file = b"CI".to_vec();
        file.extend_from_slice(&[0; 8]);
        file.extend_from_slice(&106u32.to_le_bytes()); // the mask rows
        file.extend_from_slice(&12u32.to_le_bytes());
        file.extend_from_slice(&[2, 0, 2, 0, 1, 0, 1, 0]);
        file.extend_from_slice(&[0, 0, 0, 255, 255, 255]);
        file.extend_from_slice(b"CI");
        file.extend_from_slice(&[0; 8]);
        file.extend_from_slice(&114u32.to_le_bytes()); // the color rows
        file.extend_from_slice(&12u32.to_le_bytes());
        file.extend_from_slice(&[2, 0, 1, 0, 1, 0, 4, 0]);
        (0..16u8).for_each(|gray| file.extend_from_slice(&[gray; 3]));
        file.extend_from_slice(&[xor, 0, 0, 0, and, 0, 0, 0]);
        file.extend_from_slice(&[pixels, 0, 0, 0]);
        file
    }

    #[test]
    fn color_records_take_pixels_from_the_second_bitmap_where_and_is_clear() {
        // Colors 2 and 1; the right pixel is transparent (AND set, XOR clear).
        let image = decode_os2_icon(&color_icon(0, 0b0100_0000, 0x21)).unwrap();
        assert_eq!((image.width(), image.height()), (2, 1));
        assert_eq!(image.get(0, 0), 0x020202);
        assert_eq!(image.get_argb(1, 0), CLEAR);
        // A bitmap of another size than the mask is no icon.
        let mut wrong = color_icon(0, 0, 0x21);
        wrong[32 + 14 + 4] = 3;
        assert!(decode_os2_icon(&wrong).is_err());
    }

    #[test]
    fn truncated_records_are_rejected() {
        let record = icon(0, 0);
        assert!(decode_os2_icon(&record[..record.len() - 1]).is_err());
        assert!(decode_os2_icon(b"IC").is_err());
    }
}
