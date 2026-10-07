//! Apple IIGS Finder icon files (ProDOS type `$CA`, such as `Finder.Icons`),
//! shown as a sheet of their big and small icons.
//!
//! Sources:
//! - Apple II File Type Note `$CA`, "Finder Icons File"
//!   (<https://mirrors.apple2.org.za/ftp.gno.org/doc/apple/filetypes/ftn.ca.xxxx>):
//!   the 26-byte file header (zero, ID `$0001`, zero, a 16-byte name) and the
//!   icon data records (length word, 64-byte owner path, 16-byte name filter,
//!   type and aux filters, then the big and the small icon). An icon is the
//!   QuickDraw II Auxiliary Icon record: type, size, height, width, 4-bit
//!   image, same-size mask; CiderPress II's file format notes
//!   (<https://github.com/fadden/CiderPress2>, `FileConv/Gfx/GSFinderIcon-notes.md`)
//!   give the row length as `1 + (width - 1) / 2` bytes, the first pixel in the
//!   high nibble, and a mask nibble of 0 as transparent.
//! - The colors: Deark's `modules/misc2.c`, `de_run_apple2icons`
//!   (<https://github.com/jsummers/deark>, MIT license, notice below), which
//!   shows the icons through the standard 640-mode palette blended to solid
//!   colors, two entries of it a guess by its author. Deark is also the
//!   oracle for the samples.
//! - Checked on the 11 `apple2Icons` samples of the dexvert set: each icon
//!   file ends exactly at its zero length word, and each icon cropped from
//!   its cell of the sheet matches Deark's pixel for pixel.
//!
//! Choices of this crate: the sheet is `sheet.rs`'s grid (big and small icon of
//! each record in file order, each in a cell as large as the largest icon,
//! transparent pixels stay transparent); the `iconType` color flag is ignored,
//! as in CiderPress II, because most icons are colored but say black and white.
//! Files are recognized by their header alone, since the ProDOS file type is
//! not part of the name.

// Parts of this file follow Deark's modules/misc2.c
// (Deark, https://github.com/jsummers/deark):
//
// Copyright (C) 2016-2021 Jason Summers
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
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use alloc::vec::Vec;

use crate::bytes::{le16, le32};
use crate::image::{CLEAR, check_size};
use crate::sheet::{MAX_PICTURES, sheet};
use crate::{DecodeError, Image};

const FAIL: DecodeError = DecodeError::Unrecognized;
/// Header: link, ID, path and a name that is a length byte and 15 characters.
const HEADER_LEN: usize = 26;
/// Fixed fields of an icon data record, before the big icon.
const RECORD_LEN: usize = 86;
/// An icon's type, size, height and width words.
const ICON_HEADER_LEN: usize = 8;
const MAX_SIDE: usize = 256;

/// The standard 640-mode palette with each dithered pair blended, by index.
const PALETTE: [u32; 16] = [
    0x000000, 0x000080, 0x808000, 0x808080, 0x800000, 0x800080, 0xff8000, 0xff8080, 0x008000,
    0x008080, 0x80ff00, 0x80ff80, 0xc0c0c0, 0x8080ff, 0xffff80, 0xffffff,
];

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    // The link and path handles are zero on disk, the ID is 1.
    let header = le32(data, 0) == Some(0) && le16(data, 4) == Some(1) && le32(data, 6) == Some(0);
    if !header || data.get(10).is_none_or(|&name_len| name_len > 15) {
        return Err(FAIL);
    }
    let mut icons = Vec::new();
    let mut at = HEADER_LEN;
    loop {
        let length = usize::from(le16(data, at).ok_or(FAIL)?);
        if length == 0 {
            // The list ends with a zero length word.
            return sheet(&icons);
        }
        let record = data.get(at..at + length).ok_or(FAIL)?;
        let record = record.get(RECORD_LEN..).ok_or(FAIL)?;
        let (big, rest) = read_icon(record)?;
        icons.push(big);
        // The small icon may be missing from a record.
        if !rest.is_empty() {
            icons.push(read_icon(rest)?.0);
        }
        at += length;
        // `sheet` rejects more icons than it holds; this stops reading early.
        if icons.len() > MAX_PICTURES {
            return Err(FAIL);
        }
    }
}

/// The icon at the start of `data` and what follows it.
fn read_icon(data: &[u8]) -> Result<(Image, &[u8]), DecodeError> {
    let size = usize::from(le16(data, 2).ok_or(FAIL)?);
    let height = usize::from(le16(data, 4).ok_or(FAIL)?);
    let width = usize::from(le16(data, 6).ok_or(FAIL)?);
    let row_len = width.div_ceil(2);
    if width > MAX_SIDE || height > MAX_SIDE || row_len * height > size {
        return Err(FAIL);
    }
    check_size(width, height)?;
    let image = data
        .get(ICON_HEADER_LEN..ICON_HEADER_LEN + size)
        .ok_or(FAIL)?;
    let mask = data
        .get(ICON_HEADER_LEN + size..ICON_HEADER_LEN + 2 * size)
        .ok_or(FAIL)?;
    let nibble = |bytes: &[u8], x: usize, y: usize| {
        let byte = bytes[y * row_len + x / 2];
        if x.is_multiple_of(2) {
            byte >> 4
        } else {
            byte & 15
        }
    };
    let pixels = (0..width * height).map(|i| {
        let (x, y) = (i % width, i / width);
        match nibble(mask, x, y) {
            0 => CLEAR,
            _ => 0xff00_0000 | PALETTE[usize::from(nibble(image, x, y))],
        }
    });
    let rest = &data[ICON_HEADER_LEN + 2 * size..];
    Ok((Image::from_argb(width as u32, height as u32, pixels)?, rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An icon data record: 86 fixed bytes, a 2x1 big icon and a 1x1 small one.
    fn record(big_image: u8, big_mask: u8, with_small: bool) -> Vec<u8> {
        let mut record = alloc::vec![0; RECORD_LEN];
        let icon = |image: u8, mask: u8, width: u16| {
            let mut icon = alloc::vec![0, 0, 1, 0, 1, 0];
            icon.extend_from_slice(&width.to_le_bytes());
            icon.extend_from_slice(&[image, mask]);
            icon
        };
        record.extend_from_slice(&icon(big_image, big_mask, 2));
        if with_small {
            record.extend_from_slice(&icon(0xf0, 0xf0, 1));
        }
        let length = (record.len() as u16).to_le_bytes();
        record[..2].copy_from_slice(&length);
        record
    }

    fn file(records: &[Vec<u8>]) -> Vec<u8> {
        let mut data = alloc::vec![0; HEADER_LEN];
        data[4] = 1;
        for record in records {
            data.extend_from_slice(record);
        }
        data.extend_from_slice(&[0, 0]);
        data
    }

    #[test]
    fn icons_use_the_high_nibble_first_and_a_zero_mask_nibble_is_transparent() {
        // Big icon: pixels 0x1 (opaque) and 0x4 (masked out); small: 0xf.
        let image = decode(&file(&[record(0x14, 0xf0, true)])).unwrap();
        // Two cells of 2 x 1 on one row: big icon at (4, 4), small at (10, 4).
        assert_eq!((image.width(), image.height()), (16, 9));
        assert_eq!(
            (image.get_argb(4, 4), image.get_argb(5, 4)),
            (0xff00_0080, CLEAR)
        );
        assert_eq!(image.get(10, 4), 0xffffff);
    }

    #[test]
    fn a_record_without_a_small_icon_is_one_icon() {
        let with = decode(&file(&[record(0x11, 0xff, true)])).unwrap();
        let without = decode(&file(&[record(0x11, 0xff, false)])).unwrap();
        assert_eq!(with.get(10, 4), 0xffffff);
        assert_eq!((without.width(), without.height()), (10, 9));
    }

    #[test]
    fn headers_lists_and_icons_are_validated() {
        let good = file(&[record(0x11, 0xff, true)]);
        assert!(decode(&good).is_ok());
        let mut bad = good.clone();
        bad[4] = 2;
        assert!(decode(&bad).is_err());
        assert!(decode(&file(&[])).is_err());
        // The list must end inside the file.
        assert!(decode(&good[..good.len() - 2]).is_err());
        // An icon whose rows do not fit its size.
        let mut bad = good.clone();
        bad[HEADER_LEN + RECORD_LEN + 2] = 0;
        assert!(decode(&bad).is_err());
    }
}
