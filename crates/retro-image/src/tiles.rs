//! Fixed-size tiles stored as bit planes or packed pixels, and sheets of them.
//!
//! Layouts follow, per hardware:
//! - NES pattern tables: <https://www.nesdev.org/wiki/PPU_pattern_tables>
//! - Game Boy tile data: <https://gbdev.io/pandocs/Tile_Data.html> (CC0)
//! - SNES tiles: <https://snes.nesdev.org/wiki/Tiles> (CC0)
//! - Master System tiles: <https://www.smspower.org/Development/Tiles>
//! - WonderSwan tile data: <https://ws.nesdev.org/wiki/Display/Tile_Data> (CC0)
//! - GBA and DS character data: GBATEK, "LCD VRAM Character Data"
//!   (<https://problemkaputt.de/gbatek.htm>, facts only)
//!
//! Those pages are only the sources of the layouts named below as examples.
//! This file holds no format knowledge of its own.
//!
//! A [`TileLayout`] says how the bytes of one tile encode its pixels, and one
//! decode path serves every layout. A tile is a block of `width` x `height`
//! pixels of `bpp` bits each. Its bytes are cut into planes, `bpp / plane_bits`
//! of them, and plane 0 holds the lowest bits of the color number:
//! - `plane_bits` is 1 for bit planes (NES, Game Boy, SNES, Master System)
//!   and `bpp` for packed pixels, where one byte carries 8 / `bpp` of them
//!   (Mega Drive, GBA and DS).
//! - `interleave` is how many planes are stored together row by row. With 1
//!   every plane is a block of its own (NES). With 2 the planes come in pairs
//!   (Game Boy, SNES, PC Engine) and each pair is a block of its own. With
//!   `bpp` every row holds all its planes (Master System). A last group of
//!   fewer planes is interleaved among itself (SNES 3 bpp).
//! - `order` says whether the leftmost pixel of a byte sits in its most or
//!   least significant bits (the same [`BitOrder`] as 1-bit bitmaps). GBA
//!   packed pixels have the left pixel in the low nibble.
//!
//! A layout is a constant next to the decoder that uses it:
//! `const TILE: TileLayout = TileLayout::planar(2, 2);`. Adding one touches
//! no code in this file. The layouts on the list for future decoders are
//! constants too:
//! - rgbgfx `.2bpp` and SNES 2 bpp: planar, 2 bpp, interleave 2; rgbgfx
//!   `.1bpp`: planar, 1 bpp
//! - SNES 3, 4 and 8 bpp, PC Engine background tiles: planar, interleave 2
//! - Master System and WonderSwan 4 bpp planar: planar, 4 bpp, interleave 4
//! - GBA and DS 4 bpp: packed, 4 bpp, least significant first; 8 bpp (and
//!   SNES Mode 7): packed, 8 bpp
//! - Mega Drive and WonderSwan 4 bpp packed: packed, 4 bpp, most
//!   significant first
//!
//! Tiles of another size (PC Engine sprites are 16 x 16) override `width` and
//! `height` of the closest constructor, e.g.
//! `TileLayout { width: 16, height: 16, ..TileLayout::planar(4, 1) }`; a
//! plane row is then `width / 8` bytes.
//!
//! A tile can also be put together from smaller blocks that are stored one
//! after another, each block laid out like a tile of its own size.
//! [`TileLayout::in_blocks`] gives the block size and the position of each
//! block in storage order. The Neo Geo's 16 x 16 sprite is four 8 x 8 blocks
//! (top right, bottom right, top left, bottom left), and its 8 x 8 fix tile
//! is four 2 x 8 columns. A layout that needs more than that (Neo Geo Pocket
//! stores a row as a little-endian 16-bit word) calls for a new field here
//! first.
//!
//! [`TileLayout::sheet`] draws the tiles of a data block as a picture, a
//! given number to a row, through a palette into an [`Image`].
//! [`TileLayout::unpack`] decodes them to one byte per pixel for decoders
//! that place tiles themselves (the NES nametable).
//!
//! This is not `image::planar_pixels`, which decodes the bit planes of a
//! whole picture row by row (Amiga, Atari ST, PC EGA). Here the unit is a
//! small tile that is addressed by number.

use alloc::vec::Vec;

use crate::image::check_size;
use crate::{BitOrder, DecodeError, Image};

/// How the bytes of one tile encode its pixels. See the module header for
/// the meaning of the fields; build one with [`TileLayout::planar`] or
/// [`TileLayout::packed`].
#[derive(Debug, Clone, Copy)]
pub(crate) struct TileLayout {
    /// Pixels per row of a tile. `width * plane_bits` must be a multiple of 8.
    pub width: usize,
    /// Rows of a tile.
    pub height: usize,
    /// Bits per pixel, 1 to 8. A multiple of `plane_bits`.
    pub bpp: usize,
    /// Bits of a color number stored together in a plane: 1, 2, 4 or 8.
    pub plane_bits: usize,
    /// Planes stored together row by row, at least 1.
    pub interleave: usize,
    /// Which end of a byte holds its leftmost pixel.
    pub order: BitOrder,
    /// How the tile is put together from blocks, if it is not stored in one
    /// piece. The fields above then describe each block (all but its size).
    pub blocks: Option<Blocks>,
}

/// The blocks a tile is put together from, see [`TileLayout::in_blocks`].
#[derive(Debug, Clone, Copy)]
pub(crate) struct Blocks {
    /// Pixels per row of a block.
    pub width: usize,
    /// Rows of a block.
    pub height: usize,
    /// The place of each block, in storage order, as (column, row) counted
    /// in blocks from the top left of the tile.
    pub order: &'static [(usize, usize)],
}

/// ORs the pixels of the bytes of `source`, `N` to a byte, into `row`, moved
/// up by `up` bits. `shifts[i]` is where pixel `i` of a byte sits in it.
fn merge<const N: usize>(row: &mut [u8], source: &[u8], shifts: &[u8; 8], mask: u8, up: usize) {
    for (pixels, &byte) in row.as_chunks_mut::<N>().0.iter_mut().zip(source) {
        for (value, &shift) in pixels.iter_mut().zip(shifts) {
            *value |= (byte >> shift & mask) << up;
        }
    }
}

/// Tiles decoded to one color number per byte, by [`TileLayout::unpack`].
pub(crate) struct Unpacked {
    width: usize,
    height: usize,
    /// Tile after tile, each row by row.
    pixels: Vec<u8>,
}

impl Unpacked {
    /// Row `y` of tile number `tile`. Panics if either is out of range.
    pub(crate) fn row(&self, tile: usize, y: usize) -> &[u8] {
        assert!(y < self.height);
        &self.pixels[(tile * self.height + y) * self.width..][..self.width]
    }

    fn count(&self) -> usize {
        self.pixels.len() / (self.width * self.height)
    }
}

impl TileLayout {
    /// 8 x 8 tiles of `bpp` bit planes, leftmost pixel in bit 7, with
    /// `interleave` planes stored together row by row.
    pub(crate) const fn planar(bpp: usize, interleave: usize) -> Self {
        assert!(bpp != 0 && bpp <= 8 && interleave != 0);
        Self {
            width: 8,
            height: 8,
            bpp,
            plane_bits: 1,
            interleave,
            order: BitOrder::MsbFirst,
            blocks: None,
        }
    }

    /// 8 x 8 tiles of packed pixels of `bpp` bits (1, 2, 4 or 8) each, the
    /// leftmost pixel in the end of the byte that `order` names.
    pub(crate) const fn packed(bpp: usize, order: BitOrder) -> Self {
        assert!(matches!(bpp, 1 | 2 | 4 | 8));
        Self {
            width: 8,
            height: 8,
            bpp,
            plane_bits: bpp,
            interleave: 1,
            order,
            blocks: None,
        }
    }

    /// The same layout for a tile put together from blocks of `width` x
    /// `height` pixels, which fill the tile exactly. `order` gives the place
    /// of each block, in storage order, as (column, row) counted in blocks.
    /// Planes, packing and bit order of `self` then describe each block, not
    /// the tile.
    pub(crate) const fn in_blocks(
        self,
        width: usize,
        height: usize,
        order: &'static [(usize, usize)],
    ) -> Self {
        assert!(width != 0 && height != 0);
        let (columns, rows) = (self.width / width, self.height / height);
        assert!(columns * width == self.width && rows * height == self.height);
        assert!(order.len() == columns * rows);
        let mut seen = 0;
        while seen < order.len() {
            assert!(order[seen].0 < columns && order[seen].1 < rows);
            let mut other = 0;
            while other < seen {
                assert!(order[other].0 != order[seen].0 || order[other].1 != order[seen].1);
                other += 1;
            }
            seen += 1;
        }
        Self {
            blocks: Some(Blocks {
                width,
                height,
                order,
            }),
            ..self
        }
    }

    /// Bytes of one tile.
    pub(crate) const fn tile_len(&self) -> usize {
        self.width * self.height * self.bpp / 8
    }

    /// Decodes every tile of `tiles`, which holds one tile after another. A
    /// trailing partial tile is ignored.
    pub(crate) fn unpack(&self, tiles: &[u8]) -> Unpacked {
        match self.blocks {
            None => self.unpack_whole(tiles),
            Some(blocks) => self.unpack_blocks(tiles, blocks),
        }
    }

    /// [`Self::unpack`] for a tile that is stored in one piece.
    fn unpack_whole(&self, tiles: &[u8]) -> Unpacked {
        let (len, size) = (self.tile_len(), self.width * self.height);
        let planes = self.bpp / self.plane_bits;
        let plane_row_len = self.width * self.plane_bits / 8;
        let mask = ((1u16 << self.plane_bits) - 1) as u8;
        // Where each pixel of a byte sits in it, leftmost pixel first.
        let per_byte = 8 / self.plane_bits;
        let shifts: [u8; 8] = core::array::from_fn(|i| {
            let slot = i % per_byte * self.plane_bits;
            match self.order {
                BitOrder::MsbFirst => (8 - self.plane_bits - slot) as u8,
                BitOrder::LsbFirst => slot as u8,
            }
        });
        let mut pixels = alloc::vec![0; tiles.len() / len * size];
        for (bytes, tile_pixels) in tiles.chunks_exact(len).zip(pixels.chunks_exact_mut(size)) {
            for (y, row) in tile_pixels.chunks_exact_mut(self.width).enumerate() {
                let mut plane = 0;
                for first in (0..planes).step_by(self.interleave) {
                    let in_group = self.interleave.min(planes - first);
                    for member in 0..in_group {
                        // Plane rows before this one: the groups before its
                        // group, then `y` rows of its group, each holding
                        // all its planes.
                        let start = first * self.height + y * in_group + member;
                        let source = &bytes[start * plane_row_len..][..plane_row_len];
                        let up = plane * self.plane_bits;
                        match self.plane_bits {
                            1 => merge::<8>(row, source, &shifts, mask, up),
                            2 => merge::<4>(row, source, &shifts, mask, up),
                            4 => merge::<2>(row, source, &shifts, mask, up),
                            _ => merge::<1>(row, source, &shifts, mask, up),
                        }
                        plane += 1;
                    }
                }
            }
        }
        Unpacked {
            width: self.width,
            height: self.height,
            pixels,
        }
    }

    /// [`Self::unpack`] for a tile put together from blocks: the blocks of
    /// every tile are decoded as tiles of their own size, then placed.
    fn unpack_blocks(&self, tiles: &[u8], blocks: Blocks) -> Unpacked {
        let whole = tiles.len() / self.tile_len() * self.tile_len();
        let block_layout = Self {
            width: blocks.width,
            height: blocks.height,
            blocks: None,
            ..*self
        };
        let decoded = block_layout.unpack_whole(&tiles[..whole]);
        let per_tile = blocks.order.len();
        let mut pixels = alloc::vec![0; whole / self.tile_len() * self.width * self.height];
        for (tile, tile_pixels) in pixels
            .chunks_exact_mut(self.width * self.height)
            .enumerate()
        {
            for (number, &(column, row)) in blocks.order.iter().enumerate() {
                for y in 0..blocks.height {
                    let at = (row * blocks.height + y) * self.width + column * blocks.width;
                    tile_pixels[at..][..blocks.width]
                        .copy_from_slice(decoded.row(tile * per_tile + number, y));
                }
            }
        }
        Unpacked {
            width: self.width,
            height: self.height,
            pixels,
        }
    }

    /// The tiles of `tiles` as a picture, `per_row` to a row in order, with
    /// color numbers mapped through `palette`. A last row that the tiles do
    /// not fill is color 0.
    ///
    /// Fails if `tiles` is not a whole number of tiles, is empty, `per_row`
    /// is 0, the picture would exceed the size limit, or `palette` has fewer
    /// colors than the layout uses.
    pub(crate) fn sheet(
        &self,
        tiles: &[u8],
        per_row: usize,
        palette: &[u32],
    ) -> Result<Image, DecodeError> {
        let (width, height, indices) = self.sheet_indices(tiles, per_row)?;
        Image::from_indexed(width, height, &indices, palette)
    }

    /// [`Self::sheet`] for a palette of straight `0xAARRGGBB` colors.
    pub(crate) fn sheet_argb(
        &self,
        tiles: &[u8],
        per_row: usize,
        palette: &[u32],
    ) -> Result<Image, DecodeError> {
        let (width, height, indices) = self.sheet_indices(tiles, per_row)?;
        Image::from_indexed_argb(width, height, &indices, palette)
    }

    /// The color numbers of the sheet of `tiles`, with its width and height.
    fn sheet_indices(
        &self,
        tiles: &[u8],
        per_row: usize,
    ) -> Result<(u32, u32, Vec<u8>), DecodeError> {
        if per_row == 0 || !tiles.len().is_multiple_of(self.tile_len()) {
            return Err(DecodeError::Unrecognized);
        }
        let count = tiles.len() / self.tile_len();
        let width = per_row * self.width;
        let height = count.div_ceil(per_row) * self.height;
        check_size(width, height)?;
        let unpacked = self.unpack(tiles);
        let mut indices = alloc::vec![0; width * height];
        for tile in 0..unpacked.count() {
            let (left, top) = (tile % per_row * self.width, tile / per_row * self.height);
            for y in 0..self.height {
                indices[(top + y) * width + left..][..self.width]
                    .copy_from_slice(unpacked.row(tile, y));
            }
        }
        Ok((width as u32, height as u32, indices))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Color numbers of one row of a tile.
    fn decode_row(layout: &TileLayout, tiles: &[u8], tile: usize, y: usize) -> Vec<u8> {
        layout.unpack(tiles).row(tile, y).to_vec()
    }

    #[test]
    fn nes_planes_are_two_blocks_of_eight_rows() {
        let nes = TileLayout::planar(2, 1);
        let mut tile = [0u8; 16];
        tile[0] = 0b1000_0001; // plane 0, row 0
        tile[8] = 0b0000_0001; // plane 1, row 0
        tile[9] = 0b1000_0000; // plane 1, row 1
        assert_eq!(nes.tile_len(), 16);
        assert_eq!(decode_row(&nes, &tile, 0, 0), [1, 0, 0, 0, 0, 0, 0, 3]);
        assert_eq!(decode_row(&nes, &tile, 0, 1), [2, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn game_boy_rows_interleave_the_two_planes() {
        let game_boy = TileLayout::planar(2, 2);
        let mut tile = [0u8; 16];
        tile[0] = 0b1000_0001; // row 0, plane 0
        tile[1] = 0b0000_0001; // row 0, plane 1
        tile[2] = 0b0100_0000; // row 1, plane 0
        tile[15] = 0b1000_0000; // row 7, plane 1
        assert_eq!(decode_row(&game_boy, &tile, 0, 0), [1, 0, 0, 0, 0, 0, 0, 3]);
        assert_eq!(decode_row(&game_boy, &tile, 0, 1), [0, 1, 0, 0, 0, 0, 0, 0]);
        assert_eq!(decode_row(&game_boy, &tile, 0, 7), [2, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn unpack_ignores_a_trailing_partial_tile() {
        let nes = TileLayout::planar(2, 1);
        assert_eq!(nes.unpack(&[0; 40]).count(), 2);
        assert_eq!(nes.unpack(&[0; 15]).count(), 0);
    }

    #[test]
    fn tiles_are_numbered_in_order() {
        let nes = TileLayout::planar(2, 1);
        let mut tiles = [0u8; 32];
        tiles[16] = 0x80; // tile 1, plane 0, row 0
        assert_eq!(decode_row(&nes, &tiles, 0, 0)[0], 0);
        assert_eq!(decode_row(&nes, &tiles, 1, 0)[0], 1);
    }

    #[test]
    fn planned_planar_layouts_place_their_planes() {
        // The tile has only the first pixel of row 2 set, in every plane.
        let first_pixel = |layout: TileLayout, at: &[usize]| {
            let mut tile = alloc::vec![0u8; layout.tile_len()];
            for &i in at {
                tile[i] = 0x80;
            }
            decode_row(&layout, &tile, 0, 2)[0]
        };
        // SNES 4 bpp: planes 0 and 1 interleaved, then planes 2 and 3.
        let snes = TileLayout::planar(4, 2);
        assert_eq!(first_pixel(snes, &[4]), 1);
        assert_eq!(first_pixel(snes, &[5]), 2);
        assert_eq!(first_pixel(snes, &[16 + 4]), 4);
        assert_eq!(first_pixel(snes, &[16 + 5]), 8);
        // SNES 3 bpp: plane 2 is a plain 8-byte plane after the pair.
        assert_eq!(first_pixel(TileLayout::planar(3, 2), &[16 + 2]), 4);
        // SNES 8 bpp: four pairs.
        assert_eq!(first_pixel(TileLayout::planar(8, 2), &[48 + 5]), 128);
        // Master System: one byte per plane in each row of 4.
        let master_system = TileLayout::planar(4, 4);
        assert_eq!(first_pixel(master_system, &[8 + 3]), 8);
        assert_eq!(first_pixel(master_system, &[8 + 1, 8 + 2]), 2 | 4);
        // rgbgfx `.1bpp`: one byte per row.
        assert_eq!(first_pixel(TileLayout::planar(1, 1), &[2]), 1);
    }

    #[test]
    fn packed_pixels_follow_the_pixel_order() {
        let mut tile = [0u8; 32];
        tile[0] = 0x21;
        tile[5] = 0x43; // row 1 starts after 4 bytes
        let high_first = TileLayout::packed(4, BitOrder::MsbFirst);
        assert_eq!(high_first.tile_len(), 32);
        assert_eq!(
            decode_row(&high_first, &tile, 0, 0),
            [2, 1, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(
            decode_row(&high_first, &tile, 0, 1),
            [0, 0, 4, 3, 0, 0, 0, 0]
        );
        let low_first = TileLayout::packed(4, BitOrder::LsbFirst);
        assert_eq!(
            decode_row(&low_first, &tile, 0, 0),
            [1, 2, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(
            decode_row(&low_first, &tile, 0, 1),
            [0, 0, 3, 4, 0, 0, 0, 0]
        );
    }

    #[test]
    fn packed_two_and_eight_bit_pixels() {
        let mut tile = [0u8; 64];
        tile[1] = 0b1110_0100; // 2 bpp row 0, pixels 4 to 7
        let two = TileLayout::packed(2, BitOrder::MsbFirst);
        assert_eq!(two.tile_len(), 16);
        assert_eq!(decode_row(&two, &tile, 0, 0), [0, 0, 0, 0, 3, 2, 1, 0]);
        // 8 bpp is one byte per pixel; 8 bytes per row.
        tile[8 + 3] = 200;
        let linear = TileLayout::packed(8, BitOrder::LsbFirst);
        assert_eq!(linear.tile_len(), 64);
        assert_eq!(decode_row(&linear, &tile, 0, 1)[3], 200);
    }

    #[test]
    fn wide_tiles_have_two_byte_plane_rows() {
        // PC Engine sprite: 16 x 16, four planes of 16 words each.
        let sprite = TileLayout {
            width: 16,
            height: 16,
            ..TileLayout::planar(4, 1)
        };
        assert_eq!(sprite.tile_len(), 128);
        let mut tile = [0u8; 128];
        tile[32 + 2 * 3 + 1] = 0b0000_0001; // plane 1, row 3, pixel 15
        let row = decode_row(&sprite, &tile, 0, 3);
        assert_eq!((row[15], row[7], row[0]), (2, 0, 0));
    }

    #[test]
    fn blocks_are_placed_in_storage_order() {
        // Four 8 x 8 blocks: top right, bottom right, top left, bottom left.
        let quadrants = TileLayout {
            width: 16,
            height: 16,
            ..TileLayout::planar(1, 1)
        }
        .in_blocks(8, 8, &[(1, 0), (1, 1), (0, 0), (0, 1)]);
        assert_eq!(quadrants.tile_len(), 32);
        let mut tiles = [0u8; 64];
        tiles[8 + 2] = 0x80; // block 1, row 2, leftmost pixel
        tiles[16 + 7] = 0x01; // block 2, row 7, rightmost pixel
        tiles[32 + 8] = 0x80; // the second tile, block 1, row 0
        let unpacked = quadrants.unpack(&tiles);
        assert_eq!(unpacked.count(), 2);
        assert_eq!(unpacked.row(0, 10)[8], 1);
        assert_eq!(unpacked.row(0, 7)[7], 1);
        assert_eq!(unpacked.row(0, 7).iter().sum::<u8>(), 1);
        assert_eq!(unpacked.row(1, 8)[8], 1);
        assert_eq!(unpacked.row(1, 8).iter().sum::<u8>(), 1);
    }

    #[test]
    fn blocks_need_not_be_square() {
        // An 8 x 8 tile of four columns 2 pixels wide, stored right to left.
        let columns = TileLayout::packed(4, BitOrder::LsbFirst).in_blocks(
            2,
            8,
            &[(2, 0), (3, 0), (0, 0), (1, 0)],
        );
        let mut tile = [0u8; 32];
        tile[0] = 0x21; // first column: pixels 4 and 5 of row 0
        tile[8 + 1] = 0x43; // second column: pixels 6 and 7 of row 1
        tile[24 + 7] = 0x05; // last column: pixels 2 and 3 of row 7
        assert_eq!(decode_row(&columns, &tile, 0, 0), [0, 0, 0, 0, 1, 2, 0, 0]);
        assert_eq!(decode_row(&columns, &tile, 0, 1), [0, 0, 0, 0, 0, 0, 3, 4]);
        assert_eq!(decode_row(&columns, &tile, 0, 7), [0, 0, 5, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn sheet_places_tiles_row_by_row_and_pads_the_last_row() {
        let nes = TileLayout::planar(2, 1);
        let mut tiles = [0u8; 48];
        tiles[0] = 0x80; // tile 0: color 1 at the top left
        tiles[16 + 8] = 0x01; // tile 1: color 2 at the top right of the tile
        tiles[32] = 0x80; // tile 2: color 1 at the top left
        let image = nes.sheet(&tiles, 2, &[0, 0xff, 0xff00, 0xff0000]).unwrap();
        assert_eq!((image.width(), image.height()), (16, 16));
        assert_eq!(image.get(0, 0), 0xff);
        assert_eq!(image.get(15, 0), 0xff00);
        assert_eq!(image.get(0, 8), 0xff);
        // The fourth tile does not exist.
        assert_eq!(image.get(8, 8), 0);
    }

    #[test]
    fn sheet_rejects_what_is_not_whole_tiles() {
        let nes = TileLayout::planar(2, 1);
        let palette = [0; 4];
        assert!(nes.sheet(&[0; 16], 1, &palette).is_ok());
        assert!(nes.sheet(&[0; 15], 1, &palette).is_err());
        assert!(nes.sheet(&[0; 17], 1, &palette).is_err());
        assert!(nes.sheet(&[], 1, &palette).is_err());
        assert!(nes.sheet(&[0; 16], 0, &palette).is_err());
        // A palette too short for 2 bpp.
        assert!(nes.sheet(&[0xff; 16], 1, &[0; 3]).is_err());
    }
}
