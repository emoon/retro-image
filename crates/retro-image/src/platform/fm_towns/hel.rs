//! FM Towns HEL animation, shown as its first frame.
//!
//! Sources:
//! - Layout (`he1\0\1\0\0\0`, 32-bit LE frame count minus one at offset 8,
//!   then 160x120 1-bit frames of 2400 bytes from offset 12, each frame
//!   toggling the pixels it sets on the previous picture, which starts
//!   black): Deark's `modules/misc2.c`, `fmtowns_hel`
//!   (<https://github.com/jsummers/deark>, MIT license, notice below). The
//!   10 sample sizes listed in `docs/research/gaps-pc-japan.md` (all
//!   12 mod 1000) agree with 12 + 2400 per frame.
//! - Showing only the first frame is this crate's own choice. No sample was
//!   available, so this is checked only by unit tests.

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

use crate::bytes::le32;
use crate::image::BitOrder;
use crate::{DecodeError, Image};

const HEADER_LEN: usize = 12;
const WIDTH: usize = 160;
const HEIGHT: usize = 120;
const ROW_LEN: usize = WIDTH / 8;
const FRAME_LEN: usize = ROW_LEN * HEIGHT;

/// The first frame; later ones are deltas and the file must hold them all.
pub(super) fn decode_hel(data: &[u8]) -> Result<Image, DecodeError> {
    if !data.starts_with(b"he1\0\x01\0\0\0") {
        return Err(DecodeError::Unrecognized);
    }
    let frames = le32(data, 8)
        .and_then(|n| (n as usize).checked_add(1))
        .ok_or(DecodeError::Unrecognized)?;
    let expected = frames
        .checked_mul(FRAME_LEN)
        .and_then(|len| len.checked_add(HEADER_LEN));
    if expected != Some(data.len()) {
        return Err(DecodeError::Unrecognized);
    }
    Image::from_bits(
        WIDTH as u32,
        HEIGHT as u32,
        &data[HEADER_LEN..],
        ROW_LEN,
        BitOrder::MsbFirst,
        [0x000000, 0xffffff],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn first_frame_sets_pixels_white() {
        let mut data = vec![0u8; HEADER_LEN + 2 * FRAME_LEN];
        data[..8].copy_from_slice(b"he1\0\x01\0\0\0");
        data[8] = 1; // two frames
        data[HEADER_LEN] = 0x80;
        let image = decode_hel(&data).unwrap();
        assert_eq!((image.width(), image.height()), (160, 120));
        assert_eq!(&image.rgb()[..6], &[255, 255, 255, 0, 0, 0]);
        data.pop();
        assert!(decode_hel(&data).is_err());
    }
}
