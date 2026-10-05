//! Sega Mega Drive: Nemesis-compressed tile art (`.nem`) as a sheet of tiles.
//!
//! Sources:
//! - Nemesis compression: see `nemesis.rs`.
//! - The tile layout (8 x 8 pixels, 32 bytes, 4 bytes to a row, one nibble
//!   per pixel, the high nibble on the left) and the color RAM format
//!   (32-byte palettes of 16 big-endian words `----BBB-GGG-RRR-`, so only
//!   the values 0, 2, 4, ..., 14 of a channel mean anything):
//!   Plutiedev, "Tiles and palettes" (<https://plutiedev.com/tiles-and-palettes>).
//! - The output levels of the video DAC: Plutiedev, "VDP color ramp"
//!   (<https://plutiedev.com/vdp-color-ramp>).
//! - Samples: the 156 `.nem` files of `artnem/` in
//!   <https://github.com/sonicretro/s1disasm> (data only, no code read),
//!   with palettes from its `palette/` directory.
//!
//! A `.nem` file holds tiles only. They are drawn 16 to a row, in order, so
//! a sheet is 128 pixels wide.
//!
//! Palette decisions:
//! - Alone, a file is drawn in a 16-step gray ramp (`v * 17`), which tells
//!   little about art that was drawn for a palette.
//! - A companion with the same name and the extension `.pal` supplies a
//!   palette. The extension and the content (the raw color RAM words of one to
//!   four palettes, first one used) are this decoder's own convention:
//!   the Sonic disassemblies keep palettes as `.bin` files with unrelated
//!   names, and no tool was found that pairs palettes with art. A `.pal` of
//!   any other size is ignored.
//! - A channel value of 0, 2, ..., 14 is shown as the DAC level the hardware
//!   outputs, 0, 52, 87, 116, 144, 172, 206, 255 of 255, not as `v * 17`
//!   (which would be 0, 34, 68, ...). The ramp is not linear, and a linear
//!   one makes dark art look too bright. The odd values do not exist in color
//!   RAM; their bit is ignored.
//! - Color 0 is drawn from the palette like the others, though the hardware
//!   treats it as transparent.
//!
//! Detection: the format has no signature, so it is accepted by the extension
//! alone, and rejected unless the whole stream decodes to the declared number
//! of tiles.

use alloc::vec::Vec;

use crate::bytes::be16;
use crate::image::gray_ramp;
use crate::tiles::TileLayout;
use crate::{BitOrder, Companions, DecodeError, Format, Image};

mod nemesis;

pub(super) static FORMATS: &[Format] = &[Format::with_companions(
    "Mega Drive",
    "Nemesis tile art",
    &["nem"],
    decode_nem,
)];

/// A Mega Drive tile: 4-bit pixels, the high nibble on the left.
const TILE: TileLayout = TileLayout::packed(4, BitOrder::MsbFirst);
const TILES_PER_ROW: usize = 16;
/// Bytes of one palette in color RAM, and of the four it holds.
const PALETTE_LEN: usize = 32;
const CRAM_LEN: usize = 4 * PALETTE_LEN;
/// Levels of the DAC for the channel values 0, 2, 4, ..., 14, out of 255.
const DAC: [u32; 8] = [0, 52, 87, 116, 144, 172, 206, 255];

/// Color numbers of a 4-bit tile.
const COLORS: usize = 16;

/// A color RAM word as `0xRRGGBB`.
fn color(word: u16) -> u32 {
    let level = |shift: u32| DAC[usize::from(word >> shift & 7)];
    level(1) << 16 | level(5) << 8 | level(9)
}

/// The first palette of a `.pal` companion, if it is one or more whole
/// palettes of color RAM.
fn palette(file: &[u8]) -> Option<Vec<u32>> {
    if file.is_empty() || !file.len().is_multiple_of(PALETTE_LEN) || file.len() > CRAM_LEN {
        return None;
    }
    Some(
        (0..COLORS)
            .map(|i| color(be16(file, i * 2).unwrap_or(0)))
            .collect(),
    )
}

fn decode_nem(data: &[u8], companions: &dyn Companions) -> Result<Image, DecodeError> {
    let tiles = nemesis::decompress(data).ok_or(DecodeError::Unrecognized)?;
    let palette = companions
        .get("pal")
        .and_then(|file| palette(&file))
        .unwrap_or_else(|| gray_ramp(COLORS));
    TILE.sheet(
        &tiles,
        (tiles.len() / TILE.tile_len()).min(TILES_PER_ROW),
        &palette,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_words_use_the_dac_levels_and_ignore_the_unused_bits() {
        assert_eq!(color(0x0000), 0x00_0000);
        assert_eq!(color(0x0eee), 0xff_ffff);
        // Blue 4, green E, red 0 are the DAC steps 2, 7 and 0.
        assert_eq!(color(0x04e0), 0x00_ff57);
        // The odd values and the top bits of a word carry nothing.
        assert_eq!(color(0xf111), 0x00_0000);
    }

    #[test]
    fn palettes_are_whole_lines_of_color_ram() {
        let mut file = alloc::vec![0; 32];
        file[3] = 0x0e; // color 1, red
        assert_eq!(palette(&file).unwrap()[1], 0xff_0000);
        file.resize(128, 0);
        assert!(palette(&file).is_some());
        file.resize(160, 0);
        assert!(palette(&file).is_none());
        assert!(palette(&[0; 30]).is_none());
        assert!(palette(&[]).is_none());
    }
}
