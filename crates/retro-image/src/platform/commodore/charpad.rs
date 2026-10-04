//! CharPad projects (`.ctm`, versions 4 to 9): a character set, tiles built
//! from characters and a map of tiles, rendered as the whole map. Text
//! modes only (hires and multicolour characters).
//!
//! Sources:
//! - CTM version 4 description (header fields, section order, character
//!   attributes `MMMMCCCC`),
//!   <https://github.com/martinpiper/C64Public/blob/master/ExternalTools/CharPad/Docs/CharPad%20-%20CTM%20(V4)%20Format.txt>
//!   (documentation only).
//! - Version 5 differences, found by matching section sizes in sample files
//!   and checked against `recoil2png` output: a 20-byte header with a
//!   16-bit tile count, 16-bit tile cells and map entries, colours taken
//!   from the character attributes; bit 2 of the flags byte makes every
//!   character multicolour (`01`/`10` the shared multicolours, `11` the
//!   attribute colour bits 0-2).
//! - Versions 6 to 9 are block based; `blocks.rs` lists their sources.
//!   The `tests/divergences/commodore.tsv` entries record how the CharPad
//!   projects RECOIL rejects were checked.
//!
//! Every version is parsed into a [`Project`], which is then rendered the
//! same way; the parsers only differ in where the data sits.

mod blocks;
mod legacy;

use super::vic2;
use crate::{DecodeError, Image};

/// Largest picture accepted, in pixels.
const MAX_PIXELS: usize = 1 << 24;
/// CharPad's limits: tiles up to 10x10 characters, maps up to 8192x8192 tiles.
const MAX_TILE_SIDE: usize = 10;
const MAX_MAP_SIDE: usize = 8192;

pub(super) fn decode_ctm(data: &[u8]) -> Result<Image, DecodeError> {
    let project = match data.get(..4) {
        Some(b"CTM\x04") => legacy::version4(data),
        Some(b"CTM\x05") => legacy::version5(data),
        Some([b'C', b'T', b'M', 6..=9 | 82]) => blocks::parse(data),
        _ => None,
    };
    project
        .and_then(|project| project.render())
        .ok_or(DecodeError::Unrecognized)
}

/// Whether tile and map sides are within CharPad's limits. Parsers check this
/// before multiplying sizes.
fn sizes_in_range(
    tile_width: usize,
    tile_height: usize,
    map_width: usize,
    map_height: usize,
) -> bool {
    tile_width <= MAX_TILE_SIDE
        && tile_height <= MAX_TILE_SIDE
        && map_width <= MAX_MAP_SIDE
        && map_height <= MAX_MAP_SIDE
}

/// A table of colour bytes, `stride` bytes per entry with the colour at
/// `offset` (the prerelease version 8 stores four colours per entry).
#[derive(Clone, Copy)]
struct ColorTable<'a> {
    data: &'a [u8],
    stride: usize,
    offset: usize,
}

impl<'a> ColorTable<'a> {
    fn bytes(data: &'a [u8]) -> Self {
        Self {
            data,
            stride: 1,
            offset: 0,
        }
    }

    /// The number of entries, if `data` holds a whole number of them.
    fn len(&self) -> Option<usize> {
        self.data
            .len()
            .is_multiple_of(self.stride)
            .then(|| self.data.len() / self.stride)
    }

    fn get(&self, index: usize) -> u8 {
        self.data[index * self.stride + self.offset]
    }
}

/// Where the colour of each character comes from.
#[derive(Clone, Copy)]
enum Colors<'a> {
    /// One colour for the whole project.
    Global(u8),
    PerTile(ColorTable<'a>),
    PerChar(ColorTable<'a>),
    /// One entry per tile cell.
    PerCell(ColorTable<'a>),
}

/// How a tile cell names its character.
#[derive(Clone, Copy)]
enum Cells<'a> {
    /// Cell `i` of tile `t` is character `t * cells + i`.
    Identity,
    /// 16-bit character codes, tile after tile.
    Words(&'a [u8]),
}

/// Everything needed to draw a project, as found in the file.
struct Project<'a> {
    multicolor: bool,
    background: u8,
    multi1: u8,
    multi2: u8,
    /// 8 bytes per character.
    chars: &'a [u8],
    colors: Colors<'a>,
    tile_width: usize,
    tile_height: usize,
    tile_count: usize,
    cells: Cells<'a>,
    map_width: usize,
    map_height: usize,
    /// Tile numbers, 8 or 16 bits each depending on `wide_map`.
    map: &'a [u8],
    wide_map: bool,
}

fn word(data: &[u8], index: usize) -> usize {
    usize::from(u16::from_le_bytes([data[index * 2], data[index * 2 + 1]]))
}

impl Project<'_> {
    /// Checks every size and index, so drawing cannot go out of range.
    fn is_consistent(&self) -> bool {
        if !sizes_in_range(
            self.tile_width,
            self.tile_height,
            self.map_width,
            self.map_height,
        ) {
            return false;
        }
        let tile_cells = self.tile_width * self.tile_height;
        let chars = self.chars.len() / 8;
        let map_entries = self.map_width * self.map_height;
        let colors_fit = match self.colors {
            Colors::Global(_) => true,
            Colors::PerTile(colors) => colors.len() == Some(self.tile_count),
            Colors::PerChar(colors) => colors.len() == Some(chars),
            Colors::PerCell(colors) => colors.len() == Some(self.tile_count * tile_cells),
        };
        let cells_fit = match self.cells {
            Cells::Identity => chars == self.tile_count * tile_cells,
            Cells::Words(words) => {
                words.len() == self.tile_count * tile_cells * 2
                    && (0..words.len() / 2).all(|i| word(words, i) < chars)
            }
        };
        let map_fits = self.map.len() == map_entries * if self.wide_map { 2 } else { 1 }
            && (0..map_entries).all(|i| self.tile_at(i) < self.tile_count);
        tile_cells > 0
            && map_entries > 0
            && chars > 0
            && self.chars.len().is_multiple_of(8)
            && colors_fit
            && cells_fit
            && map_fits
    }

    fn tile_at(&self, map_index: usize) -> usize {
        if self.wide_map {
            word(self.map, map_index)
        } else {
            usize::from(self.map[map_index])
        }
    }

    fn char_at(&self, tile: usize, cell: usize) -> usize {
        let cells = self.tile_width * self.tile_height;
        match self.cells {
            Cells::Identity => tile * cells + cell,
            Cells::Words(words) => word(words, tile * cells + cell),
        }
    }

    fn color_at(&self, tile: usize, cell: usize, char: usize) -> u8 {
        match self.colors {
            Colors::Global(color) => color,
            Colors::PerTile(colors) => colors.get(tile),
            Colors::PerChar(colors) => colors.get(char),
            Colors::PerCell(colors) => colors.get(tile * self.tile_width * self.tile_height + cell),
        }
    }

    fn render(&self) -> Option<Image> {
        let width = self.map_width * self.tile_width * 8;
        let height = self.map_height * self.tile_height * 8;
        if !self.is_consistent() || width.checked_mul(height)? > MAX_PIXELS {
            return None;
        }
        let mut pixels = alloc::vec![0u8; width * height];
        for cell_y in 0..self.map_height * self.tile_height {
            for cell_x in 0..self.map_width * self.tile_width {
                let tile = self
                    .tile_at(cell_y / self.tile_height * self.map_width + cell_x / self.tile_width);
                let cell = cell_y % self.tile_height * self.tile_width + cell_x % self.tile_width;
                let char = self.char_at(tile, cell);
                let color = self.color_at(tile, cell, char) & 15;
                let rows = &self.chars[char * 8..char * 8 + 8];
                for (y, &byte) in rows.iter().enumerate() {
                    let start = (cell_y * 8 + y) * width + cell_x * 8;
                    self.draw_row(byte, color, &mut pixels[start..start + 8]);
                }
            }
        }
        Some(vic2::image(width, height, pixels))
    }

    fn draw_row(&self, byte: u8, color: u8, out: &mut [u8]) {
        for (x, pixel) in out.iter_mut().enumerate() {
            *pixel = if self.multicolor {
                match byte >> (6 - (x & 6)) & 3 {
                    0 => self.background,
                    1 => self.multi1,
                    2 => self.multi2,
                    _ => color & 7,
                }
            } else if byte & (0x80 >> x) != 0 {
                color
            } else {
                self.background
            };
        }
    }
}

/// Reads little-endian fields in order, `None` once the data runs out.
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8], pos: usize) -> Self {
        Self { data, pos }
    }

    fn take(&mut self, len: usize) -> Option<&'a [u8]> {
        let bytes = self.data.get(self.pos..self.pos.checked_add(len)?)?;
        self.pos += len;
        Some(bytes)
    }

    fn byte(&mut self) -> Option<u8> {
        self.take(1).map(|b| b[0])
    }

    fn word(&mut self) -> Option<usize> {
        self.take(2).map(|b| word(b, 0))
    }

    fn at_end(&self) -> bool {
        self.pos == self.data.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    /// A version 9 project with one character (top row `0x80`), no tiles
    /// and a 2x1 map, global colouring: background 6, character colour 1.
    fn version9(mode: u8) -> Vec<u8> {
        let mut data = b"CTM\x09".to_vec();
        data.extend([mode, 0, 0, 0, 0, 0, 0, 0]); // mode, colouring, flags, flexigrid, unused
        data.extend([6, 0, 0, 0, 1, 0, 0]); // screen, MC1, MC2, bg4, three colour bases
        data.extend([0xda, 0xb0, 0, 0, 0x80, 0, 0, 0, 0, 0, 0, 0]); // characters
        data.extend([0xda, 0xb1, 0]); // materials
        data.extend([0xda, 0xb2, 2, 0, 1, 0, 0, 0, 0, 0]); // map
        data
    }

    #[test]
    fn version9_text_hires_global_colour() {
        let image = decode_ctm(&version9(0)).unwrap();
        assert_eq!((image.width(), image.height()), (16, 8));
        let first = image.rgb()[..3].to_vec();
        assert_eq!(first, vic2::image(1, 1, alloc::vec![1]).rgb());
        assert_eq!(
            image.rgb()[3..6],
            vic2::image(1, 1, alloc::vec![6]).rgb()[..]
        );
    }

    #[test]
    fn bitmap_and_extended_modes_are_rejected() {
        for mode in [2, 3, 4] {
            assert!(decode_ctm(&version9(mode)).is_err());
        }
    }

    #[test]
    fn markers_must_count_up() {
        let mut data = version9(0);
        let materials = data.iter().rposition(|&b| b == 0xb1).unwrap();
        data[materials] = 0xb5;
        assert!(decode_ctm(&data).is_err());
    }

    #[test]
    fn trailing_data_is_rejected() {
        let mut data = version9(0);
        data.push(0);
        assert!(decode_ctm(&data).is_err());
    }
}
