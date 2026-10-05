//! Files written by RGBDS's `rgbgfx`: tile data (`.2bpp`, `.1bpp`), tile map
//! (`.tilemap`), attribute map (`.attrmap`) and palette set (`.pal`).
//!
//! Sources:
//! - The `rgbgfx(1)` manual, "OUTPUT FILES" and "REVERSE MODE"
//!   (<https://rgbds.gbdev.io/docs/master/rgbgfx.1>), and the way `rgbgfx -r`
//!   puts those files back together in `src/gfx/reverse.cpp`, both from
//!   <https://github.com/gbdev/rgbds> (MIT license, notice below).
//! - Tile and attribute bit layouts: Pan Docs, "Tile Data" and "Tile Maps"
//!   (<https://gbdev.io/pandocs/Tile_Data.html>,
//!   <https://gbdev.io/pandocs/Tile_Maps.html>, CC0).
//! - Checked against RGBDS's `test/gfx/` fixtures (MIT) and against
//!   `rgbgfx` itself, built from that repository and run as a black box:
//!   see `tests/divergences/nintendo-resources.tsv`.
//!
//! The files are headerless dumps, so they are chosen by extension only.
//! - Tile data is 16 bytes a tile (8 with 1 bpp), the 2 bpp layout of every
//!   Game Boy tile. A `.2bpp` or `.1bpp` alone is a sheet of tiles, 16 to a
//!   row, in four gray shades (white and black for 1 bpp, as `rgbgfx -r`
//!   draws them).
//! - A tile map is one byte per tile with no width. Only the two sizes of
//!   Game Boy screens say what it is: 360 bytes are the 20 x 18 visible
//!   screen and 1024 bytes the 32 x 32 background map. Any other size is
//!   rejected, since a guessed width would draw nonsense. The attribute map
//!   has the same size and holds, per tile, the palette (bits 0-2), the bank
//!   of the tile (bit 3) and mirroring (bits 5 and 6, written only with
//!   `rgbgfx -m`). The tile data and attribute map are found as companions.
//! - The palette set is 2-byte little-endian RGB555 colors, 4 per palette
//!   (2 with 1 bpp). The transparent color is the word `0x8000` (and an
//!   empty slot `0xFFFF`), which is drawn as the shared transparent fill; no
//!   other word has bit 15 set. A `.pal` that is not a palette set is ignored
//!   and the image is drawn in gray. That is a size that is not a multiple
//!   of a palette, a text (JASC-PAL, GIMP, `RGB 31, 31, 31`: all printable
//!   ASCII), a word with another bit 15 pattern, the 192 or 1536 bytes of a
//!   NES emulator palette, the 768 bytes of a VGA palette, or the 16 or 32
//!   bytes of a NES Screen Tool palette (NES color numbers, all below 0x40).
//!   Without `.pal` every tile uses the gray shades whatever its palette
//!   number.
//!
//! What the files do not record is guessed as `rgbgfx -r` does by default:
//! tile IDs start at 0 (no `-b`), tiles trimmed with `-x` come out blank, and
//! the base palette number (`-l`) and palette map (`.palmap`, for more than
//! eight palettes) are not used. Tiles of bank 1 follow those of bank 0 in
//! the tile data, so they start after the highest tile ID the bank 0 cells
//! use, which is how many tiles `-N` put in bank 0 whenever bank 0 is full
//! and fully used, as `rgbgfx` makes it.
//!
//! The 2 bpp and 1 bpp files of a tile map must exist as companions of a
//! `.tilemap` or `.attrmap`, which are rejected without them. A `.2bpp` or
//! `.1bpp` draws the whole screen when its `.tilemap` is there and fits.

// Parts of this file follow RGBDS's rgbgfx (https://github.com/gbdev/rgbds):
//
// The MIT License
//
// Copyright (c) 1996-2025, Carsten Sørensen and RGBDS contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to
// deal in the Software without restriction, including without limitation the
// rights to use, copy, modify, merge, publish, distribute, sublicense, and/or
// sell copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
// FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS
// IN THE SOFTWARE.

use alloc::vec::Vec;

use super::{SHADES, TILE};
use crate::bytes::le16;
use crate::image::{TRANSPARENT_FILL, bgr555, check_size};
use crate::tiles::TileLayout;
use crate::{Companions, DecodeError, Image};

/// A tile data file: its tile layout and the colors it has when no `.pal`
/// says otherwise.
#[derive(Clone, Copy)]
struct Depth {
    layout: TileLayout,
    extension: &'static str,
    /// Colors of palette 0, and the colors a palette in a `.pal` has.
    shades: [u32; 4],
}

const TWO_BPP: Depth = Depth {
    layout: TILE,
    extension: "2bpp",
    shades: SHADES,
};

const ONE_BPP: Depth = Depth {
    layout: TileLayout::planar(1, 1),
    extension: "1bpp",
    shades: [0xff_ffff, 0x00_0000, 0x00_0000, 0x00_0000],
};

impl Depth {
    fn colors(&self) -> usize {
        1 << self.layout.bpp
    }
}

/// Tiles to a row of a sheet.
const SHEET_TILES_PER_ROW: usize = 16;
/// Byte sizes of the screens a tile map can be, and their width in tiles.
const SCREEN_WIDTHS: [(usize, usize); 2] = [(20 * 18, 20), (32 * 32, 32)];
/// Sizes of the other binary files that share the `.pal` extension: NES
/// emulator palettes (64 colors, with and without emphasis variants) and
/// VGA palettes (256 colors of 3 bytes).
const OTHER_PALETTE_LENS: [usize; 3] = [192, 768, 1536];
/// Sizes of the NES Screen Tool palettes, 16 or 32 NES color numbers, which
/// are all below this.
const NESST_PALETTE_LENS: [usize; 2] = [16, 32];
const NES_COLOR_LIMIT: u8 = 0x40;
/// The words with bit 15 set that rgbgfx writes: the transparent color and
/// an empty slot.
const TRANSPARENT: u16 = 0x8000;
const EMPTY: u16 = 0xffff;

type Palette = [u32; 4];

/// Tile data, with the files that arrange and color it.
struct Parts<'a> {
    depth: Depth,
    tiles: &'a [u8],
    tilemap: Option<&'a [u8]>,
    attributes: Option<&'a [u8]>,
    /// The palette set, when a usable `.pal` came with it.
    palettes: Option<Vec<Palette>>,
}

/// The `.2bpp` or `.1bpp` file `data`: the screen its map files draw, or else
/// a sheet of tiles.
fn decode_tiles(
    data: &[u8],
    companions: &dyn Companions,
    depth: Depth,
) -> Result<Image, DecodeError> {
    let tilemap = companions.get("tilemap");
    let attributes = companions.get("attrmap");
    let parts = Parts {
        depth,
        tiles: data,
        tilemap: tilemap.as_deref(),
        attributes: attributes.as_deref(),
        palettes: read_palettes(companions, depth),
    };
    parts.screen().or_else(|_| parts.sheet())
}

pub(super) fn decode_2bpp(data: &[u8], companions: &dyn Companions) -> Result<Image, DecodeError> {
    decode_tiles(data, companions, TWO_BPP)
}

pub(super) fn decode_1bpp(data: &[u8], companions: &dyn Companions) -> Result<Image, DecodeError> {
    decode_tiles(data, companions, ONE_BPP)
}

/// A screen from a map file `data` (a `.tilemap` or `.attrmap`) and the other
/// files; the tile data and the tile map must be there.
fn decode_map(
    data: &[u8],
    companions: &dyn Companions,
    is_tilemap: bool,
) -> Result<Image, DecodeError> {
    let (depth, tiles) = [TWO_BPP, ONE_BPP]
        .into_iter()
        .find_map(|depth| Some((depth, companions.get(depth.extension)?)))
        .ok_or(DecodeError::Unrecognized)?;
    let other = companions.get(if is_tilemap { "attrmap" } else { "tilemap" });
    let (tilemap, attributes) = if is_tilemap {
        (Some(data), other.as_deref())
    } else {
        (other.as_deref(), Some(data))
    };
    Parts {
        depth,
        tiles: &tiles,
        tilemap,
        attributes,
        palettes: read_palettes(companions, depth),
    }
    .screen()
}

pub(super) fn decode_tilemap(
    data: &[u8],
    companions: &dyn Companions,
) -> Result<Image, DecodeError> {
    decode_map(data, companions, true)
}

pub(super) fn decode_attrmap(
    data: &[u8],
    companions: &dyn Companions,
) -> Result<Image, DecodeError> {
    decode_map(data, companions, false)
}

fn read_palettes(companions: &dyn Companions, depth: Depth) -> Option<Vec<Palette>> {
    parse_palettes(&companions.get("pal")?, depth)
}

/// Whether `file` is a text, such as a JASC, GIMP or `RGB 31, 31, 31` palette.
fn is_text(file: &[u8]) -> bool {
    file.iter()
        .all(|&b| b.is_ascii_graphic() || b.is_ascii_whitespace())
}

/// The palettes of a `.pal` file, or `None` if it is something else: a text,
/// a file of a size that another kind of palette has, or one with a word
/// that rgbgfx would not write (bit 15 is clear in a color).
fn parse_palettes(file: &[u8], depth: Depth) -> Option<Vec<Palette>> {
    let stride = depth.colors() * 2;
    let nes_colors =
        NESST_PALETTE_LENS.contains(&file.len()) && file.iter().all(|&b| b < NES_COLOR_LIMIT);
    let words = file
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&w| u16::from_le_bytes(w));
    let writable = words
        .clone()
        .all(|w| w & 0x8000 == 0 || w == TRANSPARENT || w == EMPTY);
    if file.is_empty()
        || !file.len().is_multiple_of(stride)
        || OTHER_PALETTE_LENS.contains(&file.len())
        || nes_colors
        || is_text(file)
        || !writable
    {
        return None;
    }
    let palettes = file.chunks_exact(stride).map(|bytes| {
        let mut palette = depth.shades;
        for (i, color) in palette.iter_mut().take(depth.colors()).enumerate() {
            *color = rgb555(le16(bytes, i * 2).unwrap_or(0));
        }
        palette
    });
    Some(palettes.collect())
}

/// A Game Boy Color color, with bit 15 set for a transparent one.
fn rgb555(color: u16) -> u32 {
    if color & TRANSPARENT != 0 {
        return TRANSPARENT_FILL;
    }
    bgr555(color)
}

impl Parts<'_> {
    /// Palette 0, which draws everything that no attribute colors.
    fn first_palette(&self) -> Palette {
        self.palettes
            .as_ref()
            .and_then(|palettes| palettes.first().copied())
            .unwrap_or(self.depth.shades)
    }

    /// The tiles as a sheet.
    fn sheet(&self) -> Result<Image, DecodeError> {
        let palette = self.first_palette();
        self.depth
            .layout
            .sheet(self.tiles, SHEET_TILES_PER_ROW, &palette)
    }

    /// The screen the tile map arranges the tiles in. Fails without a tile
    /// map of a screen size, if the attribute map does not have the same
    /// size, or if a cell names a palette that the `.pal` lacks.
    fn screen(&self) -> Result<Image, DecodeError> {
        let len = self.tilemap.ok_or(DecodeError::Unrecognized)?.len();
        let width = SCREEN_WIDTHS
            .iter()
            .find_map(|&(screen_len, width)| (screen_len == len).then_some(width))
            .ok_or(DecodeError::Unrecognized)?;
        self.draw(width)
    }

    /// The screen of a tile map `width` tiles wide.
    fn draw(&self, width: usize) -> Result<Image, DecodeError> {
        let names = self.tilemap.ok_or(DecodeError::Unrecognized)?;
        let attributes = self.attributes.unwrap_or(&[]);
        if width == 0
            || !names.len().is_multiple_of(width)
            || self.attributes.is_some() && attributes.len() != names.len()
        {
            return Err(DecodeError::Unrecognized);
        }
        let layout = &self.depth.layout;
        if self.tiles.is_empty() || !self.tiles.len().is_multiple_of(layout.tile_len()) {
            return Err(DecodeError::Unrecognized);
        }
        let attribute = |cell: usize| attributes.get(cell).copied().unwrap_or(0);
        let height = names.len() / width;
        let (pixel_width, pixel_height) = (width * layout.width, height * layout.height);
        check_size(pixel_width, pixel_height)?;

        // Bank 1 follows the tiles of bank 0, which `-N` filled and the map
        // uses up to its highest ID.
        let in_bank0 = |cell: usize| attribute(cell) & BANK == 0;
        let bank0_tiles = if (0..names.len()).all(in_bank0) {
            0
        } else {
            let highest = (0..names.len()).filter(|&cell| in_bank0(cell));
            highest
                .map(|cell| usize::from(names[cell]) + 1)
                .max()
                .unwrap_or(0)
        };

        let tile_count = self.tiles.len() / layout.tile_len();
        let unpacked = layout.unpack(self.tiles);
        let mut colors = alloc::vec![0; pixel_width * pixel_height];
        for (cell, &id) in names.iter().enumerate() {
            let attr = attribute(cell);
            let palette = self.palette(attr & PALETTE_MASK)?;
            let tile = usize::from(id) + if in_bank0(cell) { 0 } else { bank0_tiles };
            let (left, top) = (cell % width * layout.width, cell / width * layout.height);
            for row in 0..layout.height {
                let out = &mut colors[(top + row) * pixel_width + left..][..layout.width];
                if tile >= tile_count {
                    // Tiles that `-x` left out of the file are blank.
                    out.fill(palette[0]);
                    continue;
                }
                let source = if attr & FLIP_Y != 0 {
                    layout.height - 1 - row
                } else {
                    row
                };
                let pixels = unpacked.row(tile, source);
                for (x, color) in out.iter_mut().enumerate() {
                    let x = if attr & FLIP_X != 0 {
                        layout.width - 1 - x
                    } else {
                        x
                    };
                    *color = palette[usize::from(pixels[x])];
                }
            }
        }
        Ok(Image::from_colors(
            pixel_width as u32,
            pixel_height as u32,
            colors.into_iter(),
        ))
    }

    /// Palette number `index`: from the `.pal` if there is one, and the
    /// gray shades whatever the number if there is not.
    fn palette(&self, index: u8) -> Result<Palette, DecodeError> {
        match &self.palettes {
            Some(palettes) => palettes
                .get(usize::from(index))
                .copied()
                .ok_or(DecodeError::Unrecognized),
            None => Ok(self.depth.shades),
        }
    }
}

const PALETTE_MASK: u8 = 0b111;
const BANK: u8 = 0b1000;
const FLIP_X: u8 = 0b10_0000;
const FLIP_Y: u8 = 0b100_0000;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NoCompanions;

    /// Companion files keyed by extension.
    struct Files<'a>(&'a [(&'a str, &'a [u8])]);

    impl Companions for Files<'_> {
        fn get_named(&self, _file_name: &str) -> Option<Vec<u8>> {
            None
        }

        fn get(&self, extension: &str) -> Option<Vec<u8>> {
            let (_, data) = self.0.iter().find(|(e, _)| *e == extension)?;
            Some(data.to_vec())
        }
    }

    /// Tile 0 is blank. Tile 1 has color 1 at the left and color 2 at the
    /// right of its top row. Tile 2 is solid color 3.
    fn tile_data() -> Vec<u8> {
        let mut tiles = alloc::vec![0; 48];
        tiles[16] = 0b1000_0000; // tile 1, row 0, low plane
        tiles[17] = 0b0000_0001; // tile 1, row 0, high plane
        tiles[32..].fill(0xff);
        tiles
    }

    fn pixel(image: &Image, tile_x: u32, x: u32, y: u32) -> u32 {
        image.get(tile_x * 8 + x, y)
    }

    #[test]
    fn alone_the_tile_data_is_a_gray_sheet_sixteen_tiles_wide() {
        let image = decode_2bpp(&[0; 17 * 16], &NoCompanions).unwrap();
        assert_eq!((image.width(), image.height()), (128, 16));
        let mono = decode_1bpp(&[0xff; 8], &NoCompanions).unwrap();
        assert_eq!((mono.width(), mono.height()), (128, 8));
        assert_eq!(mono.get(0, 0), 0x00_0000);
        assert_eq!(
            decode_2bpp(&[0; 15], &NoCompanions),
            Err(DecodeError::Unrecognized)
        );
    }

    #[test]
    fn a_tile_map_of_a_screen_size_arranges_the_tiles() {
        let mut map = alloc::vec![0u8; 20 * 18];
        map[1] = 1;
        map[21] = 2; // second row, second column
        let files = Files(&[("tilemap", &map)]);
        let image = decode_2bpp(&tile_data(), &files).unwrap();
        assert_eq!((image.width(), image.height()), (160, 144));
        assert_eq!(pixel(&image, 1, 0, 0), SHADES[1]);
        assert_eq!(pixel(&image, 1, 7, 0), SHADES[2]);
        assert_eq!(pixel(&image, 1, 3, 11), SHADES[3]);
        // Another size of map leaves the sheet, and a tile map needs tiles.
        let short = Files(&[("tilemap", &map[..100])]);
        assert_eq!(decode_2bpp(&tile_data(), &short).unwrap().width(), 128);
        assert!(decode_tilemap(&map, &NoCompanions).is_err());
        let tiles = Files(&[("2bpp", &tile_data())]);
        assert!(decode_tilemap(&map[..100], &tiles).is_err());
        assert_eq!(decode_tilemap(&map, &tiles), Ok(image));
    }

    #[test]
    fn attributes_pick_the_palette_mirroring_and_bank() {
        let mut map = alloc::vec![0u8; 32 * 32];
        let mut attributes = alloc::vec![0u8; 32 * 32];
        map[0] = 1; // mirrored both ways, palette 1
        attributes[0] = 0b110_0001;
        map[1] = 1; // bank 0 uses tiles 0 and 1, so bank 1 starts at tile 2
        map[2] = 0; // tile 0 of bank 1
        attributes[2] = 0b1000;
        let palettes = [
            0xff, 0x7f, 0x00, 0x7c, 0x00, 0x00, 0xe0, 0x03, // white, blue, black, green
            0x00, 0x80, 0x1f, 0x00, 0xe0, 0x03, 0x00, 0x7c, // transparent, red, green, blue
        ];
        let files = Files(&[
            ("tilemap", &map),
            ("attrmap", &attributes),
            ("pal", &palettes),
        ]);
        let image = decode_2bpp(&tile_data(), &files).unwrap();
        assert_eq!((image.width(), image.height()), (256, 256));
        // The top row of tile 1 is on the bottom row, left and right swapped.
        assert_eq!(pixel(&image, 0, 7, 7), 0xff0000);
        assert_eq!(pixel(&image, 0, 0, 7), 0x00ff00);
        assert_eq!(pixel(&image, 0, 0, 0), TRANSPARENT_FILL);
        assert_eq!(pixel(&image, 1, 0, 0), 0x0000ff);
        // Bank 1 tile 0 is the solid tile: color 3 of palette 0.
        assert_eq!(pixel(&image, 2, 5, 5), 0x00ff00);
        assert_eq!(pixel(&image, 3, 5, 5), 0xff_ffff);
        // A palette number that the file lacks spoils the screen.
        attributes[0] = 2;
        let broken = Files(&[
            ("tilemap", &map),
            ("attrmap", &attributes),
            ("pal", &palettes),
        ]);
        assert_eq!(decode_2bpp(&tile_data(), &broken).unwrap().width(), 128);
    }

    #[test]
    fn the_attribute_map_alone_needs_the_tile_map_and_tiles() {
        let mut map = alloc::vec![0u8; 20 * 18];
        map[0] = 1;
        let attributes = alloc::vec![0u8; 20 * 18];
        let all = Files(&[("tilemap", &map), ("2bpp", &tile_data())]);
        let screen = decode_attrmap(&attributes, &all).unwrap();
        assert_eq!(screen.width(), 160);
        assert!(decode_attrmap(&attributes, &Files(&[("tilemap", &map)])).is_err());
        assert!(decode_attrmap(&attributes[..10], &all).is_err());
    }

    #[test]
    fn files_that_only_share_the_pal_extension_are_ignored() {
        let mut jasc = b"JASC-PAL\r\n0100\r\n4\r\n".to_vec();
        jasc.resize(24, b' ');
        assert!(parse_palettes(&jasc, TWO_BPP).is_none());
        assert!(parse_palettes(&[0; 192], TWO_BPP).is_none());
        assert!(parse_palettes(&[0; 1536], TWO_BPP).is_none());
        assert!(parse_palettes(&[0; 12], TWO_BPP).is_none());
        let words = [0xff, 0x7f, 0, 0, 0x10, 0x42, 0, 0];
        assert_eq!(parse_palettes(&words, TWO_BPP).unwrap().len(), 1);
        // A 1 bpp palette has two colors.
        assert_eq!(parse_palettes(&words, ONE_BPP).unwrap().len(), 2);
        // The transparent word and an empty slot are rgbgfx's; other words
        // with bit 15 set are not.
        let transparent = [0x00, 0x80, 0xff, 0xff, 0x10, 0x42, 0, 0];
        assert_eq!(parse_palettes(&transparent, TWO_BPP).unwrap().len(), 1);
        assert!(parse_palettes(&[0, 0x80, 1, 0x80, 0, 0, 0, 0], TWO_BPP).is_none());
    }

    #[test]
    fn text_vga_and_nes_screen_tool_palettes_do_not_recolor_the_tiles() {
        // Palettes that other programs write under the same extension, each
        // of a size that is a whole number of rgbgfx palettes.
        let mut text = b"RGB 31, 31, 31\r\nRGB 0, 0, 0\r\n".to_vec();
        text.resize(32, b' ');
        let vga: Vec<u8> = (0..768).map(|i| (i % 64) as u8).collect();
        let nes_screen_tool = [
            0x0f, 0x00, 0x10, 0x30, 0x0f, 0x06, 0x16, 0x26, 0x0f, 0x09, 0x19, 0x29, 0x0f, 0x0c,
            0x1c, 0x2c,
        ];
        let tiles = [0xff; 16]; // every pixel is color 1
        let gray = decode_2bpp(&tiles, &NoCompanions).unwrap().get(0, 0);
        for file in [text, vga, nes_screen_tool.to_vec()] {
            assert!(
                parse_palettes(&file, TWO_BPP).is_none(),
                "{} bytes",
                file.len()
            );
            let colored = decode_2bpp(&tiles, &Files(&[("pal", &file)])).unwrap();
            assert_eq!(colored.get(0, 0), gray, "{} bytes", file.len());
        }
        // A dark rgbgfx palette of the same size is still a palette.
        let dark = [
            0x00, 0x00, 0x21, 0x04, 0x42, 0x08, 0x63, 0x0c, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        assert!(parse_palettes(&dark, TWO_BPP).is_some());
    }
}
