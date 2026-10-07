//! Game Boy Camera cartridge save (`.sav`, or `.srm` as RetroArch-based
//! emulators name it; 128 KiB of SRAM).
//!
//! Sources:
//! - Photo slots, state vector and `Magic` marker: Raphael Boichot's notes in
//!   <https://github.com/Raphael-Boichot/Inject-pictures-in-your-Game-Boy-Camera-saves>
//!   (no license stated, so used for facts only), checked against the four
//!   samples in `corpus/extra/gameboy-nes/gbcam` (the layout was confirmed
//!   by rendering slot 1 of `gb-photo_photo.sav` to a clean test image).
//! - Tile encoding and the 128x112 size: Pan Docs,
//!   <https://gbdev.io/pandocs/Gameboy_Camera.html> and
//!   <https://gbdev.io/pandocs/Tile_Data.html> (CC0).
//!
//! Slot `n` (1-30) sits at `0x2000 + (n - 1) * 0x1000`: 224 tiles of 16 bytes
//! (16 wide, 14 high), then a thumbnail and metadata that are not used here.
//! The state vector at `0x11B2` has one byte per slot: the photo's album
//! number minus one, or `0xFF` for a deleted/empty slot. An image is a single
//! picture, so the photo with the lowest album number is decoded. A save with
//! no photo (the camera's freshly formatted state) is rejected.

use alloc::vec::Vec;

use super::{SHADES, TILE};
use crate::{DecodeError, Image};

const SAVE_LEN: usize = 0x2_0000;
const STATE_VECTOR: usize = 0x11b2;
const SLOTS: usize = 30;
const MAGIC_AT: usize = 0x11d0;
const SLOT_BASE: usize = 0x2000;
const SLOT_LEN: usize = 0x1000;
const TILES_WIDE: usize = 16;
const TILES_HIGH: usize = 14;

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != SAVE_LEN || data.get(MAGIC_AT..MAGIC_AT + 5) != Some(b"Magic") {
        return Err(DecodeError::Invalid);
    }
    let slot = first_photo_slot(&data[STATE_VECTOR..STATE_VECTOR + SLOTS])?;
    let start = SLOT_BASE + slot * SLOT_LEN;
    let tiles = &data[start..start + TILES_WIDE * TILES_HIGH * TILE.tile_len()];
    TILE.sheet(tiles, TILES_WIDE, &SHADES)
}

/// The slot holding the lowest-numbered photo. Entries must be a photo
/// number below 30 or `0xFF`, with no photo listed twice.
fn first_photo_slot(vector: &[u8]) -> Result<usize, DecodeError> {
    let mut seen = Vec::new();
    let mut first: Option<(u8, usize)> = None;
    for (slot, &photo) in vector.iter().enumerate() {
        if photo == 0xff {
            continue;
        }
        if usize::from(photo) >= SLOTS || seen.contains(&photo) {
            return Err(DecodeError::Invalid);
        }
        seen.push(photo);
        if first.is_none_or(|(lowest, _)| photo < lowest) {
            first = Some((photo, slot));
        }
    }
    first.map(|(_, slot)| slot).ok_or(DecodeError::Invalid)
}
