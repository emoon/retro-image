//! DEC VT330/VT340 sixel graphics (`.six`, `.sixel`).
//!
//! Sources:
//! - VT330/VT340 Programmer Reference Manual, chapter 14, Sixel Graphics
//!   (<https://vt100.net/docs/vt3xx-gp/chapter14.html>, Wayback
//!   `https://web.archive.org/web/2023/http://vt100.net/docs/vt3xx-gp/chapter14.html`):
//!   the device control string `DCS P1 ; P2 ; P3 q ... ST`, characters `?` to
//!   `~` as six pixels with the least significant bit on top, `!` repeat,
//!   `"` raster attributes, `#` color select and define (HLS or RGB, RGB
//!   channels 0 to 100 percent), `$` carriage return, `-` new line, and P2
//!   (0 or 2: unset pixels get the background color, 1: they stay as they
//!   were).
//! - The same manual, chapter 2, Table 2-3 (VT340 default color map, also the
//!   source of the 16 default registers below; hue 0 degrees is blue, 120 red
//!   and 240 green: <https://vt100.net/docs/vt3xx-gp/chapter2.html>).
//! - libsixel's `src/fromsixel.c` (<https://github.com/saitoha/libsixel>, MIT
//!   license, notice below) for what viewers do where the manual is silent:
//!   the current color before any `#` is register 15, a color number above
//!   255 is clamped to 255, HLS results are cut to whole percent before they
//!   become 8-bit, and the picture is as large as its drawn pixels and the
//!   raster attributes' width and height together. The same tool,
//!   `sixel2png`, built from that source and run as a black box, is the oracle
//!   for the samples.
//!
//! Choices of this crate: pixels keep their register number and take the
//! final color of that register, as on the terminal's color map; registers
//! 16 to 255 start black; pixels nothing drew show register 0, or the shared
//! transparent fill when P2 is 1; the pixel aspect (P1 and `Pan`:`Pad`) is not
//! applied, because the common files say `"1;2` for plain square-pixel
//! pictures; only the first control string is read, and it must be
//! terminated (`ESC \` or `0x9c`), which is also what content detection
//! relies on. Pictures are capped at 4096 x 4096. A file of VMS
//! variable-length records (the sample `test.six`: each record starts with a
//! 16-bit length) is joined first, which no other viewer does.

// Parts of this file follow libsixel's src/fromsixel.c
// (https://github.com/saitoha/libsixel):
//
// The MIT License (MIT)
//
// Copyright (c) 2014-2016 Hayaki Saito
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
//
// That file is derived from the original "sixel" by kmiya@culti, distributed
// under a very permissive license.

use alloc::vec;
use alloc::vec::Vec;

use crate::bytes::le16;
use crate::image::{TRANSPARENT_FILL, check_size};
use crate::{DecodeError, Format, Image};

pub(super) static FORMATS: &[Format] =
    &[Format::new("DEC VT340", "Sixel", &["six", "sixel"], decode_sixel).signature()];

const FAIL: DecodeError = DecodeError::Unrecognized;
const ESC: u8 = 0x1b;
const C1_DCS: u8 = 0x90;
const C1_ST: u8 = 0x9c;
/// How far into the file the control string may start: files begin with a
/// few terminal setup sequences or a line of text.
const DCS_SEARCH_LEN: usize = 64;
const REGISTERS: usize = 256;
/// The register a picture draws in until it selects one.
const FIRST_REGISTER: usize = 15;
const MAX_SIDE: usize = 4096;
/// Largest number a parameter can hold; bigger ones stay at it.
const MAX_PARAM: usize = 1 << 20;
/// Longest record of a file of VMS records.
const MAX_RECORD: usize = 4096;
/// Pixels written across the whole picture, so that a file overpainting the
/// same area again and again cannot take forever.
const MAX_WRITES: usize = 1 << 28;
/// A pixel no sixel has drawn.
const UNSET: u16 = u16::MAX;

/// The VT340 default color map (Table 2-3), as percentages of red, green, blue.
const DEFAULT_REGISTERS: [[u32; 3]; 16] = [
    [0, 0, 0],
    [20, 20, 80],
    [80, 13, 13],
    [20, 80, 20],
    [80, 20, 80],
    [20, 80, 80],
    [80, 80, 20],
    [53, 53, 53],
    [26, 26, 26],
    [33, 33, 60],
    [60, 26, 26],
    [33, 60, 33],
    [60, 33, 60],
    [33, 60, 60],
    [60, 60, 33],
    [80, 80, 80],
];

/// One thing the sixel data does, in order.
enum Op {
    /// `"Pan;Pad;Ph;Pv`: the size the picture declares (0 if absent).
    Raster { width: usize, height: usize },
    /// `#Pc;Pu;Px;Py;Pz`: register `register` gets `color` (`0xRRGGBB`).
    Define { register: usize, color: u32 },
    /// The six pixels of `bits` (top is the lowest bit) in `register`,
    /// `count` columns wide, with the band's top left at (`x`, `y`).
    Sixels {
        x: usize,
        y: usize,
        bits: u8,
        count: usize,
        register: usize,
    },
}

fn decode_sixel(data: &[u8]) -> Result<Image, DecodeError> {
    let joined = join_records(data);
    let data = joined.as_deref().unwrap_or(data);
    let (p2, body) = control_string(data)?;
    // First pass: how large the picture is.
    let (mut width, mut height) = (0, 0);
    scan(body, |op| {
        match op {
            Op::Raster {
                width: w,
                height: h,
            } => {
                if w > 0 {
                    width = width.max(w);
                }
                if h > 0 {
                    height = height.max(h);
                }
            }
            Op::Sixels {
                x, y, bits, count, ..
            } => {
                width = width.max(x + count);
                height = height.max(y + (8 - bits.leading_zeros()) as usize);
            }
            _ => {}
        }
        if width > MAX_SIDE || height > MAX_SIDE {
            return Err(FAIL);
        }
        Ok(())
    })?;
    check_size(width, height)?;
    // Second pass: draw register numbers, with the registers' final colors.
    let mut registers = default_registers();
    let mut pixels = vec![UNSET; width * height];
    let mut writes = 0usize;
    scan(body, |op| {
        match op {
            Op::Define { register, color } => registers[register] = color,
            Op::Sixels {
                x,
                y,
                bits,
                count,
                register,
            } => {
                for row in (0..6).filter(|row| bits >> row & 1 != 0) {
                    writes += count;
                    let start = (y + row) * width + x;
                    pixels
                        .get_mut(start..start + count)
                        .ok_or(FAIL)?
                        .fill(register as u16);
                }
                if writes > MAX_WRITES {
                    return Err(FAIL);
                }
            }
            Op::Raster { .. } => {}
        }
        Ok(())
    })?;
    let unset = if p2 == 1 {
        TRANSPARENT_FILL
    } else {
        registers[0]
    };
    let colors = pixels.iter().map(|&register| match register {
        UNSET => unset,
        register => registers[usize::from(register)],
    });
    Ok(Image::from_colors(width as u32, height as u32, colors))
}

/// The data of a file made of VMS variable-length records (a 16-bit length,
/// the bytes, a pad byte to an even position), joined; `None` unless the
/// records cover the whole file. Their length bytes would otherwise land in
/// the sixel data. Records are short, which no sixel text file's first two
/// bytes (an escape or a sixel character) are mistaken for.
fn join_records(data: &[u8]) -> Option<Vec<u8>> {
    let mut joined = Vec::with_capacity(data.len());
    let (mut at, mut records) = (0, 0);
    while at < data.len() {
        let length = usize::from(le16(data, at)?);
        if length > MAX_RECORD {
            return None;
        }
        joined.extend_from_slice(data.get(at + 2..at + 2 + length)?);
        // The pad of an odd last record may be missing.
        at += 2 + length + length % 2;
        records += 1;
    }
    (records > 1).then_some(joined)
}

/// P2 and the data of the first sixel control string, up to its terminator.
fn control_string(data: &[u8]) -> Result<(usize, &[u8]), DecodeError> {
    let head = &data[..data.len().min(DCS_SEARCH_LEN)];
    let start = (0..head.len())
        .find(|&i| head[i] == C1_DCS || (head[i] == ESC && head.get(i + 1) == Some(&b'P')))
        .ok_or(FAIL)?;
    let params_at = start + if data[start] == ESC { 2 } else { 1 };
    let ([_, p2, _], _, end) = parameters::<3>(data, params_at);
    if data.get(end) != Some(&b'q') {
        return Err(FAIL);
    }
    let body = &data[end + 1..];
    let stop = (0..body.len())
        .find(|&i| body[i] == C1_ST || body[i] == ESC)
        .ok_or(FAIL)?;
    // Terminal captures can hold other escape sequences (a cursor move) before
    // the terminator; they end the picture's data, and the string ends at the
    // next `\` or `0x9c`.
    let terminated = body[stop] == C1_ST || body[stop..].iter().any(|&b| b == b'\\' || b == C1_ST);
    if !terminated {
        return Err(FAIL);
    }
    Ok((p2, &body[..stop]))
}

/// Up to `N` numbers separated by `;` from `at` on: the numbers (0 where one is
/// left out), how many there were (at least 1), and the index of the first byte
/// after the list.
fn parameters<const N: usize>(data: &[u8], at: usize) -> ([usize; N], usize, usize) {
    let mut values = [0; N];
    let (mut count, mut current, mut end) = (0, 0usize, at);
    while let Some(&byte) = data.get(end) {
        match byte {
            b'0'..=b'9' => current = (current * 10 + usize::from(byte - b'0')).min(MAX_PARAM),
            b';' => {
                if let Some(value) = values.get_mut(count) {
                    *value = current;
                }
                count += 1;
                current = 0;
            }
            _ => break,
        }
        end += 1;
    }
    if let Some(value) = values.get_mut(count) {
        *value = current;
    }
    (values, count + 1, end)
}

/// Runs the sixel data through `op`, which can stop it with an error.
fn scan(body: &[u8], mut op: impl FnMut(Op) -> Result<(), DecodeError>) -> Result<(), DecodeError> {
    let (mut x, mut y) = (0usize, 0usize);
    let mut register = FIRST_REGISTER;
    let mut repeat = 1;
    let mut at = 0;
    while let Some(&byte) = body.get(at) {
        at += 1;
        match byte {
            b'"' => {
                let ([_, _, width, height], _, end) = parameters::<4>(body, at);
                at = end;
                op(Op::Raster { width, height })?;
            }
            b'!' => {
                let ([count], _, end) = parameters::<1>(body, at);
                at = end;
                repeat = count.max(1);
            }
            b'#' => {
                let ([number, system, a, b, c], count, end) = parameters::<5>(body, at);
                at = end;
                register = number.min(REGISTERS - 1);
                let color = match system {
                    1 => Some(hls(a, b, c)),
                    2 => Some(percent_rgb([a, b, c])),
                    _ => None,
                };
                if let (Some(color), true) = (color, count > 4) {
                    op(Op::Define { register, color })?;
                }
            }
            b'$' => x = 0,
            b'-' => {
                x = 0;
                y = (y + 6).min(2 * MAX_SIDE);
            }
            b'?'..=b'~' => {
                let bits = byte - b'?';
                if bits != 0 {
                    op(Op::Sixels {
                        x,
                        y,
                        bits,
                        count: repeat,
                        register,
                    })?;
                }
                x = (x + repeat).min(2 * MAX_SIDE);
                repeat = 1;
            }
            _ => {}
        }
    }
    Ok(())
}

fn default_registers() -> [u32; REGISTERS] {
    let mut registers = [0; REGISTERS];
    for (register, percent) in registers.iter_mut().zip(DEFAULT_REGISTERS) {
        *register = percent_rgb(percent.map(|v| v as usize));
    }
    registers
}

/// `0xRRGGBB` from red, green and blue percentages (more than 100 is 100).
fn percent_rgb(percent: [usize; 3]) -> u32 {
    let [r, g, b] = percent.map(|p| (p.min(100) as u32 * 255 + 50) / 100);
    r << 16 | g << 8 | b
}

/// A color from hue (degrees; 0 is blue, 120 red, 240 green), lightness and
/// saturation (percent), cut to whole percent before becoming 8-bit.
fn hls(hue: usize, lightness: usize, saturation: usize) -> u32 {
    let hue = (hue.min(360) + 240) % 360;
    let lightness = lightness.min(100) as f64;
    let saturation = saturation.min(100) as f64;
    // Half the chroma: the color runs from `lightness - half` to `+ half`.
    let half = saturation * (1.0 - (2.0 * lightness / 100.0 - 1.0).abs()) / 2.0;
    let (min, max) = (lightness - half, lightness + half);
    let rise = |from: usize| min + (max - min) * from as f64 / 60.0;
    let (r, g, b) = match hue / 60 {
        0 => (max, rise(hue), min),
        1 => (rise(120 - hue), max, min),
        2 => (min, max, rise(hue - 120)),
        3 => (min, rise(240 - hue), max),
        4 => (rise(hue - 240), min, max),
        _ => (max, min, rise(360 - hue)),
    };
    percent_rgb([r as usize, g as usize, b as usize])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sixel(p2: &str, body: &str) -> Vec<u8> {
        let mut data = alloc::format!("\x1bP0;{p2};0q{body}").into_bytes();
        data.extend_from_slice(b"\x1b\\");
        data
    }

    #[test]
    fn a_sixel_is_a_column_of_six_with_the_low_bit_on_top() {
        // Bits 0, 1 and 5 of 'b' (0x62 - 0x3f = 0b100011), in register 2.
        let image = decode_sixel(&sixel("0", "#2;2;100;0;0#2b")).unwrap();
        assert_eq!((image.width(), image.height()), (1, 6));
        let column: Vec<u32> = (0..6).map(|y| image.get(0, y)).collect();
        // Rows 2 to 4 were not drawn: register 0, black.
        assert_eq!(column, [0xff0000, 0xff0000, 0, 0, 0, 0xff0000]);
    }

    #[test]
    fn repeats_new_lines_and_carriage_returns_place_the_bands() {
        // Two red columns, back to the left edge to draw one blue over the
        // first, a new band with three green sixels.
        let body = "#1;2;100;0;0#2;2;0;0;100#3;2;0;100;0#1!2~$#2@-#3!3F";
        let image = decode_sixel(&sixel("0", body)).unwrap();
        assert_eq!((image.width(), image.height()), (3, 9));
        assert_eq!(image.get(0, 0), 0x0000ff);
        assert_eq!(image.get(0, 1), 0xff0000);
        assert_eq!(image.get(1, 0), 0xff0000);
        assert_eq!(image.get(2, 5), 0);
        assert_eq!(image.get(2, 6), 0x00ff00);
        assert_eq!(image.get(2, 8), 0x00ff00);
    }

    #[test]
    fn raster_attributes_size_the_picture_and_p2_one_leaves_it_transparent() {
        let data = sixel("1", "\"1;1;4;7#1;2;100;100;100#1~");
        let image = decode_sixel(&data).unwrap();
        assert_eq!((image.width(), image.height()), (4, 7));
        assert_eq!(image.get(0, 0), 0xffffff);
        assert_eq!(image.get(1, 0), TRANSPARENT_FILL);
        assert_eq!(image.get(0, 6), TRANSPARENT_FILL);
        // P2 = 0: the background is register 0, redefined here.
        let data = sixel("0", "\"1;1;2;1#0;2;0;0;100#1;2;100;0;0#1~");
        assert_eq!(decode_sixel(&data).unwrap().get(1, 0), 0x0000ff);
    }

    #[test]
    fn registers_start_as_the_vt340_map_and_the_hue_starts_at_blue() {
        // No `#` at all: register 15, 80% gray. Then HLS hue 0 is blue and
        // hue 120 red.
        assert_eq!(decode_sixel(&sixel("0", "~")).unwrap().get(0, 0), 0xcccccc);
        let data = sixel("0", "#4;1;0;50;100#4~#5;1;120;50;100#5~");
        let image = decode_sixel(&data).unwrap();
        assert_eq!((image.get(0, 0), image.get(1, 0)), (0x0000ff, 0xff0000));
    }

    #[test]
    fn vms_records_are_joined_before_decoding() {
        let plain = sixel("0", "#1;2;100;0;0#1~");
        let mut records = Vec::new();
        for chunk in [&plain[..11], &plain[11..]] {
            records.extend_from_slice(&(chunk.len() as u16).to_le_bytes());
            records.extend_from_slice(chunk);
            records.resize(records.len() + chunk.len() % 2, 0);
        }
        let image = decode_sixel(&records).unwrap();
        assert_eq!(image, decode_sixel(&plain).unwrap());
        // A plain file is not mistaken for records.
        assert!(join_records(&plain).is_none());
    }

    #[test]
    fn unterminated_oversized_and_empty_pictures_are_rejected() {
        let mut data = sixel("0", "~");
        data.truncate(data.len() - 2);
        assert!(decode_sixel(&data).is_err());
        assert!(decode_sixel(&sixel("0", "\"1;1;5000;10~")).is_err());
        assert!(decode_sixel(&sixel("0", "!5000~")).is_err());
        assert!(decode_sixel(&sixel("0", "?")).is_err());
        assert!(decode_sixel(b"\x1bP0;0;0q~\x1b[0m").is_err());
    }
}
