//! NCGR and NCBR character data: the tiles (or bitmap) of a Nintendo DS
//! picture.
//!
//! Sources: NitroPaint (BSD 2-Clause, notice below),
//! `NitroPaint/object/NitroCharacter.c` (`ChrReadNcgr`, `ChrReadChars`,
//! `ChrReadBitmap`, `ChrGuessWidth`, `ChrWriteNcgr`), and GBATEK, "DS Files -
//! Video Character"; see `nitro.rs` for how the two differ. Unverified: no
//! sample file was available.
//!
//! The `RAHC` section holds, from the start of its data: the height and the
//! width of the picture in tiles (16 bits each, `0xFFFF` when not given), the
//! depth (3 = 4 bits a pixel, 4 = 8 bits), the OBJ VRAM mapping mode (0 for
//! 2D mapping), the kind (1 = bitmap, else tiles), the size of the graphics,
//! and their offset from the start of the section data (0x18).
//!
//! Tiles are 8 x 8 pixels, 32 bytes at 4 bits with the left pixel in the low
//! nibble, or 64 bytes at 8 bits. A bitmap is rows of pixels as wide as the
//! picture, nibbles likewise. A sheet of tiles is as wide as the file says
//! when it has 2D mapping and its width and height multiply to the number of
//! tiles in the data. Otherwise the width is not known: it is 32 tiles for a
//! count that is a multiple of 32 (a 256-pixel-wide sheet) and 16 for any
//! other, the shape of the other tile sheets of this crate. A bitmap whose size
//! is not given is rejected, since a guessed width would shear it.

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

use super::{Palette, first_palette, section};
use crate::bytes::{le16, le32};
use crate::image::check_size;
use crate::tiles::{TileLayout, Unpacked};
use crate::{BitOrder, DecodeError, Image};

/// A character file: the graphics and what says how to arrange them.
pub(super) struct Character<'a> {
    /// Bits a pixel, 4 or 8.
    pub bits: usize,
    bitmap: bool,
    /// Height and width of the picture in tiles, when the file gives them
    /// for 2D mapping.
    size: Option<(usize, usize)>,
    graphics: &'a [u8],
}

impl<'a> Character<'a> {
    pub(super) fn parse(data: &'a [u8]) -> Option<Self> {
        let body = section(data, b"RGCN", b"RAHC")?;
        let (tiles_y, tiles_x) = (le16(body, 0)?, le16(body, 2)?);
        let bits = match le32(body, 4)? {
            3 => 4,
            4 => 8,
            _ => return None,
        };
        let two_d = le32(body, 8)? == 0;
        let bitmap = le32(body, 0xc)? == 1;
        let size = le32(body, 0x10)? as usize;
        let offset = le32(body, 0x14)? as usize;
        let graphics = body.get(offset..offset.checked_add(size)?)?;
        let given =
            (two_d && tiles_x != 0 && tiles_y != 0 && tiles_x != 0xffff && tiles_y != 0xffff)
                .then_some((usize::from(tiles_y), usize::from(tiles_x)));
        let character = Self {
            bits,
            bitmap,
            size: given,
            graphics,
        };
        // The graphics must hold something to draw.
        (character.tile_count() > 0).then_some(character)
    }

    fn layout(&self) -> TileLayout {
        TileLayout::packed(self.bits, BitOrder::LsbFirst)
    }

    /// Whole tiles in the graphics.
    pub(super) fn tile_count(&self) -> usize {
        self.graphics.len() / self.layout().tile_len()
    }

    /// The tiles as color numbers, for a screen to draw from; `None` for a
    /// bitmap, which has no tiles to number.
    pub(super) fn tiles(&self) -> Option<Unpacked> {
        let len = self.tile_count() * self.layout().tile_len();
        (!self.bitmap).then(|| self.layout().unpack(&self.graphics[..len]))
    }

    /// The picture: a bitmap as stored, tiles as a sheet. Color numbers go
    /// through palette 0 of `palette`, or gray.
    pub(super) fn sheet(&self, palette: Option<&Palette>) -> Result<Image, DecodeError> {
        let colors = first_palette(palette, self.bits);
        let layout = self.layout();
        let count = self.tile_count();
        let matches_count = |&(y, x): &(usize, usize)| y * x == count;
        let known = self.size.filter(matches_count);
        if self.bitmap {
            let (tiles_y, tiles_x) = known.ok_or(DecodeError::Invalid)?;
            let (width, height) = (tiles_x * layout.width, tiles_y * layout.height);
            check_size(width, height)?;
            let indices: Vec<u8> = if self.bits == 8 {
                self.graphics[..width * height].to_vec()
            } else {
                let packed = &self.graphics[..width * height / 2];
                packed.iter().flat_map(|&b| [b & 15, b >> 4]).collect()
            };
            return Image::from_indexed(width as u32, height as u32, &indices, &colors);
        }
        let per_row = match known {
            Some((_, tiles_x)) => tiles_x,
            None if count.is_multiple_of(32) => 32,
            None => 16,
        };
        layout.sheet(
            &self.graphics[..count * layout.tile_len()],
            per_row,
            &colors,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::ncgr;
    use super::*;

    /// 4 bpp tile `n` with pixel 0 = `n`, pixel 1 = 15 (the rest 0).
    fn tile(n: u8) -> [u8; 32] {
        let mut tile = [0; 32];
        tile[0] = n | 15 << 4;
        tile
    }

    fn tiles(count: u8) -> Vec<u8> {
        (0..count).flat_map(tile).collect()
    }

    #[test]
    fn tiles_are_arranged_as_the_file_says_or_guessed() {
        // 2D mapping with a 2 x 3 tiles (height x width) picture.
        let sheet = ncgr(3, (2, 3), 0, 0, &tiles(6));
        let image = Character::parse(&sheet).unwrap().sheet(None).unwrap();
        assert_eq!((image.width(), image.height()), (24, 16));
        // Pixel 0 of tile 4 (second row, second column) is value 4, pixel 1 is 15.
        assert_eq!(image.get(8, 8), 4 * 0x11_1111);
        assert_eq!(image.get(9, 8), 15 * 0x11_1111);
        // Sizes not given (0xFFFF), or 1D mapping, are guessed.
        let unknown = ncgr(3, (0xffff, 0xffff), 0, 0, &tiles(6));
        assert_eq!(
            Character::parse(&unknown)
                .unwrap()
                .sheet(None)
                .unwrap()
                .width(),
            128
        );
        let linear = ncgr(3, (2, 3), 0x10, 0, &tiles(6));
        assert_eq!(
            Character::parse(&linear)
                .unwrap()
                .sheet(None)
                .unwrap()
                .width(),
            128
        );
        let wide = ncgr(3, (0xffff, 0xffff), 0, 0, &tiles(32));
        assert_eq!(
            Character::parse(&wide)
                .unwrap()
                .sheet(None)
                .unwrap()
                .width(),
            256
        );
        // A size that does not match the tile count is not trusted either.
        let wrong = ncgr(3, (2, 2), 0, 0, &tiles(6));
        assert_eq!(
            Character::parse(&wrong)
                .unwrap()
                .sheet(None)
                .unwrap()
                .width(),
            128
        );
    }

    #[test]
    fn eight_bit_tiles_hold_a_byte_a_pixel() {
        let mut data = alloc::vec![0u8; 64];
        data[9] = 200; // tile 0, row 1, pixel 1
        let file = ncgr(4, (1, 1), 0, 0, &data);
        let image = Character::parse(&file).unwrap().sheet(None).unwrap();
        assert_eq!((image.width(), image.height()), (8, 8));
        assert_eq!(image.get(1, 1), 200 * 0x01_0101);
    }

    #[test]
    fn a_bitmap_is_rows_as_wide_as_the_picture() {
        // 2 x 1 tiles of 4 bpp bitmap: 16 x 8 pixels, 8 bytes a row.
        let mut data = alloc::vec![0u8; 8 * 8];
        data[0] = 0x21; // row 0: pixel 0 = 1, pixel 1 = 2
        data[4] = 0x03; // row 0: pixel 8 = 3
        data[8] = 0x50; // row 1: pixel 0 = 0, pixel 1 = 5
        let file = ncgr(3, (1, 2), 0, 1, &data);
        let image = Character::parse(&file).unwrap().sheet(None).unwrap();
        assert_eq!((image.width(), image.height()), (16, 8));
        assert_eq!(image.get(0, 0), 0x11_1111);
        assert_eq!(image.get(1, 0), 0x22_2222);
        assert_eq!(image.get(8, 0), 0x33_3333);
        assert_eq!(image.get(1, 1), 0x55_5555);
        // Without a size a bitmap cannot be placed.
        let unsized_file = ncgr(3, (0xffff, 0xffff), 0, 1, &data);
        assert!(
            Character::parse(&unsized_file)
                .unwrap()
                .sheet(None)
                .is_err()
        );
    }

    #[test]
    fn graphics_must_exist_and_fit() {
        assert!(Character::parse(&ncgr(3, (1, 1), 0, 0, &[])).is_none());
        assert!(Character::parse(&ncgr(3, (1, 1), 0, 0, &[0; 31])).is_none());
        assert!(Character::parse(&ncgr(2, (1, 1), 0, 0, &[0; 32])).is_none());
        let file = ncgr(3, (1, 1), 0, 0, &[0; 32]);
        // The graphics size claims more than the file has.
        let mut short = file.clone();
        short[0x28] = 0xff;
        assert!(Character::parse(&short).is_none());
        // The graphics offset points past the section.
        let mut beyond = file;
        beyond[0x2c] = 0xff;
        assert!(Character::parse(&beyond).is_none());
    }
}
