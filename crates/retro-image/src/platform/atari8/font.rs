//! 8x8 character sets: FNT (one set of 128 characters) and FN2 (two sets).
//!
//! Sources:
//! - FNT: Graph2Font manual and De Re Atari ch. 3 (1024-byte charset,
//!   8 bytes per character, top row first, bit 7 leftmost).
//! - FN2: Atari FontMaker page ("dual font", 2 x 1024 bytes); docs only.
//!   Drawing the two fonts as halves of 8x16 characters: observed from
//!   `recoil2png` output.
//! - Accepted sizes (FNT 1024-1026 bytes), the sheet layout of 32
//!   characters per row and the colours: observed from `recoil2png` output.

use super::antic::fill;
use super::palette::register_rgb;
use crate::{DecodeError, Image};

const CHARS_PER_ROW: usize = 32;

/// Raw 128-character font; RECOIL also accepts 1 or 2 trailing bytes.
pub(super) fn decode_fnt(data: &[u8]) -> Result<Image, DecodeError> {
    match data.len() {
        1024..=1026 => Ok(sheet(&[&data[..1024]])),
        _ => Err(DecodeError::Unrecognized),
    }
}

/// Two 128-character fonts shown as 8x16 characters: the first font gives
/// the top half, the second the bottom half.
pub(super) fn decode_fn2(data: &[u8]) -> Result<Image, DecodeError> {
    match data.len() {
        2048 => Ok(sheet(&[&data[..1024], &data[1024..]])),
        _ => Err(DecodeError::Unrecognized),
    }
}

/// Draws 128 characters 32 to a row, white on black. Each character stacks
/// its glyph from every font in `fonts`.
fn sheet(fonts: &[&[u8]]) -> Image {
    let char_height = 8 * fonts.len() as u32;
    let rows = 128 / CHARS_PER_ROW as u32;
    let mut image = Image::new(CHARS_PER_ROW as u32 * 8, rows * char_height);
    let (background, foreground) = (register_rgb(0x00), register_rgb(0x0e));
    for (part, font) in fonts.iter().enumerate() {
        for (index, glyph) in font.chunks_exact(8).enumerate() {
            let x = (index % CHARS_PER_ROW) as u32 * 8;
            let y = (index / CHARS_PER_ROW) as u32 * char_height + part as u32 * 8;
            draw_glyph(&mut image, x, y, glyph, |set| {
                if set { foreground } else { background }
            });
        }
    }
    image
}

/// Draws an 8x8 1-bit glyph with its top-left corner at (`x`, `y`).
pub(super) fn draw_glyph(
    image: &mut Image,
    x: u32,
    y: u32,
    glyph: &[u8],
    color: impl Fn(bool) -> u32,
) {
    for (row, &bits) in glyph.iter().enumerate() {
        for column in 0..8 {
            let set = bits & (0x80 >> column) != 0;
            fill(image, x + column, y + row as u32, 1, 1, color(set));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lays_out_32_chars_per_row() {
        let mut data = [0u8; 1024];
        data[33 * 8] = 0x80; // character 33: row 1, column 1
        let image = decode_fnt(&data).unwrap();
        assert_eq!((image.width(), image.height()), (256, 32));
        let i = (8 * 256 + 8) * 3;
        assert_eq!(&image.rgb()[i..i + 3], &[0xee, 0xee, 0xee]);
    }

    #[test]
    fn rejects_wrong_sizes() {
        assert!(decode_fnt(&[0; 1027]).is_err());
        assert!(decode_fn2(&[0; 1024]).is_err());
    }
}
