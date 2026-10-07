//! Several pictures on one sheet, for files that hold a set of small
//! pictures: clip-art libraries and icon files.
//!
//! This is the crate's own choice, not part of any format. The pictures sit
//! on a fixed grid, left to right, top to bottom, each in a cell as large as
//! the largest picture, with a gutter between cells. The grid is as close to
//! square as the whole columns allow, so a thumbnail of the sheet stays
//! readable. Empty space is transparent.
//!
//! A set of more than [`MAX_PICTURES`] pictures is rejected, not cut short.
//! For the headerless Print Shop `.DAT`, which only its size identifies, the
//! cap also limits what the size rule can claim.

use crate::DecodeError;
use crate::Image;
use crate::image::{CLEAR, check_size};

/// Most pictures on one sheet.
pub(crate) const MAX_PICTURES: usize = 256;
const GUTTER: usize = 4;

/// The sheet being laid out. [`Sheet::new`] sizes it from the picture count
/// and the largest picture, so a decoder first reads all picture headers.
pub(crate) struct Sheet {
    image: Image,
    columns: usize,
    cell_width: usize,
    cell_height: usize,
}

impl Sheet {
    /// A sheet for `count` pictures of at most `cell_width` x `cell_height`
    /// pixels. Fails if there is no picture or the sheet would exceed the
    /// pixel cap, which bounds what a header can make a decoder allocate.
    pub(crate) fn new(
        count: usize,
        cell_width: usize,
        cell_height: usize,
    ) -> Result<Self, DecodeError> {
        if count == 0 || count > MAX_PICTURES {
            return Err(DecodeError::Invalid);
        }
        let (pitch_x, pitch_y) = (cell_width + GUTTER, cell_height + GUTTER);
        // The fewest columns that make the sheet at least as wide as tall.
        let columns = (1..=count)
            .find(|c| c * c * pitch_x >= count * pitch_y)
            .unwrap_or(count);
        let rows = count.div_ceil(columns);
        let (width, height) = (columns * pitch_x + GUTTER, rows * pitch_y + GUTTER);
        check_size(width, height)?;
        let background = core::iter::repeat(CLEAR);
        Ok(Self {
            image: Image::from_argb(width as u32, height as u32, background)?,
            columns,
            cell_width,
            cell_height,
        })
    }

    /// Draws picture number `index` into its cell. Parts of the picture
    /// outside the cell, or a cell outside the sheet, are dropped.
    pub(crate) fn put(&mut self, index: usize, picture: &Image) {
        let left = GUTTER + index % self.columns * (self.cell_width + GUTTER);
        let top = GUTTER + index / self.columns * (self.cell_height + GUTTER);
        self.image
            .paste(picture, left, top, self.cell_width, self.cell_height);
    }

    pub(crate) fn into_image(self) -> Image {
        self.image
    }
}

/// All `pictures` on one sheet, for decoders that hold them all already.
/// The cell is the largest picture. Fails if there are none or more than
/// [`MAX_PICTURES`], or if the sheet would pass the size cap.
pub(crate) fn sheet(pictures: &[Image]) -> Result<Image, DecodeError> {
    let width = pictures.iter().map(|p| p.width() as usize).max();
    let height = pictures.iter().map(|p| p.height() as usize).max();
    let mut sheet = Sheet::new(pictures.len(), width.unwrap_or(0), height.unwrap_or(0))?;
    for (index, picture) in pictures.iter().enumerate() {
        sheet.put(index, picture);
    }
    Ok(sheet.into_image())
}

#[cfg(test)]
mod tests {
    use super::*;

    const OPAQUE_BLACK: u32 = 0xff00_0000;

    /// A black picture of `width` x `height` pixels.
    fn picture(width: usize, height: usize) -> Image {
        Image::from_colors(width as u32, height as u32, core::iter::repeat(0x000000)).unwrap()
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
        assert_eq!(image.get_argb(4, 4), OPAQUE_BLACK);
        assert_eq!(image.get_argb(3, 4), CLEAR);
        assert_eq!(image.get_argb(12, 4), CLEAR);
        assert_eq!(image.get_argb(16, 4), OPAQUE_BLACK);
        // The ninth cell does not exist; the last cell of row 3 is empty.
        assert_eq!(image.get_argb(28, 28), CLEAR);
    }

    #[test]
    fn smaller_pictures_sit_in_the_top_left_of_their_cell() {
        let mut sheet = Sheet::new(1, 8, 8).unwrap();
        sheet.put(0, &picture(2, 3));
        let image = sheet.into_image();
        assert_eq!(image.get_argb(5, 6), OPAQUE_BLACK);
        assert_eq!(image.get_argb(6, 6), CLEAR);
        assert_eq!(image.get_argb(4, 8), CLEAR);
    }

    #[test]
    fn put_drops_what_does_not_fit() {
        let mut sheet = Sheet::new(1, 4, 4).unwrap();
        sheet.put(0, &picture(16, 16));
        sheet.put(5, &picture(4, 4));
        let image = sheet.into_image();
        assert_eq!(image.get_argb(7, 7), OPAQUE_BLACK);
        assert_eq!(image.get_argb(8, 7), CLEAR);
    }

    #[test]
    fn sheet_puts_each_picture_in_a_cell_of_the_largest() {
        // Cells of 8 x 3 have a pitch of 12 x 7: 2 columns and 1 row.
        let pictures = [picture(8, 2), picture(2, 3)];
        let image = sheet(&pictures).unwrap();
        assert_eq!((image.width(), image.height()), (28, 11));
        assert_eq!(
            (image.get_argb(4, 5), image.get_argb(4, 6)),
            (OPAQUE_BLACK, CLEAR)
        );
        assert_eq!(
            (image.get_argb(16, 6), image.get_argb(18, 6)),
            (OPAQUE_BLACK, CLEAR)
        );
        assert_eq!(sheet(&[]).err(), Some(DecodeError::Invalid));
        let many = alloc::vec![picture(1, 1); MAX_PICTURES + 1];
        assert!(sheet(&many).is_err());
        assert!(sheet(&many[1..]).is_ok());
    }

    #[test]
    fn empty_and_oversized_sheets_are_rejected() {
        assert!(Sheet::new(0, 8, 8).is_err());
        assert!(Sheet::new(MAX_PICTURES + 1, 8, 8).is_err());
        assert!(Sheet::new(MAX_PICTURES, 4000, 4000).is_err());
    }
}
