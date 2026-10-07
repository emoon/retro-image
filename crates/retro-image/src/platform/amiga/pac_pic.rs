//! AMOS "Pac.Pic." picture bank (AmBk), compressed by the Compact extension.
//!
//! Sources:
//! - Screen header (id `$12031990`, mode, palette) and picture header
//!   (id `$06071963`, bytes per row, lumps, planes, stream offsets), the
//!   three-stream decompression and the lump/column pixel order: Deark's
//!   `abk.c` and `fmtutil_decompress_stos_pictbank`
//!   (<https://github.com/jsummers/deark>, MIT license, notice below).
//! - The AmBk bank header: <http://alvyn.sourceforge.net/amos_file_formats.html>.
//! - Files without the bank header, as the AMOS picture packer saves them
//!   (Sembiance's `image/amosPicturePacker`, 11 files): five start with the
//!   screen header (`$12031990`) and are the bank's contents (verified by
//!   wrapping them in a bank header and checking the result against
//!   `recoil2png`); six start with the picture block (`$06071963`) and have
//!   no palette anywhere in the file, so they are drawn in grays, a guess
//!   (they could be HAM or half-bright; the screen header that would say so is
//!   missing).

// Parts of this file follow Deark's abk.c and
// fmtutil_decompress_stos_pictbank
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

use super::ilbm::{half_brite, ham};
use crate::bytes::{be16, be32};
use crate::codec::stos_pictbank;
use crate::image::{check_size, rgb444, widen_channel};
use crate::{DecodeError, Image};

const SCREEN_IDS: [u32; 3] = [0x1203_1990, 0x0003_1990, 0x1203_0090];
const PICTURE_ID: u32 = 0x0607_1963;
const SCREEN_HEADER_LEN: usize = 90;

/// A `Pac.Pic.` bank: the AmBk header, a screen header with the palette,
/// then the picture.
pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    const FAIL: DecodeError = DecodeError::Invalid;
    if data.get(..4) != Some(b"AmBk") || data.get(12..20) != Some(b"Pac.Pic.") {
        return Err(FAIL);
    }
    decode_screen(data.get(20..).ok_or(FAIL)?)
}

/// A screen header and picture without the bank header (the files the AMOS
/// picture packer saves). Both ids must match.
pub(super) fn decode_screen(data: &[u8]) -> Result<Image, DecodeError> {
    const FAIL: DecodeError = DecodeError::Invalid;
    let screen = data.get(..SCREEN_HEADER_LEN).ok_or(FAIL)?;
    if !SCREEN_IDS.contains(&be32(screen, 0).ok_or(FAIL)?) {
        return Err(FAIL);
    }
    let mode = be16(screen, 20).ok_or(FAIL)?;
    let mut palette = [0u32; 64];
    for (i, word) in screen[26..90].as_chunks::<2>().0.iter().enumerate() {
        palette[i] = rgb444(u16::from_be_bytes([word[0], word[1]]));
        palette[i + 32] = half_brite(palette[i]);
    }
    draw(data, SCREEN_HEADER_LEN, mode, Some(palette))
}

/// A picture block alone, as some of the packer's files hold it. It has no
/// palette, so the colors are a gray ramp over the plane count: a guess that
/// keeps the picture legible.
pub(super) fn decode_bare(data: &[u8]) -> Result<Image, DecodeError> {
    draw(data, 0, 0, None)
}

/// The picture at `start` in `data`, in `palette` (`None` for grays).
fn draw(
    data: &[u8],
    start: usize,
    mode: u16,
    palette: Option<[u32; 64]>,
) -> Result<Image, DecodeError> {
    const FAIL: DecodeError = DecodeError::Invalid;
    let picture = data.get(start..start + 24).ok_or(FAIL)?;
    if be32(picture, 0).ok_or(FAIL)? != PICTURE_ID {
        return Err(FAIL);
    }
    let row_len = usize::from(be16(picture, 8).ok_or(FAIL)?);
    let lumps = usize::from(be16(picture, 10).ok_or(FAIL)?);
    let lump_lines = usize::from(be16(picture, 12).ok_or(FAIL)?);
    let planes = usize::from(be16(picture, 14).ok_or(FAIL)?);
    let rle_pos = start.saturating_add(be32(picture, 16).ok_or(FAIL)? as usize);
    let points_pos = start.saturating_add(be32(picture, 20).ok_or(FAIL)? as usize);
    let (width, height) = (row_len * 8, lumps * lump_lines);
    if !(1..=6).contains(&planes) {
        return Err(FAIL);
    }
    check_size(width, height)?;
    let plane_len = row_len * height;
    let unpacked = stos_pictbank::unpack(data, start + 24, rle_pos, points_pos, plane_len * planes)
        .ok_or(FAIL)?;
    let palette = palette.unwrap_or_else(|| gray_ramp(planes));

    let is_ham = mode & 0x800 != 0 && planes == 6;
    let mut image = Image::new(width as u32, height as u32)?;
    let mut held = 0;
    for y in 0..height {
        let (lump, line) = (y / lump_lines, y % lump_lines);
        for x in 0..width {
            if x == 0 {
                held = palette[0];
            }
            let offset = lump * row_len * lump_lines + x / 8 * lump_lines + line;
            let value = (0..planes).fold(0, |v, p| {
                v | usize::from(unpacked[p * plane_len + offset] >> (7 - x % 8) & 1) << p
            });
            let color = if is_ham {
                ham(
                    held,
                    (value >> 4) as u32,
                    widen_channel((value & 15) as u32, 4),
                    palette[value & 15],
                )
            } else {
                palette[value]
            };
            held = color;
            image.set(x as u32, y as u32, color);
        }
    }
    Ok(image)
}

/// `2^planes` grays from black to white (the rest of the table black).
fn gray_ramp(planes: usize) -> [u32; 64] {
    let last = (1u32 << planes) - 1;
    core::array::from_fn(|i| {
        let level = if i as u32 > last {
            0
        } else {
            i as u32 * 255 / last
        };
        level * 0x01_0101
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_pictures_are_drawn_in_a_gray_ramp() {
        let ramp = gray_ramp(2);
        assert_eq!(ramp[..5], [0, 0x555555, 0xaaaaaa, 0xffffff, 0]);
        assert_eq!(gray_ramp(1)[..3], [0, 0xffffff, 0]);
    }

    #[test]
    fn picture_over_the_pixel_cap_is_rejected() {
        // 16384 x 4097 pixels, one plane: 8 MiB of plane data, which the
        // input length alone would allow with enough padding.
        let mut data = alloc::vec![0u8; 140_000];
        data[..4].copy_from_slice(b"AmBk");
        data[12..20].copy_from_slice(b"Pac.Pic.");
        data[20..24].copy_from_slice(&SCREEN_IDS[0].to_be_bytes());
        let start = 20 + SCREEN_HEADER_LEN;
        data[start..start + 4].copy_from_slice(&PICTURE_ID.to_be_bytes());
        data[start + 8..start + 10].copy_from_slice(&2048u16.to_be_bytes());
        data[start + 10..start + 12].copy_from_slice(&1u16.to_be_bytes());
        data[start + 12..start + 14].copy_from_slice(&4097u16.to_be_bytes());
        data[start + 14..start + 16].copy_from_slice(&1u16.to_be_bytes());
        data[start + 19] = 24;
        data[start + 23] = 24;
        assert!(decode(&data).is_err());
    }
}
