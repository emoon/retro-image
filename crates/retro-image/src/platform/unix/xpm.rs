//! X pixmap (`.xpm`), versions 3 and 2.
//!
//! Sources:
//! - XPM Manual, Arnaud Le Hors (<https://www.x.org/docs/XPM/xpm.pdf>),
//!   chapter 2: XPM3 is C source, `/* XPM */`, a `static char *name[] = {`
//!   declaration, then strings separated by commas: the values
//!   `width height ncolors chars_per_pixel [x_hot y_hot] [XPMEXT]`, one string
//!   per color (the pixel characters, then `key color` pairs with the keys `m`,
//!   `s`, `g4`, `g` and `c`; a color is a name, `#` and hex digits, or `None`
//!   for transparent), `height` strings of pixels, and optional extensions.
//! - XPM2, the same without the C wrapper: Wikipedia, "X PixMap"
//!   (<https://en.wikipedia.org/wiki/X_PixMap>): a `! XPM2` line, the values,
//!   the color lines and the pixel lines, unquoted. Checked on the sample
//!   `two-triangles.txt`. XPM1 is not decoded.
//! - Color names: the X11 color database `rgb.txt` (see the notice at the
//!   end of this comment), for the names that occur in the samples plus the
//!   basic eight and `gray`.
//!
//! The `c` color is used, or else `g`, `g4` or `m`. `None` is transparent and
//! becomes the shared transparent-fill gray. A `#` color has one to four hex
//! digits per channel, scaled to 8 bits. A color name outside the small table,
//! a pixel not in the color table, or a missing string fails the decode.
//! Names are matched ignoring case and spaces. Hotspots and extensions are
//! ignored. The file must open with `/* XPM */` or `! XPM2`: that is the
//! signature.
//!
//! Verification: no RECOIL oracle. Output matches ffmpeg's XPM decoder (run
//! as a black box) pixel for pixel on nine of the twelve samples. ffmpeg
//! renders 12-digit `#` colors black and rejects the XPM2 file and the one
//! without `c` keys; Pillow reads only `img.xpm` correctly. Those three match
//! an independent parser written in Python (names from the system `rgb.txt`)
//! and were reviewed by eye (see the divergence file `unix-rasters.tsv`).

// The color values follow the X Consortium rgb.txt
// (https://gitlab.freedesktop.org/xorg/app/rgb), whose license is:
//
// Copyright 1985, 1989, 1998  The Open Group
//
// Permission to use, copy, modify, distribute, and sell this software and its
// documentation for any purpose is hereby granted without fee, provided that
// the above copyright notice appear in all copies and that both that
// copyright notice and this permission notice appear in supporting
// documentation.
//
// The above copyright notice and this permission notice shall be included
// in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
// OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE OPEN GROUP BE LIABLE FOR ANY CLAIM, DAMAGES OR
// OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE,
// ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR
// OTHER DEALINGS IN THE SOFTWARE.
//
// Except as contained in this notice, the name of The Open Group shall
// not be used in advertising or otherwise to promote the sale, use or
// other dealings in this Software without prior written authorization
// from The Open Group.
//
// Copyright (c) 1994, 2008, Oracle and/or its affiliates.
//
// Permission is hereby granted, free of charge, to any person obtaining a
// copy of this software and associated documentation files (the "Software"),
// to deal in the Software without restriction, including without limitation
// the rights to use, copy, modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software, and to permit persons to whom the
// Software is furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice (including the next
// paragraph) shall be included in all copies or substantial portions of the
// Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.  IN NO EVENT SHALL
// THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
// FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS
// IN THE SOFTWARE.

use alloc::borrow::Cow;
use alloc::vec::Vec;

use super::c_source::Tokens;
use super::to_byte;
use crate::image::{TRANSPARENT_FILL, check_size};
use crate::{DecodeError, Image};

const FAIL: DecodeError = DecodeError::Unrecognized;

/// X11 color names, lower case and without spaces.
const NAMES: &[(&[u8], u32)] = &[
    (b"black", 0x00_0000),
    (b"white", 0xff_ffff),
    (b"red", 0xff_0000),
    (b"green", 0x00_ff00),
    (b"blue", 0x00_00ff),
    (b"yellow", 0xff_ff00),
    (b"cyan", 0x00_ffff),
    (b"magenta", 0xff_00ff),
    (b"gray", 0xbe_bebe),
    (b"grey", 0xbe_bebe),
    (b"darkslategray", 0x2f_4f4f),
    (b"dodgerblue", 0x1e_90ff),
    (b"firebrick", 0xb2_2222),
    (b"gainsboro", 0xdc_dcdc),
    (b"gold", 0xff_d700),
    (b"lavender", 0xe6_e6fa),
    (b"lemonchiffon", 0xff_facd),
    (b"limegreen", 0x32_cd32),
    (b"navy", 0x00_0080),
    (b"orange", 0xff_a500),
    (b"palegreen", 0x98_fb98),
    (b"peru", 0xcd_853f),
    (b"seagreen", 0x2e_8b57),
    (b"sienna", 0xa0_522d),
    (b"skyblue", 0x87_ceeb),
    (b"slategray", 0x70_8090),
    (b"tan", 0xd2_b48c),
    (b"tomato", 0xff_6347),
    (b"violet", 0xee_82ee),
    (b"wheat", 0xf5_deb3),
];

pub(super) fn decode_xpm(data: &[u8]) -> Result<Image, DecodeError> {
    let start = data
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .ok_or(FAIL)?;
    let text = &data[start..];
    let lines = if text.starts_with(b"/* XPM */") {
        c_strings(data, start)?
    } else if text.starts_with(b"! XPM2") {
        // The first line is the marker; the rest are unquoted lines.
        text.split(|&b| b == b'\n')
            .skip(1)
            .map(|line| Cow::Borrowed(line.strip_suffix(b"\r").unwrap_or(line)))
            .collect()
    } else {
        return Err(FAIL);
    };
    decode_lines(&lines)
}

/// The strings of an XPM3 array, unescaped.
fn c_strings(data: &[u8], start: usize) -> Result<Vec<Cow<'_, [u8]>>, DecodeError> {
    let mut tokens = Tokens { data, pos: start };
    // Skip the declaration up to the opening brace.
    while tokens.next().ok_or(FAIL)? != b"{" {}
    let mut strings = Vec::new();
    loop {
        tokens.skip_blank();
        match data.get(tokens.pos) {
            Some(b'"') => {
                tokens.pos += 1;
                strings.push(string(&mut tokens)?);
            }
            Some(b',') => tokens.pos += 1,
            Some(b'}') | None => return Ok(strings),
            Some(_) => return Err(FAIL),
        }
    }
}

/// The string after an opening quote, to its closing quote. `\\` and `\"`
/// stand for the character after the backslash.
fn string<'a>(tokens: &mut Tokens<'a>) -> Result<Cow<'a, [u8]>, DecodeError> {
    let rest = &tokens.data[tokens.pos..];
    let mut end = 0;
    let mut escaped = false;
    loop {
        match *rest.get(end).ok_or(FAIL)? {
            b'"' => break,
            b'\\' => {
                escaped = true;
                end += 2;
            }
            _ => end += 1,
        }
    }
    let raw = rest.get(..end).ok_or(FAIL)?;
    tokens.pos += end + 1;
    if !escaped {
        return Ok(Cow::Borrowed(raw));
    }
    let mut bytes = Vec::with_capacity(raw.len());
    let mut chars = raw.iter();
    while let Some(&byte) = chars.next() {
        bytes.push(if byte == b'\\' {
            *chars.next().ok_or(FAIL)?
        } else {
            byte
        });
    }
    Ok(Cow::Owned(bytes))
}

/// Values, color and pixel lines to a picture.
fn decode_lines(lines: &[Cow<[u8]>]) -> Result<Image, DecodeError> {
    let values = lines.first().ok_or(FAIL)?;
    let mut numbers = values
        .split(|b| b.is_ascii_whitespace())
        .filter(|t| !t.is_empty())
        .map(|t| core::str::from_utf8(t).ok()?.parse::<usize>().ok());
    let mut next = || numbers.next().flatten().ok_or(FAIL);
    let (width, height, colors, per_pixel) = (next()?, next()?, next()?, next()?);
    check_size(width, height)?;
    if colors == 0 || !(1..=8).contains(&per_pixel) {
        return Err(FAIL);
    }
    let first_row = colors.checked_add(1).ok_or(FAIL)?;
    let rows = lines
        .get(first_row..first_row.checked_add(height).ok_or(FAIL)?)
        .ok_or(FAIL)?;
    let row_len = width.checked_mul(per_pixel).ok_or(FAIL)?;
    if rows.iter().any(|row| row.len() < row_len) {
        return Err(FAIL);
    }
    let mut table: Vec<(u64, u32)> = lines[1..1 + colors]
        .iter()
        .map(|line| color_entry(line, per_pixel))
        .collect::<Result<_, _>>()?;
    table.sort_by_key(|&(key, _)| key);

    let mut image = Image::new(width as u32, height as u32);
    for (y, row) in rows.iter().enumerate() {
        for (x, pixel) in row.chunks_exact(per_pixel).take(width).enumerate() {
            let key = pack(pixel);
            let at = table
                .binary_search_by_key(&key, |&(k, _)| k)
                .map_err(|_| FAIL)?;
            image.set(x as u32, y as u32, table[at].1);
        }
    }
    Ok(image)
}

/// Up to eight pixel characters as one number.
fn pack(chars: &[u8]) -> u64 {
    chars.iter().fold(0, |key, &c| key << 8 | u64::from(c))
}

/// A color line: the pixel characters, then `key color` pairs.
fn color_entry(line: &[u8], per_pixel: usize) -> Result<(u64, u32), DecodeError> {
    let key = pack(line.get(..per_pixel).ok_or(FAIL)?);
    let words: Vec<&[u8]> = line[per_pixel..]
        .split(|&b| b == b' ' || b == b'\t')
        .filter(|w| !w.is_empty())
        .collect();
    // The color visual wins; the others serve files that have no `c`.
    let value = [&b"c"[..], b"g", b"g4", b"m"]
        .iter()
        .find_map(|wanted| value_of(&words, wanted))
        .ok_or(FAIL)?;
    Ok((key, color(value)?))
}

/// The words after `key` up to the next key, so names may hold spaces.
fn value_of<'a>(words: &'a [&'a [u8]], key: &[u8]) -> Option<&'a [&'a [u8]]> {
    let is_key = |w: &[u8]| matches!(w, b"m" | b"s" | b"g4" | b"g" | b"c");
    let start = words.iter().position(|&w| w == key)? + 1;
    let len = words[start..].iter().position(|&w| is_key(w));
    Some(&words[start..start + len.unwrap_or(words.len() - start)])
}

/// A color given as words: `None`, a name or `#` and hex digits.
fn color(words: &[&[u8]]) -> Result<u32, DecodeError> {
    let name: Vec<u8> = words
        .iter()
        .flat_map(|w| w.iter())
        .map(u8::to_ascii_lowercase)
        .collect();
    if name == b"none" {
        return Ok(TRANSPARENT_FILL);
    }
    if let Some(digits) = name.strip_prefix(b"#") {
        let per_channel = digits.len() / 3;
        if digits.len() % 3 != 0 || !(1..=4).contains(&per_channel) {
            return Err(FAIL);
        }
        let max = (1u32 << (4 * per_channel)) - 1;
        let mut color = 0;
        for channel in digits.chunks_exact(per_channel) {
            let text = core::str::from_utf8(channel).map_err(|_| FAIL)?;
            let value = u32::from_str_radix(text, 16).map_err(|_| FAIL)?;
            color = color << 8 | u32::from(to_byte(value, max));
        }
        return Ok(color);
    }
    NAMES
        .iter()
        .find(|(known, _)| *known == name)
        .map(|&(_, color)| color)
        .ok_or(FAIL)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xpm3_colors_keys_and_transparency() {
        let image = decode_xpm(
            b"/* XPM */\nstatic char *t[] = {\n/* w h n cpp */\n\"3 2 4 1 0 0\",\n\
              \"  c None\",\n\"a m white s x c #FF0000\",\n\"b c SeaGreen\",\n\
              \"\\\\ c #fff\",\n\"ab\\\\\",\n\" a \"\n};",
        )
        .unwrap();
        assert_eq!((image.width(), image.height()), (3, 2));
        let row0 = [0, 1, 2].map(|x| image.get(x, 0));
        assert_eq!(row0, [0xff0000, 0x2e8b57, 0xffffff]);
        assert_eq!(image.get(0, 1), TRANSPARENT_FILL);
        assert_eq!(image.get(1, 1), 0xff0000);
    }

    #[test]
    fn xpm2_has_unquoted_lines_and_two_character_pixels() {
        let image =
            decode_xpm(b"! XPM2\r\n2 1 2 2\r\n.. c #000000\r\n#  c #00ffff\r\n# ..\r\n").unwrap();
        assert_eq!((image.get(0, 0), image.get(1, 0)), (0x00ffff, 0));
    }

    #[test]
    fn hex_depths_scale_and_failures_are_errors() {
        let hex = |digits: &str| color(&[digits.as_bytes()]);
        assert_eq!(hex("#fff"), Ok(0xffffff));
        assert_eq!(hex("#1234"), Err(FAIL));
        assert_eq!(hex("#ffff0000ffff"), Ok(0xff00ff));
        assert_eq!(hex("#8000"), Err(FAIL));
        assert_eq!(color(&[b"Dark", b"Slate", b"Gray"]), Ok(0x2f4f4f));
        assert_eq!(color(&[b"mauve"]), Err(FAIL));
        // A pixel missing from the color table, and too few rows.
        let bad = b"/* XPM */\nstatic char *t[] = {\n\"1 1 1 1\",\n\"a c red\",\n\"b\"\n};";
        assert!(decode_xpm(bad).is_err());
        let few = b"/* XPM */\nstatic char *t[] = {\n\"1 2 1 1\",\n\"a c red\",\n\"a\"\n};";
        assert!(decode_xpm(few).is_err());
        assert!(decode_xpm(b"/* XPM */\nstatic char *t[] = {\n\"60000 60000 1 1\"};").is_err());
    }
}
