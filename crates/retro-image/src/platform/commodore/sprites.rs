//! C64 sprite collections shown as sheets: SpritePad (`.spd`) and
//! Shoot 'Em Up Construction Kit (`.a`).
//!
//! Sources:
//! - SEUCK sprites: 127 sprites of 64 bytes after a load address,
//!   <http://fileformats.archiveteam.org/wiki/Shoot_'Em_Up_Construction_Kit>;
//!   all multicolour, the shared colours and the sheet width (keeping the
//!   gap after the last column) observed from `recoil2png` output.
//! - SpritePad format (headerless and `SPD` version 1: colours, then 63
//!   bytes plus an attribute byte per sprite, attribute bits 0-3 colour and
//!   bit 7 multicolour): CSDb forum thread "SPD format",
//!   <https://csdb.dk/forums/?roomid=7&topicid=125812>.
//! - Sprite pixel semantics (multicolour `01` = multicolour 1, `10` = sprite
//!   colour, `11` = multicolour 2): <https://www.cebix.net/VIC-Article.txt>.
//! - Sheet layout (16 sprites per row, 2-pixel gaps, everything else in the
//!   background colour) observed from `recoil2png` output.

use super::vic2::rgb;
use crate::{DecodeError, Image};

const PER_ROW: usize = 16;
const GAP: usize = 2;
const SPRITE_WIDTH: usize = 24;
const SPRITE_HEIGHT: usize = 21;

/// Shared colours of a sprite collection.
struct Colors {
    background: u8,
    multi1: u8,
    multi2: u8,
}

/// Draws 64-byte sprites (63 bytes of pixels and an attribute byte).
/// `trailing_gap` keeps a gap after the last column.
fn render(sprites: &[u8], colors: &Colors, trailing_gap: bool) -> Image {
    let count = sprites.len() / 64;
    let columns = count.min(PER_ROW);
    let rows = count.div_ceil(PER_ROW);
    let width = columns * (SPRITE_WIDTH + GAP) - if trailing_gap { 0 } else { GAP };
    let height = rows * (SPRITE_HEIGHT + GAP) - GAP;
    let mut image = Image::new(width as u32, height as u32);
    let background = rgb(colors.background);
    for y in 0..height {
        for x in 0..width {
            image.set(x as u32, y as u32, background);
        }
    }
    for (i, sprite) in sprites.chunks_exact(64).enumerate() {
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
    image
}

/// SpritePad `SPD` version 1: magic, sprite count minus one, animation
/// count, the three shared colours, then the sprites.
pub(super) fn decode_spd(data: &[u8]) -> Result<Image, DecodeError> {
    let [
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
    ] = data
    else {
        return Err(DecodeError::Unrecognized);
    };
    let len = (usize::from(*count) + 1) * 64;
    let sprites = rest.get(..len).ok_or(DecodeError::Unrecognized)?;
    render_spd([*background, *multi1, *multi2], sprites)
}

/// Headerless SpritePad: the three shared colours, then the sprites.
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
    Ok(render(
        sprites,
        &Colors {
            background,
            multi1,
            multi2,
        },
        false,
    ))
}

/// SEUCK sprites: load address and 127 multicolour sprites. The file has no
/// shared colours; `recoil2png` uses dark grey, black and white.
pub(super) fn decode_seuck(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 2 + 127 * 64 {
        return Err(DecodeError::Unrecognized);
    }
    let mut sprites = data[2..].to_vec();
    for sprite in sprites.chunks_exact_mut(64) {
        sprite[63] |= 0x80;
    }
    let colors = Colors {
        background: 11,
        multi1: 0,
        multi2: 1,
    };
    Ok(render(&sprites, &colors, true))
}
