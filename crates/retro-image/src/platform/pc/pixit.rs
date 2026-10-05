//! PIXIT and pix320 self-displaying pictures, and their raw `.PIX` form.
//!
//! Sources:
//! - Deark `misc2.c` (<https://github.com/jsummers/deark>, MIT license): the
//!   COM stub starts with `BC 00 01 B8 13 00 CD 10` (`mov sp, 100h`, then
//!   `mov ax, 13h` and `int 10h`: VGA mode 13h), has the byte `BA` at offset 17
//!   and a 16-bit operand at 18; the picture record sits at that operand
//!   minus 272. Disassembling the stub explains the number: `BA` is the
//!   opcode of `mov dx, imm16`, after `mov ax, 1012h` and `mov cx, 100h`, a
//!   BIOS call that loads 256 DAC colors from `ES:DX`, so the operand is the
//!   palette's address in memory.
//!   A COM file loads at `100h` and the palette starts 16 bytes into the
//!   record, so the record's file offset is the operand less 256 and 16.
//! - The record: `PX`, 16-bit width and height, padding up to 16 bytes, a
//!   768-byte palette of 6-bit VGA values (red, green, blue), then one byte
//!   per pixel. A raw `.PIX` file is such a record alone, always 320x200:
//!   `PX 40 01 C8 00` and 64784 bytes.
//! - Checked on the sample files (Sembiance's `pixit` folder): ten COM
//!   programs, two of them named `.EXE` (they are COM files by content), all
//!   with the record at offset 72 or 74.
//!
//! Only the raw `.PIX` form is chosen by extension. The stub form is found by
//! content under any name: `.com` and `.exe` belong to every DOS program, and
//! the shared MIME package would send them all to the image viewer.
//!
//! Verification: no RECOIL oracle for this format; output matches Deark's
//! `pixit` module pixel for pixel on the sample files.

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

use alloc::vec::Vec;

use super::dac_rounded;
use crate::bytes::le16;
use crate::image::check_size;
use crate::{DecodeError, Image};

const FAIL: DecodeError = DecodeError::Unrecognized;
/// Start of every stub: `mov sp, 100h`, `mov ax, 13h`, `int 10h`.
const STUB: [u8; 8] = [0xbc, 0x00, 0x01, 0xb8, 0x13, 0x00, 0xcd, 0x10];
/// A COM file is at most one 64 KB segment, less the PSP.
const MAX_COM_LEN: usize = 65280;
const PALETTE_LEN: usize = 768;
const RECORD_HEADER_LEN: usize = 16;
/// The record sits this far before the address the stub names.
const RECORD_ADDRESS_BIAS: usize = 272;
/// Raw `.PIX` files are exactly this picture.
const RAW_LEN: usize = RECORD_HEADER_LEN + PALETTE_LEN + 320 * 200;

pub(super) fn decode_pixit(data: &[u8]) -> Result<Image, DecodeError> {
    let at = if data.starts_with(b"PX") {
        // A bare record: only the 320x200 picture is known to work.
        if data.len() != RAW_LEN || le16(data, 2) != Some(320) || le16(data, 4) != Some(200) {
            return Err(FAIL);
        }
        0
    } else {
        if data.len() > MAX_COM_LEN || !data.starts_with(&STUB) || data.get(17) != Some(&0xba) {
            return Err(FAIL);
        }
        let at = usize::from(le16(data, 18).ok_or(FAIL)?)
            .checked_sub(RECORD_ADDRESS_BIAS)
            .ok_or(FAIL)?;
        if data.get(at..at + 2) != Some(b"PX") {
            return Err(FAIL);
        }
        at
    };
    let width = usize::from(le16(data, at + 2).ok_or(FAIL)?);
    let height = usize::from(le16(data, at + 4).ok_or(FAIL)?);
    check_size(width, height)?;
    let palette_at = at + RECORD_HEADER_LEN;
    let pixels_at = palette_at + PALETTE_LEN;
    let palette: Vec<u32> = data
        .get(palette_at..pixels_at)
        .ok_or(FAIL)?
        .as_chunks::<3>()
        .0
        .iter()
        .map(|c| dac_rounded(c[0]) << 16 | dac_rounded(c[1]) << 8 | dac_rounded(c[2]))
        .collect();
    let pixels = data
        .get(pixels_at..pixels_at + width * height)
        .ok_or(FAIL)?;
    Image::from_indexed(width as u32, height as u32, pixels, &palette)
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;

    fn record(width: u16, height: u16, pixels: &[u8]) -> Vec<u8> {
        let mut record = b"PX".to_vec();
        record.extend_from_slice(&width.to_le_bytes());
        record.extend_from_slice(&height.to_le_bytes());
        record.resize(RECORD_HEADER_LEN, 0);
        let mut palette = vec![0u8; PALETTE_LEN];
        palette[3..6].copy_from_slice(&[63, 0, 32]);
        record.extend_from_slice(&palette);
        record.extend_from_slice(pixels);
        record
    }

    #[test]
    fn a_com_stub_points_at_its_record() {
        let mut com = STUB.to_vec();
        com.resize(17, 0);
        com.push(0xba);
        // The record follows 24 bytes of stub.
        let at = 24usize;
        com.extend_from_slice(&((at + RECORD_ADDRESS_BIAS) as u16).to_le_bytes());
        com.resize(at, 0);
        com.extend_from_slice(&record(2, 1, &[1, 0]));
        let image = decode_pixit(&com).unwrap();
        assert_eq!((image.width(), image.height()), (2, 1));
        assert_eq!(image.get(0, 0), 0xff0082);
        assert_eq!(image.get(1, 0), 0);
        // Found by content under the name a DOS program has.
        assert_eq!(crate::decode("picture.com", &com), Ok(image));
        com[at] = b'Q';
        assert!(decode_pixit(&com).is_err());
    }

    #[test]
    fn a_bare_record_must_be_the_320x200_picture() {
        let raw = record(320, 200, &[0; 64000]);
        assert_eq!(decode_pixit(&raw).unwrap().width(), 320);
        assert!(decode_pixit(&raw[..raw.len() - 1]).is_err());
        assert!(decode_pixit(&record(2, 1, &[0, 0])).is_err());
    }
}
