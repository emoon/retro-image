//! GameCube disc banners (`.bnr`, the `opening.bnr` of a disc) and memory
//! card saves (`.gci`, as emulators and adapters export them): the banner
//! picture, or for a save without banner the first icon.
//!
//! Sources:
//! - Layouts: YAGCD, the memory card chapter (directory entry and save image
//!   data, <https://hitmen.c02.at/files/yagcd/yagcd/chap12.html>) and the
//!   disc chapter (banner file, chap14), facts only. A `BNR1` file is 6496
//!   bytes: the magic, 0x1800 bytes of 96x32 pixels at 0x20, then the text
//!   fields from 0x1820; `BNR2` carries more text blocks after those.
//! - The pixel format is RGB5A3, not the RGB5A1 that YAGCD's table says: its
//!   own footnote and mkwiiki's "Image Formats" page
//!   (<https://wiki.tockdom.com/wiki/Image_Formats>) describe RGB5A3, and the
//!   `banner.bnr` sample below only comes out right with it. Pixels are the
//!   GX formats of `codec::gx`.
//! - A save is a 0x40-byte directory entry followed by 0x2000-byte blocks
//!   (the block count is at 0x38, followed by `FFFF`). Checked on four real
//!   saves (three Animal Crossing and one Action Replay file, from
//!   archive.org: `7-grand-dad-animal-crossing`, `ar-1.14.-usa-GC`,
//!   `DNMPMemCardDump`, `animal-crossing-memory-card-dpecial-data-bonus-letter`)
//!   and one banner (`cube/swiss/source/resources/banner.bnr` of
//!   <https://github.com/emukidid/swiss-gc>, used as a sample only): byte 7
//!   holds the banner format in its low two bits (0 none, 1 CI8 with a
//!   256-color RGB5A3 palette right after the pixels, 2 RGB5A3); the image
//!   data starts at the big-endian offset at 0x2c, counted from the end of
//!   the directory entry (so 0x40 means right after the 64 bytes of comment
//!   text), with the banner first and then the icons; the 16-bit field at 0x30
//!   gives each icon 2 bits: 0 none, 1 CI8 using a palette that follows the
//!   last icon, 2 RGB5A3, 3 CI8 with its own palette after its pixels. The
//!   Animal Crossing banners and icons and the Action Replay icon come out
//!   as the games draw them; icon formats 3 and a banner of format 2 were
//!   not in the samples and follow the documents only.
//!
//! A save with a banner shows it; a save without one shows its first icon (a
//! later frame of an animation is not shown). The alpha of each pixel
//! is kept.

use crate::bytes::{be16, be32};
use crate::codec::gx::{self, PaletteFormat, PixelFormat};
use crate::{DecodeError, Image};

const BANNER_WIDTH: usize = 96;
const BANNER_HEIGHT: usize = 32;
const ICON_SIZE: usize = 32;
/// Colors in a CI8 palette and bytes of one.
const PALETTE_COLORS: usize = 256;
const PALETTE_LEN: usize = PALETTE_COLORS * 2;
const ENTRY_LEN: usize = 0x40;
const BLOCK_LEN: usize = 0x2000;

/// How the pixels of a banner or icon are stored.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Pixels {
    /// 16-bit RGB5A3.
    Direct,
    /// 8-bit indices into a 256-color RGB5A3 palette.
    Indexed,
}

impl Pixels {
    fn len(self, width: usize, height: usize) -> usize {
        match self {
            Self::Direct => width * height * 2,
            Self::Indexed => width * height,
        }
    }
}

pub(super) fn decode_bnr(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    const TEXT_AT: usize = 0x1820;
    let valid = match data.get(..4) {
        Some(b"BNR1") => data.len() == 0x1960,
        Some(b"BNR2") => data.len() > TEXT_AT && (data.len() - TEXT_AT).is_multiple_of(0x140),
        _ => false,
    };
    if !valid {
        return Err(fail);
    }
    let pixels = data.get(0x20..).ok_or(fail)?;
    picture(Pixels::Direct, BANNER_WIDTH, BANNER_HEIGHT, pixels, &[]).ok_or(fail)
}

pub(super) fn decode_gci(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let blocks = usize::from(be16(data, 0x38).ok_or(fail)?);
    let printable = |range: core::ops::Range<usize>| {
        data.get(range)
            .is_some_and(|text| text.iter().all(u8::is_ascii_alphanumeric))
    };
    // Game code and maker code, an unused 0xff, the block count and its
    // reserved field: nothing else has data of exactly this shape and size.
    if !printable(0..6)
        || data[6] != 0xff
        || be16(data, 0x3a) != Some(0xffff)
        || blocks == 0
        || data.len() != ENTRY_LEN + blocks * BLOCK_LEN
    {
        return Err(fail);
    }
    let banner = data[7] & 3;
    let icon_formats = be16(data, 0x30).ok_or(fail)?;
    let start = ENTRY_LEN
        .checked_add(be32(data, 0x2c).ok_or(fail)? as usize)
        .ok_or(fail)?;
    let body = data.get(start..).ok_or(fail)?;

    let (kind, width, height) = match banner {
        1 => (Pixels::Indexed, BANNER_WIDTH, BANNER_HEIGHT),
        2 => (Pixels::Direct, BANNER_WIDTH, BANNER_HEIGHT),
        0 => match icon_formats & 3 {
            1 | 3 => (Pixels::Indexed, ICON_SIZE, ICON_SIZE),
            2 => (Pixels::Direct, ICON_SIZE, ICON_SIZE),
            _ => return Err(fail),
        },
        _ => return Err(fail),
    };
    let pixel_len = kind.len(width, height);
    let palette = match kind {
        Pixels::Direct => &[][..],
        // The banner's own palette, an icon's own or the one after the icons.
        Pixels::Indexed if banner != 0 || icon_formats & 3 == 3 => {
            body.get(pixel_len..pixel_len + PALETTE_LEN).ok_or(fail)?
        }
        Pixels::Indexed => {
            let icons_end = shared_palette_at(icon_formats);
            body.get(icons_end..icons_end + PALETTE_LEN).ok_or(fail)?
        }
    };
    let pixels = body.get(..pixel_len).ok_or(fail)?;
    picture(kind, width, height, pixels, palette).ok_or(fail)
}

/// Where the palette shared by CI8 icons lies, counted from the first icon:
/// after the data of every icon.
fn shared_palette_at(icon_formats: u16) -> usize {
    (0..8)
        .map(|icon| match icon_formats >> (icon * 2) & 3 {
            1 => ICON_SIZE * ICON_SIZE,
            2 => ICON_SIZE * ICON_SIZE * 2,
            3 => ICON_SIZE * ICON_SIZE + PALETTE_LEN,
            _ => 0,
        })
        .sum()
}

/// A picture from its pixel data and (for indexed ones) its palette.
fn picture(
    kind: Pixels,
    width: usize,
    height: usize,
    pixels: &[u8],
    palette: &[u8],
) -> Option<Image> {
    let argb = match kind {
        Pixels::Direct => gx::decode(PixelFormat::Rgb5A3, width, height, pixels, &[])?,
        Pixels::Indexed => {
            let colors = gx::decode_palette(PaletteFormat::Rgb5A3, palette, PALETTE_COLORS)?;
            gx::decode(PixelFormat::C8, width, height, pixels, &colors)?
        }
    };
    Some(Image::from_argb(
        width as u32,
        height as u32,
        argb.into_iter(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    /// A save with `blocks` blocks of which the first bytes after the comment
    /// are `image`; the banner and icon formats are given.
    fn gci(banner: u8, icon_formats: u16, image: &[u8]) -> Vec<u8> {
        let mut data = alloc::vec![0u8; ENTRY_LEN + BLOCK_LEN];
        data[..6].copy_from_slice(b"GAFE01");
        data[6] = 0xff;
        data[7] = banner;
        data[0x2c..0x30].copy_from_slice(&0x40u32.to_be_bytes());
        data[0x30..0x32].copy_from_slice(&icon_formats.to_be_bytes());
        data[0x38..0x3a].copy_from_slice(&1u16.to_be_bytes());
        data[0x3a..0x3c].copy_from_slice(&[0xff, 0xff]);
        let at = ENTRY_LEN + 0x40;
        data[at..at + image.len()].copy_from_slice(image);
        data
    }

    #[test]
    fn bnr_pixels_are_rgb5a3_in_4x4_blocks() {
        let mut data = alloc::vec![0u8; 0x1960];
        data[..4].copy_from_slice(b"BNR1");
        // First pixel red; the first pixel of the second block is blue.
        data[0x20..0x22].copy_from_slice(&0xfc00u16.to_be_bytes());
        data[0x20 + 32..0x20 + 34].copy_from_slice(&0x801fu16.to_be_bytes());
        let image = decode_bnr(&data).unwrap();
        assert_eq!((image.width(), image.height()), (96, 32));
        assert_eq!((image.get(0, 0), image.get(4, 0)), (0xff0000, 0x0000ff));
        assert!(decode_bnr(&data[..0x1900]).is_err());
        data[3] = b'3';
        assert!(decode_bnr(&data).is_err());
    }

    #[test]
    fn a_ci8_banner_reads_the_palette_after_its_pixels() {
        // Indices: pixel 0 is 1, pixel 1 is 2 (8x4 blocks); palette 1 red, 2 green.
        let mut image = alloc::vec![0u8; 3072 + 512];
        image[0] = 1;
        image[1] = 2;
        image[3072 + 2..3072 + 4].copy_from_slice(&0xfc00u16.to_be_bytes());
        image[3072 + 4..3072 + 6].copy_from_slice(&0x83e0u16.to_be_bytes());
        let picture = decode_gci(&gci(1, 0, &image)).unwrap();
        assert_eq!((picture.width(), picture.height()), (96, 32));
        assert_eq!((picture.get(0, 0), picture.get(1, 0)), (0xff0000, 0x00ff00));
    }

    #[test]
    fn without_a_banner_the_first_icon_is_shown() {
        // A shared-palette CI8 icon followed by a second icon (RGB5A3, 2048
        // bytes) and then the shared palette.
        let mut image = alloc::vec![0u8; 1024 + 2048 + 512];
        image[0] = 1;
        let palette_at = 1024 + 2048;
        image[palette_at + 2..palette_at + 4].copy_from_slice(&0xfc00u16.to_be_bytes());
        let icon = decode_gci(&gci(0, 0b10_01, &image)).unwrap();
        assert_eq!((icon.width(), icon.height()), (32, 32));
        assert_eq!(icon.get(0, 0), 0xff0000);
        // An RGB5A3 icon needs no palette.
        let mut direct = alloc::vec![0u8; 2048];
        direct[0..2].copy_from_slice(&0xfc00u16.to_be_bytes());
        assert_eq!(decode_gci(&gci(0, 2, &direct)).unwrap().get(0, 0), 0xff0000);
    }

    #[test]
    fn rejects_files_that_are_not_saves() {
        assert!(
            decode_gci(&gci(0, 0, &[])).is_err(),
            "no banner and no icon"
        );
        let mut wrong_size = gci(1, 0, &[0; 3584]);
        wrong_size.push(0);
        assert!(decode_gci(&wrong_size).is_err());
        let mut bad = gci(1, 0, &[0; 3584]);
        bad[6] = 0;
        assert!(decode_gci(&bad).is_err());
        let mut far = gci(1, 0, &[0; 3584]);
        far[0x2c..0x30].copy_from_slice(&0xffff_ffffu32.to_be_bytes());
        assert!(decode_gci(&far).is_err());
        assert!(decode_gci(&[0; 8]).is_err());
    }
}
