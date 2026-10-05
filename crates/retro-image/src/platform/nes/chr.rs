//! NES pattern table (`.chr`) as a sheet of tiles.
//!
//! Sources: pattern table encoding from the nesdev wiki,
//! <https://www.nesdev.org/wiki/PPU_pattern_tables>; sizes (4 KiB for one
//! pattern table, 8 KiB for both) observed from the samples in
//! `corpus/extra/gameboy-nes/nes` (famidash, hxlnt; MIT).
//!
//! The file has no palette, so colour numbers 0-3 are shown as a black to
//! white ramp. Tiles are laid out 16 to a row, 128 pixels wide; an 8 KiB file
//! shows the second pattern table below the first. [`sheet`] also draws the
//! CHR-ROM of a ROM image, up to megabytes: a bigger block gets a wider
//! sheet so that it stays a picture a viewer can show (see
//! [`tiles_per_row`]).

use super::{PATTERN, PATTERN_TABLE_LEN};
use crate::{DecodeError, Image};

/// Tiles in a row of the sheet of a pattern table.
const TABLE_TILES_PER_ROW: usize = 16;
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
    PATTERN.sheet(chr, tiles_per_row(chr.len() / PATTERN.tile_len()), &GREYS)
}

/// Tiles in a row of the sheet of `tiles` tiles: a pattern table's 16,
/// doubled until the sheet is at most four times taller than wide. Up to
/// 16 KiB of tiles keep the narrow layout.
fn tiles_per_row(tiles: usize) -> usize {
    let mut per_row = TABLE_TILES_PER_ROW;
    while tiles.div_ceil(per_row) > 4 * per_row {
        per_row *= 2;
    }
    per_row
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn big_banks_get_wider_sheets() {
        let per_row = |kib: usize| tiles_per_row(kib * 1024 / PATTERN.tile_len());
        assert_eq!(per_row(4), 16);
        assert_eq!(per_row(8), 16);
        assert_eq!(per_row(16), 16);
        assert_eq!(per_row(32), 32);
        assert_eq!(per_row(256), 64);
        assert_eq!(per_row(1024), 128);
        assert_eq!(per_row(0), 16);
    }
}
