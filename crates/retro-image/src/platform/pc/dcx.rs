//! DCX, the multi-page PCX container used by fax software.
//!
//! Sources:
//! - Deark `pcx.c` (<https://github.com/jsummers/deark>, MIT license): the
//!   magic `B1 68 DE 3A`, then up to 1023 little-endian 32-bit page offsets,
//!   ended by a zero; each page is a complete PCX file that runs up to the
//!   next page's offset (the last one to the end of the file).
//! - Wikipedia, "PCX": <https://en.wikipedia.org/wiki/PCX> (DCX is a header
//!   that introduces a set of following PCX files).
//!
//! Only the first page is decoded, with the PCX decoder (`pcx.rs`). The
//! eight sample files are fax pages (1-bit, 1728 pixels wide) except one
//! 70x46 RGB picture; `WFALL.FAX` is a two-page DCX with a `.fax` extension.
//!
//! Verification: no RECOIL oracle for this format; output was compared pixel
//! for pixel with Pillow's DCX reader (first frame) on the sample files.

// Parts of this file follow Deark's modules/pcx.c
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

use super::pcx::decode_pcx;
use crate::bytes::le32;
use crate::{DecodeError, Image};

const FAIL: DecodeError = DecodeError::Unrecognized;
const MAGIC: [u8; 4] = [0xb1, 0x68, 0xde, 0x3a];
/// Where the offset table starts, after the magic.
const TABLE_AT: usize = 4;

pub(super) fn decode_dcx(data: &[u8]) -> Result<Image, DecodeError> {
    if !data.starts_with(&MAGIC) {
        return Err(FAIL);
    }
    let offset = |index: usize| le32(data, TABLE_AT + index * 4).map(|o| o as usize);
    let first = offset(0)
        .filter(|&at| at >= TABLE_AT + 4 && at < data.len())
        .ok_or(FAIL)?;
    // The page ends where the next one starts; a zero ends the table.
    let end = offset(1)
        .filter(|&next| next > first && next <= data.len())
        .unwrap_or(data.len());
    decode_pcx(&data[first..end])
}

#[cfg(test)]
mod tests {
    use alloc::vec;
    use alloc::vec::Vec;

    use super::*;

    /// A 2x1 one-bit PCX: black, white.
    fn pcx() -> Vec<u8> {
        let mut page = vec![0u8; 128];
        page[..4].copy_from_slice(&[10, 5, 1, 1]);
        page[8..10].copy_from_slice(&1u16.to_le_bytes());
        page[65] = 1;
        page[66..68].copy_from_slice(&2u16.to_le_bytes());
        page.extend_from_slice(&[0x40, 0]);
        page
    }

    fn dcx(pages: &[Vec<u8>]) -> Vec<u8> {
        let mut file = MAGIC.to_vec();
        let mut at = TABLE_AT + (pages.len() + 1) * 4;
        for page in pages {
            file.extend_from_slice(&(at as u32).to_le_bytes());
            at += page.len();
        }
        file.extend_from_slice(&[0; 4]);
        pages.iter().for_each(|page| file.extend_from_slice(page));
        file
    }

    #[test]
    fn decodes_the_first_page_only() {
        let mut second = pcx();
        second[2] = 9; // not a PCX any more: must not be reached
        let image = decode_dcx(&dcx(&[pcx(), second])).unwrap();
        assert_eq!((image.width(), image.height()), (2, 1));
        assert_eq!(image.rgb(), &[0, 0, 0, 255, 255, 255]);
    }

    #[test]
    fn rejects_a_table_pointing_outside_the_file() {
        let mut file = dcx(&[pcx()]);
        file[TABLE_AT..TABLE_AT + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(decode_dcx(&file).is_err());
        assert!(decode_dcx(&MAGIC).is_err());
    }
}
