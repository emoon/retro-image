//! NES pattern table (`.chr`) as a sheet of tiles.
//!
//! Sources: pattern table encoding from the nesdev wiki,
//! <https://www.nesdev.org/wiki/PPU_pattern_tables>; sizes (4 KiB for one
//! pattern table, 8 KiB for both) observed from the samples in
//! `corpus/extra/gameboy-nes/nes` (famidash, hxlnt; MIT).
//!
//! The file has no palette, so colour numbers 0-3 are shown as a black to
//! white ramp. Tiles are laid out 16 to a row, 128 pixels wide; an 8 KiB file
//! shows the second pattern table below the first.

use super::{PATTERN, PATTERN_TABLE_LEN};
use crate::{DecodeError, Image};

const TILES_PER_ROW: usize = 16;
const GREYS: [u32; 4] = [0x00_0000, 0x55_5555, 0xaa_aaaa, 0xff_ffff];

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != PATTERN_TABLE_LEN && data.len() != 2 * PATTERN_TABLE_LEN {
        return Err(DecodeError::Unrecognized);
    }
    sheet(data)
}

/// The tiles of `chr` as a sheet of grey tiles: how every NES graphics dump
/// is drawn.
pub(super) fn sheet(chr: &[u8]) -> Result<Image, DecodeError> {
    PATTERN.sheet(chr, TILES_PER_ROW, &GREYS)
}
