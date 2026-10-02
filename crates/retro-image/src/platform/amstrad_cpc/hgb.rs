//! FutureOS wallpapers (HGB).
//!
//! Sources:
//! - CPC screen line addressing (`(y & 7) * 0x800 + (y >> 3) * row bytes`)
//!   and mode 2 pixels (MSB left): <https://cpctech.cpcwiki.de/docs/screen.html>,
//!   <https://cpctech.cpcwiki.de/docs/graphics.html>.
//! - HGB is a 512x256 mode 2 screen (64 bytes per line): CPCrulez FutureOS
//!   wallpaper page,
//!   <https://cpcrulez.fr/GamesTest//applications_graphic-futureos_wallpaper_hgb.htm>
//!   (more pages in `docs/formats/sinclair-cpc-bbc-misc.md`); shown white on
//!   black with rows doubled (observed from `recoil2png` output).

use super::amsdos::strip_amsdos;
use crate::{DecodeError, Image};

const HGB_LEN: usize = 16384;
const HGB_ROW_BYTES: usize = 64;

pub(super) fn decode_hgb(data: &[u8]) -> Result<Image, DecodeError> {
    // A blank first 128 bytes would pass as a header, so check the size first.
    let screen = if data.len() == HGB_LEN {
        data
    } else {
        strip_amsdos(data)
    };
    if screen.len() != HGB_LEN {
        return Err(DecodeError::Unrecognized);
    }
    let mut image = Image::new(512, 256);
    for y in 0..256 {
        let line = (y & 7) * 0x800 + (y >> 3) * HGB_ROW_BYTES;
        for (column, &byte) in screen[line..line + HGB_ROW_BYTES].iter().enumerate() {
            for bit in 0..8 {
                let color = if byte & (0x80 >> bit) != 0 {
                    0xffffff
                } else {
                    0
                };
                image.set((column * 8 + bit) as u32, y as u32, color);
            }
        }
    }
    Ok(image.scaled(1, 2))
}
