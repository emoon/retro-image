//! DEGAS and DEGAS Elite pictures (`PI1`-`PI3`, `SUH`, `PC1`-`PC3`).
//!
//! Sources:
//! - <https://temlib.org/AtariForumWiki/index.php/DEGAS_file_format>
//! - <https://temlib.org/AtariForumWiki/index.php/DEGAS_Elite_file_format>
//! - <https://temlib.org/AtariForumWiki/index.php/DEGAS_Elite_Compressed_file_format>
//! - <http://fileformats.archiveteam.org/wiki/DEGAS_image> (`SUH` = hi-res).
//! - Taller low-resolution `PI1` files (`34 + 160 * lines` bytes) are shown
//!   at their full height: observed from `recoil2png` output.

use super::common::{
    Resolution, SCREEN_LEN, be16, decode_screen, line_planes_to_interleaved, palette_words,
    unpack_bits,
};
use crate::{DecodeError, Image};

const HEADER_LEN: usize = 34;
const ELITE_LEN: usize = HEADER_LEN + SCREEN_LEN + 32;

/// Uncompressed DEGAS: resolution word, 16 palette words, screen.
pub(super) fn decode_pi(data: &[u8]) -> Result<Image, DecodeError> {
    let resolution = be16(data, 0)
        .and_then(Resolution::from_index)
        .ok_or(DecodeError::Unrecognized)?;
    if data.len() < HEADER_LEN + SCREEN_LEN {
        return Err(DecodeError::Unrecognized);
    }
    let words = palette_words(data, 2, 16).ok_or(DecodeError::Unrecognized)?;
    let bitmap = &data[HEADER_LEN..];
    let tall_lines = (data.len() - HEADER_LEN) / 160;
    let image = if resolution == Resolution::Low
        && data.len() > ELITE_LEN
        && (data.len() - HEADER_LEN).is_multiple_of(160)
    {
        let palette = super::common::st_palette(&words);
        super::common::planar_image(bitmap, 320, tall_lines as u32, 4, &palette, 1)
    } else {
        decode_screen(resolution, bitmap, &words)
    };
    image.ok_or(DecodeError::Unrecognized)
}

/// DEGAS Elite compressed: resolution word with bit 15 set, palette,
/// PackBits per line per plane.
pub(super) fn decode_pc(data: &[u8]) -> Result<Image, DecodeError> {
    let word = be16(data, 0).ok_or(DecodeError::Unrecognized)?;
    if word & 0x8000 == 0 {
        return Err(DecodeError::Unrecognized);
    }
    let resolution = Resolution::from_index(word & 3).ok_or(DecodeError::Unrecognized)?;
    let words = palette_words(data, 2, 16).ok_or(DecodeError::Unrecognized)?;
    let (unpacked, _) =
        unpack_bits(&data[HEADER_LEN..], SCREEN_LEN).ok_or(DecodeError::Unrecognized)?;
    let bitmap = line_planes_to_interleaved(
        &unpacked,
        resolution.width(),
        resolution.height(),
        resolution.planes(),
    )
    .ok_or(DecodeError::Unrecognized)?;
    decode_screen(resolution, &bitmap, &words).ok_or(DecodeError::Unrecognized)
}
