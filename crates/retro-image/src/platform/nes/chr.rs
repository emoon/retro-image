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

use alloc::vec::Vec;

use super::{PATTERN_TABLE_LEN, tile_pixel};
use crate::{DecodeError, Image};

const TILES_PER_ROW: usize = 16;
const GREYS: [u32; 4] = [0x00_0000, 0x55_5555, 0xaa_aaaa, 0xff_ffff];

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != PATTERN_TABLE_LEN && data.len() != 2 * PATTERN_TABLE_LEN {
        return Err(DecodeError::Unrecognized);
    }
    let tiles = data.len() / 16;
    let (width, height) = (TILES_PER_ROW * 8, tiles / TILES_PER_ROW * 8);
    let indices: Vec<u8> = (0..width * height)
        .map(|i| {
            let (x, y) = (i % width, i / width);
            tile_pixel(data, y / 8 * TILES_PER_ROW + x / 8, x % 8, y % 8)
        })
        .collect();
    Image::from_indexed(width as u32, height as u32, &indices, &GREYS)
}
