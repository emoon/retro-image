//! Shared by the DOS clip-art libraries (Print Shop, PrintMaster,
//! PrintPartner): a library shows as one sheet of its pictures.
//!
//! This is the crate's own choice, not part of any format. The pictures sit
//! on a fixed grid, left to right, top to bottom, each in a cell as large as
//! the largest picture, with a gutter between cells. The grid is as close to
//! square as the whole columns allow, so a thumbnail of the sheet stays
//! readable. Empty space is the shared transparent-fill grey. All pictures
//! are 1-bit, black on white: the libraries store a set bit as black.
//!
//! A library of more than [`MAX_PICTURES`] pictures is rejected, not cut
//! short. The largest sample has 235. The cap also keeps the size-only
//! formats from claiming big files of other kinds: a ColoRIX picture named
//! `DATA.DAT` is 1629 pictures long and ends in zero fill.

use crate::image::{TRANSPARENT_FILL, check_size};
use crate::{BitOrder, DecodeError, Image};

/// Most pictures in one library.
pub(super) const MAX_PICTURES: usize = 256;
const GUTTER: usize = 4;

/// The sheet being laid out. [`Sheet::new`] sizes it from the picture count
/// and the largest picture, so a decoder first reads all picture headers.
pub(super) struct Sheet {
    image: Image,
    columns: usize,
    cell_width: usize,
    cell_height: usize,
}

impl Sheet {
    /// A sheet for `count` pictures of at most `cell_width` x `cell_height`
    /// pixels. Fails if there is no picture or the sheet would exceed the
    /// pixel cap, which bounds what a header can make a decoder allocate.
    pub(super) fn new(
        count: usize,
        cell_width: usize,
        cell_height: usize,
    ) -> Result<Self, DecodeError> {
        if count == 0 || count > MAX_PICTURES {
            return Err(DecodeError::Unrecognized);
        }
        let (pitch_x, pitch_y) = (cell_width + GUTTER, cell_height + GUTTER);
        // The fewest columns that make the sheet at least as wide as tall.
        let columns = (1..=count)
            .find(|c| c * c * pitch_x >= count * pitch_y)
            .unwrap_or(count);
        let rows = count.div_ceil(columns);
        let (width, height) = (columns * pitch_x + GUTTER, rows * pitch_y + GUTTER);
        check_size(width, height)?;
        let background = core::iter::repeat(TRANSPARENT_FILL);
        Ok(Self {
            image: Image::from_colors(width as u32, height as u32, background),
            columns,
            cell_width,
            cell_height,
        })
    }

    /// Draws picture number `index` into its cell. Parts of the picture
    /// outside the cell, or a cell outside the sheet, are dropped.
    pub(super) fn put(&mut self, index: usize, picture: &Image) {
        let left = GUTTER + index % self.columns * (self.cell_width + GUTTER);
        let top = GUTTER + index / self.columns * (self.cell_height + GUTTER);
        let width = picture.width() as usize;
        let rows = picture.rgb().chunks_exact(width * 3).take(self.cell_height);
        for (y, row) in (top..).zip(rows) {
            if y >= self.image.height() as usize {
                return;
            }
            // `left` is inside the sheet: the column is below `columns`.
            let target = &mut self.image.row_mut(y as u32)[left * 3..];
            let len = target.len().min(row.len()).min(self.cell_width * 3);
            target[..len].copy_from_slice(&row[..len]);
        }
    }

    pub(super) fn into_image(self) -> Image {
        self.image
    }
}

/// A picture of `width` x `height` pixels from 1-bit rows of whole bytes,
/// most significant bit leftmost, a set bit black.
pub(super) fn black_on_white(
    width: usize,
    height: usize,
    rows: &[u8],
) -> Result<Image, DecodeError> {
    check_size(width, height)?;
    Image::from_bits(
        width as u32,
        height as u32,
        rows,
        width.div_ceil(8),
        BitOrder::MsbFirst,
        [0xffffff, 0x000000],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picture(width: usize, height: usize) -> Image {
        black_on_white(
            width,
            height,
            &alloc::vec![0xff; width.div_ceil(8) * height],
        )
        .unwrap()
    }

    #[test]
    fn grid_is_about_square_and_pictures_keep_their_cells() {
        // 8 pictures of 8 x 8 have a pitch of 12: 3 columns and 3 rows
        // make the sheet 40 x 40.
        let mut sheet = Sheet::new(8, 8, 8).unwrap();
        for index in 0..8 {
            sheet.put(index, &picture(8, 8));
        }
        let image = sheet.into_image();
        assert_eq!((image.width(), image.height()), (40, 40));
        // First cell is black, its gutter is the fill, the second cell starts at 16.
        assert_eq!(image.get(4, 4), 0);
        assert_eq!(image.get(3, 4), TRANSPARENT_FILL);
        assert_eq!(image.get(12, 4), TRANSPARENT_FILL);
        assert_eq!(image.get(16, 4), 0);
        // The ninth cell does not exist; the last cell of row 3 is empty.
        assert_eq!(image.get(28, 28), TRANSPARENT_FILL);
    }

    #[test]
    fn smaller_pictures_sit_in_the_top_left_of_their_cell() {
        let mut sheet = Sheet::new(1, 8, 8).unwrap();
        sheet.put(0, &picture(2, 3));
        let image = sheet.into_image();
        assert_eq!(image.get(5, 6), 0);
        assert_eq!(image.get(6, 6), TRANSPARENT_FILL);
        assert_eq!(image.get(4, 8), TRANSPARENT_FILL);
    }

    #[test]
    fn put_drops_what_does_not_fit() {
        let mut sheet = Sheet::new(1, 4, 4).unwrap();
        sheet.put(0, &picture(16, 16));
        sheet.put(5, &picture(4, 4));
        let image = sheet.into_image();
        assert_eq!(image.get(7, 7), 0);
        assert_eq!(image.get(8, 7), TRANSPARENT_FILL);
    }

    #[test]
    fn empty_and_oversized_sheets_are_rejected() {
        assert!(Sheet::new(0, 8, 8).is_err());
        assert!(Sheet::new(MAX_PICTURES + 1, 8, 8).is_err());
        assert!(Sheet::new(MAX_PICTURES, 4000, 4000).is_err());
    }
}
