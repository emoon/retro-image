//! Nintendo DS and DSi ROM banner icon (`.nds`, `.dsi`, `.srl`): the 32x32
//! picture the DS menu and flash cart menus show for a game.
//!
//! Sources:
//! - GBATEK, "DS Cartridge Header" and "DS Cartridge Icon/Title"
//!   (<https://problemkaputt.de/gbatek.htm>, no license stated, so facts
//!   only): the banner offset at header `0x68`, the two header CRC-16 values
//!   at `0x15C` and `0x15E`, the banner versions and their CRC-16 fields, the
//!   icon bitmap (32x32, 4 bpp, 4x4 tiles of 8x8) and palette (16 colors of
//!   15-bit BGR, color 0 transparent), and the DSi animation (8 bitmaps, 8
//!   palettes and a list of 16-bit tokens).
//! - The CRC-16 is the BIOS one: start value `0xFFFF`, reflected polynomial
//!   `0xA001`. Reverse engineered from samples: all four CRCs of the banners
//!   in `corpus/extra/nintendo-rom-icons` match, and the low nibble of a
//!   bitmap byte is the left pixel (GBATEK does not say; the renders of
//!   GodMode9i and nds-bootstrap show their logos the right way round).
//!
//! The header carries the Nintendo logo checksum (always `0xCF56`) and a
//! checksum of the header, so a file is accepted only when both are right, the
//! banner lies inside the file and its own CRC-16 matches. That is strict
//! enough for content detection.
//!
//! Color 0 is transparent.
//! A DSi banner (version `0x0103`) with an animation is drawn as its first
//! frame: the first token of the sequence picks a bitmap, a palette and
//! flips. A first token of 0 means the static icon. The TWiLight Menu
//! `BOOT.NDS` in the samples is a real animated banner, so the CRC check of the
//! animation and a first token that picks bitmap 0 and palette 0 are sampled;
//! picking another bitmap or palette and the flips are only tested by a unit
//! test built from GBATEK's description.

use crate::bytes::{le16, le32};
use crate::image::{CLEAR, bgr555};
use crate::tiles::TileLayout;
use crate::{BitOrder, DecodeError, Image};

/// The cartridge header holds everything up to its own checksum.
const HEADER_LEN: usize = 0x160;
const BANNER_AT: usize = 0x68;
const LOGO_CRC_AT: usize = 0x15c;
const LOGO_CRC: u16 = 0xcf56;
const HEADER_CRC_AT: usize = 0x15e;

/// Banner versions: original, with Chinese, with Korean titles, with the DSi
/// animation.
const VERSIONS: [u16; 4] = [0x0001, 0x0002, 0x0003, ANIMATED];
const ANIMATED: u16 = 0x0103;
/// Offsets inside a banner. The static part ends with the Korean title; the
/// CRC-16 at `+2` covers `0x20..0x840` in every version.
const CRC_AT: usize = 2;
const BITMAP_AT: usize = 0x20;
const PALETTE_AT: usize = 0x220;
const STATIC_END: usize = 0x840;
const ANIMATION_CRC_AT: usize = 8;
const ANIMATION_BITMAPS_AT: usize = 0x1240;
const ANIMATION_PALETTES_AT: usize = 0x2240;
const SEQUENCE_AT: usize = 0x2340;
const ANIMATION_END: usize = 0x23c0;
const BITMAP_LEN: usize = 0x200;
const PALETTE_LEN: usize = 0x20;

/// 8x8 tiles of 4-bit pixels, the left pixel in the low nibble.
const TILE: TileLayout = TileLayout::packed(4, BitOrder::LsbFirst);
const TILES_PER_ROW: usize = 4;

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let header = data.get(..HEADER_LEN).ok_or(fail)?;
    if le16(header, LOGO_CRC_AT) != Some(LOGO_CRC)
        || le16(header, HEADER_CRC_AT) != Some(crc16(&header[..HEADER_CRC_AT]))
    {
        return Err(fail);
    }
    let at = le32(header, BANNER_AT).ok_or(fail)? as usize;
    let banner = data.get(at..).ok_or(fail)?;
    first_frame(banner)
}

/// The icon a banner shows first.
fn first_frame(banner: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let version = le16(banner, 0).ok_or(fail)?;
    let shown = banner.get(..STATIC_END).ok_or(fail)?;
    if !VERSIONS.contains(&version) || le16(banner, CRC_AT) != Some(crc16(&shown[BITMAP_AT..])) {
        return Err(fail);
    }
    let static_icon = (BITMAP_AT, PALETTE_AT, 0);
    let (bitmap_at, palette_at, flips) = if version == ANIMATED {
        let animation = banner.get(..ANIMATION_END).ok_or(fail)?;
        if le16(banner, ANIMATION_CRC_AT) != Some(crc16(&animation[ANIMATION_BITMAPS_AT..])) {
            return Err(fail);
        }
        match le16(banner, SEQUENCE_AT).ok_or(fail)? {
            0 => static_icon,
            token => (
                ANIMATION_BITMAPS_AT + BITMAP_LEN * usize::from(token >> 8 & 7),
                ANIMATION_PALETTES_AT + PALETTE_LEN * usize::from(token >> 11 & 7),
                token >> 14,
            ),
        }
    } else {
        static_icon
    };
    let bitmap = banner.get(bitmap_at..bitmap_at + BITMAP_LEN).ok_or(fail)?;
    let palette = banner
        .get(palette_at..palette_at + PALETTE_LEN)
        .ok_or(fail)?;
    let mut colors = [CLEAR; 16];
    for (color, &word) in colors.iter_mut().zip(palette.as_chunks::<2>().0).skip(1) {
        *color = 0xff00_0000 | bgr555(u16::from_le_bytes(word));
    }
    let icon = TILE.sheet_argb(bitmap, TILES_PER_ROW, &colors)?;
    flipped(&icon, flips & 1 != 0, flips & 2 != 0)
}

/// The icon mirrored left to right and/or top to bottom.
fn flipped(icon: &Image, horizontal: bool, vertical: bool) -> Result<Image, DecodeError> {
    let (width, height) = (icon.width(), icon.height());
    let source = |x: u32, y: u32| {
        let x = if horizontal { width - 1 - x } else { x };
        let y = if vertical { height - 1 - y } else { y };
        icon.get_argb(x, y)
    };
    Image::from_argb(
        width,
        height,
        (0..height).flat_map(|y| (0..width).map(move |x| source(x, y))),
    )
}

/// CRC-16 as the DS BIOS computes it: start value `0xFFFF`, reflected
/// polynomial `0xA001`.
fn crc16(data: &[u8]) -> u16 {
    data.iter().fold(0xffff, |crc, &byte| {
        (0..8).fold(crc ^ u16::from(byte), |crc, _| {
            if crc & 1 != 0 {
                crc >> 1 ^ 0xa001
            } else {
                crc >> 1
            }
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    /// A ROM with a valid header and `banner` at offset 0x200.
    fn rom(banner: &[u8]) -> Vec<u8> {
        let mut rom = alloc::vec![0; 0x200];
        rom[BANNER_AT..BANNER_AT + 4].copy_from_slice(&0x200u32.to_le_bytes());
        rom[LOGO_CRC_AT..LOGO_CRC_AT + 2].copy_from_slice(&LOGO_CRC.to_le_bytes());
        let crc = crc16(&rom[..HEADER_CRC_AT]);
        rom[HEADER_CRC_AT..HEADER_CRC_AT + 2].copy_from_slice(&crc.to_le_bytes());
        rom.extend_from_slice(banner);
        rom
    }

    /// A blank banner of `version`; call `seal` once its bytes are set.
    fn banner(version: u16, len: usize) -> Vec<u8> {
        let mut banner = alloc::vec![0; len];
        banner[..2].copy_from_slice(&version.to_le_bytes());
        banner
    }

    fn seal(banner: &mut [u8]) {
        let crc = crc16(&banner[BITMAP_AT..STATIC_END]);
        banner[CRC_AT..CRC_AT + 2].copy_from_slice(&crc.to_le_bytes());
        if banner.len() >= ANIMATION_END {
            let crc = crc16(&banner[ANIMATION_BITMAPS_AT..ANIMATION_END]);
            banner[ANIMATION_CRC_AT..ANIMATION_CRC_AT + 2].copy_from_slice(&crc.to_le_bytes());
        }
    }

    #[test]
    fn crc16_is_the_bios_checksum() {
        // CRC-16/MODBUS check value.
        assert_eq!(crc16(b"123456789"), 0x4b37);
    }

    #[test]
    fn low_nibble_is_the_left_pixel_and_color_0_is_transparent() {
        let mut banner = banner(1, STATIC_END);
        banner[BITMAP_AT] = 0x21; // tile 0, row 0: color 1, then color 2
        banner[BITMAP_AT + 32] = 0x03; // tile 1, row 0: color 3 at its left
        banner[PALETTE_AT + 2..PALETTE_AT + 4].copy_from_slice(&0x001fu16.to_le_bytes());
        banner[PALETTE_AT + 4..PALETTE_AT + 6].copy_from_slice(&0x7c00u16.to_le_bytes());
        banner[PALETTE_AT + 6..PALETTE_AT + 8].copy_from_slice(&0x03e0u16.to_le_bytes());
        seal(&mut banner);
        let icon = decode(&rom(&banner)).unwrap();
        assert_eq!((icon.width(), icon.height()), (32, 32));
        assert_eq!(icon.get(0, 0), 0xff0000);
        assert_eq!(icon.get(1, 0), 0x0000ff);
        assert_eq!(icon.get_argb(2, 0), CLEAR);
        assert_eq!(icon.get(8, 0), 0x00ff00);
        assert_eq!(icon.get_argb(9, 0), CLEAR);
    }

    #[test]
    fn a_bad_checksum_is_rejected() {
        let mut banner = banner(1, STATIC_END);
        seal(&mut banner);
        let good = rom(&banner);
        assert!(decode(&good).is_ok());
        let mut bad_banner = good.clone();
        bad_banner[0x200 + BITMAP_AT] ^= 1;
        assert!(decode(&bad_banner).is_err());
        let mut bad_header = good.clone();
        bad_header[0x20] ^= 1;
        assert!(decode(&bad_header).is_err());
        assert!(decode(&good[..good.len() - 1]).is_err());
        assert!(decode(&good[..0x100]).is_err());
    }

    #[test]
    fn an_animated_banner_shows_the_first_token_frame() {
        let mut banner = banner(ANIMATED, ANIMATION_END);
        // Bitmap 2 has color 1 at its top left pixel; palette 5 makes
        // color 1 red.
        banner[ANIMATION_BITMAPS_AT + 2 * BITMAP_LEN] = 0x01;
        banner[ANIMATION_PALETTES_AT + 5 * PALETTE_LEN + 2..][..2]
            .copy_from_slice(&0x001fu16.to_le_bytes());
        let token = 0x8000u16 | 0x4000 | 5 << 11 | 2 << 8 | 3;
        banner[SEQUENCE_AT..SEQUENCE_AT + 2].copy_from_slice(&token.to_le_bytes());
        seal(&mut banner);
        let icon = decode(&rom(&banner)).unwrap();
        // Flipped both ways, the top left pixel lands bottom right.
        assert_eq!(icon.get(31, 31), 0xff0000);
        assert_eq!(icon.get_argb(0, 0), CLEAR);
        // A first token of 0 is the static icon.
        banner[SEQUENCE_AT] = 0;
        banner[SEQUENCE_AT + 1] = 0;
        banner[BITMAP_AT] = 0x01;
        banner[PALETTE_AT + 2..PALETTE_AT + 4].copy_from_slice(&0x7c00u16.to_le_bytes());
        seal(&mut banner);
        assert_eq!(decode(&rom(&banner)).unwrap().get(0, 0), 0x0000ff);
    }
}
