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

fn tt_rgb(word: u16) -> u32 {
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
    let mut wide = Image::new(640, 480);
    for (i, p) in image.rgb().chunks_exact(3).enumerate() {
        let color = u32::from_be_bytes([0, p[0], p[1], p[2]]);
        let (x, y) = ((i % 320) as u32, (i / 320) as u32);
        wide.set(x * 2, y, color);
        wide.set(x * 2 + 1, y, color);
    }
    Ok(wide)
}

/// TT medium: resolution word 4, 16 palette words, 640x480 in 4 planes.
pub(super) fn decode_pi5(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 2 + 32 + BITMAP_LEN || be16(data, 0) != Some(4) {
        return Err(DecodeError::Unrecognized);
    }
    let palette: alloc::vec::Vec<u32> = palette_words(data, 2, 16)
        .ok_or(DecodeError::Unrecognized)?
        .into_iter()
        .map(tt_rgb)
        .collect();
    planar_image(&data[34..], 640, 480, 4, &palette, 1).ok_or(DecodeError::Unrecognized)
}
