//! Dreamcast VMU files: data and game files (`.vms`) and `ICONDATA_VMS`,
//! shown as their icons.
//!
//! Sources:
//! - Marcus Comstedt, "Dreamcast Programming": VMS File Header
//!   (<https://mc.pp.se/dc/vms/fileheader.html>) and ICONDATA_VMS
//!   (<https://mc.pp.se/dc/vms/icondata.html>), 2000, no license stated,
//!   used as prose facts only.
//! - KallistiOS `kernel/arch/dreamcast/util/vmu_pkg.c` (BSD-style license,
//!   <https://github.com/KallistiOS/KallistiOS>): the header including its
//!   icon palette is 0x80 bytes, and the CRC covers header, icons, eyecatch
//!   and the data length given at 0x48, with the CRC field itself zeroed.
//! - The ICONDATA_VMS layout cross-checked against `vms_icondata.php` of
//!   <https://github.com/mrneo240/NeoDC-Icondata-Tool> (BSD-2-Clause,
//!   copyright 2018 NeoDC/HaydenK); nothing is copied from it.
//! - The CRC is CRC-16/XMODEM (polynomial 0x1021, initial value 0, most
//!   significant bit first), the name for the algorithm Comstedt gives as C.
//! - Real sample: KallistiOS `examples/dreamcast/vmu/vmu_game/romdisk/TETRIS.VMS`
//!   (a game file with two icons), in `corpus/extra/small-consoles/vmu`.
//!   No data file or `ICONDATA_VMS` sample was available; those two are
//!   decoded from the documents alone and covered by unit tests only.
//!
//! A data file starts with the header; a game file (a program for the VMU's
//! own processor) has it in its second 512-byte block, at 0x200. Both have 1
//! to 3 icons of 32 x 32 pixels, 4 bits each, the high nibble on the left,
//! through a 16-color palette of little-endian ARGB4444 words (offset 0x60 of
//! the header, icons from 0x80). The eyecatch (72 x 56, the picture the
//! Dreamcast file manager shows) is not drawn.
//!
//! Palette decisions: each 4-bit channel is expanded with `v * 17` (0xf is
//! 255), and the alpha channel (0 is transparent, 0xf opaque) is blended
//! onto [`TRANSPARENT_FILL`], since an `Image` has no alpha.
//!
//! The frames of an animated icon are drawn side by side, in one row of
//! 32-pixel cells. `ICONDATA_VMS` holds a monochrome icon (128 bytes, 1 is
//! black and 0 transparent) and optionally a color icon in the format above
//! (palette, then pixels); the color icon is drawn when there is one.
//!
//! Detection: only a data file has something to check, the CRC, so it is
//! the one format here with `.signature()`. A game file is accepted only by
//! extension, after a structural check (counts, zeroed reserved bytes,
//! printable description), and `ICONDATA_VMS` by its offsets, which must
//! point into the file. A file named exactly `ICONDATA_VMS`, as on a VMU,
//! has no extension, and a format is chosen by the text after the last dot,
//! so it is never tried by name: only a copy named `ICONDATA.VMS` (the name
//! the NeoDC tool writes) reaches this decoder, and the decoder has no
//! signature to find the file by content. `.dci` files (Nexus dumps: the same data with each
//! 4-byte group byte-reversed, plus a directory entry) are not decoded; that
//! byte order is not confirmed by any document or sample.

use alloc::vec::Vec;

use crate::bytes::{le16, le32};
use crate::image::{TRANSPARENT_FILL, over_fill, widen_channel};
use crate::tiles::TileLayout;
use crate::{BitOrder, DecodeError, Format, Image};

pub(super) static FORMATS: &[Format] = &[
    Format::new("Dreamcast", "VMU data file", &["vms"], decode_data).signature(),
    Format::new("Dreamcast", "VMU game file", &["vms"], decode_game),
    Format::new("Dreamcast", "ICONDATA_VMS", &["vms"], decode_icondata),
];

/// One 32 x 32 icon of 4-bit pixels, the high nibble on the left.
const ICON: TileLayout = TileLayout {
    width: 32,
    height: 32,
    ..TileLayout::packed(4, BitOrder::MsbFirst)
};
/// The monochrome icon of `ICONDATA_VMS`: one bit per pixel, 1 is black.
const MONO_ICON: TileLayout = TileLayout {
    width: 32,
    height: 32,
    ..TileLayout::planar(1, 1)
};

/// Bytes of a header: descriptions, counts, CRC, reserved bytes, palette.
const HEADER_LEN: usize = 0x80;
const PALETTE_AT: usize = 0x60;
/// Where a game file's header starts.
const GAME_HEADER_AT: usize = 0x200;
/// Eyecatch sizes by type 0 to 3: none, 16-bit direct color, 256 colors with
/// a 512-byte palette, 16 colors with a 32-byte palette.
const EYECATCH_LEN: [usize; 4] = [0, 72 * 56 * 2, 512 + 72 * 56, 32 + 72 * 56 / 2];

/// The parts of a header that locate the rest of the file.
struct Header {
    /// Where the header starts in the file.
    at: usize,
    icons: usize,
    eyecatch_len: usize,
}

impl Header {
    /// Reads the header at `at` and checks what any VMS header must satisfy.
    fn parse(data: &[u8], at: usize) -> Option<Self> {
        let icons = usize::from(le16(data, at + 0x40)?);
        let eyecatch = usize::from(le16(data, at + 0x44)?);
        let reserved_is_zero = data
            .get(at + 0x4c..at + PALETTE_AT)?
            .iter()
            .all(|&b| b == 0);
        let header = Self {
            at,
            icons,
            eyecatch_len: *EYECATCH_LEN.get(eyecatch)?,
        };
        let known = (1..=3).contains(&icons) && reserved_is_zero;
        (known && data.len() >= header.data_at()).then_some(header)
    }

    /// Where the file's own data starts: after the icons and the eyecatch.
    fn data_at(&self) -> usize {
        self.at + HEADER_LEN + self.icons * ICON.tile_len() + self.eyecatch_len
    }

    /// The icons as a row of cells, through the header's palette.
    fn sheet(&self, data: &[u8]) -> Result<Image, DecodeError> {
        let palette = palette(data, self.at + PALETTE_AT).ok_or(DecodeError::Unrecognized)?;
        let icons = &data[self.at + HEADER_LEN..][..self.icons * ICON.tile_len()];
        ICON.sheet(icons, self.icons, &palette)
    }
}

/// Sixteen ARGB4444 colors at `at`, blended onto the transparent fill.
fn palette(data: &[u8], at: usize) -> Option<Vec<u32>> {
    let words = data.get(at..at + 32)?;
    Some(
        words
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&word| argb4444(u16::from_le_bytes(word)))
            .collect(),
    )
}

/// A color word as `0xRRGGBB`, its alpha blended onto the transparent fill.
fn argb4444(word: u16) -> u32 {
    let channel = |shift: u32| widen_channel(u32::from(word >> shift & 15), 4);
    let color = channel(8) << 16 | channel(4) << 8 | channel(0);
    over_fill(color, (channel(12)) as u8)
}

/// CRC-16/XMODEM of the concatenation of `parts`.
fn crc16(parts: &[&[u8]]) -> u16 {
    let mut crc = 0u16;
    for &byte in parts.iter().copied().flatten() {
        crc ^= u16::from(byte) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                crc << 1 ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    crc
}

/// A data file: the header at the start, and a CRC that matches.
fn decode_data(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let header = Header::parse(data, 0).ok_or(fail)?;
    let end = header
        .data_at()
        .checked_add(le32(data, 0x48).ok_or(fail)? as usize)
        .filter(|&end| end <= data.len())
        .ok_or(fail)?;
    // The CRC field, at 0x46, counts as zero.
    let crc = crc16(&[&data[..0x46], &[0, 0], &data[0x48..end]]);
    if le16(data, 0x46) != Some(crc) {
        return Err(fail);
    }
    header.sheet(data)
}

/// A game file: the header in the second block, which has no CRC to check.
/// The short description must be printable ASCII.
fn decode_game(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let header = Header::parse(data, GAME_HEADER_AT).ok_or(fail)?;
    let description = &data[GAME_HEADER_AT..][..16];
    if !description.iter().all(|b| (0x20..0x7f).contains(b)) {
        return Err(fail);
    }
    header.sheet(data)
}

/// `ICONDATA_VMS`: a 16-byte description, then the offsets of the monochrome
/// icon and of the color icon (0 if there is none).
fn decode_icondata(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    if !data[..data.len().min(16)]
        .iter()
        .all(|b| (0x20..0x7f).contains(b))
    {
        return Err(fail);
    }
    let offset = |at| le32(data, at).map(|o| o as usize);
    let (mono, color) = (offset(0x10).ok_or(fail)?, offset(0x14).ok_or(fail)?);
    // Offsets point behind this header and must leave room for their icon.
    let icon = |at: usize, len: usize| {
        let end = at.checked_add(len)?;
        (at >= 0x18).then(|| data.get(at..end)).flatten()
    };
    if color != 0 {
        let color_icon = icon(color, 32 + ICON.tile_len()).ok_or(fail)?;
        let palette = palette(color_icon, 0).ok_or(fail)?;
        return ICON.sheet(&color_icon[32..], 1, &palette);
    }
    let mono_icon = icon(mono, MONO_ICON.tile_len()).ok_or(fail)?;
    MONO_ICON.sheet(mono_icon, 1, &[TRANSPARENT_FILL, 0x00_0000])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A header at `at` with `icons` icons and no eyecatch; the first pixel
    /// of icon 0 is color 1, and palette entry 0 is transparent.
    fn header_at(at: usize, icons: u16) -> Vec<u8> {
        let mut file = alloc::vec![0; at + HEADER_LEN + usize::from(icons) * 512];
        file[at..at + 16].copy_from_slice(b"Test            ");
        file[at + 0x40..at + 0x42].copy_from_slice(&icons.to_le_bytes());
        file[at + PALETTE_AT + 2..at + PALETTE_AT + 4].copy_from_slice(&0xf08fu16.to_le_bytes());
        file[at + HEADER_LEN] = 0x10;
        file
    }

    #[test]
    fn crc16_matches_the_xmodem_check_value() {
        assert_eq!(crc16(&[b"1234", b"56789"]), 0x31c3);
    }

    #[test]
    fn alpha_is_blended_onto_the_fill_and_channels_use_v_times_17() {
        assert_eq!(argb4444(0x0123), TRANSPARENT_FILL);
        assert_eq!(argb4444(0xf08f), 0x00_88ff);
        // Alpha 8 of 15 is 136 of 255: half-way, rounded.
        assert_eq!(argb4444(0x8fff), 0xe2_e2e2);
    }

    #[test]
    fn data_files_need_a_matching_crc() {
        let mut file = header_at(0, 1);
        let crc = crc16(&[&file]);
        file[0x46..0x48].copy_from_slice(&crc.to_le_bytes());
        let image = decode_data(&file).unwrap();
        assert_eq!((image.width(), image.height()), (32, 32));
        // The high nibble is the left pixel: color 1 first, then color 0.
        assert_eq!(
            (image.get(0, 0), image.get(1, 0)),
            (0x00_88ff, TRANSPARENT_FILL)
        );
        // Padding after the data does not matter, but a changed byte does.
        file.extend_from_slice(&[0; 100]);
        assert!(decode_data(&file).is_ok());
        file[0x20] ^= 1;
        assert!(decode_data(&file).is_err());
    }

    #[test]
    fn game_files_have_their_header_in_the_second_block() {
        let file = header_at(GAME_HEADER_AT, 2);
        let image = decode_game(&file).unwrap();
        assert_eq!((image.width(), image.height()), (64, 32));
        assert!(decode_data(&file).is_err());
        assert!(decode_game(&header_at(GAME_HEADER_AT, 4)).is_err());
    }

    #[test]
    fn icondata_prefers_the_color_icon() {
        let mut file = alloc::vec![0; 0x18];
        file[..16].copy_from_slice(b"Icon            ");
        file[0x10..0x14].copy_from_slice(&0x18u32.to_le_bytes());
        let mut mono = alloc::vec![0; 128];
        mono[0] = 0x80;
        file.extend_from_slice(&mono);
        let image = decode_icondata(&file).unwrap();
        assert_eq!((image.get(0, 0), image.get(1, 0)), (0, TRANSPARENT_FILL));
        let color_at = file.len();
        file[0x14..0x18].copy_from_slice(&(color_at as u32).to_le_bytes());
        assert!(decode_icondata(&file).is_err(), "the color icon is cut off");
        file.extend_from_slice(&[0; 32 + 512]);
        file[color_at + 2..color_at + 4].copy_from_slice(&0xf0f0u16.to_le_bytes());
        file[color_at + 32] = 0x10;
        assert_eq!(decode_icondata(&file).unwrap().get(0, 0), 0x00_ff00);
    }
}
