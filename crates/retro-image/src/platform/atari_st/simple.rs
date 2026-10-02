//! Simple uncompressed ST screen formats: NEOchrome (`NEO`) and Doodle (`DOO`).
//!
//! Sources:
//! - NEOchrome, including the 640x400 `0xBABE` virtual canvas:
//!   <https://temlib.org/AtariForumWiki/index.php/NEOchrome_file_format>
//! - Doodle: <https://temlib.org/AtariForumWiki/index.php/Doodle_file_format>

use super::common::{
    Resolution, SCREEN_LEN, be16, decode_screen, palette_words, planar_image, st_palette,
};
use crate::{DecodeError, Image};

const NEO_HEADER_LEN: usize = 128;
const CANVAS_FLAG: u16 = 0xbabe;

/// NEOchrome: flag word, resolution word, 16 palette words, ..., screen at 128.
pub(super) fn decode_neo(data: &[u8]) -> Result<Image, DecodeError> {
    let flag = be16(data, 0).ok_or(DecodeError::Unrecognized)?;
    let words = palette_words(data, 4, 16).ok_or(DecodeError::Unrecognized)?;
    let bitmap = data
        .get(NEO_HEADER_LEN..)
        .ok_or(DecodeError::Unrecognized)?;
    let image = match flag {
        0 if data.len() == NEO_HEADER_LEN + SCREEN_LEN => {
            let resolution = be16(data, 2)
                .and_then(Resolution::from_index)
                .ok_or(DecodeError::Unrecognized)?;
            decode_screen(resolution, bitmap, &words)
        }
        CANVAS_FLAG if data.len() == NEO_HEADER_LEN + 4 * SCREEN_LEN => {
            planar_image(bitmap, 640, 400, 4, &st_palette(&words), 1)
        }
        _ => None,
    };
    image.ok_or(DecodeError::Unrecognized)
}

/// Doodle: a raw 32000-byte high-resolution screen.
pub(super) fn decode_doo(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != SCREEN_LEN {
        return Err(DecodeError::Unrecognized);
    }
    decode_screen(Resolution::High, data, &[0]).ok_or(DecodeError::Unrecognized)
}
