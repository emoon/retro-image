//! NewIcons image: the first (normal) image of the `IM1=` tool types.
//!
//! Sources:
//! - Layout (header of transparency flag `B`/`C`, width + `$21`, height +
//!   `$21` and a two-character colour count, then 8-bit RGB palette entries
//!   and chunky pixels of just enough bits for the colour count, packed 7
//!   bits per character: `$20-$6F` and `$A1-$D0` are values, `$D1-$FF` are
//!   runs of 1-47 times 7 zero bits; each line is flushed and padded): Dirk
//!   Stöcker, "Amiga Icon Format" (2002), NewIcon extension section
//!   (<http://www.evillabs.net/index.php/Amiga_Icon_Formats>).
//! - The pixels start at the first line end after the whole palette;
//!   transparency means colour 0 is transparent; up to 512 stored colours;
//!   unused palette entries are black: Deark's `modules/amigaicon.c`
//!   (<https://github.com/jsummers/deark>, MIT licence, notice below).

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

use crate::{DecodeError, Image};

const MAX_COLORS: usize = 512;

/// Decodes the `IM1=` lines among `tool_types`; transparent pixels get
/// `background`.
pub(super) fn decode(tool_types: &[&[u8]], background: u32) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let mut lines = tool_types.iter().filter_map(|t| t.strip_prefix(b"IM1="));
    let (header, first) = lines.next().ok_or(fail)?.split_at_checked(5).ok_or(fail)?;
    let transparent = match header[0] {
        b'B' => true,
        b'C' => false,
        _ => return Err(fail),
    };
    let [width, height, colors_high, colors_low] =
        [header[1], header[2], header[3], header[4]].map(|c| c.checked_sub(0x21));
    let (width, height) = (width.ok_or(fail)?, height.ok_or(fail)?);
    let colors = usize::from(colors_high.ok_or(fail)?) << 6 | usize::from(colors_low.ok_or(fail)?);
    if width == 0 || height == 0 || !(1..=MAX_COLORS).contains(&colors) {
        return Err(fail);
    }

    // The palette: lines of 8-bit values until all RGB triplets are read.
    let mut rgb = Vec::new();
    unpack_line(first, 8, colors * 3, &mut rgb)?;
    while rgb.len() < colors * 3 {
        unpack_line(lines.next().ok_or(fail)?, 8, colors * 3, &mut rgb)?;
    }
    let mut palette: Vec<u32> = rgb
        .chunks_exact(3)
        .map(|c| c[0] << 16 | c[1] << 8 | c[2])
        .take(256)
        .collect();
    palette.resize(256, 0);
    if transparent {
        palette[0] = background;
    }

    let bits = colors.next_power_of_two().trailing_zeros().max(1);
    let len = usize::from(width) * usize::from(height);
    let mut pixels = Vec::with_capacity(len);
    for line in lines {
        unpack_line(line, bits, len, &mut pixels)?;
    }
    let indices = pixels
        .iter()
        .map(|&i| u8::try_from(i).map_err(|_| fail))
        .collect::<Result<Vec<u8>, _>>()?;
    Image::from_indexed(width.into(), height.into(), &indices, &palette)
}

/// Appends the `bits`-bit values packed in one line to `out`, up to `limit`
/// values; the bits left over at the end of the line are padding.
fn unpack_line(
    line: &[u8],
    bits: u32,
    limit: usize,
    out: &mut Vec<u32>,
) -> Result<(), DecodeError> {
    let mut acc = 0;
    let mut len = 0;
    let mut push = |bit: u32| {
        acc = acc << 1 | bit;
        len += 1;
        if len == bits {
            if out.len() < limit {
                out.push(acc);
            }
            (acc, len) = (0, 0);
        }
    };
    for &c in line {
        match c {
            0x20..=0x6f | 0xa1..=0xd0 => {
                let value = if c <= 0x6f { c - 0x20 } else { c - 0xa1 + 0x50 };
                (0..7).rev().for_each(|i| push(u32::from(value >> i & 1)));
            }
            0xd1..=0xff => (0..7 * u32::from(c - 0xd0)).for_each(|_| push(0)),
            _ => return Err(DecodeError::Unrecognized),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Packs 7-bit values into NewIcons characters.
    fn chars(values: &[u8]) -> Vec<u8> {
        values
            .iter()
            .map(|&v| if v < 0x50 { v + 0x20 } else { v - 0x50 + 0xa1 })
            .collect()
    }

    #[test]
    fn unpacks_seven_bit_characters_and_zero_runs() {
        let mut out = Vec::new();
        // 0x7f, 0x00 = 1111111 0000000; then a run of 7 zero bits; 4 bits each.
        let mut line = chars(&[0x7f, 0x00]);
        line.push(0xd1);
        unpack_line(&line, 4, 100, &mut out).unwrap();
        // 21 bits: 1111 1110 0000 0000 0000, 1 bit of padding dropped.
        assert_eq!(out, [0xf, 0xe, 0, 0, 0]);
    }

    #[test]
    fn rejects_characters_outside_the_encoding() {
        assert!(unpack_line(&[0x70], 8, 10, &mut Vec::new()).is_err());
    }

    #[test]
    fn decodes_palette_then_pixels_line_by_line() {
        // 2x1, 2 colours (1 bit per pixel), colour 0 transparent.
        let mut first = b"IM1=B\x23\x22!#".to_vec();
        // Palette bytes 12 34 56 ab cd ef as 7-bit groups (48 bits, 7 chars).
        let bits: u64 = 0x1234_56ab_cdef;
        let groups: Vec<u8> = (0..7)
            .map(|i| ((bits << 1) >> (42 - 7 * i) & 0x7f) as u8)
            .collect();
        first.extend(chars(&groups));
        // Pixels 1, 0: bits 10 then padding.
        let pixels = [b"IM1=".as_slice(), &chars(&[0b100_0000])].concat();
        let image = decode(&[b"*** DON'T EDIT", &first, &pixels], 0x959595).unwrap();
        assert_eq!((image.width(), image.height()), (2, 1));
        assert_eq!(image.get(0, 0), 0xabcdef);
        assert_eq!(image.get(1, 0), 0x959595);
    }

    #[test]
    fn missing_pixels_fail() {
        let first = [b"IM1=C\x22\x22!\x22".as_slice(), &chars(&[0; 4])].concat();
        assert!(decode(&[&first], 0).is_err());
    }
}
