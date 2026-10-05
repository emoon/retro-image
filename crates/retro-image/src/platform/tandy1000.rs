//! Tandy 1000.
//!
//! Sources:
//! - DeskMate Paint (`.PNT`): Deark `misc2.c` (<https://github.com/jsummers/deark>,
//!   MIT licence): signature `$13 "PNT"`, pixels from offset 22, 312x176 at
//!   4 bits per pixel (high nibble first), stored raw or as (value, count)
//!   byte pairs.
//! - Palette: observed from `recoil2png` output.

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
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

use alloc::vec::Vec;

use crate::{DecodeError, Format, Image};

/// DeskMate's 16 colours.
const PALETTE: [u32; 16] = [
    0x000000, 0x000099, 0x009900, 0x339999, 0x990000, 0xcc33cc, 0xcc6600, 0x999999, 0x996633,
    0x6633ff, 0x33cc00, 0x66cccc, 0xffcccc, 0xff99ff, 0xffff00, 0xffffff,
];

pub(super) static FORMATS: &[Format] =
    &[Format::new("Tandy 1000", "DeskMate Paint", &["pnt"], decode_pnt).signature()];

const WIDTH: usize = 312;
const HEIGHT: usize = 176;
const PIXELS_AT: usize = 22;

fn decode_pnt(data: &[u8]) -> Result<Image, DecodeError> {
    if !data.starts_with(b"\x13PNT") || data.len() <= PIXELS_AT {
        return Err(DecodeError::Unrecognized);
    }
    let len = WIDTH / 2 * HEIGHT;
    let src = &data[PIXELS_AT..];
    let pixels = if src.len() == len {
        src.to_vec()
    } else {
        unpack_runs(src, len).ok_or(DecodeError::Unrecognized)?
    };
    let indices: Vec<u8> = pixels.iter().flat_map(|&b| [b >> 4, b & 15]).collect();
    Image::from_indexed(WIDTH as u32, HEIGHT as u32, &indices, &PALETTE)
}

/// (value, count) byte pairs that must fill exactly `len` bytes. One run of
/// zero-count pairs is accepted as a lost disk sector (`SHIP3.pnt` lost 256
/// pairs to zeros): the pairs after it still line up, and the shortfall at
/// the end of the stream is the size of the hole, which is left black. A
/// hole of more than a quarter of the picture, a second run, or any other
/// short or overlong stream means the file is not usable.
fn unpack_runs(src: &[u8], len: usize) -> Option<Vec<u8>> {
    let (pairs, rest) = src.as_chunks::<2>();
    if !rest.is_empty() {
        return None;
    }
    let mut out = Vec::with_capacity(len);
    let mut hole_at = None;
    let mut previous_zero = false;
    for pair in pairs {
        let count = usize::from(pair[1]);
        if count == 0 {
            if !previous_zero && hole_at.replace(out.len()).is_some() {
                return None;
            }
        } else {
            if out.len() + count > len {
                return None;
            }
            out.resize(out.len() + count, pair[0]);
        }
        previous_zero = count == 0;
    }
    match (out.len().cmp(&len), hole_at) {
        (core::cmp::Ordering::Equal, _) => Some(out),
        (core::cmp::Ordering::Less, Some(at)) if len - out.len() <= len / 4 => {
            let hole = len - out.len();
            out.splice(at..at, core::iter::repeat_n(0, hole));
            Some(out)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_must_fill_the_picture_exactly() {
        assert_eq!(unpack_runs(&[7, 2, 9, 1], 3), Some(alloc::vec![7, 7, 9]));
        assert_eq!(unpack_runs(&[7, 2], 3), None);
        assert_eq!(unpack_runs(&[7, 4], 3), None);
        assert_eq!(unpack_runs(&[7, 3, 1, 1], 3), None);
    }

    #[test]
    fn one_zero_run_is_a_lost_sector() {
        // The hole sits where the zero pairs were, and may be at most a
        // quarter of the picture.
        assert_eq!(unpack_runs(&[7, 2, 0, 0, 0, 0, 9, 2], 8), None);
        assert_eq!(
            unpack_runs(&[7, 6, 0, 0, 0, 0, 9, 6], 16),
            Some(alloc::vec![7, 7, 7, 7, 7, 7, 0, 0, 0, 0, 9, 9, 9, 9, 9, 9])
        );
        // Two separate runs, a hole over a quarter of the picture, or a
        // zero run that does not explain a short stream are rejected.
        assert_eq!(unpack_runs(&[7, 6, 0, 0, 9, 6, 0, 0], 16), None);
        assert_eq!(unpack_runs(&[7, 2, 0, 0, 9, 2], 20), None);
        assert_eq!(unpack_runs(&[7, 2, 9, 2], 8), None);
        // All zeros: the hole is the whole picture.
        assert_eq!(unpack_runs(&[0; 8], 8), None);
    }
}
