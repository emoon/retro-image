//! C64 character sets shown as glyph sheets: `.64c` fonts and SEUCK `.g`
//! fonts.
//!
//! Sources:
//! - Layout (load address, then 8 bytes per character):
//!   <https://www.c64-wiki.com/wiki/Character_set>,
//!   <http://fileformats.archiveteam.org/wiki/Shoot_'Em_Up_Construction_Kit>.
//! - Sheet layout (32 characters per row, white on black, last character
//!   padded) observed from `recoil2png` output.

use crate::{DecodeError, Image};

const PER_ROW: usize = 32;

/// SEUCK font: a 64-character set (the only size seen in samples).
pub(super) fn decode_seuck_font(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 2 + 64 * 8 {
        return Err(DecodeError::Unrecognized);
    }
    decode_font(data)
}

/// Load address, then up to 256 characters of 8 bytes.
pub(super) fn decode_font(data: &[u8]) -> Result<Image, DecodeError> {
    let glyphs = data.get(2..).ok_or(DecodeError::Unrecognized)?;
    if glyphs.is_empty() || glyphs.len() > 256 * 8 {
        return Err(DecodeError::Unrecognized);
    }
    let rows = glyphs.len().div_ceil(8 * PER_ROW);
    let mut image = Image::new((PER_ROW * 8) as u32, (rows * 8) as u32);
    for (i, &byte) in glyphs.iter().enumerate() {
        let char = i / 8;
        let x = char % PER_ROW * 8;
        let y = char / PER_ROW * 8 + i % 8;
        for bit in 0..8 {
            if byte & (0x80 >> bit) != 0 {
                image.set((x + bit) as u32, y as u32, 0xffffff);
            }
        }
    }
    Ok(image)
}
