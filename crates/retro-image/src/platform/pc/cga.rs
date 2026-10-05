//! CGA graphics memory, as the PC formats that dump a screen store it.
//!
//! Sources:
//! - Wikipedia, "Color Graphics Adapter":
//!   <https://en.wikipedia.org/wiki/Color_Graphics_Adapter> (in the 320x200
//!   and 640x200 modes even scan lines live in the first bank of video
//!   memory and odd lines in the second, which starts 8192 bytes later; the
//!   320x200 mode has 2 bits per pixel, leftmost pixel in the high bits).
//! - Deark `drhalo.c` (<https://github.com/jsummers/deark>, MIT license): the
//!   same layout for Dr. Halo pictures, and the four-bank Hercules variant.

// Parts of this file follow Deark's modules/drhalo.c
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

/// Distance between the banks of CGA video memory.
pub(super) const BANK_STRIDE: usize = 8192;

/// Four pixel values per byte, leftmost pixel in the high bits (the 320x200
/// 4-color mode, 2 bits per pixel).
pub(super) fn unpack_2bit(bytes: &[u8]) -> Vec<u8> {
    bytes
        .iter()
        .flat_map(|&b| [b >> 6, b >> 4 & 3, b >> 2 & 3, b & 3])
        .collect()
}

/// Rows `0..rows` of `row_len` bytes, row `i` taken from bank `i % banks`
/// (the banks are `bank_stride` bytes apart) at row `i / banks` of that bank.
/// Rows `data` is too short for are left out.
pub(super) fn deinterlace(
    data: &[u8],
    rows: usize,
    row_len: usize,
    banks: usize,
    bank_stride: usize,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(rows * row_len);
    for i in 0..rows {
        let at = (i / banks) * row_len + (i % banks) * bank_stride;
        out.extend_from_slice(data.get(at..at + row_len).unwrap_or(&[]));
    }
    out
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;

    #[test]
    fn cga_banks_are_interleaved() {
        let mut planes = vec![0u8; 2 * BANK_STRIDE];
        planes[0] = 1; // row 0
        planes[BANK_STRIDE] = 2; // row 1
        planes[80] = 3; // row 2
        let rows = deinterlace(&planes, 200, 80, 2, BANK_STRIDE);
        assert_eq!([rows[0], rows[80], rows[160]], [1, 2, 3]);
    }

    #[test]
    fn two_bit_pixels_unpack_high_bits_first() {
        assert_eq!(unpack_2bit(&[0b0001_1011]), [0, 1, 2, 3]);
    }
}
