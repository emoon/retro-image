//! PHO digitised photos: MAP files holding a bitmap 4 screen, shown in
//! four grays.
//!
//! Sources:
//! - PHO files are MAP files whose mode byte says 40 columns (the byte
//!   bitmap 4 shares), holding bitmap 4 banks, with no palette: reverse
//!   engineered from the nine photos of the Teo-Drive n°3 disk magazine
//!   (A.S.C.I., 1991), <http://dcmoto.free.fr/programmes/teo-drive-3/index.html>,
//!   compared with the dcmoto screenshots published on that page
//!   (14.png to 22.png).
//! - The grays: the screenshots show colors 0-3 at levels 249, 219, 183
//!   and 122, which are palette values `EEE`, `AAA`, `666` and `222`
//!   through the EF9369 gamma curve (`palette.rs`) to within 2 levels.
//! - Prehisto, "Les fichiers graphiques Thomson",
//!   <http://web.archive.org/web/20251005163132/http://collection.thomson.free.fr/code/articles/prehisto_bulletin/page.php?XI=0&XJ=13>,
//!   describes PHO files from TO-PHOTO as bitmap 16 with a black-to-white
//!   palette instead; no such file was found, so those would show wrongly.

use super::map::{self, Screen};
use crate::{DecodeError, Image};

/// Colors 0-3 as the Teo-Drive viewer sets them; the rest stay black.
const PALETTE: [u16; 16] = [
    0xeee, 0xaaa, 0x666, 0x222, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

pub(super) fn decode_pho(data: &[u8]) -> Result<Image, DecodeError> {
    let mut map = map::parse(data)?;
    map.screen = match map.screen {
        Screen::Columns40 { rama, ramb } | Screen::Bitmap4 { rama, ramb } => {
            Screen::Bitmap4 { rama, ramb }
        }
        _ => return Err(DecodeError::Unrecognized),
    };
    map.render(&map.palette.unwrap_or(PALETTE))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitmap4_in_greys_lightest_first() {
        // 1 column, 8 lines: RAMA 0x30, RAMB 0x50 -> colors 0, 1, 2, 3.
        let data = map::file(&[0x00, 0x00, 0x00, 0x08, 0x30, 0, 0, 0x08, 0x50, 0, 0]);
        let image = decode_pho(&data).unwrap();
        let row: [u32; 4] = core::array::from_fn(|x| image.get(x as u32, 0));
        assert_eq!(row, [0xf8f8f8, 0xdcdcdc, 0xb7b7b7, 0x7c7c7c]);
        let bitmap16 = map::file(&[0x40, 0x00, 0x00, 0x08, 0x30, 0, 0, 0, 0]);
        assert!(decode_pho(&bitmap16).is_err());
    }
}
