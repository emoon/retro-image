//! ZX Spectrum.
//!
//! Sources:
//! - Screen memory layout (bitmap interleave, attribute byte): ZX Spectrum
//!   hardware documentation, see `docs/formats/sinclair-cpc-bbc-misc.md`.
//! - Palette levels (0x00 / 0xCD / 0xFF): observed from `recoil2png` output.

use crate::{DecodeError, Format, Image};

pub(super) const SCR: Format = Format::new("ZX Spectrum", "Screen dump", &["scr"], decode_scr);

const WIDTH: u32 = 256;
const HEIGHT: u32 = 192;
const BITMAP_LEN: usize = 6144;
const SCR_LEN: usize = BITMAP_LEN + 768;

/// Raw 6912-byte dump of screen memory: bitmap then attributes.
fn decode_scr(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != SCR_LEN {
        return Err(DecodeError::Unrecognized);
    }
    let (bitmap, attributes) = data.split_at(BITMAP_LEN);
    let mut image = Image::new(WIDTH, HEIGHT);
    for y in 0..HEIGHT {
        for column in 0..WIDTH / 8 {
            let pixels = bitmap[bitmap_offset(y) + column as usize];
            let attribute = attributes[(y / 8 * 32 + column) as usize];
            for bit in 0..8 {
                let x = column * 8 + bit;
                let set = pixels & (0x80 >> bit) != 0;
                image.set(x, y, attribute_color(attribute, set));
            }
        }
    }
    Ok(image)
}

/// Byte offset of pixel row `y`: the row number's bits are stored as
/// third (bits 7-6), character row (bits 5-3) and pixel line (bits 2-0),
/// laid out in memory as third, pixel line, character row.
fn bitmap_offset(y: u32) -> usize {
    let third = (y >> 6) & 3;
    let char_row = (y >> 3) & 7;
    let line = y & 7;
    ((third << 11) | (line << 8) | (char_row << 5)) as usize
}

/// Attribute byte: bit 7 flash (ignored), bit 6 bright, bits 5-3 paper, bits 2-0 ink.
fn attribute_color(attribute: u8, ink: bool) -> u32 {
    let index = if ink {
        attribute & 7
    } else {
        (attribute >> 3) & 7
    };
    let bright = attribute & 0x40 != 0;
    color(index, bright)
}

/// Color index bits: 0 blue, 1 red, 2 green.
fn color(index: u8, bright: bool) -> u32 {
    let level: u32 = if bright { 0xff } else { 0xcd };
    let channel = |bit: u8| if index & bit != 0 { level } else { 0 };
    channel(2) << 16 | channel(4) << 8 | channel(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitmap_offset_follows_screen_interleave() {
        assert_eq!(bitmap_offset(0), 0);
        assert_eq!(bitmap_offset(1), 0x100);
        assert_eq!(bitmap_offset(8), 0x20);
        assert_eq!(bitmap_offset(64), 0x800);
        assert_eq!(bitmap_offset(191), 0x17e0);
    }

    #[test]
    fn colors_use_grb_bits_and_bright_level() {
        assert_eq!(color(0, true), 0x000000);
        assert_eq!(color(1, false), 0x0000cd);
        assert_eq!(color(2, false), 0xcd0000);
        assert_eq!(color(4, true), 0x00ff00);
        assert_eq!(color(7, false), 0xcdcdcd);
    }

    #[test]
    fn rejects_wrong_size() {
        assert_eq!(decode_scr(&[0; 6144]), Err(DecodeError::Unrecognized));
    }

    #[test]
    fn decodes_ink_and_paper() {
        let mut data = [0u8; SCR_LEN];
        data[0] = 0x80; // top-left pixel set
        data[BITMAP_LEN] = 0x4a; // bright, paper 1 (blue), ink 2 (red)
        let image = decode_scr(&data).unwrap();
        assert_eq!(&image.rgb()[0..6], &[0xff, 0, 0, 0, 0, 0xff]);
    }
}
