//! Palm OS ImageViewer databases (`.pdb` of type `vIMG`, creator `View`):
//! the first image record, 1, 2 or 4 bits of gray.
//!
//! Sources:
//! - Deark's `modules/palmpdb.c` (<https://github.com/jsummers/deark>, MIT
//!   licence, notice below): the Palm database header (type and creator at
//!   60 and 64, a record count at 76, then 8-byte list entries of a 32-bit
//!   offset, attributes and a 3-byte id; a record runs to the next offset),
//!   the ImageViewer image record (id `$6F8000`: a 32-byte name, a version
//!   byte whose low bits give the compression, a type byte, 20 bytes of
//!   fields, 16-bit width and height at 54 and 56, pixels from 58), its
//!   compression (a control above 128 repeats the next byte `control - 127`
//!   times, otherwise `control + 1` bytes are copied) and the gray levels
//!   (0 is white). Deark is also the oracle for the samples.
//! - Checked on the 10 `palmDatabase` samples of the dexvert set, all
//!   ImageViewer files: type 0 (2 bits) and 255 (1 bit), raw (version 0) and
//!   compressed (version 1).
//!
//! Choices of this crate: type 2 is 4 bits and any other type 1 bit, as in
//! Deark; the 4-bit type has no sample here. The text record that can follow
//! the image is ignored. TealPaint files and bare Palm bitmap resources have
//! no signature in the file and are not decoded.

// Parts of this file follow Deark's modules/palmpdb.c
// (Deark, https://github.com/jsummers/deark):
//
// Copyright (C) 2017 Jason Summers
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
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use alloc::borrow::Cow;
use alloc::vec::Vec;

use crate::bytes::{be16, be32};
use crate::image::check_size;
use crate::{BitOrder, DecodeError, Format, Image};

pub(super) static FORMATS: &[Format] =
    &[Format::new("Palm OS", "ImageViewer", &["pdb"], decode_imageviewer).signature()];

const FAIL: DecodeError = DecodeError::Unrecognized;
/// The database header, which is followed by the record list.
const HEADER_LEN: usize = 78;
const ENTRY_LEN: usize = 8;
const IMAGE_ID: u32 = 0x6f_8000;
/// An image record's name, version, type, other fields, width and height.
const IMAGE_HEADER_LEN: usize = 58;

fn decode_imageviewer(data: &[u8]) -> Result<Image, DecodeError> {
    if data.get(60..68) != Some(b"vIMGView") {
        return Err(FAIL);
    }
    let records = usize::from(be16(data, 76).ok_or(FAIL)?);
    // A list entry: the record's offset and its id.
    let entry = |i: usize| {
        let at = HEADER_LEN + ENTRY_LEN * i;
        Some((be32(data, at)? as usize, be32(data, at + 4)? & 0xff_ffff))
    };
    let (index, start) = (0..records)
        .filter_map(|i| Some((i, entry(i)?)))
        .find(|(_, (_, id))| *id == IMAGE_ID)
        .map(|(i, (start, _))| (i, start))
        .ok_or(FAIL)?;
    let end = if index + 1 < records {
        entry(index + 1).ok_or(FAIL)?.0
    } else {
        data.len()
    };
    let record = data.get(start..end).ok_or(FAIL)?;
    if record.len() < IMAGE_HEADER_LEN {
        return Err(FAIL);
    }
    let compressed = match record[32] {
        0 => false,
        1 => true,
        _ => return Err(FAIL),
    };
    let bits = match record[33] {
        0 => 2,
        2 => 4,
        _ => 1,
    };
    let width = usize::from(be16(record, 54).ok_or(FAIL)?);
    let height = usize::from(be16(record, 56).ok_or(FAIL)?);
    check_size(width, height)?;
    let row_len = (width * bits).div_ceil(8);
    let size = row_len * height;
    let packed = &record[IMAGE_HEADER_LEN..];
    let rows: Cow<[u8]> = if compressed {
        Cow::Owned(unpack(packed, size).ok_or(FAIL)?)
    } else {
        Cow::Borrowed(packed.get(..size).ok_or(FAIL)?)
    };
    if bits == 1 {
        let colors = [0xffffff, 0x000000];
        return Image::from_bits(
            width as u32,
            height as u32,
            &rows,
            row_len,
            BitOrder::MsbFirst,
            colors,
        );
    }
    // The first pixel is in the high bits; 0 is white.
    let top = (1usize << bits) - 1;
    let colors = (0..width * height).map(|i| {
        let bit = i % width * bits;
        let byte = rows[i / width * row_len + bit / 8];
        let level = 255 - (usize::from(byte >> (8 - bits - bit % 8)) & top) * 255 / top;
        level as u32 * 0x01_0101
    });
    Ok(Image::from_colors(width as u32, height as u32, colors))
}

/// `size` bytes of ImageViewer compression: a control above 128 repeats the
/// next byte `control - 127` times, any other copies `control + 1` bytes.
fn unpack(packed: &[u8], size: usize) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut at = 0;
    while out.len() < size {
        let control = *packed.get(at)?;
        at += 1;
        if control > 128 {
            let value = *packed.get(at)?;
            at += 1;
            out.resize(out.len() + usize::from(control) - 127, value);
        } else {
            let count = usize::from(control) + 1;
            out.extend_from_slice(packed.get(at..at + count)?);
            at += count;
        }
    }
    out.truncate(size);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A database with one image record of `bits_type`, `version`, `pixels`.
    fn database(version: u8, bits_type: u8, width: u16, height: u16, pixels: &[u8]) -> Vec<u8> {
        let mut data = alloc::vec![0u8; HEADER_LEN + ENTRY_LEN];
        data[60..68].copy_from_slice(b"vIMGView");
        data[76..78].copy_from_slice(&1u16.to_be_bytes());
        let start = data.len() as u32;
        data[78..82].copy_from_slice(&start.to_be_bytes());
        data[82..86].copy_from_slice(&IMAGE_ID.to_be_bytes());
        let mut record = alloc::vec![0u8; IMAGE_HEADER_LEN];
        record[32] = version;
        record[33] = bits_type;
        record[54..56].copy_from_slice(&width.to_be_bytes());
        record[56..58].copy_from_slice(&height.to_be_bytes());
        data.extend_from_slice(&record);
        data.extend_from_slice(pixels);
        data
    }

    #[test]
    fn two_bit_pixels_start_in_the_high_bits_and_zero_is_white() {
        // 3x1 at 2 bits: 0, 1, 3.
        let data = database(0, 0, 3, 1, &[0b00_01_11_00]);
        let image = decode_imageviewer(&data).unwrap();
        assert_eq!([0, 1, 2].map(|x| image.get(x, 0) & 0xff), [255, 170, 0]);
    }

    #[test]
    fn one_bit_pixels_set_is_black_and_runs_unpack() {
        // 8x1, compressed: control 129 repeats the next byte twice, so 0xf0
        // comes out and the row's single byte leaves the rest cut off.
        let data = database(1, 255, 8, 1, &[129, 0xf0]);
        let image = decode_imageviewer(&data).unwrap();
        assert_eq!((image.get(0, 0), image.get(4, 0)), (0, 0xffffff));
        assert_eq!(
            unpack(&[1, 5, 6, 130, 9], 5),
            Some(alloc::vec![5, 6, 9, 9, 9])
        );
        assert_eq!(unpack(&[1, 5], 2), None);
    }

    #[test]
    fn the_type_creator_and_data_are_checked() {
        let data = database(0, 0, 4, 1, &[0xff]);
        assert!(decode_imageviewer(&data).is_ok());
        let mut other = data.clone();
        other[64] = b'X';
        assert!(decode_imageviewer(&other).is_err());
        assert!(decode_imageviewer(&data[..data.len() - 1]).is_err());
        let mut other = data;
        other[HEADER_LEN + ENTRY_LEN + 32] = 2;
        assert!(decode_imageviewer(&other).is_err());
    }
}
