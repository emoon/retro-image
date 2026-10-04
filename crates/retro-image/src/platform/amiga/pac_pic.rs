//! AMOS "Pac.Pic." picture bank (AmBk), compressed by the Compact extension.
//!
//! Sources:
//! - Screen header (id `$12031990`, mode, palette) and picture header
//!   (id `$06071963`, bytes per row, lumps, planes, stream offsets), the
//!   three-stream decompression and the lump/column pixel order: Deark's
//!   `abk.c` and `fmtutil_decompress_stos_pictbank`
//!   (<https://github.com/jsummers/deark>, MIT licence,
//!   Copyright (C) 2016-2026 Jason Summers).
//! - The AmBk bank header: <http://alvyn.sourceforge.net/amos_file_formats.html>.

use super::ilbm::{half_brite, ham, rgb12};
use crate::bytes::{be16, be32};
use crate::codec::stos_pictbank;
use crate::image::check_size;
use crate::{DecodeError, Image};

const SCREEN_IDS: [u32; 3] = [0x1203_1990, 0x0003_1990, 0x1203_0090];
const PICTURE_ID: u32 = 0x0607_1963;
const SCREEN_HEADER_LEN: usize = 90;

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    if data.get(..4) != Some(b"AmBk") || data.get(12..20) != Some(b"Pac.Pic.") {
        return Err(fail);
    }
    let screen = data.get(20..20 + SCREEN_HEADER_LEN).ok_or(fail)?;
    if !SCREEN_IDS.contains(&be32(screen, 0).ok_or(fail)?) {
        return Err(fail);
    }
    let mode = be16(screen, 20).ok_or(fail)?;
    let mut palette = [0u32; 64];
    for (i, word) in screen[26..90].as_chunks::<2>().0.iter().enumerate() {
        palette[i] = rgb12(u16::from_be_bytes([word[0], word[1]]));
        palette[i + 32] = half_brite(palette[i]);
    }

    let start = 20 + SCREEN_HEADER_LEN;
    let picture = data.get(start..start + 24).ok_or(fail)?;
    if be32(picture, 0).ok_or(fail)? != PICTURE_ID {
        return Err(fail);
    }
    let row_len = usize::from(be16(picture, 8).ok_or(fail)?);
    let lumps = usize::from(be16(picture, 10).ok_or(fail)?);
    let lump_lines = usize::from(be16(picture, 12).ok_or(fail)?);
    let planes = usize::from(be16(picture, 14).ok_or(fail)?);
    let rle_pos = start.saturating_add(be32(picture, 16).ok_or(fail)? as usize);
    let points_pos = start.saturating_add(be32(picture, 20).ok_or(fail)? as usize);
    let (width, height) = (row_len * 8, lumps * lump_lines);
    if !(1..=6).contains(&planes) {
        return Err(fail);
    }
    check_size(width, height)?;
    let plane_len = row_len * height;
    let unpacked = stos_pictbank::unpack(data, start + 24, rle_pos, points_pos, plane_len * planes)
        .ok_or(fail)?;

    let is_ham = mode & 0x800 != 0 && planes == 6;
    let mut image = Image::new(width as u32, height as u32);
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
                    (value & 15) as u32 * 0x11,
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

#[cfg(test)]
mod tests {
    use super::*;

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
