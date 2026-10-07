//! WordPerfect Graphics (`.WPG`, version 1): the first bitmap of files made
//! of one.
//!
//! Sources:
//! - Structure: "WordPerfect Graphics Metafile", Encyclopedia of Graphics
//!   File Formats, <https://www.fileformat.info/format/wpg/egff.htm> (a
//!   16-byte prefix: `FF 57 50 43`, the offset of the records, product type 1,
//!   file type `16`, version, key; records of a type byte, a length of 1, 3 or
//!   5 bytes (`FF`, then a word, whose top bit says a second word follows),
//!   and the data; color map record `0E` with a start word, a count word and
//!   RGB bytes; bitmap type 1 record `0B` with width, height, bits per pixel
//!   and two resolutions as words, then the data, "which may be RLE
//!   compressed"). Words are little-endian, in spite of what that article
//!   says, as the samples show.
//! - The run-length coding, the white set bit of 1-bit bitmaps, and the 1-bit
//!   color map being ignored: Deark's `wpg` module (`wpg.c`;
//!   <https://github.com/jsummers/deark>, MIT license, notice below). Codes:
//!   `00 n` repeats the row above `n` times, `01` to `7F` copies that many
//!   bytes, `80 n` is `n` bytes of `FF`, `81` to `FF` repeats the next byte
//!   `n & 7F` times. 1-bit bitmaps have all-zero color maps in the samples.
//! - Only files whose bitmap is in a type 1 record (WordPerfect 5.0 clip art
//!   and files saved by WordPerfect for Windows), 1, 4 or 8 bits per pixel,
//!   version 1, are decoded; the 4 and 8-bit ones need their color map. That is
//!   all the samples hold apart from two: `input.wpg` is version 2 and vector
//!   only, `BADGE.wpg` is an OLE container. A file of vector records and no
//!   bitmap is rejected, as is a type 2 bitmap record (WordPerfect 5.1,
//!   no sample) and 2 bits per pixel (no sample).
//! - The row length: the bitmap is `ceil(bits * width / 8)` bytes per row,
//!   except that `cup.wpg` (158 pixels, 4 bits) has 80 where that is 79, and
//!   `DARVADER.WPG` has 2 bytes left over. The row length is therefore taken
//!   from the data (`unpacked / height`), which must be that or one more. Deark
//!   uses 79 and shows `cup.wpg` sheared (see `dos-clipart.tsv`). One sample
//!   is the only evidence for the longer row, which fits an even number of
//!   bytes per row, but 1-bit rows of 25 bytes are not padded.
//! - Checked on 47 files (Sembiance `wpg`, 19, and `WP50ART1.EXE` on
//!   cd.textfiles.com `swinnund/disk3/CLIPART/`, 28): 44 match Deark pixel for
//!   pixel, `cup.wpg` is right where Deark is not, and the two exceptions above
//!   are rejected. The corpus group `corpus/extra/dos-clipart/` keeps 27 of
//!   them.

// Parts of this file follow Deark's modules/wpg.c
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

use crate::bytes::{le16, le32};
use crate::image::check_size;
use crate::{BitOrder, DecodeError, Image};

const FAIL: DecodeError = DecodeError::Invalid;
const MAGIC: &[u8] = b"\xffWPC";
const PREFIX_LEN: usize = 16;
const PRODUCT_TYPE: u8 = 1;
const FILE_TYPE: u8 = 0x16;
const MAJOR_VERSION: u8 = 1;
const RECORD_BITMAP_1: u8 = 0x0b;
const RECORD_COLOR_MAP: u8 = 0x0e;
const BITMAP_HEADER_LEN: usize = 10;

struct Record<'a> {
    kind: u8,
    data: &'a [u8],
}

/// The records after the prefix, up to the first one that does not fit.
fn records(data: &[u8]) -> Result<impl Iterator<Item = Record<'_>>, DecodeError> {
    if !data.starts_with(MAGIC)
        || data.get(8) != Some(&PRODUCT_TYPE)
        || data.get(9) != Some(&FILE_TYPE)
        || data.get(10) != Some(&MAJOR_VERSION)
    {
        return Err(FAIL);
    }
    let start = le32(data, 4)
        .and_then(|at| usize::try_from(at).ok())
        .filter(|&at| at >= PREFIX_LEN)
        .ok_or(FAIL)?;
    let mut rest = data.get(start..).ok_or(FAIL)?;
    Ok(core::iter::from_fn(move || {
        let (&kind, tail) = rest.split_first()?;
        let (&short, tail) = tail.split_first()?;
        let (len, tail) = if short == 0xff {
            let word = usize::from(le16(tail, 0)?);
            if word & 0x8000 != 0 {
                let low = usize::from(le16(tail, 2)?);
                ((word & 0x7fff) << 16 | low, tail.get(4..)?)
            } else {
                (word, tail.get(2..)?)
            }
        } else {
            (usize::from(short), tail)
        };
        let (data, tail) = tail.split_at_checked(len)?;
        rest = tail;
        Some(Record { kind, data })
    }))
}

/// Bytes of rows from `data`, `row_len` bytes to the row. Stops at the end of
/// `data` or after `limit` bytes; `None` for a repeat with no row above.
fn unpack(data: &[u8], row_len: usize, limit: usize) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut rest = data;
    while let [code, tail @ ..] = rest {
        rest = tail;
        match code {
            0 => {
                let (&count, tail) = rest.split_first()?;
                rest = tail;
                let above = out.len().checked_sub(row_len)?;
                for _ in 0..count {
                    out.extend_from_within(above..above + row_len);
                }
            }
            0x80 => {
                let (&count, tail) = rest.split_first()?;
                rest = tail;
                out.resize(out.len() + usize::from(count), 0xff);
            }
            1..=0x7f => {
                let (bytes, tail) = rest.split_at_checked(usize::from(*code))?;
                out.extend_from_slice(bytes);
                rest = tail;
            }
            _ => {
                let (&value, tail) = rest.split_first()?;
                rest = tail;
                out.resize(out.len() + usize::from(code & 0x7f), value);
            }
        }
        if out.len() >= limit {
            break;
        }
    }
    Some(out)
}

pub(super) fn decode_wpg(data: &[u8]) -> Result<Image, DecodeError> {
    let mut palette = None;
    for record in records(data)? {
        match record.kind {
            RECORD_COLOR_MAP => palette = Some(record.data),
            RECORD_BITMAP_1 => return decode_bitmap(record.data, palette),
            _ => {}
        }
    }
    Err(FAIL)
}

/// A bitmap type 1 record: width, height, bits per pixel and the two
/// resolutions, then the coded rows.
fn decode_bitmap(record: &[u8], color_map: Option<&[u8]>) -> Result<Image, DecodeError> {
    let word = |at| le16(record, at).map(usize::from).ok_or(FAIL);
    let (width, height, bits) = (word(0)?, word(2)?, word(4)?);
    if !matches!(bits, 1 | 4 | 8) {
        return Err(FAIL);
    }
    check_size(width, height)?;
    let coded = record.get(BITMAP_HEADER_LEN..).ok_or(FAIL)?;
    let min_row_len = (bits * width).div_ceil(8);
    // A row is `min_row_len` bytes, or one more where the writer padded it.
    // Which one decides what the repeat codes copy, so each is unpacked and
    // the data must come out as exactly `height` rows of that length. Only
    // if neither does is the one that leaves less than a row over taken
    // (`DARVADER.WPG` has 2 bytes too many).
    let candidates: Vec<(usize, Vec<u8>)> = [min_row_len, min_row_len + 1]
        .into_iter()
        // More than a longer row's worth of data cannot be a picture.
        .filter_map(|row_len| Some((row_len, unpack(coded, row_len, (row_len + 1) * height)?)))
        .collect();
    let (row_len, rows) = candidates
        .iter()
        .find(|(row_len, rows)| rows.len() == row_len * height)
        .or_else(|| {
            candidates
                .iter()
                .find(|(row_len, rows)| rows.len() / height == *row_len)
        })
        .ok_or(FAIL)?;
    let row_len = *row_len;
    if bits == 1 {
        return Image::from_bits(
            width as u32,
            height as u32,
            rows,
            row_len,
            BitOrder::MsbFirst,
            [0x000000, 0xffffff],
        );
    }
    let palette = color_palette(color_map.ok_or(FAIL)?, 1 << bits)?;
    let indices: Vec<u8> = rows
        .chunks_exact(row_len)
        .take(height)
        .flat_map(|row| {
            (0..width).map(move |x| match bits {
                4 => row[x / 2] >> (4 - 4 * (x % 2)) & 15,
                _ => row[x],
            })
        })
        .collect();
    Image::from_indexed(width as u32, height as u32, &indices, &palette)
}

/// `colors` entries from a color map record: a start word, a count word and
/// RGB bytes. Entries the record does not define are black.
fn color_palette(record: &[u8], colors: usize) -> Result<Vec<u32>, DecodeError> {
    let start = usize::from(le16(record, 0).ok_or(FAIL)?);
    let count = usize::from(le16(record, 2).ok_or(FAIL)?);
    let entries = record.get(4..).ok_or(FAIL)?.as_chunks::<3>().0;
    let mut palette = alloc::vec![0u32; colors];
    for (slot, &[red, green, blue]) in palette
        .iter_mut()
        .skip(start)
        .zip(entries.iter().take(count))
    {
        *slot = u32::from(red) << 16 | u32::from(green) << 8 | u32::from(blue);
    }
    Ok(palette)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(records: &[(u8, Vec<u8>)]) -> Vec<u8> {
        let mut data = b"\xffWPC\x10\0\0\0\x01\x16\x01\0\0\0\0\0".to_vec();
        for (kind, body) in records {
            data.push(*kind);
            if body.len() < 0xff {
                data.push(body.len() as u8);
            } else {
                data.push(0xff);
                data.extend_from_slice(&(body.len() as u16).to_le_bytes());
            }
            data.extend_from_slice(body);
        }
        data
    }

    fn bitmap(width: u16, height: u16, bits: u16, coded: &[u8]) -> (u8, Vec<u8>) {
        let mut body = Vec::new();
        for value in [width, height, bits, 72, 72] {
            body.extend_from_slice(&value.to_le_bytes());
        }
        body.extend_from_slice(coded);
        (RECORD_BITMAP_1, body)
    }

    #[test]
    fn run_length_codes() {
        // Row of 2: literal, repeat it twice, 0xff x 2, run of 3 x 0x0f.
        let coded = [2, 0xaa, 0xbb, 0, 2, 0x80, 2, 0x83, 0x0f];
        let rows = unpack(&coded, 2, usize::MAX).unwrap();
        assert_eq!(
            rows,
            [0xaa, 0xbb, 0xaa, 0xbb, 0xaa, 0xbb, 0xff, 0xff, 15, 15, 15]
        );
        // A repeat with no row above, and short literals.
        assert!(unpack(&[0, 1], 2, usize::MAX).is_none());
        assert!(unpack(&[3, 1], 2, usize::MAX).is_none());
    }

    #[test]
    fn one_bit_bitmaps_are_black_where_clear_and_ignore_the_color_map() {
        // 9 x 2: 2 bytes a row.
        let data = file(&[
            (RECORD_COLOR_MAP, alloc::vec![0, 0, 2, 0, 9, 9, 9, 8, 8, 8]),
            bitmap(9, 2, 1, &[4, 0x7f, 0x80, 0xff, 0x00]),
        ]);
        let image = decode_wpg(&data).unwrap();
        assert_eq!((image.width(), image.height()), (9, 2));
        assert_eq!(image.get(0, 0), 0x000000);
        assert_eq!(image.get(1, 0), 0xffffff);
        assert_eq!(image.get(8, 0), 0xffffff);
        assert_eq!(image.get(8, 1), 0x000000);
    }

    #[test]
    fn four_bit_rows_may_be_padded_to_an_even_length() {
        // 3 pixels: 2 bytes a row, or 3 where padded (the data is 3 a row).
        let colors = alloc::vec![0, 0, 3, 0, 0, 0, 0, 255, 0, 0, 0, 255, 0];
        let palette = (RECORD_COLOR_MAP, colors);
        let coded = [6, 0x01, 0x20, 0, 0x01, 0x20, 0];
        let padded = file(&[palette.clone(), bitmap(3, 2, 4, &coded)]);
        let image = decode_wpg(&padded).unwrap();
        let row: Vec<u32> = (0..3).map(|x| image.get(x, 1)).collect();
        assert_eq!(row, [0, 0xff0000, 0x00ff00]);
        // Without padding the same pixels take 2 bytes a row.
        let coded = [4, 0x01, 0x20, 0x01, 0x20];
        let plain = file(&[palette, bitmap(3, 2, 4, &coded)]);
        assert_eq!(decode_wpg(&plain).unwrap(), image);
        // The colors need a color map.
        assert!(decode_wpg(&file(&[bitmap(3, 2, 4, &[4, 1, 2, 3, 4])])).is_err());
    }

    #[test]
    fn a_repeat_code_decides_the_padded_row_length() {
        // 3 pixels at 4 bits: 2 bytes a row, 3 where padded. One literal row
        // of 3 bytes, then "repeat the row above once". Read as 2-byte rows
        // the repeat copies 2 bytes and the data is 5 bytes, which is
        // `2 * 2` and a byte over; only 3-byte rows give exactly 2 rows.
        let colors = alloc::vec![0, 0, 3, 0, 0, 0, 0, 255, 0, 0, 0, 255, 0];
        let data = file(&[
            (RECORD_COLOR_MAP, colors),
            bitmap(3, 2, 4, &[3, 0x01, 0x20, 0, 0, 1]),
        ]);
        let image = decode_wpg(&data).unwrap();
        let rows: Vec<Vec<u32>> = (0..2)
            .map(|y| (0..3).map(|x| image.get(x, y)).collect())
            .collect();
        assert_eq!(rows[0], [0, 0xff0000, 0x00ff00]);
        assert_eq!(rows[1], rows[0]);
    }

    #[test]
    fn long_record_lengths_and_other_records_are_walked() {
        let long = alloc::vec![0u8; 300];
        let data = file(&[(0x01, long), bitmap(8, 1, 1, &[1, 0x0f])]);
        assert_eq!(decode_wpg(&data).unwrap().get(4, 0), 0xffffff);
        // Vector records only, other versions, other bit depths.
        assert!(decode_wpg(&file(&[(0x05, alloc::vec![0; 8])])).is_err());
        let mut v2 = data.clone();
        v2[10] = 2;
        assert!(decode_wpg(&v2).is_err());
        assert!(decode_wpg(&file(&[bitmap(8, 1, 2, &[1, 0x0f])])).is_err());
    }
}
