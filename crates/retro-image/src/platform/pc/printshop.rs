//! The Print Shop (`.DAT`) and The New Print Shop (`.POG`) clip-art
//! libraries (Broderbund, DOS), shown as a sheet of their pictures.
//!
//! Sources:
//! - Layout: Deark's `printshop.c` (`printshop` and `newprintshop`;
//!   <https://github.com/jsummers/deark>, MIT license, notice below), and the
//!   survey note `docs/research/gaps-computers-extra.md` C7. Every picture is
//!   88x52 pixels, 11 bytes per row, 572 bytes, most significant bit
//!   leftmost, a set bit black. A `.DAT` file is the pictures back to back; a
//!   `.POG` file has a 10-byte header with the picture count as a
//!   little-endian word at offset 8. The `.NAM` and `.PNM` files beside them
//!   only hold 16-byte picture names and are not read.
//! - Deark identifies a `.DAT` by `size % 572 == 0` and a name starting with
//!   `GR`; the name is not visible here, so only the size is used. Two real
//!   files break that rule, `GRSCHOOL.DAT` (101 pictures and 84 bytes) and
//!   `grcps.dat` (46 pictures and 56 bytes): both end in zero fill or `1A`
//!   marks, up to a multiple of 128 bytes. They are not accepted. No byte
//!   rule tells such a tail from the zero fill that ends Atari ST pictures
//!   named `.DAT` (three in `corpus/hostile/atari-st/`), and `.DAT` is too
//!   common an extension to claim them on the strength of a size.
//! - A `.POG` file must be exactly `10 + 572 * count` bytes.
//!   [`crate::sheet`] lays the sheet out.
//! - Checked on 21 `.DAT` and 25 `.POG` files (Sembiance `printShopDAT` and
//!   `pog`, and the Print Shop and New Print Shop disks on the textfiles CD,
//!   `swinnund/disk3/CLIPART/`). Each picture matches Deark's output pixel
//!   for pixel (see `dos-clipart.tsv`). The corpus group
//!   `corpus/extra/dos-clipart/` keeps 7 and 11 of them, and the two padded
//!   `.DAT` files that are not accepted.

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

use super::clipart::black_on_white;
use crate::bytes::le16;
use crate::sheet::Sheet;
use crate::{DecodeError, Image};

const FAIL: DecodeError = DecodeError::Unrecognized;
const WIDTH: usize = 88;
const HEIGHT: usize = 52;
const PICTURE_LEN: usize = WIDTH / 8 * HEIGHT;
const POG_HEADER_LEN: usize = 10;
const POG_COUNT_AT: usize = 8;

pub(super) fn decode_dat(data: &[u8]) -> Result<Image, DecodeError> {
    library(data, data.len() / PICTURE_LEN)
}

pub(super) fn decode_pog(data: &[u8]) -> Result<Image, DecodeError> {
    let count = usize::from(le16(data, POG_COUNT_AT).ok_or(FAIL)?);
    library(data.get(POG_HEADER_LEN..).ok_or(FAIL)?, count)
}

/// A sheet of the `count` back-to-back pictures that are all of `data`.
fn library(data: &[u8], count: usize) -> Result<Image, DecodeError> {
    if data.len() != count * PICTURE_LEN {
        return Err(FAIL);
    }
    let mut sheet = Sheet::new(count, WIDTH, HEIGHT)?;
    for (index, bytes) in data.as_chunks::<PICTURE_LEN>().0.iter().enumerate() {
        sheet.put(index, &black_on_white(WIDTH, HEIGHT, bytes)?);
    }
    Ok(sheet.into_image())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sheet::MAX_PICTURES;
    use alloc::vec::Vec;

    fn pictures(count: usize) -> Vec<u8> {
        // Picture `n` has its first pixel black when `n` is odd, else its second.
        (0..count)
            .flat_map(|n| {
                let mut picture = alloc::vec![0u8; PICTURE_LEN];
                picture[0] = if n % 2 == 1 { 0x80 } else { 0x40 };
                picture
            })
            .collect()
    }

    #[test]
    fn dat_is_whole_pictures_and_nothing_else() {
        let mut data = pictures(2);
        let sheet = decode_dat(&data).unwrap();
        // Two pictures: 2 columns of 92 pixels and a gutter, 1 row.
        assert_eq!((sheet.width(), sheet.height()), (2 * 92 + 4, 56 + 4));
        assert_eq!(sheet.get(4 + 92, 4), 0);
        data.extend_from_slice(&[0, 0x1a, 0x1a]);
        assert!(decode_dat(&data).is_err());
        assert!(decode_dat(&data[..PICTURE_LEN - 1]).is_err());
    }

    #[test]
    fn a_library_over_the_picture_cap_is_not_a_print_shop_file() {
        assert!(decode_dat(&pictures(MAX_PICTURES)).is_ok());
        assert!(decode_dat(&pictures(MAX_PICTURES + 1)).is_err());
    }

    #[test]
    fn pog_count_comes_from_the_header() {
        let mut data = alloc::vec![0u8; POG_HEADER_LEN];
        data[POG_COUNT_AT] = 3;
        data.extend(pictures(3));
        assert!(decode_pog(&data).is_ok());
        data[POG_COUNT_AT] = 4;
        assert!(decode_pog(&data).is_err());
        data[POG_COUNT_AT] = 2;
        assert!(
            decode_pog(&data).is_err(),
            "a whole extra picture is not padding"
        );
        data[POG_COUNT_AT] = 0;
        assert!(decode_pog(&data).is_err());
    }
}
