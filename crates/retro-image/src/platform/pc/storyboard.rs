//! IBM PC Storyboard pictures (`.PIC`, `.CAP`, `.TEM`): the `EP_CAP` files
//! and the later 16-color and 256-color files.
//!
//! Sources:
//! - Layouts, the run-length codes, the palette tables and the dimension
//!   rules: Deark's `storyboard` module (`storyboard.c`;
//!   <https://github.com/jsummers/deark>, MIT license, notice below), and the
//!   survey note `docs/research/gaps-computers-extra.md` C13. Deark calls the
//!   `EP_CAP` layout supported and the later one "highly experimental": its
//!   author could not work out what most of the 2048-byte header holds, nor
//!   how the pixel values map to the palette. No other documentation exists.
//! - `EP_CAP` (`.CAP`, `.PIC`): the text `EP_CAP`, a zero, the screen mode
//!   (4: 320x200 in four colors), three bytes that are `28 00 00` in the
//!   samples, the row length in bytes and the height as words, then a word
//!   with the size of the data and the data. The data is words: 0 ends it,
//!   below `0x8000` copies that many bytes, from `0x8000` on repeats the next byte
//!   `n - 0x8000` times. The rows are whole, in order, 2 bits per pixel, most
//!   significant bits leftmost. The palette is not in the file. Pictures use
//!   the high-intensity cyan, magenta and white set (Deark's choice; the
//!   samples show those colors). Mode 3 (text, one sample) and mode 6
//!   (640x200, no sample) are not decoded.
//! - The later files: bytes 0 to 3 are `00` or `54`, then `84`, `C1` and a
//!   code: `84` with code 1, 3, 7 or 8 is 640x200, 640x350, 640x480 in 16
//!   colors or 320x200 in 4, and `86` is 640x480 in 256. Words at 5 and 7
//!   are the width and height. The 16 colors are 16 EGA palette bytes at 13;
//!   in 4 colors byte 13 picks the palette (6 is the green, red, yellow set,
//!   else cyan, magenta, white) and byte 14 the background. The 256-color
//!   palette is three runs of 256 6-bit values at 128 (red, green, blue). The
//!   run-length coded data starts at 2048, or at 2816 for 256 colors. The
//!   pixels are bit planes in strips: each plane is 8-pixel-wide columns, top
//!   to bottom, then the next column; 256 colors are plain bytes. Codes: 1 to
//!   `7F` copies that many bytes, `81` to `FF` repeats the next byte
//!   `n - 0x80` times. The 256-color files use `01` to `7E` and `81` to `FC`
//!   the same way, and give `FD` to `FF` other meanings: `FD` (a word follows)
//!   and `FE` (a byte follows) copy that many bytes from the row above, `FF`
//!   repeats the next byte as often as a word says. A `00` code is not
//!   decoded: Deark guesses it switches compression off, and no sample has
//!   it. The 320x200 256-color kind (code `85`) has no sample either and is
//!   not decoded.
//! - Color assignments: the maps from pixel values to the 16 palette bytes
//!   (and to the 4 colors) are Deark's, found by trial against pictures. They
//!   are the unconfirmed part: the 16 samples render plausibly (screenshots
//!   with ordinary window colors), but nothing independent checks the order.
//! - Checked on 10 `EP_CAP` and 16 later files (Sembiance
//!   `ibmStoryboardPic`; the corpus keeps 7 and 16 of them). All streams
//!   decode to exactly the size the header gives, and each output matches
//!   Deark's pixel for pixel (see `dos-clipart.tsv`).
//!   The data of the 4-color file `91P1040.PIC` is followed by 170 more bytes
//!   that look like more coded rows; they are ignored, as Deark does.

// Parts of this file follow Deark's modules/storyboard.c
// (Deark, https://github.com/jsummers/deark):
//
// Copyright (C) 2022 Jason Summers
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

use super::{CGA_PALETTE, cga_set, dac_rounded, ega_64};
use crate::bytes::le16;
use crate::image::{check_size, planar_pixels};
use crate::{DecodeError, Image};

const FAIL: DecodeError = DecodeError::Unrecognized;

// `EP_CAP` files.
const OLD_MAGIC: &[u8] = b"EP_CAP";
const OLD_MODE_AT: usize = 7;
const OLD_CGA_MODE: u8 = 4;
const OLD_ROW_LEN_AT: usize = 11;
const OLD_HEIGHT_AT: usize = 13;
const OLD_DATA_AT: usize = 17;
/// Pixels per byte in the 4-color mode.
const OLD_PIXELS_PER_BYTE: usize = 4;
/// Cyan, magenta and white on black.
const OLD_PALETTE: [u32; 4] = cga_set([11, 13, 15]);

// Later files.
const NEW_WIDTH_AT: usize = 5;
const NEW_HEIGHT_AT: usize = 7;
const NEW_PALETTE_AT: usize = 13;
const NEW_DATA_AT: usize = 2048;
const NEW_256_PALETTE_AT: usize = 128;
const NEW_256_DATA_AT: usize = 2816;
/// Pixel value to palette byte number, in 4 colors.
const MAP_4: [usize; 4] = [3, 1, 2, 0];
/// The same for 16 colors in 640x200, and in 640x350 and 640x480.
const MAP_16_LOW: [usize; 16] = [0, 11, 13, 15, 6, 4, 2, 9, 8, 3, 5, 7, 14, 12, 10, 1];
const MAP_16_HIGH: [usize; 16] = [0, 11, 6, 4, 13, 15, 2, 9, 8, 3, 14, 12, 5, 7, 10, 1];

/// The screen a later file was made for.
#[derive(Clone, Copy)]
enum Screen {
    Cga320x200,
    Ega640x200,
    Ega640x350,
    Ega640x480,
    Vga640x480,
}

impl Screen {
    fn from_header(header: &[u8]) -> Option<Self> {
        let &[first, kind, 0xc1, code, ..] = header else {
            return None;
        };
        if !matches!(first, 0x00 | 0x54) {
            return None;
        }
        match (kind, code) {
            (0x84, 8) => Some(Self::Cga320x200),
            (0x84, 1) => Some(Self::Ega640x200),
            (0x84, 3) => Some(Self::Ega640x350),
            (0x84, 7) => Some(Self::Ega640x480),
            (0x86, _) => Some(Self::Vga640x480),
            _ => None,
        }
    }

    /// Screen size in pixels.
    fn size(self) -> (usize, usize) {
        match self {
            Self::Cga320x200 => (320, 200),
            Self::Ega640x200 => (640, 200),
            Self::Ega640x350 => (640, 350),
            Self::Ega640x480 | Self::Vga640x480 => (640, 480),
        }
    }

    /// Bit planes of the screens drawn as planes, which is not the
    /// 256-color one (that is a byte a pixel and decoded on its own).
    fn planes(self) -> usize {
        match self {
            Self::Cga320x200 => 2,
            _ => 4,
        }
    }
}

/// Run-length codes. `long` is the variant of the 256-color files, which
/// has codes that copy from the row above (`row_len` bytes back). Returns
/// `need` bytes; data beyond them is left alone.
fn unpack(data: &[u8], need: usize, long: bool, row_len: usize) -> Option<Vec<u8>> {
    let (last_copy, last_run) = if long { (0x7e, 0xfc) } else { (0x7f, 0xff) };
    // The header may promise far more than the data can hold.
    let mut out = Vec::with_capacity(need.min(data.len().saturating_mul(64)));
    let mut rest = data;
    while out.len() < need {
        let (&code, tail) = rest.split_first()?;
        rest = tail;
        match code {
            1..=0x7f if code <= last_copy => {
                let (bytes, tail) = rest.split_at_checked(usize::from(code))?;
                out.extend_from_slice(bytes);
                rest = tail;
            }
            0x81..=0xff if code <= last_run => {
                let (&value, tail) = rest.split_first()?;
                out.resize(out.len() + usize::from(code & 0x7f), value);
                rest = tail;
            }
            0xfd..=0xff if long => {
                let (count, tail) = if code == 0xfe {
                    let (&count, tail) = rest.split_first()?;
                    (usize::from(count), tail)
                } else {
                    let count = usize::from(le16(rest, 0)?);
                    (count, rest.get(2..)?)
                };
                rest = tail;
                if code == 0xff {
                    let (&value, tail) = rest.split_first()?;
                    out.resize(out.len() + count, value);
                    rest = tail;
                } else {
                    let from = out.len().checked_sub(row_len)?;
                    if count > row_len {
                        return None;
                    }
                    out.extend_from_within(from..from + count);
                }
            }
            _ => return None,
        }
    }
    out.truncate(need);
    Some(out)
}

pub(super) fn decode_old(data: &[u8]) -> Result<Image, DecodeError> {
    if !data.starts_with(OLD_MAGIC)
        || data.get(OLD_MAGIC.len()) != Some(&0)
        || data.get(OLD_MODE_AT) != Some(&OLD_CGA_MODE)
    {
        return Err(FAIL);
    }
    let row_len = usize::from(le16(data, OLD_ROW_LEN_AT).ok_or(FAIL)?);
    let height = usize::from(le16(data, OLD_HEIGHT_AT).ok_or(FAIL)?);
    let width = row_len * OLD_PIXELS_PER_BYTE;
    check_size(width, height)?;
    let coded = data.get(OLD_DATA_AT..).ok_or(FAIL)?;
    let rows = unpack_old(coded, row_len * height).ok_or(FAIL)?;
    let indices: Vec<u8> = rows
        .iter()
        .flat_map(|&byte| (0..4).map(move |i| byte >> (6 - 2 * i) & 3))
        .collect();
    Image::from_indexed(width as u32, height as u32, &indices, &OLD_PALETTE)
}

/// Words: 0 ends the data, below `0x8000` that many bytes follow, else the
/// next byte repeats `word - 0x8000` times. Returns `need` bytes.
fn unpack_old(data: &[u8], need: usize) -> Option<Vec<u8>> {
    // The header may promise far more than the data can hold.
    let mut out = Vec::with_capacity(need.min(data.len().saturating_mul(64)));
    let mut rest = data;
    while out.len() < need {
        let count = usize::from(le16(rest, 0)?);
        rest = rest.get(2..)?;
        match count {
            0 => return None,
            1..0x8000 => {
                let (bytes, tail) = rest.split_at_checked(count)?;
                out.extend_from_slice(bytes);
                rest = tail;
            }
            _ => {
                let (&value, tail) = rest.split_first()?;
                out.resize(out.len() + count - 0x8000, value);
                rest = tail;
            }
        }
    }
    out.truncate(need);
    Some(out)
}

pub(super) fn decode_new(data: &[u8]) -> Result<Image, DecodeError> {
    let screen = Screen::from_header(data).ok_or(FAIL)?;
    let width = usize::from(le16(data, NEW_WIDTH_AT).ok_or(FAIL)?);
    let height = usize::from(le16(data, NEW_HEIGHT_AT).ok_or(FAIL)?);
    let (screen_width, screen_height) = screen.size();
    if width > screen_width || height > screen_height {
        return Err(FAIL);
    }
    check_size(width, height)?;
    if let Screen::Vga640x480 = screen {
        return decode_256(data, width, height);
    }
    let palette = palette(screen, data)?;
    let planes = screen.planes();
    // Plane `p` is `strips` columns of `height` bytes; `rows` has them
    // row by row instead.
    let strips = width.div_ceil(8);
    let plane_len = strips * height;
    let columns = unpack(
        data.get(NEW_DATA_AT..).ok_or(FAIL)?,
        plane_len * planes,
        false,
        0,
    )
    .ok_or(FAIL)?;
    let mut rows = alloc::vec![0u8; columns.len()];
    for (plane, columns) in columns.chunks_exact(plane_len).enumerate() {
        for (strip, column) in columns.chunks_exact(height).enumerate() {
            for (y, &byte) in column.iter().enumerate() {
                rows[plane * plane_len + y * strips + strip] = byte;
            }
        }
    }
    let values = planar_pixels(&rows, width, height, strips, planes, |plane, y| {
        plane * plane_len + y * strips
    });
    let indices: Vec<u8> = values.into_iter().map(|v| v as u8).collect();
    Image::from_indexed(width as u32, height as u32, &indices, &palette)
}

/// The 4 or 16 colors of the screen, indexed by pixel value.
fn palette(screen: Screen, header: &[u8]) -> Result<Vec<u32>, DecodeError> {
    let byte = |at: usize| header.get(at).copied().ok_or(FAIL);
    if let Screen::Cga320x200 = screen {
        let mut colors = if byte(NEW_PALETTE_AT)? & 15 == 6 {
            cga_set([10, 12, 14])
        } else {
            cga_set([11, 13, 15])
        };
        colors[0] = CGA_PALETTE[usize::from(byte(NEW_PALETTE_AT + 1)? & 15)];
        return Ok(MAP_4.iter().map(|&i| colors[i]).collect());
    }
    let low = matches!(screen, Screen::Ega640x200);
    let registers = header
        .get(NEW_PALETTE_AT..NEW_PALETTE_AT + 16)
        .ok_or(FAIL)?;
    let mut colors = [0; 16];
    for (color, &register) in colors.iter_mut().zip(registers) {
        // Register 6 is the EGA's dark yellow; on a 200-line screen it
        // showed as brown, which is register 20.
        let register = if register == 6 && low { 20 } else { register };
        if register > 63 {
            return Err(FAIL);
        }
        *color = ega_64(register);
    }
    let map = if low { &MAP_16_LOW } else { &MAP_16_HIGH };
    Ok(map.iter().map(|&i| colors[i]).collect())
}

fn decode_256(data: &[u8], width: usize, height: usize) -> Result<Image, DecodeError> {
    let table = data
        .get(NEW_256_PALETTE_AT..NEW_256_PALETTE_AT + 3 * 256)
        .ok_or(FAIL)?;
    let (reds, rest) = table.split_at(256);
    let (greens, blues) = rest.split_at(256);
    let palette: Vec<u32> = (0..256)
        .map(|i| dac_rounded(reds[i]) << 16 | dac_rounded(greens[i]) << 8 | dac_rounded(blues[i]))
        .collect();
    let coded = data.get(NEW_256_DATA_AT..).ok_or(FAIL)?;
    let indices = unpack(coded, width * height, true, width).ok_or(FAIL)?;
    Image::from_indexed(width as u32, height as u32, &indices, &palette)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_files_expand_words_into_two_bit_pixels() {
        // 1 byte per row, 2 rows: a run of 2 x 0b1110_0100 (white, magenta,
        // cyan, black), ends with the zero word.
        let mut data = b"EP_CAP\0\x04\x28\0\0\x01\0\x02\0\x05\0".to_vec();
        data.extend_from_slice(&[0x02, 0x80, 0xe4, 0, 0]);
        let image = decode_old(&data).unwrap();
        assert_eq!((image.width(), image.height()), (4, 2));
        assert_eq!(image.get(0, 1), 0xffffff);
        assert_eq!(image.get(1, 0), 0xff55ff);
        assert_eq!(image.get(2, 0), 0x55ffff);
        assert_eq!(image.get(3, 1), 0);
        // The data stops short, or the mode is another one.
        let mut short = data.clone();
        short.truncate(OLD_DATA_AT + 2);
        assert!(decode_old(&short).is_err());
        data[OLD_MODE_AT] = 6;
        assert!(decode_old(&data).is_err());
    }

    #[test]
    fn short_codes_copy_and_repeat() {
        assert_eq!(
            unpack(&[2, 7, 8, 0x83, 9, 1, 1], 6, false, 0).unwrap(),
            [7, 8, 9, 9, 9, 1]
        );
        // The runs overshoot: the end is cut off. 0 and 0x80 are not codes.
        assert_eq!(unpack(&[0x85, 4], 2, false, 0).unwrap(), [4, 4]);
        assert!(unpack(&[0, 1], 1, false, 0).is_none());
        assert!(unpack(&[0x80, 1], 1, false, 0).is_none());
        assert!(unpack(&[2, 7], 2, false, 0).is_none());
    }

    #[test]
    fn long_codes_copy_the_row_above_and_repeat_with_words() {
        // Rows of 3 bytes: a literal row, then 2 bytes copied from 3 back
        // (word count), 1 more byte from 3 back (byte count), and a run of
        // 4 (word count).
        let data = [3, 1, 2, 3, 0xfd, 2, 0, 0xfe, 1, 0xff, 4, 0, 9];
        assert_eq!(
            unpack(&data, 10, true, 3).unwrap(),
            [1, 2, 3, 1, 2, 3, 9, 9, 9, 9]
        );
        // The 0x7f and 0xfd..0xff of the short codes mean something else here.
        assert!(unpack(&[0x7f, 0, 0], 1, true, 3).is_none());
        // Nothing above to copy, or more than a row.
        assert!(unpack(&[0xfe, 1, 5], 1, true, 3).is_none());
        assert!(unpack(&[3, 1, 2, 3, 0xfe, 4], 7, true, 3).is_none());
        // In the short code set 0xfe is a run.
        assert_eq!(unpack(&[0xfe, 5], 126, false, 0).unwrap(), [5; 126]);
    }

    #[test]
    fn later_files_are_planes_stored_in_strips() {
        // 640x200 EGA is too big for a test; a 16x2 picture in the 640x200
        // mode has 2 strips of 2 bytes per plane, 4 planes.
        let mut header = alloc::vec![0u8; NEW_DATA_AT];
        header[..4].copy_from_slice(&[0, 0x84, 0xc1, 1]);
        header[NEW_WIDTH_AT] = 16;
        header[NEW_HEIGHT_AT] = 2;
        // Register i is color i; registers 1 and 4 are white.
        for i in 0..16 {
            header[NEW_PALETTE_AT + i] = 0;
        }
        header[NEW_PALETTE_AT + 1] = 0x3f;
        header[NEW_PALETTE_AT + 4] = 0x3f;
        // Plane 0 has strip 1, row 1 set (pixels 8 to 15 of row 1, value 1).
        let mut planes = alloc::vec![0u8; 4 * 4];
        planes[3] = 0xff;
        // As literal runs of 4 bytes per plane.
        for plane in planes.chunks(4) {
            header.push(4);
            header.extend_from_slice(plane);
        }
        let image = decode_new(&header).unwrap();
        assert_eq!((image.width(), image.height()), (16, 2));
        // Pixel value 1 maps to register 11 in the 640x200 order, which is
        // black here; value 0 is register 0, also black. Check the shape
        // through the registers instead: set register 11 white.
        header[NEW_PALETTE_AT + 11] = 0x3f;
        let image = decode_new(&header).unwrap();
        assert_eq!(image.get(8, 1), 0xffffff);
        assert_eq!(image.get(7, 1), 0);
        assert_eq!(image.get(8, 0), 0);
        assert_eq!(image.get(15, 1), 0xffffff);
    }

    #[test]
    fn later_files_check_the_header() {
        let mut header = alloc::vec![0u8; NEW_DATA_AT + 2];
        header[..4].copy_from_slice(&[0, 0x84, 0xc1, 1]);
        header[NEW_WIDTH_AT] = 8;
        header[NEW_HEIGHT_AT] = 1;
        header[NEW_DATA_AT] = 4;
        assert!(decode_new(&header).is_err(), "data is short");
        header[NEW_WIDTH_AT + 1] = 3; // 776 pixels wide
        assert!(decode_new(&header).is_err(), "wider than the screen");
        header[NEW_WIDTH_AT + 1] = 0;
        header[2] = 0xc2;
        assert!(decode_new(&header).is_err());
    }
}
