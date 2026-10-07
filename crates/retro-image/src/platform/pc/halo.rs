//! Dr. Halo CUT and PAL.
//!
//! Sources:
//! - Encyclopedia of Graphics File Formats, "Dr. Halo":
//!   <https://www.fileformat.info/format/drhalo/egff.htm>. CUT: width,
//!   height and a zero reserved word (little-endian), then per scan line a
//!   16-bit byte count and run-length data: a control byte with bit 7 set
//!   repeats the next byte (control & 0x7f) times, otherwise that many literal
//!   bytes follow; a control byte of 0 or 0x80 ends the line. PAL: 40-byte
//!   header (`AH`, version, size, type 0x0A, ..., MaxIndex at 0x0c,
//!   MaxRed/MaxGreen/MaxBlue at 0x0e, 0x10, 0x12) followed by 16-bit RGB entries.
//! - Overview: <https://www.graphicsacademy.com/format_drhalo.php>.
//!
//! The CUT file has no magic, so it is chosen by extension and validated by
//! structure: every line must decode to exactly the width. Without a PAL the
//! picture is a gray ramp stretched to the highest index used (as Deark does). PAL values are scaled from their declared maxima
//! (`MaxRed` etc.) to 8 bits.
//!
//! The Dr. Halo PIC variant is in `halo_pic.rs`.
//!
//! The PAL layout (16-bit samples, 512-byte block rule) and the gray stretch
//! were checked against Deark's output on the Dr. Halo samples in the corpus.

// Parts of this file follow Deark's modules/drhalo.c
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
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

use alloc::vec::Vec;

use crate::bytes::le16;
use crate::image::check_size;
use crate::{Companions, DecodeError, Image};

const CUT_HEADER_LEN: usize = 6;
const PAL_HEADER_LEN: usize = 40;

pub(super) fn decode_cut(data: &[u8], companions: &dyn Companions) -> Result<Image, DecodeError> {
    const FAIL: DecodeError = DecodeError::Invalid;
    let word = |at| le16(data, at).map(usize::from).ok_or(FAIL);
    let (width, height) = (word(0)?, word(2)?);
    if word(4)? != 0 {
        return Err(FAIL);
    }
    check_size(width, height)?;
    // Each line costs at least its count word and a terminator.
    if height > data.len() / 3 {
        return Err(FAIL);
    }
    let mut pixels = Vec::with_capacity(width * height);
    let mut pos = CUT_HEADER_LEN;
    for _ in 0..height {
        let len = word(pos)?;
        let line = data.get(pos + 2..pos + 2 + len).ok_or(FAIL)?;
        pos += 2 + len;
        unpack_line(line, width, &mut pixels)?;
    }
    let palette = companions
        .get("pal")
        .and_then(|pal| parse_pal(&pal))
        .unwrap_or_else(|| grey_ramp(&pixels));
    Image::from_indexed(width as u32, height as u32, &pixels, &palette)
}

/// Gray ramp stretched so the highest index in use is white. The files carry
/// no palette, and raw indices (often 0 and 1) would render near black.
fn grey_ramp(pixels: &[u8]) -> Vec<u32> {
    let max = u32::from(pixels.iter().copied().max().unwrap_or(1)).max(1);
    (0..=max)
        .map(|v| ((v * 255 + max / 2) / max) * 0x01_01_01)
        .collect()
}

/// Appends exactly `width` pixels decoded from one line's run data.
fn unpack_line(line: &[u8], width: usize, out: &mut Vec<u8>) -> Result<(), DecodeError> {
    const FAIL: DecodeError = DecodeError::Invalid;
    let start = out.len();
    let mut pos = 0;
    while let Some(&control) = line.get(pos) {
        pos += 1;
        let n = usize::from(control & 0x7f);
        if n == 0 {
            break;
        }
        if out.len() - start + n > width {
            return Err(FAIL);
        }
        if control & 0x80 != 0 {
            let value = *line.get(pos).ok_or(FAIL)?;
            pos += 1;
            out.resize(out.len() + n, value);
        } else {
            out.extend_from_slice(line.get(pos..pos + n).ok_or(FAIL)?);
            pos += n;
        }
    }
    if out.len() - start == width {
        Ok(())
    } else {
        Err(FAIL)
    }
}

/// Palette entries of a PAL file, or `None` if it isn't one.
///
/// Entries are three little-endian 16-bit samples starting at offset 40, and
/// `MaxIndex` (0x0c) + 1 of them are stored. An entry never straddles a
/// 512-byte block: one that would starts at the next block instead
/// (layout as implemented by Deark's drhalo module, MIT).
fn parse_pal(data: &[u8]) -> Option<Vec<u32>> {
    if data.get(..2)? != b"AH" || *data.get(6)? != 0x0a || *data.get(7)? != 0 {
        return None;
    }
    let max = |at| match le16(data, at) {
        Some(m @ 1..=255) => u32::from(m),
        _ => 255,
    };
    let (mr, mg, mb) = (max(0x0e), max(0x10), max(0x12));
    let scale = |v: u16, m: u32| (u32::from(v) * 255 / m).min(255);
    let count = (usize::from(le16(data, 0x0c)?) + 1).min(256);
    let mut colors = Vec::with_capacity(count);
    let mut pos = PAL_HEADER_LEN;
    for _ in 0..count {
        if pos % 512 > 506 {
            pos = pos.next_multiple_of(512);
        }
        let (r, g, b) = (le16(data, pos)?, le16(data, pos + 2)?, le16(data, pos + 4)?);
        colors.push(scale(r, mr) << 16 | scale(g, mg) << 8 | scale(b, mb));
        pos += 6;
    }
    colors.resize(256, 0); // indices past MaxIndex are black
    Some(colors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NoCompanions;

    fn cut() -> Vec<u8> {
        // 4x2: line 1 is a run of four 3s; line 2 is 1, 2 then a run of two 9s.
        let mut d = alloc::vec![4, 0, 2, 0, 0, 0];
        d.extend_from_slice(&[3, 0, 0x84, 3, 0]);
        d.extend_from_slice(&[6, 0, 2, 1, 2, 0x82, 9, 0]);
        d
    }

    #[test]
    fn decodes_with_stretched_grey_palette() {
        let image = decode_cut(&cut(), &NoCompanions).unwrap();
        assert_eq!((image.width(), image.height()), (4, 2));
        assert_eq!(&image.rgb()[..3], &[85, 85, 85]);
        assert_eq!(&image.rgb()[12..15], &[28, 28, 28]);
        assert_eq!(&image.rgb()[21..24], &[255, 255, 255]);
    }

    #[test]
    fn pal_companion_is_scaled() {
        struct Pal;
        impl Companions for Pal {
            fn get_named(&self, _file_name: &str) -> Option<alloc::borrow::Cow<'_, [u8]>> {
                None
            }
            fn get(&self, ext: &str) -> Option<alloc::borrow::Cow<'_, [u8]>> {
                (ext == "pal")
                    .then(|| {
                        let mut p = alloc::vec![0u8; PAL_HEADER_LEN];
                        p[..2].copy_from_slice(b"AH");
                        p[6] = 0x0a;
                        p[0x0c] = 3;
                        p[0x0e] = 63;
                        p[0x10] = 63;
                        p[0x12] = 63;
                        p.extend_from_slice(&[0; 18]);
                        p.extend_from_slice(&[63, 0, 0, 0, 21, 0]);
                        p
                    })
                    .map(Into::into)
            }
        }
        let image = decode_cut(&cut(), &Pal).unwrap();
        assert_eq!(&image.rgb()[..3], &[255, 0, 85]);
    }

    #[test]
    fn rejects_wrong_line_widths() {
        let mut d = cut();
        d[8] = 0x83; // first run now yields 3 pixels
        assert!(decode_cut(&d, &NoCompanions).is_err());
        assert!(decode_cut(&cut()[..10], &NoCompanions).is_err());
    }
}
