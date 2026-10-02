//! Overscan pictures: DUO by Anders Eriksson (`DU1`, `DUO`, `DU2`), two
//! alternating screens shown in quick succession, and Fullscreen
//! Construction Kit (`KID`).
//!
//! Sources:
//! - <https://temlib.org/AtariForumWiki/index.php/DUO_file_format>
//! - <http://fileformats.archiveteam.org/wiki/DUO>
//! - Fullscreen Construction Kit: <http://fileformats.archiveteam.org/wiki/Fullscreen_Construction_Kit>
//!   (size, magic); the line layout is derived from sample files.
//! - Observed from `recoil2png` output: the two screens are averaged per
//!   component (rounding down); medium resolution lines are doubled.

use super::common::{palette_words, planar_image, st_palette};
use crate::{DecodeError, Image};

const SCREEN_LEN: usize = 56784;
const HEIGHT: u32 = 273;

/// Low resolution: 16-word palette, two 416x273 screens.
pub(super) fn decode_duo(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 32 + 2 * SCREEN_LEN {
        return Err(DecodeError::Unrecognized);
    }
    decode(data, 16, 416, 4, 1).ok_or(DecodeError::Unrecognized)
}

/// Medium resolution: 4-word palette, two 832x273 screens, optional padding.
pub(super) fn decode_du2(data: &[u8]) -> Result<Image, DecodeError> {
    let len = 8 + 2 * SCREEN_LEN;
    if data.len() != len && data.len() != len + 24 {
        return Err(DecodeError::Unrecognized);
    }
    decode(data, 4, 832, 2, 2).ok_or(DecodeError::Unrecognized)
}

/// Fullscreen Construction Kit (`KID`): `KD`, 16 palette words, then 274
/// lines of 230 bytes, the first 224 of which hold 448 low-resolution
/// pixels. Sources: <http://fileformats.archiveteam.org/wiki/Fullscreen_Construction_Kit>
/// (size, magic); the line layout is derived from sample files.
pub(super) fn decode_kid(data: &[u8]) -> Result<Image, DecodeError> {
    const LINE: usize = 230;
    if data.len() != 34 + 274 * LINE || data.get(..2) != Some(b"KD") {
        return Err(DecodeError::Unrecognized);
    }
    let palette = st_palette(&palette_words(data, 2, 16).ok_or(DecodeError::Unrecognized)?);
    let bitmap: alloc::vec::Vec<u8> = data[34..]
        .as_chunks::<LINE>()
        .0
        .iter()
        .flat_map(|line| &line[..224])
        .copied()
        .collect();
    planar_image(&bitmap, 448, 274, 4, &palette, 1).ok_or(DecodeError::Unrecognized)
}

fn decode(data: &[u8], colors: usize, width: u32, planes: u32, y_scale: u32) -> Option<Image> {
    let palette = st_palette(&palette_words(data, 0, colors)?);
    let first = &data[colors * 2..];
    let second = &first[SCREEN_LEN..];
    let a = planar_image(first, width, HEIGHT, planes, &palette, y_scale)?;
    let b = planar_image(second, width, HEIGHT, planes, &palette, y_scale)?;
    Some(Image::blend(&[&a, &b]))
}
