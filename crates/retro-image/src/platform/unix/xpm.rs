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
//!   end of this comment): every name without a number, plus `gray0` to
//!   `gray100` (and `grey`) worked out from the rule that `rgb.txt` follows.
//!   The numbered variants of other colors (`red3`, `snow2`) are not included.
//!
//! The `c` color is used, or else `g`, `g4` or `m`. `None` is transparent. A
//! `#` color has one to four hex digits per channel. They are the top bits of a
//! 16-bit value, so `#fff` is 0xf0f0f0 and `#3a7` is the same as
//! `#3000a0007000`; the high byte is the 8-bit value (X(7),
//! <https://www.x.org/releases/current/doc/man/man7/X.7.xhtml>, "Color Names").
//! A color name outside those, a pixel not in the color table, or a missing
//! string fails the decode. Names are matched ignoring case and spaces.
//! Hotspots and extensions are ignored. The file must open with `/* XPM */` or
//! `! XPM2`: that is the signature.
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
use crate::image::{CLEAR, check_size};
use crate::{DecodeError, Image};

const FAIL: DecodeError = DecodeError::Invalid;

/// X11 color names from `rgb.txt` that have no number (`red3` and `gray50` do),
/// lower case and without spaces, sorted for binary search.
const NAMES: &[(&[u8], u32)] = &[
    (b"aliceblue", 0xf0_f8ff),
    (b"antiquewhite", 0xfa_ebd7),
    (b"aquamarine", 0x7f_ffd4),
    (b"azure", 0xf0_ffff),
    (b"beige", 0xf5_f5dc),
    (b"bisque", 0xff_e4c4),
    (b"black", 0x00_0000),
    (b"blanchedalmond", 0xff_ebcd),
    (b"blue", 0x00_00ff),
    (b"blueviolet", 0x8a_2be2),
    (b"brown", 0xa5_2a2a),
    (b"burlywood", 0xde_b887),
    (b"cadetblue", 0x5f_9ea0),
    (b"chartreuse", 0x7f_ff00),
    (b"chocolate", 0xd2_691e),
    (b"coral", 0xff_7f50),
    (b"cornflowerblue", 0x64_95ed),
    (b"cornsilk", 0xff_f8dc),
    (b"cyan", 0x00_ffff),
    (b"darkblue", 0x00_008b),
    (b"darkcyan", 0x00_8b8b),
    (b"darkgoldenrod", 0xb8_860b),
    (b"darkgray", 0xa9_a9a9),
    (b"darkgreen", 0x00_6400),
    (b"darkgrey", 0xa9_a9a9),
    (b"darkkhaki", 0xbd_b76b),
    (b"darkmagenta", 0x8b_008b),
    (b"darkolivegreen", 0x55_6b2f),
    (b"darkorange", 0xff_8c00),
    (b"darkorchid", 0x99_32cc),
    (b"darkred", 0x8b_0000),
    (b"darksalmon", 0xe9_967a),
    (b"darkseagreen", 0x8f_bc8f),
    (b"darkslateblue", 0x48_3d8b),
    (b"darkslategray", 0x2f_4f4f),
    (b"darkslategrey", 0x2f_4f4f),
    (b"darkturquoise", 0x00_ced1),
    (b"darkviolet", 0x94_00d3),
    (b"debianred", 0xd7_0751),
    (b"deeppink", 0xff_1493),
    (b"deepskyblue", 0x00_bfff),
    (b"dimgray", 0x69_6969),
    (b"dimgrey", 0x69_6969),
    (b"dodgerblue", 0x1e_90ff),
    (b"firebrick", 0xb2_2222),
    (b"floralwhite", 0xff_faf0),
    (b"forestgreen", 0x22_8b22),
    (b"gainsboro", 0xdc_dcdc),
    (b"ghostwhite", 0xf8_f8ff),
    (b"gold", 0xff_d700),
    (b"goldenrod", 0xda_a520),
    (b"gray", 0xbe_bebe),
    (b"green", 0x00_ff00),
    (b"greenyellow", 0xad_ff2f),
    (b"grey", 0xbe_bebe),
    (b"honeydew", 0xf0_fff0),
    (b"hotpink", 0xff_69b4),
    (b"indianred", 0xcd_5c5c),
    (b"ivory", 0xff_fff0),
    (b"khaki", 0xf0_e68c),
    (b"lavender", 0xe6_e6fa),
    (b"lavenderblush", 0xff_f0f5),
    (b"lawngreen", 0x7c_fc00),
    (b"lemonchiffon", 0xff_facd),
    (b"lightblue", 0xad_d8e6),
    (b"lightcoral", 0xf0_8080),
    (b"lightcyan", 0xe0_ffff),
    (b"lightgoldenrod", 0xee_dd82),
    (b"lightgoldenrodyellow", 0xfa_fad2),
    (b"lightgray", 0xd3_d3d3),
    (b"lightgreen", 0x90_ee90),
    (b"lightgrey", 0xd3_d3d3),
    (b"lightpink", 0xff_b6c1),
    (b"lightsalmon", 0xff_a07a),
    (b"lightseagreen", 0x20_b2aa),
    (b"lightskyblue", 0x87_cefa),
    (b"lightslateblue", 0x84_70ff),
    (b"lightslategray", 0x77_8899),
    (b"lightslategrey", 0x77_8899),
    (b"lightsteelblue", 0xb0_c4de),
    (b"lightyellow", 0xff_ffe0),
    (b"limegreen", 0x32_cd32),
    (b"linen", 0xfa_f0e6),
    (b"magenta", 0xff_00ff),
    (b"maroon", 0xb0_3060),
    (b"mediumaquamarine", 0x66_cdaa),
    (b"mediumblue", 0x00_00cd),
    (b"mediumorchid", 0xba_55d3),
    (b"mediumpurple", 0x93_70db),
    (b"mediumseagreen", 0x3c_b371),
    (b"mediumslateblue", 0x7b_68ee),
    (b"mediumspringgreen", 0x00_fa9a),
    (b"mediumturquoise", 0x48_d1cc),
    (b"mediumvioletred", 0xc7_1585),
    (b"midnightblue", 0x19_1970),
    (b"mintcream", 0xf5_fffa),
    (b"mistyrose", 0xff_e4e1),
    (b"moccasin", 0xff_e4b5),
    (b"navajowhite", 0xff_dead),
    (b"navy", 0x00_0080),
    (b"navyblue", 0x00_0080),
    (b"oldlace", 0xfd_f5e6),
    (b"olivedrab", 0x6b_8e23),
    (b"orange", 0xff_a500),
    (b"orangered", 0xff_4500),
    (b"orchid", 0xda_70d6),
    (b"palegoldenrod", 0xee_e8aa),
    (b"palegreen", 0x98_fb98),
    (b"paleturquoise", 0xaf_eeee),
    (b"palevioletred", 0xdb_7093),
    (b"papayawhip", 0xff_efd5),
    (b"peachpuff", 0xff_dab9),
    (b"peru", 0xcd_853f),
    (b"pink", 0xff_c0cb),
    (b"plum", 0xdd_a0dd),
    (b"powderblue", 0xb0_e0e6),
    (b"purple", 0xa0_20f0),
    (b"red", 0xff_0000),
    (b"rosybrown", 0xbc_8f8f),
    (b"royalblue", 0x41_69e1),
    (b"saddlebrown", 0x8b_4513),
    (b"salmon", 0xfa_8072),
    (b"sandybrown", 0xf4_a460),
    (b"seagreen", 0x2e_8b57),
    (b"seashell", 0xff_f5ee),
    (b"sienna", 0xa0_522d),
    (b"skyblue", 0x87_ceeb),
    (b"slateblue", 0x6a_5acd),
    (b"slategray", 0x70_8090),
    (b"slategrey", 0x70_8090),
    (b"snow", 0xff_fafa),
    (b"springgreen", 0x00_ff7f),
    (b"steelblue", 0x46_82b4),
    (b"tan", 0xd2_b48c),
    (b"thistle", 0xd8_bfd8),
    (b"tomato", 0xff_6347),
    (b"turquoise", 0x40_e0d0),
    (b"violet", 0xee_82ee),
    (b"violetred", 0xd0_2090),
    (b"wheat", 0xf5_deb3),
    (b"white", 0xff_ffff),
    (b"whitesmoke", 0xf5_f5f5),
    (b"yellow", 0xff_ff00),
    (b"yellowgreen", 0x9a_cd32),
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

    let mut image = Image::new(width as u32, height as u32)?;
    for (y, row) in rows.iter().enumerate() {
        for (x, pixel) in row.chunks_exact(per_pixel).take(width).enumerate() {
            let key = pack(pixel);
            let at = table
                .binary_search_by_key(&key, |&(k, _)| k)
                .map_err(|_| FAIL)?;
            image.set_argb(x as u32, y as u32, table[at].1);
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
    let argb = if is_none(value) {
        CLEAR
    } else {
        0xff00_0000 | color(value)?
    };
    Ok((key, argb))
}

/// The words after `key` up to the next key, so names may hold spaces.
fn value_of<'a>(words: &'a [&'a [u8]], key: &[u8]) -> Option<&'a [&'a [u8]]> {
    let is_key = |w: &[u8]| matches!(w, b"m" | b"s" | b"g4" | b"g" | b"c");
    let start = words.iter().position(|&w| w == key)? + 1;
    let len = words[start..].iter().position(|&w| is_key(w));
    Some(&words[start..start + len.unwrap_or(words.len() - start)])
}

/// Whether the color words say `None`, which is transparent.
fn is_none(words: &[&[u8]]) -> bool {
    let letters = words.iter().flat_map(|w| w.iter());
    letters
        .map(u8::to_ascii_lowercase)
        .eq(b"none".iter().copied())
}

/// A color given as words: a name or `#` and hex digits.
fn color(words: &[&[u8]]) -> Result<u32, DecodeError> {
    let name: Vec<u8> = words
        .iter()
        .flat_map(|w| w.iter())
        .map(u8::to_ascii_lowercase)
        .collect();
    if let Some(digits) = name.strip_prefix(b"#") {
        let per_channel = digits.len() / 3;
        if digits.len() % 3 != 0 || !(1..=4).contains(&per_channel) {
            return Err(FAIL);
        }
        let mut color = 0;
        for channel in digits.chunks_exact(per_channel) {
            let text = core::str::from_utf8(channel).map_err(|_| FAIL)?;
            let value = u32::from_str_radix(text, 16).map_err(|_| FAIL)?;
            // The digits are the top bits of a 16-bit value, and the high
            // byte of that is the 8-bit value.
            color = color << 8 | (value << (16 - 4 * per_channel)) >> 8;
        }
        return Ok(color);
    }
    named(&name).ok_or(FAIL)
}

/// The color of an X11 name: `grayN` and `greyN` for N up to 100, or an entry
/// of [`NAMES`].
fn named(name: &[u8]) -> Option<u32> {
    let level = name
        .strip_prefix(b"gray")
        .or_else(|| name.strip_prefix(b"grey"))
        .filter(|digits| !digits.is_empty() && digits.len() <= 3)
        .filter(|digits| {
            digits.iter().all(u8::is_ascii_digit) && (digits[0] != b'0' || digits.len() == 1)
        })
        .and_then(|digits| core::str::from_utf8(digits).ok()?.parse::<u32>().ok())
        .filter(|&level| level <= 100);
    if let Some(level) = level {
        // rgb.txt rounds level * 2.55 in floating point: .5 falls to the
        // lower value at 50 and 90 (127 and 229) but rises at 10 (26).
        return Some((f64::from(level) * 2.55 + 0.5) as u32 * 0x01_0101);
    }
    let at = NAMES
        .binary_search_by(|(known, _)| (*known).cmp(name))
        .ok()?;
    Some(NAMES[at].1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xpm3_colors_keys_and_transparency() {
        let image = decode_xpm(
            b"/* XPM */\nstatic char *t[] = {\n/* w h n cpp */\n\"3 2 4 1 0 0\",\n\
              \"  c None\",\n\"a m white s x c #FF0000\",\n\"b c SeaGreen\",\n\
              \"\\\\ c #ffffff\",\n\"ab\\\\\",\n\" a \"\n};",
        )
        .unwrap();
        assert_eq!((image.width(), image.height()), (3, 2));
        let row0 = [0, 1, 2].map(|x| image.get(x, 0));
        assert_eq!(row0, [0xff0000, 0x2e8b57, 0xffffff]);
        assert_eq!(image.get_argb(0, 1), CLEAR);
        assert_eq!(image.get(1, 1), 0xff0000);
    }

    #[test]
    fn xpm2_has_unquoted_lines_and_two_character_pixels() {
        let image =
            decode_xpm(b"! XPM2\r\n2 1 2 2\r\n.. c #000000\r\n#  c #00ffff\r\n# ..\r\n").unwrap();
        assert_eq!((image.get(0, 0), image.get(1, 0)), (0x00ffff, 0));
    }

    #[test]
    fn gray_levels_and_more_color_names() {
        let name = |text: &str| color(&[text.as_bytes()]);
        // grayN follows rgb.txt, whose 2.55 factor makes .5 round down at
        // 50 and 90 but up at 10.
        assert_eq!(name("gray0"), Ok(0x000000));
        assert_eq!(name("gray10"), Ok(0x1a1a1a));
        assert_eq!(name("gray50"), Ok(0x7f7f7f));
        assert_eq!(name("Grey51"), Ok(0x828282));
        assert_eq!(name("gray90"), Ok(0xe5e5e5));
        assert_eq!(name("gray100"), Ok(0xffffff));
        assert_eq!(name("gray101"), Err(FAIL));
        assert_eq!(name("gray050"), Err(FAIL));
        assert_eq!(name("gray"), Ok(0xbebebe));
        assert_eq!(name("lightblue"), Ok(0xadd8e6));
        assert_eq!(color(&[b"Dark", b"Green"]), Ok(0x006400));
        assert!(NAMES.windows(2).all(|pair| pair[0].0 < pair[1].0), "sorted");
    }

    #[test]
    fn hex_digits_are_top_bits_and_failures_are_errors() {
        let hex = |digits: &str| color(&[digits.as_bytes()]);
        // Fewer than four digits are the top bits of a 16-bit value.
        assert_eq!(hex("#fff"), Ok(0xf0f0f0));
        assert_eq!(hex("#3a7"), Ok(0x30a070));
        assert_eq!(hex("#fffeeeddd"), Ok(0xffeedd));
        assert_eq!(hex("#abcdef"), Ok(0xabcdef));
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
