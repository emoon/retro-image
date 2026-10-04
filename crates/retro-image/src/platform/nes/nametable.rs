//! Drawing a NES nametable: the part `.nss` sessions and `.nam` files share.
//!
//! Sources: nametable, attribute and pattern layout from the nesdev wiki,
//! <https://www.nesdev.org/wiki/PPU_nametables>,
//! <https://www.nesdev.org/wiki/PPU_attribute_tables>,
//! <https://www.nesdev.org/wiki/PPU_pattern_tables>.

use super::{MASTER_PALETTE, tile_pixel};
use crate::image::check_size;
use crate::{DecodeError, Image};

/// A nametable of `width` x `height` tiles and what it is drawn with.
pub(super) struct Nametable<'a> {
    pub width: usize,
    pub height: usize,
    /// Pattern table, at least 4 KiB; the first 4 KiB are used.
    pub pattern: &'a [u8],
    /// One tile number per byte, `width * height` of them.
    pub names: &'a [u8],
    /// One byte per 4x4 tiles, as a raster of `ceil(width / 4)` by
    /// `ceil(height / 4)` bytes; two bits per 2x2 tiles, top-left lowest.
    pub attributes: &'a [u8],
    /// Colour numbers `$00-$3F` of the four background palettes: the shared
    /// background colour, then three colours for each of the four.
    pub palette: &'a [u8],
}

impl Nametable<'_> {
    pub(super) fn draw(&self) -> Result<Image, DecodeError> {
        let (width, height) = (self.width * 8, self.height * 8);
        check_size(width, height)?;
        let attribute_columns = self.width.div_ceil(4);
        let colors = (0..width * height).map(|i| {
            let (x, y) = (i % width, i / width);
            let (tx, ty) = (x / 8, y / 8);
            let value = tile_pixel(
                self.pattern,
                usize::from(self.names[ty * self.width + tx]),
                x % 8,
                y % 8,
            );
            let attribute = self.attributes[ty / 4 * attribute_columns + tx / 4];
            let shift = (ty / 2 % 2 * 2 + tx / 2 % 2) * 2;
            let subpalette = usize::from(attribute >> shift & 3);
            let color = if value == 0 {
                self.palette[0]
            } else {
                self.palette[subpalette * 4 + usize::from(value)]
            };
            MASTER_PALETTE[usize::from(color & 0x3f)]
        });
        Ok(Image::from_colors(width as u32, height as u32, colors))
    }
}
