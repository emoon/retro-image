//! PlayStation memory card images (`.mcr`, `.mcd`), shown as the icons of the
//! saves on the card.
//!
//! Sources: psx-spx, "Memory Card Data Format"
//! (<https://problemkaputt.de/psxspx-memory-card-data-format.htm>, facts
//! only), and for the 4-bit pixel order and the 15-bit colors the TIM
//! section of psx-spx, as used by `playstation.rs`.
//!
//! No real memory card image was available, so this decoder follows the
//! document alone and is covered by unit tests built from it. It is
//! unverified against a card from a real console or emulator.
//!
//! A card is 128 KiB: 16 blocks of 8 KiB, 128-byte frames. Frame 0 of block
//! 0 is the header: `MC`, zeros and an XOR checksum byte at the end. Frames
//! 1 to 15 are the directory, one for each of the blocks 1 to 15; a state
//! word of `0x51` marks the first block of a file. That block starts with a
//! title frame: `SC`, an icon flag (`0x11` to `0x13`: 1 to 3 frames), the
//! title in Shift-JIS, and at 0x60 a palette of 16 15-bit colors. The icon
//! frames follow, 128 bytes each: 16 x 16 pixels, 4 bits each, the low nibble
//! on the left.
//!
//! The picture has a row for every save in directory order, 3 icon frames
//! wide (a save with fewer frames leaves the rest in its color 0), each save
//! drawn with its own palette. A card without a valid save is rejected. The
//! color `0000` is transparent; `8000` is solid black.
//!
//! Detection: size, `MC` and the header checksum are strict enough for
//! `.signature()`. Single-save wrappers (`.mcs`, `.psv`, `.mcb`, `.psx`) and
//! DexDrive images (`.gme`) have extra headers whose layouts were not checked,
//! and are not read.

use alloc::vec::Vec;

use crate::bytes::{le16, le32};
use crate::image::{CLEAR, bgr555};
use crate::tiles::TileLayout;
use crate::{BitOrder, DecodeError, Format, Image};

pub(super) static FORMATS: &[Format] =
    &[Format::new("PlayStation", "Memory card image", &["mcr", "mcd"], decode).signature()];

const CARD_LEN: usize = 128 * 1024;
const BLOCK_LEN: usize = 8 * 1024;
const FRAME_LEN: usize = 128;
/// The state word of the first block of a file.
const FIRST_BLOCK: u32 = 0x51;
/// An icon frame: 16 x 16 pixels, 4 bits each, the low nibble on the left.
const ICON: TileLayout = TileLayout {
    width: 16,
    height: 16,
    ..TileLayout::packed(4, BitOrder::LsbFirst)
};
const MAX_FRAMES: usize = 3;

/// The XOR of the first 127 bytes of a frame, which its last byte holds.
fn frame_is_checked(frame: &[u8]) -> bool {
    frame[..FRAME_LEN - 1].iter().fold(0, |sum, b| sum ^ b) == frame[FRAME_LEN - 1]
}

/// A palette entry as `0xAARRGGBB`: `0000` is transparent.
fn color(word: u16) -> u32 {
    if word == 0 {
        CLEAR
    } else {
        0xff00_0000 | bgr555(word)
    }
}

/// The icon of the save whose title frame starts the block `block`, as a
/// row of `MAX_FRAMES` frames. `None` if the block has no valid title frame.
fn icon_row(card: &[u8], block: usize) -> Option<Image> {
    let title = &card[block * BLOCK_LEN..][..FRAME_LEN];
    let frames = usize::from(title[2].checked_sub(0x10)?);
    if &title[..2] != b"SC" || !(1..=MAX_FRAMES).contains(&frames) {
        return None;
    }
    let palette: Vec<u32> = (0..16)
        .map(|i| color(le16(title, 0x60 + i * 2).unwrap_or(0)))
        .collect();
    let mut icons = alloc::vec![0; MAX_FRAMES * ICON.tile_len()];
    let shown = &card[block * BLOCK_LEN + FRAME_LEN..][..frames * ICON.tile_len()];
    icons[..shown.len()].copy_from_slice(shown);
    ICON.sheet_argb(&icons, MAX_FRAMES, &palette).ok()
}

fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    if data.len() != CARD_LEN || &data[..2] != b"MC" || !frame_is_checked(&data[..FRAME_LEN]) {
        return Err(fail);
    }
    let rows: Vec<Image> = (1..16)
        .filter(|&block| le32(data, block * FRAME_LEN) == Some(FIRST_BLOCK))
        .filter_map(|block| icon_row(data, block))
        .collect();
    let width = rows.first().ok_or(fail)?.width();
    let height = ICON.height as u32 * rows.len() as u32;
    let pixels = rows.iter().flat_map(|row| {
        (0..row.height()).flat_map(move |y| (0..row.width()).map(move |x| row.get_argb(x, y)))
    });
    Ok(Image::from_argb(width, height, pixels))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A card with the header checksum set and one save in `block`: `frames`
    /// icon frames, color 1 a solid red, the first pixel of frame 0 color 1.
    fn card_with_save(block: usize, frames: u8) -> Vec<u8> {
        let mut card = alloc::vec![0; CARD_LEN];
        card[..2].copy_from_slice(b"MC");
        card[FRAME_LEN - 1] = b'M' ^ b'C';
        card[block * FRAME_LEN] = FIRST_BLOCK as u8;
        let title = block * BLOCK_LEN;
        card[title..title + 2].copy_from_slice(b"SC");
        card[title + 2] = 0x10 + frames;
        card[title + 0x62..title + 0x64].copy_from_slice(&0x001fu16.to_le_bytes());
        card[title + FRAME_LEN] = 0x01;
        card
    }

    #[test]
    fn a_save_is_a_row_of_its_icon_frames() {
        let card = card_with_save(3, 2);
        let image = decode(&card).unwrap();
        assert_eq!((image.width(), image.height()), (48, 16));
        // Color 1, red 31 of 31; the low nibble is the left pixel, and color 0
        // is transparent.
        assert_eq!(image.get(0, 0), 0xff_0000);
        assert_eq!(image.get_argb(1, 0), CLEAR);
    }

    #[test]
    fn saves_are_rows_in_directory_order_with_their_own_palettes() {
        let mut card = card_with_save(2, 1);
        let second = card_with_save(5, 3);
        card[5 * FRAME_LEN] = FIRST_BLOCK as u8;
        card[5 * BLOCK_LEN..6 * BLOCK_LEN].copy_from_slice(&second[5 * BLOCK_LEN..6 * BLOCK_LEN]);
        card[5 * BLOCK_LEN + 0x62..5 * BLOCK_LEN + 0x64].copy_from_slice(&0x03e0u16.to_le_bytes());
        let image = decode(&card).unwrap();
        assert_eq!((image.width(), image.height()), (48, 32));
        assert_eq!(image.get(0, 0), 0xff_0000);
        assert_eq!(image.get(0, 16), 0x00_ff00);
    }

    #[test]
    fn cards_without_a_valid_save_or_header_are_rejected() {
        let mut card = card_with_save(1, 1);
        assert!(decode(&card).is_ok());
        card[FRAME_LEN - 1] ^= 1;
        assert!(decode(&card).is_err());
        card[FRAME_LEN - 1] ^= 1;
        card[BLOCK_LEN + 2] = 0x14; // a flag that is not 1 to 3 frames
        assert!(decode(&card).is_err());
        assert!(decode(&card[..CARD_LEN - 1]).is_err());
    }
}
