//! "Mode 5" pictures by SyX: a mode 1 overscan bitmap (GFX) whose pen
//! colours are reprogrammed every line, and pen 0 six times per line (CM5).
//!
//! Sources:
//! - CM5 holds the changing palette and GFX the screen: cpcwiki CM5 page,
//!   <https://www.cpcwiki.eu/index.php?title=CM5&redirect=no> (blocks
//!   automated fetches and has no Wayback Machine snapshot; read as a
//!   search snippet only), see `docs/research/sinclair-cpc-bbc-misc.md`.
//! - Layout (GFX: 256 linear lines of 72 bytes, 288 mode 1 pixels; CM5: the
//!   colour of pen 3, then per line pen 2, pen 1 and pen 0 for each 48-pixel
//!   column band, all as `0x40 | hardware colour`; exact sizes): reverse
//!   engineered from samples and `recoil2png` output. RECOIL also rejects
//!   colours without bit 6; we accept them, as the GFX file already
//!   identifies the format.

use super::amsdos::strip_amsdos;
use super::hardware::{Mode, hardware_color};
use crate::{Companions, DecodeError, Image};

const WIDTH: usize = 288;
const HEIGHT: usize = 256;
const LINE_BYTES: usize = WIDTH / 4;
const GFX_LEN: usize = LINE_BYTES * HEIGHT;
const LINE_COLORS: usize = 8;
const CM5_LEN: usize = 1 + LINE_COLORS * HEIGHT;
/// Width of each band of pen 0 colours.
const BAND: usize = 48;

/// CM5 with its GFX companion; the bitmap is needed, so CM5 alone fails.
pub(super) fn decode_cm5(data: &[u8], companions: &dyn Companions) -> Result<Image, DecodeError> {
    let colors = strip_amsdos(data);
    let gfx = companions.get("gfx").ok_or(DecodeError::Unrecognized)?;
    let gfx = strip_amsdos(&gfx);
    // Colours are `0x40 | n`; a bare hardware number `n` (seen for black in
    // a sample) means the same.
    let valid = |c: &u8| c & !0x5f == 0;
    if colors.len() != CM5_LEN || gfx.len() != GFX_LEN || !colors.iter().all(valid) {
        return Err(DecodeError::Unrecognized);
    }
    let mut image = Image::new(WIDTH as u32, HEIGHT as u32);
    for y in 0..HEIGHT {
        let line = &gfx[y * LINE_BYTES..][..LINE_BYTES];
        let line_colors = &colors[1 + y * LINE_COLORS..][..LINE_COLORS];
        for x in 0..WIDTH {
            let value = match Mode::One.pen(line[x / 4], x % 4) {
                0 => line_colors[2 + x / BAND],
                1 => line_colors[1],
                2 => line_colors[0],
                _ => colors[0],
            };
            image.set(x as u32, y as u32, hardware_color(value));
        }
    }
    Ok(image)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use alloc::vec::Vec;

    struct Gfx(Vec<u8>);

    impl Companions for Gfx {
        fn get(&self, extension: &str) -> Option<Vec<u8>> {
            (extension == "gfx").then(|| self.0.clone())
        }
    }

    #[test]
    fn pen0_changes_every_48_pixels() {
        let mut colors = vec![0x54; CM5_LEN];
        colors[1 + 2 + 1] = 0x4b; // line 0, pen 0 of the second band: white
        let image = decode_cm5(&colors, &Gfx(vec![0; GFX_LEN])).unwrap();
        assert_eq!(image.get(47, 0), 0x000000);
        assert_eq!(image.get(48, 0), 0xffffff);
        assert_eq!(image.get(96, 0), 0x000000);
        assert!(decode_cm5(&colors, &crate::NoCompanions).is_err());
    }
}
