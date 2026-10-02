//! DUO overscan pictures by Anders Eriksson (`DU1`, `DUO`, `DU2`): two
//! alternating screens shown in quick succession.
//!
//! Sources:
//! - <https://temlib.org/AtariForumWiki/index.php/DUO_file_format>
//! - <http://fileformats.archiveteam.org/wiki/DUO>
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

fn decode(data: &[u8], colors: usize, width: u32, planes: u32, y_scale: u32) -> Option<Image> {
    let palette = st_palette(&palette_words(data, 0, colors)?);
    let first = &data[colors * 2..];
    let second = &first[SCREEN_LEN..];
    let a = planar_image(first, width, HEIGHT, planes, &palette, y_scale)?;
    let b = planar_image(second, width, HEIGHT, planes, &palette, y_scale)?;
    let mut image = Image::new(width, HEIGHT * y_scale);
    for (i, (pa, pb)) in a
        .rgb()
        .chunks_exact(3)
        .zip(b.rgb().chunks_exact(3))
        .enumerate()
    {
        let mix = |k: usize| (u32::from(pa[k]) + u32::from(pb[k])) / 2;
        let i = i as u32;
        image.set(i % width, i / width, mix(0) << 16 | mix(1) << 8 | mix(2));
    }
    Some(image)
}
