//! Game Boy Advance (and Nintendo DS) raw tile dumps: `.4bpp` and `.8bpp`
//! tiles with their `.gbapal` palette.
//!
//! Sources:
//! - Tile and palette layouts: GBATEK, "LCD VRAM Character Data" and "LCD
//!   Color Palettes" (<https://problemkaputt.de/gbatek.htm>, no license, facts
//!   only; the DS uses the same layouts). A 4 bpp tile is 32 bytes, 4 bytes a
//!   row, with the left pixel in the low nibble. An 8 bpp tile is 64 bytes, a
//!   byte a pixel. A palette is 16-bit little-endian BGR555 colors, 16 to a
//!   palette in 32 bytes, or 256 in 512 bytes.
//! - The extensions: the `pret` decompilations (for example
//!   <https://github.com/pret/pokeemerald>) build tile data to `.4bpp` and
//!   `.8bpp` and palettes to `.gbapal`; those names are read from the
//!   repository's `.gitignore` (the repositories show no license, so nothing
//!   else of them was read).
//! - Checked against SuperFamiconv v0.12 (MIT, <https://github.com/Optiroc/SuperFamiconv>)
//!   in its `gba` and `gba_affine` modes, run as a black box: its native tile
//!   and palette data (`tiles.bin`, `palette.bin`, renamed) and the tile image
//!   it writes of them. See `tests/divergences/nintendo-resources.tsv`.
//!
//! The files are headerless dumps, so they are chosen by extension only, and
//! the same extensions mean planar tiles for the Super Nintendo; this module
//! takes `.4bpp` and `.8bpp` to be the linear tiles of the Game Boy Advance
//! and the DS, and the Super Nintendo's planar dumps are not decoded (no
//! toolchain gives them an extension of their own). `.lz` files (the GBA BIOS
//! compression) are not decoded either: the extension is shared by tile data,
//! tile maps and palettes, and only the stem before it, which a decoder does
//! not see, says which.
//!
//! Tiles are drawn as a sheet 16 to a row. Colors come from palette 0 of the
//! `.gbapal` with the same name (the first 16 colors for 4 bpp, 256 for 8
//! bpp), or else a gray ramp. Color 0 is transparent on the console but is
//! drawn as its palette color, as for the other tile sheets here.

use alloc::vec::Vec;

use crate::image::{bgr555, gray_ramp};
use crate::tiles::TileLayout;
use crate::{BitOrder, Companions, DecodeError, Format, Image};

const TILES_PER_ROW: usize = 16;

pub(super) static FORMATS: &[Format] = &[
    Format::with_companions("Game Boy Advance", "4 bpp tile data", &["4bpp"], |d, c| {
        decode(d, c, 4)
    }),
    Format::with_companions("Game Boy Advance", "8 bpp tile data", &["8bpp"], |d, c| {
        decode(d, c, 8)
    }),
];

/// The tiles of `data` as a sheet, `bits` bits a pixel.
fn decode(data: &[u8], companions: &dyn Companions, bits: usize) -> Result<Image, DecodeError> {
    let colors = 1usize << bits;
    let palette = companions
        .get("gbapal")
        .map(|file| palette_colors(&file, colors))
        .unwrap_or_else(|| gray_ramp(colors));
    TileLayout::packed(bits, BitOrder::LsbFirst).sheet(data, TILES_PER_ROW, &palette)
}

/// The first `count` colors of a palette file as 0xRRGGBB, black for those
/// the file lacks.
fn palette_colors(file: &[u8], count: usize) -> Vec<u32> {
    let mut colors: Vec<u32> = file
        .as_chunks::<2>()
        .0
        .iter()
        .take(count)
        .map(|&word| bgr555(u16::from_le_bytes(word)))
        .collect();
    colors.resize(count, 0);
    colors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NoCompanions;

    struct Palette(Vec<u8>);

    impl Companions for Palette {
        fn get(&self, extension: &str) -> Option<Vec<u8>> {
            (extension == "gbapal").then(|| self.0.clone())
        }

        fn get_named(&self, _name: &str) -> Option<Vec<u8>> {
            None
        }
    }

    #[test]
    fn four_bit_tiles_have_the_left_pixel_in_the_low_nibble() {
        let mut tile = [0u8; 32];
        tile[0] = 0x21; // pixel 0 = 1, pixel 1 = 2
        tile[4] = 0xf0; // row 1: pixel 0 = 0, pixel 1 = 15
        let image = decode(&tile, &NoCompanions, 4).unwrap();
        assert_eq!((image.width(), image.height()), (128, 8));
        assert_eq!(image.get(0, 0), 0x11_1111);
        assert_eq!(image.get(1, 0), 0x22_2222);
        assert_eq!(image.get(1, 1), 0xff_ffff);
        assert_eq!(
            decode(&tile[..31], &NoCompanions, 4),
            Err(DecodeError::Invalid)
        );
    }

    #[test]
    fn eight_bit_tiles_hold_a_byte_a_pixel_and_a_palette_colors_them() {
        let mut tile = [0u8; 64];
        tile[9] = 200; // row 1, pixel 1
        let mut palette = alloc::vec![0u8; 512];
        palette[400..402].copy_from_slice(&0x03e0u16.to_le_bytes()); // color 200: green
        let image = decode(&tile, &Palette(palette), 8).unwrap();
        assert_eq!(image.get(1, 1), 0x00_ff00);
        // A palette file shorter than the colors in use leaves them black.
        let short = decode(&tile, &Palette(alloc::vec![0xff; 32]), 8).unwrap();
        assert_eq!(short.get(1, 1), 0);
        assert_eq!(short.get(0, 0), 0xff_ffff);
    }
}
