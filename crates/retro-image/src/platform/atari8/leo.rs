//! LEO (Larka Edytor Obiektow, "Larka object editor"): a 256-glyph ANTIC mode 4
//! character set shown as 16x16-pixel objects.
//!
//! Sources:
//! - Just Solve "Larka Edytor Obiekt"
//!   (<http://fileformats.archiveteam.org/wiki/Larka_Edytor_Obiekt>; ANTIC 4
//!   objects, 5 colours). ANTIC mode 4 glyphs and the inverse bit: De Re Atari
//!   ch. 2 (<https://www.atariarchives.org/dere/chapt02.php>).
//! - The layout is reverse engineered from the corpus sample PINGWINKI.LEO by
//!   flipping single bytes and reading back with `recoil2png` which pixels
//!   change: 256 glyphs of 8 bytes, a 256-byte table (one entry per cell of
//!   the 32 x 8 cell picture: low 7 bits the glyph, bit 7 selecting colour 3
//!   for pixel value 3), a second 256-byte table that RECOIL ignores, five
//!   colours, and 15 ignored bytes. The cell order within the picture, which
//!   interleaves the four glyphs of an object, and the colour order (the
//!   registers in shadow order, playfield 0-3 then background) are observed from `recoil2png` output.

use super::font::draw_multicolor_glyph;
use crate::{DecodeError, Image};

const GLYPHS: usize = 256;
const COLUMNS: usize = 32;
const ROWS: usize = 8;

/// Exactly 2580 bytes: glyphs, cell table, ignored table, the colour
/// registers playfield 0-3 and background, 15 ignored bytes.
pub(super) fn decode_leo(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 2580 {
        return Err(DecodeError::Unrecognized);
    }
    let (glyphs, rest) = data.split_at(GLYPHS * 8);
    let (cells, rest) = rest.split_at(GLYPHS);
    let [pf0, pf1, pf2, pf3, background] = rest[GLYPHS..GLYPHS + 5] else {
        return Err(DecodeError::Unrecognized);
    };
    let mut image = Image::new(COLUMNS as u32 * 8, ROWS as u32 * 8);
    for (index, &cell) in cells.iter().enumerate() {
        // Bit 7 of the index is the column parity, bit 6 the row parity, bits
        // 5-4 the row pair, bits 3-0 the column pair.
        let column = (index >> 7) + 2 * (index & 15);
        let row = (index >> 6 & 1) + 2 * (index >> 4 & 3);
        // Glyphs 128-255 are those of cells whose index has bit 6 set.
        let glyph = usize::from(cell & 0x7f) | (index & 0x40) << 1;
        let third = if cell & 0x80 != 0 { pf3 } else { pf2 };
        draw_multicolor_glyph(
            &mut image,
            column as u32 * 8,
            row as u32 * 8,
            &glyphs[glyph * 8..][..8],
            [background, pf0, pf1, third],
        );
    }
    Ok(image)
}
