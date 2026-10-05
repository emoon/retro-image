//! Drawing a NES nametable: the part `.nss` sessions and `.nam` files share.
//!
//! Sources: nametable, attribute and pattern layout from the nesdev wiki,
//! <https://www.nesdev.org/wiki/PPU_nametables>,
//! <https://www.nesdev.org/wiki/PPU_attribute_tables>,
//! <https://www.nesdev.org/wiki/PPU_pattern_tables>.

use alloc::vec::Vec;

use super::{PATTERN, PATTERN_TABLE_LEN};
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
    /// The `0xRRGGBB` value of each colour number `$00-$3F`.
    pub master: &'a [u32; 64],
}

impl Nametable<'_> {
    pub(super) fn draw(&self) -> Result<Image, DecodeError> {
        let (width, height) = (self.width * PATTERN.width, self.height * PATTERN.height);
        check_size(width, height)?;
        let attribute_columns = self.width.div_ceil(4);
        let mut colors = Vec::with_capacity(width * height);
        let pattern = PATTERN.unpack(&self.pattern[..PATTERN_TABLE_LEN]);
        for y in 0..height {
            let (ty, row) = (y / PATTERN.height, y % PATTERN.height);
            for tx in 0..self.width {
                let tile = usize::from(self.names[ty * self.width + tx]);
                let attribute = self.attributes[ty / 4 * attribute_columns + tx / 4];
                let shift = (ty / 2 % 2 * 2 + tx / 2 % 2) * 2;
                let subpalette = usize::from(attribute >> shift & 3);
                colors.extend(pattern.row(tile, row).iter().map(|&value| {
                    let color = if value == 0 {
                        self.palette[0]
                    } else {
                        self.palette[subpalette * 4 + usize::from(value)]
                    };
                    self.master[usize::from(color & 0x3f)]
                }));
            }
        }
        Ok(Image::from_colors(
            width as u32,
            height as u32,
            colors.into_iter(),
        ))
    }
}
