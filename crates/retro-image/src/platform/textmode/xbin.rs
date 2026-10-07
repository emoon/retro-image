//! XBin (XB): a text screen of any size with an optional palette, font and
//! run-length compression.
//!
//! Sources:
//! - The XBin specification
//!   (<https://web.archive.org/web/20120204063040/http://www.acid.org/info/xbin/x_spec.htm>):
//!   11-byte header ("XBIN", 1Ah, width, height, font size, flags), the
//!   flags (palette, font, compress, non-blink, 512 characters), the 48-byte
//!   6-bit palette, the font, and the compression: a byte whose top two
//!   bits select no compression, a repeated character, a repeated attribute
//!   or a repeated pair, and whose low six bits are the count minus one.
//!   Without a font the VGA default (16 pixels high) applies.
//! - Deark `modules/bintext.c` (<https://github.com/jsummers/deark>, MIT
//!   license): runs are expanded as one stream (the specification keeps
//!   them within a row, so for valid files that's the same), image data
//!   missing at the end is blank, and 512-character files are not
//!   supported (the specification doesn't say how the attribute selects
//!   the second 256 characters).

// The handling of short data and 512-character files follows Deark
// `modules/bintext.c`, under this license:
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

use super::font::{Font, VGA_8X16};
use super::sauce;
use super::screen::{self, MAX_CELLS, MAX_COLUMNS, PALETTE, Style};
use crate::bytes::le16;
use crate::{DecodeError, Image};

const HEADER_LEN: usize = 11;
const PALETTE_FLAG: u8 = 1;
const FONT_FLAG: u8 = 2;
const COMPRESS_FLAG: u8 = 4;
const NON_BLINK_FLAG: u8 = 8;
const CHARS_512_FLAG: u8 = 16;

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    const FAIL: DecodeError = DecodeError::Invalid;
    let (content, _) = sauce::split(data);
    let header = content.get(..HEADER_LEN).ok_or(FAIL)?;
    let word = |at| le16(header, at).map(usize::from).ok_or(FAIL);
    let (width, height) = (word(5)?, word(7)?);
    let (font_height, flags) = (usize::from(header[9]), header[10]);
    let cells = width * height;
    if &header[..5] != b"XBIN\x1a"
        || flags & CHARS_512_FLAG != 0
        || width == 0
        || width > MAX_COLUMNS
        || height == 0
        || cells > MAX_CELLS
    {
        return Err(FAIL);
    }
    let mut pos = HEADER_LEN;
    let mut take = |len: usize| {
        let bytes = content.get(pos..pos + len).ok_or(FAIL);
        pos += len;
        bytes
    };
    let palette = match flags & PALETTE_FLAG {
        0 => PALETTE,
        _ => screen::vga_palette(take(48)?).ok_or(FAIL)?,
    };
    let font = match flags & FONT_FLAG {
        0 => VGA_8X16,
        _ => Font::new(font_height, take(256 * font_height)?).ok_or(FAIL)?,
    };
    let image_data = &content[pos..];
    let mut pairs = match flags & COMPRESS_FLAG {
        0 => image_data.get(..cells * 2).unwrap_or(image_data).to_vec(),
        _ => decompress(image_data, cells),
    };
    pairs.resize(cells * 2, 0);
    let ice = flags & NON_BLINK_FLAG != 0;
    let (cells, rows) = screen::attribute_cells(&pairs, width, &palette, ice)?;
    let style = Style {
        font,
        nine_pixels: false,
    };
    screen::render(&cells, width, rows, &style)
}

/// Expands XBin runs into at most `cells` (character, attribute) pairs;
/// stops early if the data runs out.
fn decompress(src: &[u8], cells: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(cells * 2);
    let mut i = 0;
    while out.len() < cells * 2 {
        let Some(&control) = src.get(i) else {
            break;
        };
        let count = usize::from(control & 63) + 1;
        i += 1;
        let (fixed, step) = match control >> 6 {
            0 => (0, 2),
            1 | 2 => (1, 1),
            _ => (2, 0),
        };
        let Some(head) = src.get(i..i + fixed) else {
            break;
        };
        i += fixed;
        for _ in 0..count {
            let Some(varying) = src.get(i..i + step) else {
                break;
            };
            i += step;
            let pair = match control >> 6 {
                0 => [varying[0], varying[1]],
                1 => [head[0], varying[0]],
                2 => [varying[0], head[0]],
                _ => [head[0], head[1]],
            };
            out.extend_from_slice(&pair);
        }
    }
    out.truncate(cells * 2);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn decompresses_all_four_run_types() {
        let src = [
            0x01, b'a', 1, b'b', 2, // two literal pairs
            0x42, b'c', 3, 4, 5, // character run of 3
            0x81, 6, b'd', b'e', // attribute run of 2
            0xc2, b'f', 7, // pair run of 3
        ];
        let out = decompress(&src, 10);
        assert_eq!(
            out,
            [
                b'a', 1, b'b', 2, b'c', 3, b'c', 4, b'c', 5, b'd', 6, b'e', 6, b'f', 7, b'f', 7,
                b'f', 7
            ]
        );
        assert_eq!(decompress(&src, 3).len(), 6, "stops at the cell count");
        assert_eq!(decompress(&[0x43, b'x', 1], 10), [b'x', 1], "data ran out");
        assert_eq!(decompress(&[0xff, b'x', 1], 1 << 18).len(), 128);
    }

    fn xbin(width: u16, height: u16, flags: u8, rest: &[u8]) -> Vec<u8> {
        let mut data = b"XBIN\x1a".to_vec();
        data.extend_from_slice(&width.to_le_bytes());
        data.extend_from_slice(&height.to_le_bytes());
        data.extend_from_slice(&[8, flags]);
        data.extend_from_slice(rest);
        data
    }

    #[test]
    fn reads_header_palette_font_and_flags() {
        let mut rest = vec![0u8; 48];
        rest[3..6].copy_from_slice(&[63, 63, 63]); // color 1 white
        rest[27..30].copy_from_slice(&[0, 0, 42]); // color 9 dark blue
        let mut font = vec![0u8; 256 * 8];
        font[b'#' as usize * 8] = 0x80;
        rest.extend_from_slice(&font);
        rest.extend_from_slice(&[0xc1, b'#', 0x91]); // two '#' on blink-bit blue
        let data = xbin(
            2,
            1,
            PALETTE_FLAG | FONT_FLAG | COMPRESS_FLAG | NON_BLINK_FLAG,
            &rest,
        );
        let image = decode(&data).unwrap();
        assert_eq!((image.width(), image.height()), (16, 8));
        assert_eq!(image.get(0, 0), 0xffffff, "glyph pixel in colour 1");
        assert_eq!(image.get(1, 0), 0x0000aa, "non-blink background 9");
    }

    #[test]
    fn rejects_bad_headers_and_pads_short_data() {
        let image = decode(&xbin(3, 2, 0, b"A\x07")).unwrap();
        assert_eq!(
            (image.width(), image.height()),
            (24, 32),
            "default 8x16 font"
        );
        assert!(decode(&xbin(0, 2, 0, b"")).is_err());
        assert!(decode(&xbin(80, 2, CHARS_512_FLAG, b"")).is_err());
        assert!(decode(&xbin(4096, 1, 0, b"")).is_err());
        assert!(decode(&xbin(1000, 1000, 0, b"")).is_err(), "cell limit");
        assert!(
            decode(&xbin(1, 1, FONT_FLAG, &[0; 100])).is_err(),
            "short font"
        );
        let mut wrong = xbin(1, 1, 0, b"");
        wrong[4] = 0;
        assert!(decode(&wrong).is_err());
    }
}
