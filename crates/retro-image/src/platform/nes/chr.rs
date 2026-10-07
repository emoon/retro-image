//! NES pattern table (`.chr`) as a sheet of tiles.
//!
//! Sources: pattern table encoding from the nesdev wiki,
//! <https://www.nesdev.org/wiki/PPU_pattern_tables>; sizes observed from the
//! samples in `corpus/extra/gameboy-nes/nes` (famidash, hxlnt; MIT: 4 and
//! 8 KiB) and the `.chr` files of `christopherpow/nes-test-roms` in
//! `corpus/extra/nintendo-rom-icons` (1.5, 2, 3, 4, 8, 16 and 128 KiB, all
//! multiples of 512 bytes, so a file is a whole number of 32-tile blocks, not
//! only whole pattern tables). Other lengths are not taken: `.chr` is also the
//! extension of Borland BGI fonts and Atari 8-bit character sets.
//!
//! The file has no palette, so color numbers 0-3 are shown as a black to
//! white ramp. Tiles are laid out 16 to a row, 128 pixels wide; an 8 KiB file
//! shows the second pattern table below the first. [`sheet`] also draws the
//! CHR-ROM of a ROM image, up to megabytes: a bigger block gets a wider
//! sheet so that it stays a picture a viewer can show (see
//! [`tiles_per_row`]).

use super::PATTERN;
use crate::{DecodeError, Image};

/// Tiles in a row of the sheet of a pattern table.
const TABLE_TILES_PER_ROW: usize = 16;
const GREYS: [u32; 4] = [0x00_0000, 0x55_5555, 0xaa_aaaa, 0xff_ffff];

/// Most bytes a `.chr` file may have: the 256 KiB of the biggest common
/// CHR-ROM (MMC3 boards).
pub(super) const MAX_LEN: usize = 256 * 1024;

/// Bytes of 32 tiles. A file is a whole number of these.
const BLOCK_LEN: usize = 512;

/// Bytes of an Atari 8-bit character set (128 characters of 8 bytes), which
/// the Atari decoders claim under the same extension: the fonts of
/// `corpus/hostile/atari8/pigwa-forever` are 1024 bytes and look like noise
/// as tiles. A 1 KiB NES file is not taken for tiles.
const ATARI_FONT_LEN: usize = 1024;

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() > MAX_LEN || !data.len().is_multiple_of(BLOCK_LEN) || data.len() == ATARI_FONT_LEN
    {
        return Err(DecodeError::Invalid);
    }
    sheet(data)
}

/// The tiles of `chr` as a sheet of gray tiles: how every NES graphics dump
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
    fn whole_blocks_of_32_tiles_up_to_the_cap_are_a_pattern_table() {
        let size = |len: usize| {
            decode(&alloc::vec![0; len])
                .map(|image| (image.width(), image.height()))
                .ok()
        };
        assert_eq!(size(1536), Some((128, 48)));
        assert_eq!(size(BLOCK_LEN), Some((128, 16)));
        assert_eq!(size(MAX_LEN), Some((512, 2048)));
        assert_eq!(size(0), None);
        assert_eq!(size(1535), None);
        // Whole tiles, but not whole blocks: the size of a BGI font, say.
        assert_eq!(size(2000), None);
        assert_eq!(size(16), None);
        assert_eq!(size(ATARI_FONT_LEN), None);
        assert_eq!(size(MAX_LEN + BLOCK_LEN), None);
    }

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
