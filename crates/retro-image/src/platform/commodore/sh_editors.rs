//! Pictures of the 3lux Super Hires editors, packed with the editors' own
//! RLE: SH1 (Super Hires Editor I, 96x168) and SH2 (Super Hires Editor II,
//! 192x168).
//!
//! Sources, none documented anywhere; everything below was found with the
//! maintainer's permission to disassemble the editors:
//! - File and packer: the save routines of `SuperHiRes Editor V1.0` and
//!   `SuperHires Editor V2.3` from the CSDb tools archive
//!   (<https://csdb.dk>, `super-hires-editor-1.0-3lux.d64` and
//!   `super-hireseditorv2.3.d64`), read after running the programs'
//!   crunchers in a 6502 emulator. V1 writes memory `$8000-$989F` (6304
//!   bytes), V2 `$8000-$A17F` (8576 bytes), both as [`escape_first_rle`]
//!   without a load address. `recoil2png` takes the output of the V1.0 save
//!   routine run in the emulator as it is, as `.sh1`.
//! - Picture layout: reverse engineered by flipping bytes of such files in
//!   `recoil2png` and watching the pixels, then checked on random data
//!   (a Python model of the layout below matched `recoil2png` on every
//!   pixel). The memory is a hires bitmap of 8x8 cells, sprite layers
//!   drawn as strips of bytes, the screen RAM and the sprite colors:
//!   - the bitmap: cells row by row, eight bytes per cell;
//!   - each sprite layer: one strip of 168 bytes (a byte per line, bit 7
//!     leftmost) for every 8 pixels of width, left to right;
//!   - the screen RAM, one byte per cell: set pixels use the high nibble,
//!     clear ones the low nibble;
//!   - a color byte for every 24 pixels (one sprite column): the low
//!     nibble colors the first layer, the high nibble the second.
//!
//!   SH1 has two layers (the second wins), SH2 has one. Sprite pixels hide
//!   the bitmap. The strips of three columns form one 24-pixel-wide sprite,
//!   hence a color per three columns.

use super::superhires::{bit, hires, render};
use super::unpack::escape_first_rle;
use crate::{DecodeError, Image};

const HEIGHT: usize = 168;

/// Memory layout of a Super Hires editor.
struct Layout {
    /// 8-pixel columns across the picture.
    columns: usize,
    /// Sprite layers, the last one on top.
    layers: usize,
}

impl Layout {
    const fn cells(&self) -> usize {
        self.columns * HEIGHT / 8
    }

    /// Start of the first sprite layer; the bitmap is in front of it.
    const fn strips(&self) -> usize {
        self.cells() * 8
    }

    const fn screen(&self) -> usize {
        self.strips() + self.layers * self.columns * HEIGHT
    }

    const fn colors(&self) -> usize {
        self.screen() + self.cells()
    }

    /// Bytes of memory the editor saves.
    const fn len(&self) -> usize {
        self.colors() + self.columns / 3
    }

    fn decode(&self, data: &[u8]) -> Result<Image, DecodeError> {
        let memory = escape_first_rle(data, self.len()).ok_or(DecodeError::Invalid)?;
        let width = self.columns * 8;
        Ok(render(width, HEIGHT, |x, y| {
            let cell = y / 8 * self.columns + x / 8;
            let bitmap = &memory[cell * 8 + y % 8..][..1];
            let color = hires(bit(bitmap, x % 8), memory[self.screen() + cell]);
            let sprite_colors = memory[self.colors() + x / 24];
            (0..self.layers).fold(color, |color, layer| {
                let strip = self.strips() + (layer * self.columns + x / 8) * HEIGHT;
                if bit(&memory[strip + y..][..1], x % 8) {
                    if layer == 0 {
                        sprite_colors & 15
                    } else {
                        sprite_colors >> 4
                    }
                } else {
                    color
                }
            })
        }))
    }
}

const SH1: Layout = Layout {
    columns: 12,
    layers: 2,
};
const SH2: Layout = Layout {
    columns: 24,
    layers: 1,
};

/// Super Hires Editor I: 96x168 with two sprite layers.
pub(super) fn decode_sh1(data: &[u8]) -> Result<Image, DecodeError> {
    SH1.decode(data)
}

/// Super Hires Editor II: 192x168 with one sprite layer.
pub(super) fn decode_sh2(data: &[u8]) -> Result<Image, DecodeError> {
    SH2.decode(data)
}
