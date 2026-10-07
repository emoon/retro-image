//! Digi-Pic DGI: a 640x400 CGA digitized picture.
//!
//! Sources:
//! - Deark `misc2.c` (<https://github.com/jsummers/deark>, MIT license): a
//!   64008-byte file with the eight bytes `01 04 00 00 00 00 00 00` at offset
//!   32000. The picture is four 320x200 CGA screens (`cga.rs`), each two
//!   8000-byte banks, in the order top left, top right, then, after the
//!   eight magic bytes, bottom left, bottom right. Deark shows it with the
//!   low-intensity palette 0 of CGA (black, green, red, brown); no
//!   documentation of the program was found, so that palette is taken over
//!   from Deark and is a guess.
//! - Checked on 6 sample files (Sembiance's `dgi` folder), all of this size
//!   with the magic; the screens join without seams across the quadrant
//!   borders, which confirms the order and the layout.
//!
//! Verification: no RECOIL oracle for this format; output matches Deark's
//! `dgi` module pixel for pixel on the sample files.

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

use alloc::vec;

use super::cga::{deinterlace, unpack_2bit};
use super::pcpaint::CGA_4;
use crate::{DecodeError, Image};

const FAIL: DecodeError = DecodeError::Invalid;
const FILE_LEN: usize = 64008;
const MAGIC_AT: usize = 32000;
const MAGIC: [u8; 8] = [1, 4, 0, 0, 0, 0, 0, 0];
const SCREEN_WIDTH: usize = 320;
const SCREEN_HEIGHT: usize = 200;
const ROW_LEN: usize = SCREEN_WIDTH / 4;
/// Bytes in one bank: half the rows of a screen.
const BANK_LEN: usize = ROW_LEN * SCREEN_HEIGHT / 2;
/// Where each screen starts in the file and where it goes in the picture.
const SCREENS: [(usize, usize, usize); 4] = [
    (0, 0, 0),
    (2 * BANK_LEN, SCREEN_WIDTH, 0),
    (4 * BANK_LEN + MAGIC.len(), 0, SCREEN_HEIGHT),
    (6 * BANK_LEN + MAGIC.len(), SCREEN_WIDTH, SCREEN_HEIGHT),
];
/// CGA palette 0 at low intensity.
const PALETTE: [u32; 4] = CGA_4[1];

/// The size and the magic in the middle of the file together are the
/// signature.
pub(super) fn decode_dgi(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != FILE_LEN || data[MAGIC_AT..][..MAGIC.len()] != MAGIC {
        return Err(FAIL);
    }
    let width = 2 * SCREEN_WIDTH;
    let mut indices = vec![0u8; width * 2 * SCREEN_HEIGHT];
    for (start, x, y) in SCREENS {
        let screen = &data[start..start + 2 * BANK_LEN];
        let rows = deinterlace(screen, SCREEN_HEIGHT, ROW_LEN, 2, BANK_LEN);
        for (row, line) in unpack_2bit(&rows)
            .as_chunks::<SCREEN_WIDTH>()
            .0
            .iter()
            .enumerate()
        {
            let at = (y + row) * width + x;
            indices[at..at + SCREEN_WIDTH].copy_from_slice(line);
        }
    }
    Image::from_indexed(width as u32, 2 * SCREEN_HEIGHT as u32, &indices, &PALETTE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quadrants_land_in_their_corners() {
        let mut file = vec![0u8; FILE_LEN];
        file[MAGIC_AT..][..MAGIC.len()].copy_from_slice(&MAGIC);
        // First pixel of each screen: colors 1, 2, 3 and 2.
        for (&(start, _, _), value) in SCREENS.iter().zip([1u8, 2, 3, 2]) {
            file[start] = value << 6;
        }
        let image = decode_dgi(&file).unwrap();
        let at = |x, y| image.get(x, y);
        assert_eq!(at(0, 0), PALETTE[1]);
        assert_eq!(at(320, 0), PALETTE[2]);
        assert_eq!(at(0, 200), PALETTE[3]);
        assert_eq!(at(320, 200), PALETTE[2]);
        assert_eq!(at(1, 0), PALETTE[0]);
        file[MAGIC_AT] = 0;
        assert!(decode_dgi(&file).is_err());
    }
}
