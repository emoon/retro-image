//! "Image 72" fonts (`.FNT`): 8-pixel-wide bitmap fonts.
//!
//! No documentation was found (survey: `docs/research/amiga-apple-misc.md`,
//! "Wave 5"). Reverse engineered from `corpus/8X14R.FNT` and
//! `corpus/FATCAT.FNT` and by black-box probing of `recoil2png` with
//! synthetic files:
//!
//! - Headered: bytes `00 08 h`, then 256 glyphs of `h` bytes (one byte per
//!   row, most significant bit leftmost), `3 + 256 * h` bytes in all.
//! - Headerless: 896 bytes, 8 bytes per glyph. Only the first 96 glyphs are
//!   shown, so the trailing 128 bytes are ignored.
//! - Shown as a sheet of 32 glyphs per row, white on black.

use crate::platform::amstrad_cpc::has_amsdos_header;
use crate::{DecodeError, Image};

const PER_ROW: usize = 32;
/// Headerless files hold 112 glyphs of 8 bytes but show only 3 rows of them.
const HEADERLESS_LEN: usize = 896;
const HEADERLESS_ROWS: usize = 3;

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    match data {
        [0, 8, height, glyphs @ ..]
            if *height != 0 && glyphs.len() == 256 * usize::from(*height) =>
        {
            sheet(glyphs, usize::from(*height), 256)
        }
        // 896 bytes with an AMSDOS header are Amstrad CPC fonts.
        _ if data.len() == HEADERLESS_LEN && !has_amsdos_header(data) => {
            sheet(data, 8, PER_ROW * HEADERLESS_ROWS)
        }
        _ => Err(DecodeError::Unrecognized),
    }
}

/// Lays out `count` glyphs of `height` bytes taken from the start of `glyphs`.
fn sheet(glyphs: &[u8], height: usize, count: usize) -> Result<Image, DecodeError> {
    let rows = count / PER_ROW;
    let mut image = Image::new((PER_ROW * 8) as u32, (rows * height) as u32)?;
    for (index, glyph) in glyphs.chunks_exact(height).take(count).enumerate() {
        let (left, top) = (index % PER_ROW * 8, index / PER_ROW * height);
        for (y, &bits) in glyph.iter().enumerate() {
            for x in (0..8).filter(|x| bits & (0x80 >> x) != 0) {
                image.set((left + x) as u32, (top + y) as u32, 0xff_ffff);
            }
        }
    }
    Ok(image)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headered_font_is_a_32_by_8_sheet() {
        let mut data = alloc::vec![0, 8, 2];
        data.resize(3 + 256 * 2, 0);
        data[3 + 2 * 33 + 1] = 0x81; // glyph 33, row 1
        let image = decode(&data).unwrap();
        assert_eq!((image.width(), image.height()), (256, 16));
        assert_eq!((image.get(8, 3), image.get(15, 3)), (0xffffff, 0xffffff));
        assert_eq!(image.get(9, 3), 0);
    }

    #[test]
    fn headerless_fonts_show_96_glyphs() {
        let image = decode(&[0; 896]).unwrap();
        assert_eq!((image.width(), image.height()), (256, 24));
        assert!(decode(&[0; 897]).is_err());
        assert!(decode(&[0, 8, 0]).is_err());
    }
}
