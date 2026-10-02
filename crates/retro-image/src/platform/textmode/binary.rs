//! Raw text-mode screens of (character, attribute) pairs: Binary Text
//! (BIN), ArtWorx Data Format (ADF) and iCE Draw (IDF).
//!
//! Sources:
//! - The SAUCE specification (<https://www.acid.org/info/sauce/sauce.htm>):
//!   BinaryText (DataType 5) is a memory copy of a text screen whose width
//!   is twice the FileType; the ANSiFlags select non-blink mode and 9-pixel
//!   letter spacing.
//! - Deark `modules/bintext.c` (<https://github.com/jsummers/deark>, MIT
//!   licence): without a width, BIN is 160 columns wide; FileType 1 with a
//!   TInfo1 means TInfo1 is half the width (written by ACiDDraw). ADF is a
//!   version byte (1), a 64-entry 6-bit EGA palette of which entries 0-5,
//!   20, 7 and 56-63 are the 16 text colours, an 8x16 font, then an
//!   80-column non-blink screen; the palette bytes are all 0-63.
//! - libansilove `src/loaders/icedraw.c`
//!   (<https://github.com/ansilove/libansilove>, BSD-2-Clause): IDF is a
//!   12-byte header ("\x04" "1.4", then x1, y1, x2, y2 as little-endian
//!   words; the width is x2 + 1), the screen, an 8x16 font and a 6-bit
//!   16-colour palette at the end; a character 1 starts a run (attribute
//!   byte unused, a count word, then the character and attribute to repeat);
//!   iCE Draw always uses non-blink colours.
//! - Reverse engineered from samples: BIN files without SAUCE of 4000 bytes
//!   are one 80x25 screen (`COMPUTER.BIN`, `LM_SCR1.BIN` from the dexvert
//!   samples), not 12.5 rows of 160 columns as Deark assumes. Other BIN
//!   files without SAUCE (Deark: 160 columns) are not accepted: their size
//!   is all there is to go on, and `.bin` is too common. IDF's run count is a word
//!   (`ICE-9605.IDF` decodes to exactly 200 rows of 80 cells); and IDF
//!   files may carry SAUCE, which libansilove reads as part of the palette
//!   (`SQ-FORCE.IDF`).

// The BinaryText width quirk and the ADF layout follow Deark `modules/bintext.c`,
// under this licence:
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
//
// The IDF layout follows libansilove `src/loaders/icedraw.c`, under this
// licence:
//
// Copyright (c) 2011-2026, Stefan Vogt, Brian Cassidy, and Frederic Cambus
// All rights reserved.
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are met:
//
//   * Redistributions of source code must retain the above copyright
//     notice, this list of conditions and the following disclaimer.
//
//   * Redistributions in binary form must reproduce the above copyright
//     notice, this list of conditions and the following disclaimer in the
//     documentation and/or other materials provided with the distribution.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
// AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
// ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS
// BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
// CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
// SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
// INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
// CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
// ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
// POSSIBILITY OF SUCH DAMAGE.

use alloc::vec::Vec;

use super::font::Font;
use super::sauce::{self, BINARY_TEXT, DEFAULT_STYLE};
use super::screen::{self, MAX_CELLS, PALETTE, Style};
use crate::bytes::le16;
use crate::{DecodeError, Image};

/// One 80x25 screen of pairs.
const SCREEN_LEN: usize = 80 * 25 * 2;

/// Binary Text. `.bin` is a very common extension, so without a BinaryText
/// SAUCE record only a plausible 80x25 screen is accepted.
pub(super) fn decode_bin(data: &[u8]) -> Result<Image, DecodeError> {
    let (pairs, sauce) = sauce::split(data);
    let sauce = sauce.filter(|s| s.data_type == BINARY_TEXT);
    let width = match sauce.map(|s| (s.file_type, s.tinfo1)) {
        Some((1, half @ 1..)) => usize::from(half) * 2,
        Some((half @ 1.., _)) => usize::from(half) * 2,
        // A BinaryText record without a width: Deark's 160 columns.
        Some(_) => 160,
        _ if plausible_screen(pairs) => 80,
        _ => return Err(DecodeError::Unrecognized),
    };
    let ice = sauce.as_ref().is_some_and(|s| s.ice());
    let (cells, rows) = screen::attribute_cells(pairs, width, &PALETTE, ice)?;
    screen::render(&cells, width, rows, &sauce::style_of(sauce.as_ref()))
}

/// Whether `pairs` looks like one 80x25 text screen: at least 95% of the
/// characters printable ASCII or CP437 shading and line drawing
/// (B0h-DFh), and at most 32 different attributes. Real screens use a few
/// colours; random data, code and compressed data spread over all 256.
fn plausible_screen(pairs: &[u8]) -> bool {
    if pairs.len() != SCREEN_LEN {
        return false;
    }
    let art = |c: &u8| matches!(c, 0x20..=0x7e | 0xb0..=0xdf);
    let printable = pairs.iter().step_by(2).filter(|c| art(c)).count();
    let mut seen = [false; 256];
    pairs
        .iter()
        .skip(1)
        .step_by(2)
        .for_each(|&a| seen[usize::from(a)] = true);
    let attributes = seen.iter().filter(|&&s| s).count();
    printable * 100 >= SCREEN_LEN / 2 * 95 && attributes <= 32
}

const ADF_PALETTE_LEN: usize = 64 * 3;
const ADF_FONT_AT: usize = 1 + ADF_PALETTE_LEN;
const ADF_SCREEN_AT: usize = ADF_FONT_AT + 4096;
/// The EGA palette entries of the 16 text colours.
const ADF_COLORS: [usize; 16] = [0, 1, 2, 3, 4, 5, 20, 7, 56, 57, 58, 59, 60, 61, 62, 63];

/// ArtWorx Data Format. No signature: the version byte, the 0-63 palette
/// values and whole 80-column rows must all check out (`.adf` is also the
/// Amiga disk image extension).
pub(super) fn decode_adf(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let (content, _) = sauce::split(data);
    let ega = content.get(1..ADF_FONT_AT).ok_or(fail)?;
    let pairs = content.get(ADF_SCREEN_AT..).ok_or(fail)?;
    let palette_ok = ega.iter().all(|&v| v <= 63) && ega.iter().any(|&v| v != 0);
    if content[0] != 1 || !palette_ok || pairs.is_empty() || pairs.len() % 160 != 0 {
        return Err(fail);
    }
    let rgb: Vec<u8> = ADF_COLORS
        .iter()
        .flat_map(|&i| ega[i * 3..i * 3 + 3].iter().copied())
        .collect();
    let palette = screen::vga_palette(&rgb).ok_or(fail)?;
    let font = Font::new(16, &content[ADF_FONT_AT..ADF_SCREEN_AT]).ok_or(fail)?;
    let (cells, rows) = screen::attribute_cells(pairs, 80, &palette, true)?;
    screen::render(&cells, 80, rows, &with_font(font))
}

const IDF_HEADER_LEN: usize = 12;
const IDF_FONT_LEN: usize = 4096;
const IDF_PALETTE_LEN: usize = 48;

/// iCE Draw: the "\x04" "1.4" header, a valid width and 0-63 palette values.
pub(super) fn decode_idf(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let (content, _) = sauce::split(data);
    let word = |at| le16(content, at).map(usize::from).ok_or(fail);
    if !content.starts_with(b"\x041.4") || word(4)? > word(8)? {
        return Err(fail);
    }
    let width = word(8)? + 1;
    let tail = IDF_FONT_LEN + IDF_PALETTE_LEN;
    let screen_end = content
        .len()
        .checked_sub(tail)
        .filter(|&end| end >= IDF_HEADER_LEN)
        .ok_or(fail)?;
    let palette = screen::vga_palette(&content[content.len() - IDF_PALETTE_LEN..]).ok_or(fail)?;
    let font = Font::new(16, &content[screen_end..screen_end + IDF_FONT_LEN]).ok_or(fail)?;
    let pairs = unpack_idf(&content[IDF_HEADER_LEN..screen_end], width)?;
    let (cells, rows) = screen::attribute_cells(&pairs, width, &palette, true)?;
    screen::render(&cells, width, rows, &with_font(font))
}

/// Expands IDF runs (`01 xx count-word character attribute`) into pairs.
/// Fails if the output would exceed the cell limit for `width`.
fn unpack_idf(src: &[u8], width: usize) -> Result<Vec<u8>, DecodeError> {
    let limit = (MAX_CELLS / width.max(1)) * width * 2;
    let mut pairs = Vec::new();
    let mut i = 0;
    while i + 1 < src.len() {
        if src[i] == 1 {
            let (Some(count), Some(&[character, attribute])) =
                (le16(src, i + 2), src.get(i + 4..i + 6))
            else {
                break;
            };
            for _ in 0..count {
                pairs.extend_from_slice(&[character, attribute]);
            }
            i += 6;
        } else {
            pairs.extend_from_slice(&src[i..i + 2]);
            i += 2;
        }
        if pairs.len() > limit {
            return Err(DecodeError::Unrecognized);
        }
    }
    Ok(pairs)
}

/// The default style with an embedded font.
fn with_font(font: Font<'_>) -> Style<'_> {
    Style {
        font,
        ..DEFAULT_STYLE
    }
}

#[cfg(test)]
mod tests {
    use super::super::sauce::with_sauce;
    use super::*;
    use alloc::vec;

    #[test]
    fn bin_width_comes_from_sauce_or_a_plausible_screen() {
        let mut screen = vec![0u8; SCREEN_LEN];
        for (i, pair) in screen.as_chunks_mut::<2>().0.iter_mut().enumerate() {
            pair.copy_from_slice(&[b' ' + (i % 90) as u8, 0x07 + (i % 3) as u8 * 0x10]);
        }
        assert_eq!(decode_bin(&screen).unwrap().width(), 80 * 8);
        assert!(decode_bin(&[0u8; SCREEN_LEN]).is_err(), "NUL characters");
        let noise: Vec<u8> = (0..SCREEN_LEN as u32)
            .map(|i| (i.wrapping_mul(2_654_435_761) >> 13) as u8)
            .collect();
        assert!(decode_bin(&noise).is_err(), "pseudo-random bytes");
        let mut colourful = screen.clone();
        for (i, a) in colourful.iter_mut().skip(1).step_by(2).enumerate() {
            *a = i as u8;
        }
        assert!(decode_bin(&colourful).is_err(), "too many attributes");
        assert!(
            decode_bin(&[0x20u8; 320 * 3]).is_err(),
            "no SAUCE, not 80x25"
        );
        assert!(decode_bin(&[]).is_err());
        // FileType 40: 80 columns; flags: iCE and 9 pixels.
        let data = with_sauce(&[b'A', 0x9e, b'B', 0x07], 5, 40, 0, 0x05, b"", &[]);
        let image = decode_bin(&data).unwrap();
        assert_eq!((image.width(), image.height()), (80 * 9, 16));
        assert_eq!(image.get(0, 0), PALETTE[9], "iCE background");
        // ACiDDraw: FileType 1 and the half width in TInfo1.
        let data = with_sauce(&[0; 320], 5, 1, 80, 0, b"", &[]);
        assert_eq!(decode_bin(&data).unwrap().width(), 160 * 8);
        // A record of another type doesn't vouch for the data.
        let data = with_sauce(&[0x20; 320], 1, 1, 80, 0, b"", &[]);
        assert!(decode_bin(&data).is_err());
    }

    /// An ADF file: grey palette entry 7, blank font, `pairs`.
    fn adf(pairs: &[u8]) -> Vec<u8> {
        let mut data = vec![1u8];
        let mut ega = [0u8; ADF_PALETTE_LEN];
        ega[7 * 3..7 * 3 + 3].copy_from_slice(&[42, 42, 42]);
        ega[20 * 3] = 42; // brown's red
        data.extend_from_slice(&ega);
        data.extend_from_slice(&[0; 4096]);
        data.extend_from_slice(pairs);
        data
    }

    #[test]
    fn adf_maps_the_ega_palette_and_checks_its_layout() {
        let mut pairs = vec![0u8; 160];
        pairs[1] = 0x67; // grey on brown
        let image = decode_adf(&adf(&pairs)).unwrap();
        assert_eq!((image.width(), image.height()), (640, 16));
        assert_eq!(image.get(0, 0), 0xaa0000, "background from EGA entry 20");
        assert!(decode_adf(&adf(&pairs[..150])).is_err(), "partial row");
        let mut bad = adf(&pairs);
        bad[0] = 0;
        assert!(decode_adf(&bad).is_err(), "version");
        bad[0] = 1;
        bad[100] = 64;
        assert!(decode_adf(&bad).is_err(), "palette value");
        let with_record = with_sauce(&adf(&pairs), 1, 0, 0, 0, b"", &[]);
        assert!(decode_adf(&with_record).is_ok());
    }

    #[test]
    fn idf_runs_expand_and_the_width_comes_from_the_header() {
        let mut data = b"\x041.4\0\0\0\0\x03\0\x18\0".to_vec();
        // 'a' on blue, then a run of four 'b' on red.
        data.extend_from_slice(&[b'a', 0x10, 1, 0, 4, 0, b'b', 0x40]);
        data.extend_from_slice(&[0; IDF_FONT_LEN]);
        let mut palette = [0u8; IDF_PALETTE_LEN];
        palette[3..6].copy_from_slice(&[0, 0, 63]);
        palette[12..15].copy_from_slice(&[63, 0, 0]);
        data.extend_from_slice(&palette);
        let image = decode_idf(&data).unwrap();
        assert_eq!((image.width(), image.height()), (4 * 8, 2 * 16));
        assert_eq!(image.get(0, 0), 0x0000ff);
        assert_eq!(image.get(0, 17), 0xff0000, "run wrapped to row 2");
        assert_eq!(image.get(8, 17), 0, "padding");
        let last = data.len() - 1;
        data[last] = 64;
        assert!(decode_idf(&data).is_err(), "palette value");
        assert!(decode_idf(&data[..IDF_HEADER_LEN]).is_err());
    }

    #[test]
    fn idf_runs_stop_at_the_cell_limit() {
        let mut src = vec![];
        for _ in 0..10 {
            src.extend_from_slice(&[1, 0, 0xff, 0xff, b'x', 7]);
        }
        assert!(unpack_idf(&src, 80).is_err());
        assert_eq!(unpack_idf(&[1, 0, 2, 0, b'x'], 80).unwrap(), b"");
    }
}
