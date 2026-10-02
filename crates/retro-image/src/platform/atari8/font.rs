//! 8x8 character sets: FNT (one set of 128 characters) and FN2 (two sets).
//!
//! Sources:
//! - FNT: Graph2Font manual and De Re Atari ch. 3 (1024-byte charset,
//!   8 bytes per character, top row first, bit 7 leftmost).
//! - FN2: Atari FontMaker page ("dual font", 2 x 1024 bytes); docs only.
//!   Drawing the two fonts as halves of 8x16 characters: observed from
//!   `recoil2png` output.
//! - SIF: Super-IRG Font Editor doc SIFE.TXT by Bill Kendrick (two 1024-byte
//!   ANTIC mode 4 charsets flipped every frame).
//! - JGP: Just Solve "Jet Graphics Planner" (exactly 2054 bytes, 4 colours).
//!   The binary-load header, the two charsets stacked as 8x16 characters
//!   and the grey colours: observed from `recoil2png` output.
//! - Accepted sizes (FNT 1024-1026 bytes), the sheet layout of 32
//!   characters per row and the colours (SIF: 0x00, 0x4C, 0xCC, 0x8C, the
//!   two charsets mixed): observed from `recoil2png` output.

use super::antic::{Bitmap, fill, mix};
use super::palette::register_rgb;
use super::screen::GREY_COLORS;
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

/// Super-IRG font: two ANTIC mode 4 charsets shown on alternate frames.
pub(super) fn decode_sif(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 2048 {
        return Err(DecodeError::Unrecognized);
    }
    const COLORS: [u8; 4] = [0x00, 0x4c, 0xcc, 0x8c];
    let charset = |font: &[u8]| {
        let mut image = Image::new(CHARS_PER_ROW as u32 * 8, 32);
        for (index, glyph) in font.chunks_exact(8).enumerate() {
            let x = (index % CHARS_PER_ROW) as u32 * 8;
            let y = (index / CHARS_PER_ROW) as u32 * 8;
            draw_multicolor_glyph(&mut image, x, y, glyph, COLORS);
        }
        image
    };
    Ok(mix(&charset(&data[..1024]), &charset(&data[1024..])))
}

/// Jet Graphics Planner: a DOS binary-load header for $A000-$A7FF, then two
/// ANTIC mode 4 charsets shown as 8x16 characters (first charset on top),
/// in greys.
pub(super) fn decode_jgp(data: &[u8]) -> Result<Image, DecodeError> {
    let Some(charsets) = data.strip_prefix(&[0xff, 0xff, 0x00, 0xa0, 0xff, 0xa7]) else {
        return Err(DecodeError::Unrecognized);
    };
    if charsets.len() != 2048 {
        return Err(DecodeError::Unrecognized);
    }
    let mut image = Image::new(CHARS_PER_ROW as u32 * 8, 64);
    for (part, charset) in charsets.chunks_exact(1024).enumerate() {
        for (index, glyph) in charset.chunks_exact(8).enumerate() {
            let x = (index % CHARS_PER_ROW) as u32 * 8;
            let y = (index / CHARS_PER_ROW) as u32 * 16 + part as u32 * 8;
            draw_multicolor_glyph(&mut image, x, y, glyph, GREY_COLORS);
        }
    }
    Ok(image)
}

/// Draws an ANTIC mode 4 glyph (4x8 pixels of 2 bits, each 2 pixels wide)
/// with its top-left corner at (`x`, `y`); `colors` are background and
/// playfield 0-2.
pub(super) fn draw_multicolor_glyph(
    image: &mut Image,
    x: u32,
    y: u32,
    glyph: &[u8],
    colors: [u8; 4],
) {
    let bitmap = Bitmap {
        data: glyph,
        bytes_per_line: 1,
        lines: 8,
        bits: 2,
    };
    for row in 0..8 {
        for column in 0..4 {
            let color = register_rgb(colors[usize::from(bitmap.pixel(column, row))]);
            fill(image, x + 2 * column as u32, y + row as u32, 2, 1, color);
        }
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
