//! C64 character sets shown as glyph sheets: `.64c` fonts, SEUCK `.g`
//! fonts and Star Painter `.zs` fonts.
//!
//! Sources:
//! - Layout (load address, then 8 bytes per character):
//!   <https://www.c64-wiki.com/wiki/Character_set>,
//!   <http://fileformats.archiveteam.org/wiki/Shoot_'Em_Up_Construction_Kit>.
//! - Star Painter fonts: reverse engineered from samples. Load address
//!   `$F0B0`, then 9 bytes per character: a width byte and 8 rows; the
//!   1024 bytes hold 113 characters and the start of a 114th. Checked
//!   against `recoil2png` output, which shows 128 character cells.
//! - Sheet layout (32 characters per row, white on black, last character
//!   padded) observed from `recoil2png` output.

use crate::{DecodeError, Image};

const PER_ROW: usize = 32;

/// SEUCK font: a 64-character set (the only size seen in samples).
pub(super) fn decode_seuck_font(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 2 + 64 * 8 {
        return Err(DecodeError::Invalid);
    }
    decode_font(data)
}

/// Star Painter font: 9-byte records (width, 8 rows) after the load
/// address, shown as a 128-character sheet; rows past the end are blank.
pub(super) fn decode_star_painter_font(data: &[u8]) -> Result<Image, DecodeError> {
    const RECORD: usize = 9;
    if data.len() != 2 + 1024 {
        return Err(DecodeError::Invalid);
    }
    let mut glyphs = alloc::vec![0u8; 128 * 8];
    for (glyph, record) in glyphs
        .as_chunks_mut::<8>()
        .0
        .iter_mut()
        .zip(data[2..].chunks(RECORD))
    {
        let rows = record.get(1..).unwrap_or_default();
        glyph[..rows.len()].copy_from_slice(rows);
    }
    sheet(&glyphs)
}

/// Load address, then up to 256 characters of 8 bytes.
pub(super) fn decode_font(data: &[u8]) -> Result<Image, DecodeError> {
    let glyphs = data.get(2..).ok_or(DecodeError::Invalid)?;
    if glyphs.is_empty() || glyphs.len() > 256 * 8 {
        return Err(DecodeError::Invalid);
    }
    sheet(glyphs)
}

/// Glyphs of 8 bytes, white on black, `PER_ROW` per row.
fn sheet(glyphs: &[u8]) -> Result<Image, DecodeError> {
    let rows = glyphs.len().div_ceil(8 * PER_ROW);
    let mut image = Image::new((PER_ROW * 8) as u32, (rows * 8) as u32)?;
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
