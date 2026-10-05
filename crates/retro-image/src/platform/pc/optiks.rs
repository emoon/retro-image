//! OPTIKS Quick View self-displaying pictures (`.COM`): a two-color picture
//! behind a fixed viewer stub.
//!
//! Sources:
//! - Deark `misc2.c` (<https://github.com/jsummers/deark>, MIT license): the
//!   file starts `E9 39 01 0D 0A "OPT"` (a jump over the banner text). Two
//!   viewer versions put the picture at a fixed offset: 5680 (bytes `B4 10 00
//!   BB` at 316, versions 2.11 to 2.15) or 5744 (bytes `17 70 1E 8D` at 316,
//!   versions 2.16 to 3.01); either way the bytes `C3 FD 44 E8` sit 13 bytes
//!   before the picture. The picture is a 16-bit count of bytes per row, a
//!   16-bit height and the bitmap as PackBits rows, most significant bit
//!   leftmost; the width is eight times the row length.
//! - Checked on 6 sample files (Sembiance's `optiksCOM` folder), all version
//!   2.16 or later (offset 5744), up to 640x1249 pixels. A set bit is white,
//!   as in Deark's output; checked by eye.
//!
//! Verification: no RECOIL oracle for this format; output matches Deark's
//! `optiks_com` module pixel for pixel on the sample files.

// Parts of this file follow Deark's modules/misc2.c
// (Deark, https://github.com/jsummers/deark):
//
// Copyright (C) 2016-2021 Jason Summers
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

use super::mono;
use crate::bytes::{be32, le16};
use crate::codec::packbits;
use crate::image::check_size;
use crate::{DecodeError, Image};

const FAIL: DecodeError = DecodeError::Unrecognized;
const STUB: [u8; 8] = [0xe9, 0x39, 0x01, 0x0d, 0x0a, b'O', b'P', b'T'];
/// Where the code that tells the viewer version sits.
const VERSION_AT: usize = 316;
/// The viewer versions: the bytes at `VERSION_AT` and where the picture is.
const VERSIONS: [(u32, usize); 2] = [(0xbb00_10b4, 5680), (0x8d1e_7017, 5744)];
/// Viewer code that ends 13 bytes before the picture.
const BEFORE_PICTURE: u32 = 0xe844_fdc3;

pub(super) fn decode_optiks(data: &[u8]) -> Result<Image, DecodeError> {
    if !data.starts_with(&STUB) {
        return Err(FAIL);
    }
    let version = be32(data, VERSION_AT).ok_or(FAIL)?;
    let (_, at) = *VERSIONS.iter().find(|(v, _)| *v == version).ok_or(FAIL)?;
    if be32(data, at - 13) != Some(BEFORE_PICTURE) {
        return Err(FAIL);
    }
    let row_len = usize::from(le16(data, at).ok_or(FAIL)?);
    let height = usize::from(le16(data, at + 2).ok_or(FAIL)?);
    check_size(row_len * 8, height)?;
    let packed = data.get(at + 4..).ok_or(FAIL)?;
    let (bitmap, _) = packbits::unpack(packed, row_len * height).ok_or(FAIL)?;
    mono(&bitmap, row_len * 8, height, row_len)
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;

    #[test]
    fn picture_follows_a_checked_stub() {
        let at = VERSIONS[1].1;
        let mut com = vec![0u8; at];
        com[..STUB.len()].copy_from_slice(&STUB);
        com[VERSION_AT..VERSION_AT + 4].copy_from_slice(&VERSIONS[1].0.to_be_bytes());
        com[at - 13..at - 9].copy_from_slice(&BEFORE_PICTURE.to_be_bytes());
        // One byte per row, two rows: a literal 0x80 and a run of one 0x01.
        com.extend_from_slice(&[1, 0, 2, 0, 0, 0x80, 0xff, 0x01]);
        let image = decode_optiks(&com).unwrap();
        assert_eq!((image.width(), image.height()), (8, 2));
        assert_eq!(image.get(0, 0), 0xffffff);
        assert_eq!(image.get(7, 1), 0xffffff);
        assert_eq!(image.get(1, 0), 0);
        com[at - 13] = 0;
        assert!(decode_optiks(&com).is_err());
    }
}
