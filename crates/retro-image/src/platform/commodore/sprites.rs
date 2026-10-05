//! C64 sprite collections shown as sheets: SpritePad (`.spd`) and
//! Shoot 'Em Up Construction Kit (`.a`).
//!
//! Sources:
//! - SEUCK sprites: 127 sprites of 64 bytes after a load address,
//!   <http://fileformats.archiveteam.org/wiki/Shoot_'Em_Up_Construction_Kit>;
//!   all multicolor, the shared colors and the sheet width (keeping the
//!   gap after the last column) observed from `recoil2png` output.
//! - SpritePad format (headerless and `SPD` version 1: colors, then 63
//!   bytes plus an attribute byte per sprite, attribute bits 0-3 color and
//!   bit 7 multicolor): CSDb forum thread "SPD format",
//!   <https://csdb.dk/forums/?roomid=7&topicid=125812>.
//! - SpritePad 2 files (`SPD` versions 3 to 5): reverse engineered from 45
//!   sample files in `corpus/extra/spritepad-spd45/`. Header field order
//!   follows c64lib's SPD reader (MIT,
//!   <https://github.com/c64lib/gradle-retro-assembler-plugin/tree/master/processors/spritepad>);
//!   the flags value and the 6 bytes per animation were found by matching
//!   file sizes in all samples, and the sprite attribute bits are the same
//!   as in version 1. Rendered sheets were reviewed visually. `recoil2png`
//!   rejects these versions.
//! - Sprite pixel semantics (multicolor `01` = multicolor 1, `10` = sprite
//!   color, `11` = multicolor 2): <https://www.cebix.net/VIC-Article.txt>.
//! - Sheet layout (16 sprites per row, 2-pixel gaps, everything else in the
//!   background color) observed from `recoil2png` output.

use super::vic2::rgb;
use crate::bytes::le16;
use crate::image::check_size;
use crate::{DecodeError, Image};

const PER_ROW: usize = 16;
const GAP: usize = 2;
/// SpritePad 2 flag: animation records follow the sprites.
const ANIMATIONS: u8 = 2;
const ANIMATION_LEN: usize = 6;
const SPRITE_WIDTH: usize = 24;
const SPRITE_HEIGHT: usize = 21;

/// Shared colors of a sprite collection.
struct Colors {
    background: u8,
    multi1: u8,
    multi2: u8,
}

/// Draws 64-byte sprites (63 bytes of pixels and an attribute byte).
/// `trailing_gap` keeps a gap after the last column.
fn render(sprites: &[u8], colors: &Colors, trailing_gap: bool) -> Result<Image, DecodeError> {
    let count = sprites.len() / 64;
    let columns = count.min(PER_ROW);
    let rows = count.div_ceil(PER_ROW);
    let width = columns * (SPRITE_WIDTH + GAP) - if trailing_gap { 0 } else { GAP };
    let height = rows * (SPRITE_HEIGHT + GAP) - GAP;
    check_size(width, height)?;
    let mut image = Image::new(width as u32, height as u32);
    let background = rgb(colors.background);
    for y in 0..height {
        for x in 0..width {
            image.set(x as u32, y as u32, background);
        }
    }
    for (i, sprite) in sprites.as_chunks::<64>().0.iter().enumerate() {
        let left = i % PER_ROW * (SPRITE_WIDTH + GAP);
        let top = i / PER_ROW * (SPRITE_HEIGHT + GAP);
        let attribute = sprite[63];
        for y in 0..SPRITE_HEIGHT {
            for x in 0..SPRITE_WIDTH {
                let byte = sprite[y * 3 + x / 8];
                let color = if attribute & 0x80 != 0 {
                    match byte >> (6 - (x & 6)) & 3 {
                        0 => colors.background,
                        1 => colors.multi1,
                        2 => attribute,
                        _ => colors.multi2,
                    }
                } else if byte & (0x80 >> (x % 8)) != 0 {
                    attribute
                } else {
                    colors.background
                };
                image.set((left + x) as u32, (top + y) as u32, rgb(color));
            }
        }
    }
    Ok(image)
}

/// SpritePad `SPD`. Version 1: magic, sprite count minus one, animation
/// count, the three shared colors, then the sprites. Versions 3 to 5: see
/// [`decode_spd_v3`].
pub(super) fn decode_spd(data: &[u8]) -> Result<Image, DecodeError> {
    match data {
        [b'S', b'P', b'D', 3..=5, ..] => decode_spd_v3(data),
        [
            b'S',
            b'P',
            b'D',
            1,
            count,
            _,
            background,
            multi1,
            multi2,
            rest @ ..,
        ] => {
            let len = (usize::from(*count) + 1) * 64;
            let sprites = rest.get(..len).ok_or(DecodeError::Unrecognized)?;
            render_spd([*background, *multi1, *multi2], sprites)
        }
        _ => Err(DecodeError::Unrecognized),
    }
}

/// SpritePad `SPD` versions 3 to 5 (SpritePad 2): after the version byte come
/// flags (2 = animations present), the 16-bit sprite and tile counts, the
/// sprite and tile animation counts minus one, tile width and height, the
/// three shared colors and, from version 4 on, four bytes of overlay
/// distances (16 bytes before them, 20 with). The sprites follow, then
/// 6 bytes per sprite animation when flags say so. The size must match
/// exactly; files with tiles are not decoded because no sample shows where
/// their data sits.
fn decode_spd_v3(data: &[u8]) -> Result<Image, DecodeError> {
    let header_len = if data[3] == 3 { 16 } else { 20 };
    let header = data.get(..header_len).ok_or(DecodeError::Unrecognized)?;
    let flags = header[4];
    let count = usize::from(le16(header, 5).ok_or(DecodeError::Unrecognized)?);
    let tiles = le16(header, 7).ok_or(DecodeError::Unrecognized)?;
    let animations = if flags == ANIMATIONS {
        (usize::from(header[9]) + 1) * ANIMATION_LEN
    } else {
        0
    };
    let sprites_end = header_len + count * 64;
    if (flags != 0 && flags != ANIMATIONS)
        || tiles != 0
        || count == 0
        || data.len() != sprites_end + animations
    {
        return Err(DecodeError::Unrecognized);
    }
    render_spd(
        [header[13], header[14], header[15]],
        &data[header_len..sprites_end],
    )
}

/// Headerless SpritePad: the three shared colors, then the sprites.
pub(super) fn decode_spd_raw(data: &[u8]) -> Result<Image, DecodeError> {
    let [background, multi1, multi2, sprites @ ..] = data else {
        return Err(DecodeError::Unrecognized);
    };
    if sprites.len() % 64 != 0 {
        return Err(DecodeError::Unrecognized);
    }
    render_spd([*background, *multi1, *multi2], sprites)
}

fn render_spd(colors: [u8; 3], sprites: &[u8]) -> Result<Image, DecodeError> {
    if sprites.is_empty() || colors.iter().any(|&c| c > 15) {
        return Err(DecodeError::Unrecognized);
    }
    let [background, multi1, multi2] = colors;
    render(
        sprites,
        &Colors {
            background,
            multi1,
            multi2,
        },
        false,
    )
}

/// SEUCK sprites: load address and 127 multicolor sprites. The file has no
/// shared colors; `recoil2png` uses dark gray, black and white.
pub(super) fn decode_seuck(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 2 + 127 * 64 {
        return Err(DecodeError::Unrecognized);
    }
    let mut sprites = data[2..].to_vec();
    for sprite in sprites.as_chunks_mut::<64>().0 {
        sprite[63] |= 0x80;
    }
    let colors = Colors {
        background: 11,
        multi1: 0,
        multi2: 1,
    };
    render(&sprites, &colors, true)
}
