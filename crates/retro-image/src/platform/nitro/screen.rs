//! NSCR screens: the tile map of a Nintendo DS background, drawn with the
//! tiles of an NCGR.
//!
//! Sources: NitroPaint (BSD 2-Clause, notice below),
//! `NitroPaint/object/NitroScreen.c` (`ScrReadNscr`, `ScriReadScreenDataAs`,
//! `nscrGetTileEx`), and GBATEK, "DS Files - Video Screen" and "LCD VRAM BG
//! Screen Data Format"; see `nitro.rs` for how the two differ. Unverified: no
//! sample file was available.
//!
//! The `NRCS` section holds, from the start of its data: the width and the
//! height in pixels (16 bits each), the color mode, the screen format (0 text,
//! 1 affine, 2 affine extended), the size of the entries, and the entries from
//! offset 0xC. A text entry is 16 bits: tile number in bits 0-9, mirroring in
//! bits 10 (horizontal) and 11 (vertical), and the palette number in bits
//! 12-15. Text screens wider or taller than 32 tiles are stored as panels of
//! 32 x 32 tiles, panel after panel from left to right and top to bottom, each
//! in rows. An affine entry is one byte, a tile number, and an affine extended
//! entry is a 16-bit text entry in plain rows.
//!
//! The palette number selects 16 colors of 4 bit tiles, or a 256-color
//! extended palette of 8 bit tiles (palette number times 16 or 256, as
//! NitroPaint indexes it). A tile the character data does not have is drawn
//! as color 0.

// Parts of this file follow NitroPaint (https://github.com/Garhoogin/NitroPaint):
//
// BSD 2-Clause License
//
// Copyright (c) 2020, Garhoogin
// All rights reserved.
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are met:
//
// 1. Redistributions of source code must retain the above copyright notice, this
//    list of conditions and the following disclaimer.
//
// 2. Redistributions in binary form must reproduce the above copyright notice,
//    this list of conditions and the following disclaimer in the documentation
//    and/or other materials provided with the distribution.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
// AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
// DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
// FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
// DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
// SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
// CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
// OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
// OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

use alloc::vec::Vec;

use super::character::Character;
use super::{Palette, pixel_color, section};
use crate::bytes::{le16, le32};
use crate::image::check_size;
use crate::{DecodeError, Image};

/// Tiles to a side of a panel of a text screen.
const PANEL: usize = 32;
const TILE: usize = 8;

/// A screen: its size in tiles and one entry per tile, in rows.
pub(super) struct Screen {
    width: usize,
    height: usize,
    entries: Vec<u16>,
}

impl Screen {
    pub(super) fn parse(data: &[u8]) -> Option<Self> {
        let body = section(data, b"RCSN", b"NRCS")?;
        let width = usize::from(le16(body, 0)?) / TILE;
        let height = usize::from(le16(body, 2)?) / TILE;
        let format = le16(body, 6)?;
        let size = le32(body, 8)? as usize;
        let payload = body.get(0xc..size.checked_add(0xc)?)?;
        let tiles = width * height;
        if tiles == 0 {
            return None;
        }
        let words = || {
            let words = payload.get(..tiles * 2)?;
            Some(
                words
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|&w| u16::from_le_bytes(w)),
            )
        };
        let entries = match format {
            0 => unpanel(words()?, width, height),
            1 => payload
                .get(..tiles)?
                .iter()
                .map(|&b| u16::from(b))
                .collect(),
            2 => words()?.collect(),
            _ => return None,
        };
        Some(Self {
            width,
            height,
            entries,
        })
    }

    /// The screen drawn with `character` and `palette`. Fails if the
    /// character data is a bitmap.
    pub(super) fn draw(
        &self,
        character: &Character,
        palette: Option<&Palette>,
    ) -> Result<Image, DecodeError> {
        let tiles = character.tiles().ok_or(DecodeError::Unrecognized)?;
        let count = character.tile_count();
        let (width, height) = (self.width * TILE, self.height * TILE);
        check_size(width, height)?;
        let bits = character.bits;
        let mut colors = alloc::vec![0; width * height];
        for (cell, &entry) in self.entries.iter().enumerate() {
            let tile = usize::from(entry & 0x3ff);
            let (flip_x, flip_y) = (entry & 0x400 != 0, entry & 0x800 != 0);
            let number = usize::from(entry >> 12);
            let (left, top) = (cell % self.width * TILE, cell / self.width * TILE);
            for row in 0..TILE {
                let out = &mut colors[(top + row) * width + left..][..TILE];
                if tile >= count {
                    out.fill(pixel_color(palette, bits, 0, 0));
                    continue;
                }
                let source = tiles.row(tile, if flip_y { TILE - 1 - row } else { row });
                for (x, color) in out.iter_mut().enumerate() {
                    let index = source[if flip_x { TILE - 1 - x } else { x }];
                    *color = pixel_color(palette, bits, number, index);
                }
            }
        }
        Ok(Image::from_colors(
            width as u32,
            height as u32,
            colors.into_iter(),
        ))
    }
}

/// The entries of a text screen stored in panels as one list in rows.
/// `stored` holds `width * height` entries.
fn unpanel(stored: impl Iterator<Item = u16>, width: usize, height: usize) -> Vec<u16> {
    let mut entries = alloc::vec![0; width * height];
    let mut stored = stored;
    for panel_y in (0..height).step_by(PANEL) {
        for panel_x in (0..width).step_by(PANEL) {
            for y in panel_y..height.min(panel_y + PANEL) {
                for x in panel_x..width.min(panel_x + PANEL) {
                    entries[y * width + x] = stored.next().unwrap_or(0);
                }
            }
        }
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::super::testing::{ncgr, nclr, nscr};
    use super::*;

    fn words(entries: &[u16]) -> Vec<u8> {
        entries.iter().flat_map(|e| e.to_le_bytes()).collect()
    }

    /// Two 8 bit tiles: tile 0 blank; tile 1 has value 1 at the top left
    /// and value 2 at the top right.
    fn tiles() -> Vec<u8> {
        let mut data = alloc::vec![0u8; 128];
        data[64] = 1;
        data[64 + 7] = 2;
        data
    }

    #[test]
    fn text_entries_pick_tile_mirroring_and_palette() {
        let characters = ncgr(4, (1, 2), 0, 0, &tiles());
        let character = Character::parse(&characters).unwrap();
        // 2 x 2 tiles: tile 1; tile 1 mirrored horizontally; tile 1
        // mirrored vertically; a tile that does not exist.
        let entries = words(&[1, 1 | 0x400, 1 | 0x800, 9]);
        let screen = Screen::parse(&nscr(16, 16, 0, &entries)).unwrap();
        let image = screen.draw(&character, None).unwrap();
        assert_eq!((image.width(), image.height()), (16, 16));
        let gray = |v: u32| v * 0x01_0101;
        assert_eq!((image.get(0, 0), image.get(7, 0)), (gray(1), gray(2)));
        assert_eq!((image.get(8, 0), image.get(15, 0)), (gray(2), gray(1)));
        assert_eq!((image.get(0, 15), image.get(7, 15)), (gray(1), gray(2)));
        assert_eq!(image.get(8, 8), 0);
    }

    #[test]
    fn the_palette_number_offsets_into_the_colors() {
        // 4 bit tile of value 1 everywhere, in palette numbers 0 and 2.
        let characters = ncgr(3, (1, 1), 0, 0, &[0x11; 32]);
        let character = Character::parse(&characters).unwrap();
        let mut colors = alloc::vec![0u16; 48];
        colors[1] = 0x001f; // palette 0, color 1: red
        colors[33] = 0x03e0; // palette 2, color 1: green
        colors[32] = 0x7c00; // palette 2 color 0, never used: color 0 is palette 0's
        let palette = Palette::parse(&nclr(3, &colors)).unwrap();
        let first = Screen::parse(&nscr(8, 8, 0, &words(&[0]))).unwrap();
        let third = Screen::parse(&nscr(8, 8, 0, &words(&[2 << 12]))).unwrap();
        assert_eq!(
            first.draw(&character, Some(&palette)).unwrap().get(0, 0),
            0xff0000
        );
        assert_eq!(
            third.draw(&character, Some(&palette)).unwrap().get(0, 0),
            0x00ff00
        );
    }

    #[test]
    fn wide_text_screens_are_stored_in_panels_of_32_tiles() {
        // 40 x 2 tiles: panel 0 holds 32 x 2 entries, panel 1 holds 8 x 2.
        let stored: Vec<u16> = (0..80).collect();
        let screen = Screen::parse(&nscr(320, 16, 0, &words(&stored))).unwrap();
        assert_eq!(screen.entries[31], 31);
        assert_eq!(screen.entries[32], 64); // second panel, first row
        assert_eq!(screen.entries[40 + 32], 72); // second row of the second panel
        assert_eq!(screen.entries[40], 32); // second row of the first panel
        // Affine extended entries are plain rows.
        let plain = Screen::parse(&nscr(320, 16, 2, &words(&stored))).unwrap();
        assert_eq!(plain.entries[32], 32);
    }

    #[test]
    fn affine_entries_are_bytes() {
        let screen = Screen::parse(&nscr(16, 8, 1, &[1, 0])).unwrap();
        assert_eq!(screen.entries, [1, 0]);
    }

    #[test]
    fn a_screen_must_hold_all_its_entries() {
        assert!(Screen::parse(&nscr(16, 16, 0, &words(&[1, 2, 3]))).is_none());
        assert!(Screen::parse(&nscr(16, 16, 1, &[1, 2, 3])).is_none());
        assert!(Screen::parse(&nscr(0, 16, 0, &[])).is_none());
        assert!(Screen::parse(&nscr(16, 16, 7, &[0; 8])).is_none());
        // Bitmaps have no tiles to draw from.
        let bitmap = ncgr(4, (1, 1), 0, 1, &[0; 64]);
        let screen = Screen::parse(&nscr(8, 8, 0, &words(&[0]))).unwrap();
        assert!(
            screen
                .draw(&Character::parse(&bitmap).unwrap(), None)
                .is_err()
        );
    }
}
