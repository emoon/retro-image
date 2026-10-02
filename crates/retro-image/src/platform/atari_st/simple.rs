//! Uncompressed ST screen formats, told apart by size and header.
//!
//! Sources:
//! - NEOchrome, including the 640x400 `0xBABE` virtual canvas:
//!   <https://temlib.org/AtariForumWiki/index.php/NEOchrome_file_format>
//! - Doodle: <https://temlib.org/AtariForumWiki/index.php/Doodle_file_format>
//! - Art Director: <https://temlib.org/AtariForumWiki/index.php/Art_Director_file_format>
//! - GFA Artist ("1000 colours off"):
//!   <https://temlib.org/AtariForumWiki/index.php/GFA_Artist_file_format>
//! - Palette Master: <http://fileformats.archiveteam.org/wiki/Palette_Master>
//! - PaintPro / PlusPaint (`PIC`, single and double height):
//!   <https://temlib.org/AtariForumWiki/index.php?title=PaintPro_ST/PlusPaint_ST>
//! - Dali (`SD0`-`SD2`): <https://temlib.org/AtariForumWiki/index.php/Dali_file_format>
//! - Synthetic Arts: <https://temlib.org/AtariForumWiki/index.php/Synthetic_Arts_file_format>
//! - PaintShop `DA4`: <https://temlib.org/AtariForumWiki/index.php/PaintShop_file_format>
//! - RGB Intermediate: <https://temlib.org/AtariForumWiki/index.php/RGB_Intermediate_file_format>
//! - ColorSTar `BIL` is GFA Artist or DEGAS by size: survey notes in
//!   `docs/formats/atari-st-tt-falcon.md`; checked against `recoil2png` output.

use super::common::{
    Resolution, SCREEN_LEN, be16, decode_screen, palette_words, planar_image, st_palette,
};
use crate::{DecodeError, Image};

const NEO_HEADER_LEN: usize = 128;
const CANVAS_FLAG: u16 = 0xbabe;
const DEGAS_LEN: usize = 34 + SCREEN_LEN;

fn ok(image: Option<Image>) -> Result<Image, DecodeError> {
    image.ok_or(DecodeError::Unrecognized)
}

fn words(data: &[u8], offset: usize) -> Result<alloc::vec::Vec<u16>, DecodeError> {
    palette_words(data, offset, 16).ok_or(DecodeError::Unrecognized)
}

/// NEOchrome: flag word, resolution word, 16 palette words, ..., screen at 128.
pub(super) fn decode_neo(data: &[u8]) -> Result<Image, DecodeError> {
    let flag = be16(data, 0).ok_or(DecodeError::Unrecognized)?;
    let words = words(data, 4)?;
    let bitmap = data
        .get(NEO_HEADER_LEN..)
        .ok_or(DecodeError::Unrecognized)?;
    ok(match flag {
        0 if data.len() == NEO_HEADER_LEN + SCREEN_LEN => be16(data, 2)
            .and_then(Resolution::from_index)
            .and_then(|resolution| decode_screen(resolution, bitmap, &words)),
        CANVAS_FLAG if data.len() == NEO_HEADER_LEN + 4 * SCREEN_LEN => {
            planar_image(bitmap, 640, 400, 4, &st_palette(&words), 1)
        }
        _ => None,
    })
}

/// Doodle: a raw 32000-byte high-resolution screen.
pub(super) fn decode_doo(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != SCREEN_LEN {
        return Err(DecodeError::Unrecognized);
    }
    ok(decode_screen(Resolution::High, data, &[]))
}

/// `ART` files: Art Director, GFA Artist, Palette Master or a Doodle-like
/// high-resolution screen, by size.
pub(super) fn decode_art(data: &[u8]) -> Result<Image, DecodeError> {
    match data.len() {
        SCREEN_LEN => decode_doo(data),
        // GFA Artist: palette, then screen.
        32032 => decode_gfa_artist(data),
        // Art Director: screen, 8 palettes, 8 display times, 248 unknown bytes.
        32512 => {
            let times = &data[SCREEN_LEN + 256..SCREEN_LEN + 264];
            // Which palette shows when no time is set is unknown; reject.
            let palette = times
                .iter()
                .position(|&t| t != 0)
                .ok_or(DecodeError::Unrecognized)?;
            let words = words(data, SCREEN_LEN + palette * 32)?;
            ok(decode_screen(Resolution::Low, data, &words))
        }
        // Palette Master: 32768-byte screen area, then palettes; only the
        // first palette is used.
        36864 => {
            let words = words(data, 32768)?;
            ok(decode_screen(Resolution::Low, data, &words))
        }
        _ => Err(DecodeError::Unrecognized),
    }
}

fn decode_gfa_artist(data: &[u8]) -> Result<Image, DecodeError> {
    let words = words(data, 0)?;
    ok(decode_screen(Resolution::Low, &data[32..], &words))
}

/// ColorSTar `BIL`: GFA Artist layout or a low-resolution DEGAS picture.
pub(super) fn decode_bil(data: &[u8]) -> Result<Image, DecodeError> {
    match data.len() {
        32032 => decode_gfa_artist(data),
        DEGAS_LEN if be16(data, 0) == Some(0) => {
            let words = words(data, 2)?;
            ok(decode_screen(Resolution::Low, &data[34..], &words))
        }
        _ => Err(DecodeError::Unrecognized),
    }
}

/// PaintPro / PlusPaint `PIC`: DEGAS, optionally with a double-height
/// bitmap; also a bare high-resolution screen.
pub(super) fn decode_pic(data: &[u8]) -> Result<Image, DecodeError> {
    let resolution = be16(data, 0).and_then(Resolution::from_index);
    match (data.len(), resolution) {
        (SCREEN_LEN, _) => decode_doo(data),
        (DEGAS_LEN, Some(resolution)) => {
            ok(decode_screen(resolution, &data[34..], &words(data, 2)?))
        }
        (64034, Some(resolution)) => {
            let palette = super::common::screen_palette(resolution, &words(data, 2)?);
            ok(planar_image(
                &data[34..],
                resolution.width(),
                resolution.height() * 2,
                resolution.planes(),
                &palette,
                resolution.y_scale(),
            ))
        }
        _ => Err(DecodeError::Unrecognized),
    }
}

/// Dali `SD0`-`SD2`: long 0, palette, 92 reserved bytes, screen; the
/// resolution comes from the extension.
fn decode_dali(data: &[u8], resolution: Resolution) -> Result<Image, DecodeError> {
    if data.len() != 128 + SCREEN_LEN || data[..4] != [0; 4] {
        return Err(DecodeError::Unrecognized);
    }
    ok(decode_screen(resolution, &data[128..], &words(data, 4)?))
}

pub(super) fn decode_sd0(data: &[u8]) -> Result<Image, DecodeError> {
    decode_dali(data, Resolution::Low)
}

pub(super) fn decode_sd1(data: &[u8]) -> Result<Image, DecodeError> {
    decode_dali(data, Resolution::Medium)
}

pub(super) fn decode_sd2(data: &[u8]) -> Result<Image, DecodeError> {
    decode_dali(data, Resolution::High)
}

/// Synthetic Arts: medium-resolution screen, 3 words, palette.
pub(super) fn decode_srt(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != SCREEN_LEN + 38 {
        return Err(DecodeError::Unrecognized);
    }
    ok(decode_screen(
        Resolution::Medium,
        data,
        &words(data, SCREEN_LEN + 6)?,
    ))
}

/// Sinbad Slideshow: low-resolution screen, palette, padding to 32768.
pub(super) fn decode_ssb(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 32768 {
        return Err(DecodeError::Unrecognized);
    }
    ok(decode_screen(
        Resolution::Low,
        data,
        &words(data, SCREEN_LEN)?,
    ))
}

/// PaintShop `DA4`: raw 640x800 monochrome.
pub(super) fn decode_da4(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 2 * SCREEN_LEN {
        return Err(DecodeError::Unrecognized);
    }
    ok(planar_image(
        data,
        640,
        800,
        1,
        &super::common::MONO_PALETTE,
        1,
    ))
}

/// RGB Intermediate: three low-resolution DEGAS pictures holding the red,
/// green and blue component (0-15) of each pixel.
pub(super) fn decode_rgb(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 3 * DEGAS_LEN {
        return Err(DecodeError::Unrecognized);
    }
    let gun = |i: usize| &data[i * DEGAS_LEN + 34..(i + 1) * DEGAS_LEN];
    let (red, green, blue) = (gun(0), gun(1), gun(2));
    let mut image = Image::new(320, 200);
    for y in 0..200u32 {
        let start = y as usize * 160;
        for x in 0..320 {
            let level = |plane: &[u8]| {
                let line = &plane[start..start + 160];
                super::common::interleaved_index(line, x, 4) as u32 * 0x11
            };
            image.set(x, y, level(red) << 16 | level(green) << 8 | level(blue));
        }
    }
    Ok(image)
}
