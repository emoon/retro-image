//! Print Shop GS clip art (Broderbund, Apple IIGS): one 88x52 picture in
//! eight colors.
//!
//! Sources:
//! - Layout and palette: Deark's `printshop_gs` module in `printshop.c`
//!   (<https://github.com/jsummers/deark>, MIT license, notice below). The
//!   file is 1716 bytes: three bit planes of 52 rows of 11 bytes, most
//!   significant bit leftmost. The first plane in the file is the lowest
//!   bit of the color number, 0 to 7: white, yellow, red, orange, blue,
//!   green, purple, black. The survey note
//!   `docs/research/gaps-computers-extra.md` C7 gives the same eight colors
//!   in the opposite bit order (from CiderPress II, taken from an emulator
//!   screenshot), which is the same assignment.
//! - The files are ProDOS type `$F8` and have no extension on disk; `.psg`
//!   is this crate's own convention for them. Recognition is by the exact
//!   size, so the format has no signature. The 572-byte monochrome variant
//!   and the Apple II clip art of the same program (ProDOS type `B`) have no
//!   sample here and are not decoded.
//! - Checked on 11 files (Sembiance `printShopGSGraphic`, renamed with the
//!   extension): the output matches Deark's pixel for pixel (see
//!   `dos-clipart.tsv`). The picture is not stretched; Deark notes that 88:52
//!   would make the pixels square.

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

use crate::image::planar_pixels;
use crate::{DecodeError, Image};

const WIDTH: usize = 88;
const HEIGHT: usize = 52;
const ROW_LEN: usize = WIDTH / 8;
const PLANE_LEN: usize = ROW_LEN * HEIGHT;
const PLANES: usize = 3;
const FILE_LEN: usize = PLANE_LEN * PLANES;

/// White, yellow, red, orange, blue, green, purple, black.
const PALETTE: [u32; 8] = [
    0xffffff, 0xffff00, 0xff0000, 0xff6600, 0x0000ff, 0x00ff00, 0xcc00cc, 0x000000,
];

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != FILE_LEN {
        return Err(DecodeError::Unrecognized);
    }
    let pixels = planar_pixels(data, WIDTH, HEIGHT, ROW_LEN, PLANES, |plane, y| {
        plane * PLANE_LEN + y * ROW_LEN
    });
    let indices: alloc::vec::Vec<u8> = pixels.into_iter().map(|v| v as u8).collect();
    Image::from_indexed(WIDTH as u32, HEIGHT as u32, &indices, &PALETTE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_plane_is_the_low_bit_of_the_color() {
        let mut data = alloc::vec![0u8; FILE_LEN];
        data[0] = 0x80; // plane 0 only: color 1
        data[PLANE_LEN + 1] = 0x40; // plane 1 only, pixel 9: color 2
        data[2 * PLANE_LEN + ROW_LEN] = 0x80; // plane 2 only, row 1: color 4
        data[ROW_LEN + 87 / 8] = 0x01; // plane 0 and
        data[PLANE_LEN + ROW_LEN + 87 / 8] = 0x01; // plane 1, last pixel of row 1: color 3
        let image = decode(&data).unwrap();
        assert_eq!((image.width(), image.height()), (88, 52));
        assert_eq!(image.get(0, 0), 0xffff00);
        assert_eq!(image.get(9, 0), 0xff0000);
        assert_eq!(image.get(0, 1), 0x0000ff);
        assert_eq!(image.get(87, 1), 0xff6600);
        assert_eq!(image.get(1, 0), 0xffffff);
    }

    #[test]
    fn only_the_exact_size_is_accepted() {
        assert!(decode(&[0; FILE_LEN - 1]).is_err());
        assert!(decode(&[0; FILE_LEN + 1]).is_err());
    }
}
