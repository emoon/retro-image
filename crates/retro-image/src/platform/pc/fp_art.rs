//! PFS: First Publisher clip art (`.ART`): 1-bit pictures from the DOS
//! desktop publisher.
//!
//! Sources:
//! - Layout: Deark's `fp_art` module in `modules/misc2.c`
//!   (<https://github.com/jsummers/deark>, MIT license, notice below), and
//!   the survey note `docs/research/gaps-computers-extra.md` C6.
//!   - Standard resolution: four little-endian words (left, right, top,
//!     bottom), then rows of `ceil(width / 16) * 2` bytes, most significant
//!     bit leftmost, a set bit white. `width = right - left` and
//!     `height = bottom - top`; the left edge is not applied to the pixels.
//!   - High resolution: the first word is `FFFF`, then the horizontal and
//!     vertical resolution (Deark identifies 300 and 300 only), width,
//!     height and a word that is 1, then PackBits rows. The row length is
//!     not stored: it is the one of `ceil(width / 8)` to that plus 3 that
//!     makes the unpacked data cover the whole input.
//! - There is no signature. Standard-resolution files are recognized by
//!   size: `8 + row_len * height`, or one row more. All 258 samples checked
//!   fit (the 17 Sembiance `pfsFirstPublisher` files and 241 from the First
//!   Publisher clip-art disks on the textfiles CD,
//!   `swinnund/disk3/CLIPART/`); the corpus group `corpus/extra/dos-clipart/`
//!   keeps 70 of them. The extra row is the Deark case
//!   "`BANNER.ART` from version 3.0"; here 11 files of `ART_FPUB.EXE` have
//!   it, always all white, and it is not part of the picture.
//! - No sample is high resolution, so that path is checked only by a unit
//!   test built from the documented layout.
//! - Output compared with Deark pixel for pixel (see `dos-clipart.tsv`).

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

use crate::bytes::le16;
use crate::codec::packbits;
use crate::image::check_size;
use crate::{BitOrder, DecodeError, Image};

const FAIL: DecodeError = DecodeError::Unrecognized;
const STANDARD_HEADER_LEN: usize = 8;
const HIGH_HEADER_LEN: usize = 12;
const HIGH_MARK: u16 = 0xffff;
/// The only resolution Deark recognizes, in both directions.
const HIGH_DPI: u16 = 300;
const HIGH_MAX_SIDE: usize = 8192;
const WHITE_ON_BLACK: [u32; 2] = [0x000000, 0xffffff];

pub(super) fn decode_art(data: &[u8]) -> Result<Image, DecodeError> {
    if le16(data, 0) == Some(HIGH_MARK) {
        decode_high(data)
    } else {
        decode_standard(data)
    }
}

fn decode_standard(data: &[u8]) -> Result<Image, DecodeError> {
    let word = |at| le16(data, at).map(usize::from).ok_or(FAIL);
    let (left, right, top, bottom) = (word(0)?, word(2)?, word(4)?, word(6)?);
    let width = right.checked_sub(left).ok_or(FAIL)?;
    let height = bottom.checked_sub(top).ok_or(FAIL)?;
    check_size(width, height)?;
    let row_len = width.div_ceil(16) * 2;
    // Some files carry one more row after the picture.
    let len = STANDARD_HEADER_LEN + row_len * height;
    if data.len() != len && data.len() != len + row_len {
        return Err(FAIL);
    }
    mono(&data[STANDARD_HEADER_LEN..], width, height, row_len)
}

fn decode_high(data: &[u8]) -> Result<Image, DecodeError> {
    let word = |at| le16(data, at).map(usize::from).ok_or(FAIL);
    let (x_dpi, y_dpi) = (word(2)?, word(4)?);
    let (width, height) = (word(6)?, word(8)?);
    if x_dpi != usize::from(HIGH_DPI)
        || y_dpi != usize::from(HIGH_DPI)
        || word(10)? != 1
        || width > HIGH_MAX_SIDE
        || height > HIGH_MAX_SIDE
    {
        return Err(FAIL);
    }
    check_size(width, height)?;
    let packed = &data[HIGH_HEADER_LEN..];
    let min_row_len = width.div_ceil(8);
    // Larger candidates first: too large a row length runs the input dry,
    // while too small a one can still end inside the last run.
    let (row_len, bitmap) = (min_row_len..=min_row_len + 3)
        .rev()
        .find_map(|row_len| match packbits::unpack(packed, row_len * height) {
            Some((bitmap, used)) if used == packed.len() => Some((row_len, bitmap)),
            _ => None,
        })
        .ok_or(FAIL)?;
    mono(&bitmap, width, height, row_len)
}

fn mono(bitmap: &[u8], width: usize, height: usize, row_len: usize) -> Result<Image, DecodeError> {
    Image::from_bits(
        width as u32,
        height as u32,
        bitmap,
        row_len,
        BitOrder::MsbFirst,
        WHITE_ON_BLACK,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    fn words(values: &[u16]) -> Vec<u8> {
        values.iter().flat_map(|v| v.to_le_bytes()).collect()
    }

    #[test]
    fn standard_rows_are_padded_to_words_and_the_left_edge_is_ignored() {
        // Left 5, right 22: 17 pixels wide, 4 bytes per row, 2 rows.
        let mut data = words(&[5, 22, 0, 2]);
        data.extend_from_slice(&[0x80, 0x00, 0x80, 0, 0, 0, 0, 0]);
        let image = decode_art(&data).unwrap();
        assert_eq!((image.width(), image.height()), (17, 2));
        assert_eq!(image.get(0, 0), 0xffffff);
        assert_eq!(image.get(16, 0), 0xffffff);
        assert_eq!(image.get(1, 0), 0);
        assert_eq!(image.get(0, 1), 0);
    }

    #[test]
    fn standard_size_must_match_exactly() {
        let mut data = words(&[0, 16, 0, 1]);
        data.extend_from_slice(&[0xff, 0xff]);
        assert!(decode_art(&data).is_ok());
        // One extra row is allowed, a stray byte is not.
        data.extend_from_slice(&[0xff, 0xff]);
        assert_eq!(decode_art(&data).unwrap().height(), 1);
        data.push(0);
        assert!(decode_art(&data).is_err());
        assert!(decode_art(&data[..9]).is_err());
        assert!(decode_art(&words(&[16, 0, 0, 1])).is_err());
    }

    #[test]
    fn high_resolution_row_length_is_inferred_from_the_unpacked_size() {
        // 9 x 2 pixels: 2 bytes per row, or 3 with one byte of padding.
        for row_len in [2usize, 3] {
            let mut data = words(&[HIGH_MARK, 300, 300, 9, 2, 1]);
            // One PackBits run of `2 * row_len` bytes of 0x80.
            data.extend_from_slice(&[(1 - (2 * row_len) as i8) as u8, 0x80]);
            let image = decode_art(&data).unwrap();
            assert_eq!((image.width(), image.height()), (9, 2));
            assert_eq!(image.get(0, 1), 0xffffff);
            assert_eq!(image.get(1, 1), 0);
        }
    }

    #[test]
    fn high_resolution_rejects_other_headers() {
        let mut data = words(&[HIGH_MARK, 300, 300, 9, 2, 1]);
        data.extend_from_slice(&[0xfd, 0x80]);
        assert!(decode_art(&data).is_ok());
        data[10] = 2;
        assert!(decode_art(&data).is_err());
    }
}
