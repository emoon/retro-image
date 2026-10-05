//! PrintPartner clip-art libraries (`.GPH`, Acropolis Software, DOS), shown
//! as a sheet of their pictures.
//!
//! Sources:
//! - Layout: Deark's `printptnr.c` (`pp_gph`;
//!   <https://github.com/jsummers/deark>, MIT license, notice below), whose
//!   notes say the compression schemes were worked out by trial, and the
//!   survey note `docs/research/gaps-computers-extra.md` C7. A file is
//!   a text header (`PrintPartner Art v1.0`, a copyright line, `1A`), then
//!   records of a name length, a 20-byte name, a compression type, the
//!   height, the width in bytes, and the data. The width in pixels is eight
//!   times the width in bytes. A set bit is black, most significant bit
//!   leftmost. Types:
//!   1. rows as they are;
//!   2. a byte-wise run-length coding after a little-endian word with the
//!      coded length: a byte with bit 7 set repeats the next byte (low 7 bits
//!      times), one with bit 7 clear is followed by that many literal bytes;
//!   3. a pixel-wise run-length coding after a little-endian word with the
//!      number of nibbles: each nibble is a run of 0 to 7 pixels, black if
//!      bit 3 is set, white otherwise; the nibbles are packed high first.
//!
//!   Deark does not know what a run of 0 means; the samples have none, so it
//!   draws nothing.
//! - Types 2 and 3 stop short of the full picture in the samples (the white
//!   end of the last rows is left out), so the missing part is white. All
//!   10 samples (Sembiance `printPartnerGraphics`) parse to exactly the end
//!   of the file, use all three types, and have pictures from 88x52 up to
//!   304x195; their pictures match Deark's pixel for pixel (see
//!   `dos-clipart.tsv`). The pixels are about twice as tall as wide in
//!   Deark's notes; the picture is not stretched.
//! - The header text makes this a signature, as do the records, which must
//!   run exactly up to the end of the file or to zero fill and `1A` marks.
//!   [`crate::sheet`] lays the sheet out.

// Parts of this file follow Deark's modules/printptnr.c
// (Deark, https://github.com/jsummers/deark):
//
// Copyright (C) 2017 Jason Summers
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

use super::clipart::{black_on_white, is_padding};
use crate::bytes::le16;
use crate::sheet::Sheet;
use crate::{DecodeError, Image};

const FAIL: DecodeError = DecodeError::Unrecognized;
const MAGIC: &[u8] = b"PrintPartner";
const END_OF_HEADER: u8 = 0x1a;
/// Where the header's `1A` must be at the latest (Deark reads 255 bytes).
const HEADER_SEARCH_LEN: usize = 255;
const NAME_LEN: usize = 20;

enum Packing<'a> {
    Raw(&'a [u8]),
    Bytes(&'a [u8]),
    Nibbles { data: &'a [u8], count: usize },
}

struct Record<'a> {
    /// Width in bytes.
    row_len: usize,
    height: usize,
    packing: Packing<'a>,
}

impl Record<'_> {
    fn width(&self) -> usize {
        self.row_len * 8
    }

    /// The rows: `row_len * height` bytes, a set bit black.
    fn rows(&self) -> Vec<u8> {
        let len = self.row_len * self.height;
        match self.packing {
            Packing::Raw(rows) => rows.to_vec(),
            Packing::Bytes(coded) => unpack_bytes(coded, len),
            Packing::Nibbles { data, count } => unpack_nibbles(data, count, len),
        }
    }
}

/// Runs of repeated bytes and literal bytes. Missing data is white (0),
/// data beyond `len` is dropped. A literal that runs past the coded data is
/// cut off there.
fn unpack_bytes(coded: &[u8], len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(len.min(coded.len().saturating_mul(127)));
    let mut rest = coded;
    while let [control, tail @ ..] = rest {
        let count = usize::from(control & 0x7f);
        if control & 0x80 != 0 {
            let [value, tail @ ..] = tail else { break };
            out.resize(out.len() + count, *value);
            rest = tail;
        } else {
            let (literal, tail) = tail.split_at(count.min(tail.len()));
            out.extend_from_slice(literal);
            rest = tail;
        }
        if out.len() >= len {
            break;
        }
    }
    out.resize(len, 0);
    out
}

/// Runs of black and white pixels, `count` nibbles of `data`, high nibble
/// first, over an all-white picture of `len` bytes.
fn unpack_nibbles(data: &[u8], count: usize, len: usize) -> Vec<u8> {
    let mut out = alloc::vec![0u8; len];
    let mut pixel = 0;
    for index in 0..count {
        let Some(&byte) = data.get(index / 2) else {
            break;
        };
        let nibble = if index % 2 == 0 { byte >> 4 } else { byte & 15 };
        let run = usize::from(nibble & 7);
        if nibble & 8 != 0 {
            for at in pixel..pixel + run {
                if let Some(bits) = out.get_mut(at / 8) {
                    *bits |= 0x80 >> (at % 8);
                }
            }
        }
        pixel += run;
    }
    out
}

/// The records at the start of `rest`, up to the first bytes that are not
/// one; `rest` then holds those bytes.
struct Records<'a> {
    rest: &'a [u8],
}

impl<'a> Iterator for Records<'a> {
    type Item = Record<'a>;

    fn next(&mut self) -> Option<Record<'a>> {
        let [name_len, rest @ ..] = self.rest else {
            return None;
        };
        let (fields, body) = rest.split_at_checked(NAME_LEN + 3)?;
        let [.., packing, height, row_len] = fields else {
            return None;
        };
        let (row_len, height) = (usize::from(*row_len), usize::from(*height));
        if usize::from(*name_len) > NAME_LEN || row_len == 0 || height == 0 {
            return None;
        }
        let (packing, coded_len) = match packing {
            1 => {
                let len = row_len * height;
                (Packing::Raw(body.get(..len)?), len)
            }
            2 => {
                let len = usize::from(le16(body, 0)?);
                (Packing::Bytes(body.get(2..2 + len)?), 2 + len)
            }
            3 => {
                let count = usize::from(le16(body, 0)?);
                let len = count.div_ceil(2);
                let data = body.get(2..2 + len)?;
                (Packing::Nibbles { data, count }, 2 + len)
            }
            _ => return None,
        };
        self.rest = &body[coded_len..];
        Some(Record {
            row_len,
            height,
            packing,
        })
    }
}

pub(super) fn decode_gph(data: &[u8]) -> Result<Image, DecodeError> {
    if !data.starts_with(MAGIC) {
        return Err(FAIL);
    }
    let header_end = data
        .iter()
        .take(HEADER_SEARCH_LEN)
        .position(|&b| b == END_OF_HEADER)
        .ok_or(FAIL)?;
    let body = &data[header_end + 1..];
    let mut records = Records { rest: body };
    let (mut count, mut cell_width, mut cell_height) = (0, 0, 0);
    for record in records.by_ref() {
        count += 1;
        cell_width = cell_width.max(record.width());
        cell_height = cell_height.max(record.height);
    }
    if !is_padding(records.rest) {
        return Err(FAIL);
    }
    let mut sheet = Sheet::new(count, cell_width, cell_height)?;
    for (index, record) in (Records { rest: body }).enumerate() {
        let picture = black_on_white(record.width(), record.height, &record.rows())?;
        sheet.put(index, &picture);
    }
    Ok(sheet.into_image())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The header, then each record as (type, height, row length, data).
    fn file(records: &[(u8, u8, u8, &[u8])]) -> Vec<u8> {
        let mut data = b"PrintPartner Art v1.0\r\n\x1a".to_vec();
        for &(packing, height, row_len, coded) in records {
            data.push(4);
            data.extend_from_slice(b"Name                ");
            data.extend_from_slice(&[packing, height, row_len]);
            data.extend_from_slice(coded);
        }
        data
    }

    #[test]
    fn raw_rows_are_black_where_set() {
        let image = decode_gph(&file(&[(1, 2, 1, &[0x80, 0x01])])).unwrap();
        assert_eq!((image.width(), image.height()), (8 + 8, 2 + 8));
        assert_eq!(image.get(4, 4), 0);
        assert_eq!(image.get(5, 4), 0xffffff);
        assert_eq!(image.get(11, 5), 0);
    }

    #[test]
    fn byte_runs_repeat_or_copy_and_a_short_picture_ends_white() {
        // 2 rows of 2 bytes: a run of three 0xff, one literal 0x0f; the
        // last row is cut short.
        let coded = [0x83, 0xff, 0x01, 0x0f];
        let rows = Record {
            row_len: 2,
            height: 2,
            packing: Packing::Bytes(&coded),
        }
        .rows();
        assert_eq!(rows, [0xff, 0xff, 0xff, 0x0f]);
        let short = unpack_bytes(&[0x82, 0xff], 4);
        assert_eq!(short, [0xff, 0xff, 0, 0]);
        assert_eq!(unpack_bytes(&[0x85, 0xff], 2), [0xff, 0xff]);
    }

    #[test]
    fn nibble_runs_alternate_white_and_black() {
        // 3 white, 5 black, then 2 black and 7 white: nibbles 3, d, a, 7.
        let rows = unpack_nibbles(&[0x3d, 0xa7], 4, 2);
        assert_eq!(rows, [0b0001_1111, 0b1100_0000]);
        // A run of 0 draws nothing; nibbles past the data are ignored.
        assert_eq!(unpack_nibbles(&[0x8f], 5, 1), [0xfe]);
    }

    #[test]
    fn records_must_fill_the_file_up_to_padding() {
        let mut data = file(&[(1, 1, 1, &[0xff])]);
        assert!(decode_gph(&data).is_ok());
        data.extend_from_slice(&[0, 0x1a]);
        assert!(decode_gph(&data).is_ok());
        data.push(9);
        assert!(decode_gph(&data).is_err());
        assert!(decode_gph(&file(&[(1, 1, 1, &[])])).is_err());
        assert!(decode_gph(&file(&[(4, 1, 1, &[0])])).is_err());
        assert!(decode_gph(&file(&[(1, 0, 1, &[])])).is_err());
        assert!(decode_gph(b"PrintPartner").is_err());
    }
}
