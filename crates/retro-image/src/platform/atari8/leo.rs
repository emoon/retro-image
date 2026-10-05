//! LEO (Larka Edytor Obiektow, "Larka object editor"): a 256-glyph ANTIC mode 4
//! character set shown as 16x16-pixel objects.
//!
//! Sources:
//! - Just Solve "Larka Edytor Obiekt"
//!   (<http://fileformats.archiveteam.org/wiki/Larka_Edytor_Obiekt>; ANTIC 4
//!   objects, 5 colors). ANTIC mode 4 glyphs and the inverse bit: De Re Atari
//!   ch. 2 (<https://www.atariarchives.org/dere/chapt02.php>).
//! - The layout is reverse engineered from the corpus sample PINGWINKI.LEO by
//!   flipping single bytes and reading back with `recoil2png` which pixels
//!   change: 256 glyphs of 8 bytes, a 256-byte table (one entry per cell of
//!   the 32 x 8 cell picture: low 7 bits the glyph, bit 7 selecting color 3
//!   for pixel value 3), a second 256-byte table that RECOIL ignores, five
//!   colors, and 15 ignored bytes. The cell order within the picture, which
//!   interleaves the four glyphs of an object, and the color order (the
//!   registers in shadow order, playfield 0-3 then background) are observed from `recoil2png` output.

use super::font::draw_multicolor_glyph;
use crate::{DecodeError, Image};

const GLYPHS: usize = 256;
const COLUMNS: usize = 32;
const ROWS: usize = 8;

/// Exactly 2580 bytes: glyphs, cell table, ignored table, the color
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::atari8::palette::register_rgb;
    use alloc::vec;

    #[test]
    fn cells_interleave_the_four_glyphs_of_an_object() {
        let mut data = vec![0; 2580];
        // Cell table: every cell shows glyph 1 (so index bit 6 selects glyphs 1 or 129).
        for entry in &mut data[2048..2304] {
            *entry = 1;
        }
        data[8] = 0xff; // glyph 1, first line: pixel value 3
        data[129 * 8] = 0x55; // glyph 129: pixel value 1
        data[2560..2565].copy_from_slice(&[0x22, 0x44, 0x66, 0x88, 0x0a]);
        let image = decode_leo(&data).unwrap();
        assert_eq!((image.width(), image.height()), (256, 64));
        // Cell 0 is the top left, 1 two cells right, 128 one cell right and
        // 64 one cell down, which shows glyph 129 instead of 1.
        assert_eq!(image.get(0, 0), register_rgb(0x66));
        assert_eq!(image.get(16, 0), register_rgb(0x66));
        assert_eq!(image.get(8, 0), register_rgb(0x66));
        assert_eq!(image.get(0, 8), register_rgb(0x22));
        assert!(decode_leo(&data[..2579]).is_err());
    }
}
