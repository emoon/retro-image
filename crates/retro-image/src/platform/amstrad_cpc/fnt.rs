//! Advanced OCP Art Studio fonts (FNT).
//!
//! Sources:
//! - 96 characters (ASCII 32-127) of 8 bytes, most significant bit left:
//!   OCP Art Studio file formats, <https://cpctech.cpcwiki.de/docs/artstud.html>.
//! - Shown as a sheet of 32 characters per row, white on black: observed
//!   from `recoil2png` output.

use super::amsdos::strip_amsdos;
use crate::{DecodeError, Image};

const CHARACTERS: usize = 96;
const PER_ROW: usize = 32;

pub(super) fn decode_fnt(data: &[u8]) -> Result<Image, DecodeError> {
    let font = strip_amsdos(data);
    if font.len() != CHARACTERS * 8 {
        return Err(DecodeError::Unrecognized);
    }
    let mut image = Image::new((PER_ROW * 8) as u32, (CHARACTERS / PER_ROW * 8) as u32)?;
    for (index, glyph) in font.as_chunks::<8>().0.iter().enumerate() {
        let (left, top) = (index % PER_ROW * 8, index / PER_ROW * 8);
        for (y, &bits) in glyph.iter().enumerate() {
            for x in 0..8 {
                let color = if bits & (0x80 >> x) != 0 { 0xffffff } else { 0 };
                image.set((left + x) as u32, (top + y) as u32, color);
            }
        }
    }
    Ok(image)
}
