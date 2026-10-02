//! Atari TT DEGAS-style pictures (`PI4` TT low, `PI5` TT medium).
//!
//! Sources:
//! - <https://temlib.org/AtariForumWiki/index.php/DEGAS_file_format>
//!   (View ST/TT extension: resolution word, palette, 153600-byte bitmap)
//! - <http://cd.textfiles.com/crawlycrypt1/graphics/view132/viewttst.txt>
//! - TT palette word `....RRRR GGGGBBBB`: hardware register listing,
//!   <https://temlib.org/AtariForumWiki/index.php/Atari_ST/STe/MSTe/TT/F030_Hardware_Register_Listing>
//! - Observed from `recoil2png` output: 4-bit components times 0x11, and
//!   TT low resolution (320x480) shown with doubled pixels.

use super::common::{be16, palette_words, planar_image};
use crate::{DecodeError, Image};

const BITMAP_LEN: usize = 153600;
const TT_MEDIUM_LEN: usize = 34 + BITMAP_LEN;
const ST_240_LEN: usize = 34 + 38400;

/// TT palette word `....RRRR GGGGBBBB` to `0xRRGGBB`.
pub(super) fn tt_rgb(word: u16) -> u32 {
    let word = u32::from(word);
    ((word >> 8 & 0xf) * 0x110000) | ((word >> 4 & 0xf) * 0x1100) | ((word & 0xf) * 0x11)
}

/// TT low: resolution word 7, 256 palette words, 320x480 in 8 planes.
pub(super) fn decode_pi4(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 2 + 512 + BITMAP_LEN || be16(data, 0) != Some(7) {
        return Err(DecodeError::Unrecognized);
    }
    let palette: alloc::vec::Vec<u32> = palette_words(data, 2, 256)
        .ok_or(DecodeError::Unrecognized)?
        .into_iter()
        .map(tt_rgb)
        .collect();
    let image = planar_image(&data[514..], 320, 480, 8, &palette, 1);
    let image = image.ok_or(DecodeError::Unrecognized)?;
    Ok(image.scaled(2, 1))
}

/// TT high (`PI6`): resolution word 6, two palette words, 1280x960
/// monochrome. Source: <http://fileformats.archiveteam.org/wiki/Extended_DEGAS_image>
/// (extension only); the layout is derived from sample files.
pub(super) fn decode_pi6(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 6 + BITMAP_LEN || be16(data, 0) != Some(6) {
        return Err(DecodeError::Unrecognized);
    }
    super::common::mono_image(&data[6..], 1280, 960, 160).ok_or(DecodeError::Unrecognized)
}

/// TT medium: resolution word 4, 16 palette words, 640x480 in 4 planes;
/// files of 320x240 in 4 planes exist too (derived from sample files).
pub(super) fn decode_pi5(data: &[u8]) -> Result<Image, DecodeError> {
    let (width, height) = match data.len() {
        TT_MEDIUM_LEN => (640, 480),
        ST_240_LEN => (320, 240),
        _ => return Err(DecodeError::Unrecognized),
    };
    if be16(data, 0) != Some(4) {
        return Err(DecodeError::Unrecognized);
    }
    let words = palette_words(data, 2, 16).ok_or(DecodeError::Unrecognized)?;
    // The 320x240 variant uses ST/STE palette words (observed from
    // `recoil2png` output).
    let palette: alloc::vec::Vec<u32> = if width == 320 {
        super::common::st_palette(&words)
    } else {
        words.into_iter().map(tt_rgb).collect()
    };
    planar_image(&data[34..], width, height, 4, &palette, 1).ok_or(DecodeError::Unrecognized)
}
