//! OS 3.5 GlowIcon: the first (normal) image of a `FORM ICON`.
//!
//! Sources:
//! - Layout (`FACE`: width - 1, height - 1, flags, aspect, palette size;
//!   `IMAG`: transparent colour, colour count - 1, flags (bit 0
//!   transparency, bit 1 palette present), image and palette storage
//!   (0 one byte per entry, 1 run-length), bits per pixel, image and palette
//!   byte counts - 1, image data, then RGB palette; the run-length scheme is
//!   ByteRun1 over a bit stream of `depth`-bit pixels or 8-bit palette
//!   bytes, with control byte 128 a no-op): Dirk Stöcker, "Amiga Icon
//!   Format" (2002), OS3.5 extension section
//!   (<http://www.evillabs.net/index.php/Amiga_Icon_Formats>).
//! - Palette entries the palette data doesn't define are black: Deark's
//!   `modules/amigaicon.c` (<https://github.com/jsummers/deark>, MIT
//!   licence, notice below).
//!
//! The `FACE` aspect byte is ignored: icons are shown with square pixels.

// Parts of this file follow Deark's modules/amigaicon.c
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

use alloc::vec::Vec;

use super::iff;
use crate::bytes::be16;
use crate::{DecodeError, Image};

const IMAG_HEADER_LEN: usize = 10;

/// Decodes the contents of a `FORM ICON`; transparent pixels get
/// `background`.
pub(super) fn decode(contents: &[u8], background: u32) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let face = iff::find(contents, b"FACE").ok_or(fail)?;
    let width = u32::from(*face.first().ok_or(fail)?) + 1;
    let height = u32::from(*face.get(1).ok_or(fail)?) + 1;
    let imag = iff::find(contents, b"IMAG").ok_or(fail)?;
    let header = imag.get(..IMAG_HEADER_LEN).ok_or(fail)?;
    let [
        transparent,
        colors,
        flags,
        image_format,
        palette_format,
        depth,
    ] = [0, 1, 2, 3, 4, 5].map(|i| header[i]);
    let has_transparency = flags & 1 != 0;
    // The first image always has a palette.
    if flags & 2 == 0 || !(1..=8).contains(&depth) {
        return Err(fail);
    }
    let image_len = usize::from(be16(header, 6).ok_or(fail)?) + 1;
    let palette_len = usize::from(be16(header, 8).ok_or(fail)?) + 1;
    let image_data = imag
        .get(IMAG_HEADER_LEN..IMAG_HEADER_LEN + image_len)
        .ok_or(fail)?;
    let palette_start = IMAG_HEADER_LEN + image_len;
    let palette_data = imag
        .get(palette_start..palette_start + palette_len)
        .ok_or(fail)?;

    let rgb = unpack(
        palette_data,
        palette_format,
        8,
        (usize::from(colors) + 1) * 3,
    )?;
    let mut palette: Vec<u32> = rgb
        .as_chunks::<3>()
        .0
        .iter()
        .map(|c| u32::from(c[0]) << 16 | u32::from(c[1]) << 8 | u32::from(c[2]))
        .collect();
    palette.resize(256, 0);
    if has_transparency {
        palette[usize::from(transparent)] = background;
    }
    let len = width as usize * height as usize;
    let pixels = unpack(image_data, image_format, depth.into(), len)?;
    Image::from_indexed(width, height, &pixels, &palette)
}

/// The first `count` entries of `data` stored with `format`: 0 is one byte
/// per entry, 1 is run-length packed `bits`-bit entries.
fn unpack(data: &[u8], format: u8, bits: u32, count: usize) -> Result<Vec<u8>, DecodeError> {
    match format {
        0 => data.get(..count).map(<[u8]>::to_vec),
        1 => unpack_runs(data, bits, count),
        _ => None,
    }
    .ok_or(DecodeError::Unrecognized)
}

/// ByteRun1 over a bit stream: an 8-bit code `n`, then `n + 1` literal
/// entries (`n` < 128) or one entry repeated `257 - n` times (`n` > 128).
fn unpack_runs(data: &[u8], bits: u32, count: usize) -> Option<Vec<u8>> {
    let mut input = Bits { data, pos: 0 };
    let mut out = Vec::with_capacity(count);
    while out.len() < count {
        match input.read(8)? {
            n @ 0..=127 => {
                for _ in 0..=n {
                    if out.len() == count {
                        break;
                    }
                    out.push(input.read(bits)?);
                }
            }
            128 => {}
            n => {
                let value = input.read(bits)?;
                out.resize(out.len() + 257 - usize::from(n), value);
            }
        }
    }
    out.truncate(count);
    Some(out)
}

/// Most significant bit first.
struct Bits<'a> {
    data: &'a [u8],
    /// In bits.
    pos: usize,
}

impl Bits<'_> {
    /// The next `count` (at most 8) bits.
    fn read(&mut self, count: u32) -> Option<u8> {
        let mut value = 0;
        for _ in 0..count {
            let byte = self.data.get(self.pos / 8)?;
            value = value << 1 | (byte >> (7 - self.pos % 8) & 1);
            self.pos += 1;
        }
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unpacks_runs_of_narrow_entries() {
        // 3-bit entries: literal (code 1) 5, 2; run (code 0xfe = 3 times) 7;
        // no-op code 0x80; literal (code 0) 1.
        // Bits: 00000001 101 010 11111110 111 10000000 00000000 001
        let data = [0x01, 0xab, 0xfb, 0xc0, 0x00, 0x10];
        let out = unpack_runs(&data, 3, 6).unwrap();
        assert_eq!(out, [5, 2, 7, 7, 7, 1]);
    }

    #[test]
    fn run_past_count_is_cut_and_truncated_input_fails() {
        assert_eq!(unpack_runs(&[0x81, 0x09], 8, 3).unwrap(), [9; 3]);
        assert!(unpack_runs(&[0x02, 0x01], 8, 3).is_none());
    }

    /// `FORM ICON` contents: a 2x1 FACE and one IMAG with the given flags,
    /// uncompressed pixels 0, 1 and a 2-colour uncompressed palette.
    fn form(flags: u8) -> Vec<u8> {
        let mut data = b"FACE\0\0\0\x06\x01\x00\x00\x00\x00\x05".to_vec();
        data.extend(b"IMAG\0\0\0\x12");
        data.extend([0, 1, flags, 0, 0, 1, 0, 1, 0, 5]);
        data.extend([0, 1, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66]);
        data
    }

    #[test]
    fn decodes_palette_and_transparency() {
        let image = decode(&form(2), 0x959595).unwrap();
        assert_eq!((image.width(), image.height()), (2, 1));
        assert_eq!((image.get(0, 0), image.get(1, 0)), (0x112233, 0x445566));
        let image = decode(&form(3), 0x959595).unwrap();
        assert_eq!(image.get(0, 0), 0x959595);
    }

    #[test]
    fn first_image_needs_a_palette() {
        assert!(decode(&form(1), 0).is_err());
    }
}
