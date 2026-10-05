//! PrintMaster clip-art libraries (`.SHP`, Unison World, DOS), shown as a
//! sheet of their pictures.
//!
//! Sources:
//! - Layout: Deark's `printshop.c` (`printmaster`;
//!   <https://github.com/jsummers/deark>, MIT license, notice below) and the
//!   survey note `docs/research/gaps-computers-extra.md` C7. A file is
//!   records back to back: `0B`, the height, the width in pixels and one more
//!   byte, then `ceil(width / 8) * height` bytes of rows (most significant
//!   bit leftmost, a set bit black), then one more byte. The fourth header
//!   byte is 0 in every sample and the last byte is 0 or an ASCII letter
//!   (`I`, `s`, ...); neither is read. The `.SDR` file beside a library only
//!   holds 16-byte picture names and is not read.
//! - A file ends with its last record, or with zero fill or `1A` marks
//!   (`MONEY.SHP`, a school library): the samples are padded to a multiple of
//!   128 bytes. Records must run exactly up to that padding, which with the
//!   leading `0B` is what makes this a signature. [`super::clipart`] lays the
//!   sheet out.
//! - Checked on 31 files (`corpus/extra/dos-clipart/`: Sembiance
//!   `printMasterShape` and the PrintMaster disks on the textfiles CD,
//!   `swinnund/disk3/CLIPART/`). All pictures are 88x52, and each matches
//!   Deark's output pixel for pixel (see `dos-clipart.tsv`).

// Parts of this file follow Deark's modules/printshop.c
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

use super::clipart::{MAX_PICTURES, Sheet, black_on_white, is_padding};
use crate::{DecodeError, Image};

const RECORD_MARK: u8 = 0x0b;
const HEADER_LEN: usize = 4;
const TRAILER_LEN: usize = 1;

struct Record<'a> {
    width: usize,
    height: usize,
    rows: &'a [u8],
}

/// The records at the start of `rest`, up to the first bytes that are not
/// one; `rest` then holds those bytes.
struct Records<'a> {
    rest: &'a [u8],
}

impl<'a> Iterator for Records<'a> {
    type Item = Record<'a>;

    fn next(&mut self) -> Option<Record<'a>> {
        let [RECORD_MARK, height, width, _, body @ ..] = self.rest else {
            return None;
        };
        let (width, height) = (usize::from(*width), usize::from(*height));
        let rows_len = width.div_ceil(8) * height;
        let record_len = HEADER_LEN + rows_len + TRAILER_LEN;
        if width == 0 || height == 0 || self.rest.len() < record_len {
            return None;
        }
        let rows = &body[..rows_len];
        self.rest = &self.rest[record_len..];
        Some(Record {
            width,
            height,
            rows,
        })
    }
}

pub(super) fn decode_shp(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let mut records = Records { rest: data };
    let (mut count, mut cell_width, mut cell_height) = (0, 0, 0);
    for record in records.by_ref() {
        count += 1;
        if count > MAX_PICTURES {
            return Err(fail);
        }
        cell_width = cell_width.max(record.width);
        cell_height = cell_height.max(record.height);
    }
    if !is_padding(records.rest) {
        return Err(fail);
    }
    let mut sheet = Sheet::new(count, cell_width, cell_height)?;
    for (index, record) in (Records { rest: data }).enumerate() {
        sheet.put(
            index,
            &black_on_white(record.width, record.height, record.rows)?,
        );
    }
    Ok(sheet.into_image())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    /// A record of `width` x `height` pixels with its first pixel black.
    fn record(width: u8, height: u8) -> Vec<u8> {
        let rows_len = usize::from(width).div_ceil(8) * usize::from(height);
        let mut data = alloc::vec![RECORD_MARK, height, width, 0];
        data.push(0x80);
        data.resize(HEADER_LEN + rows_len, 0);
        data.push(0x73);
        data
    }

    #[test]
    fn records_of_different_sizes_share_a_cell_as_big_as_the_largest() {
        let mut data = record(9, 2);
        data.extend(record(3, 5));
        let sheet = decode_shp(&data).unwrap();
        // Two cells of 9 x 5 pixels.
        assert_eq!((sheet.width(), sheet.height()), (2 * 13 + 4, 9 + 4));
        assert_eq!(sheet.get(4, 4), 0);
        assert_eq!(sheet.get(5, 4), 0xffffff);
        assert_eq!(sheet.get(4 + 13, 4), 0);
        assert_eq!(sheet.get(4 + 13, 8), 0xffffff);
        // The first picture is only 2 rows tall.
        assert_eq!(sheet.get(4, 6), 0xc0c0c0);
    }

    #[test]
    fn only_padding_may_follow_the_last_record() {
        let mut data = record(8, 1);
        assert!(decode_shp(&data).is_ok());
        data.extend_from_slice(&[0, 0, 0x1a]);
        assert!(decode_shp(&data).is_ok());
        data.push(5);
        assert!(decode_shp(&data).is_err());
        // A truncated record is not padding.
        assert!(decode_shp(&record(8, 1)[..5]).is_err());
    }

    #[test]
    fn empty_pictures_and_other_marks_are_rejected() {
        assert!(decode_shp(&record(0, 4)).is_err());
        assert!(decode_shp(&record(4, 0)).is_err());
        let mut data = record(8, 1);
        data[0] = 0x0c;
        assert!(decode_shp(&data).is_err());
        assert!(decode_shp(&[]).is_err());
    }
}
