//! Several icons on one sheet, for files that hold a set of small pictures.
//!
//! No external format knowledge: the sheet is this crate's own layout, icons
//! left to right wrapped at 512 pixels, 4 pixels apart, on the shared
//! transparent-fill grey.

use alloc::vec::Vec;

use crate::image::{TRANSPARENT_FILL, check_size};
use crate::{DecodeError, Image};

const SHEET_WIDTH: usize = 512;
const GAP: usize = 4;

/// An icon as `0xRRGGBB` pixels, row-major. Transparent pixels are already
/// [`TRANSPARENT_FILL`] (or any color the decoder chose).
pub(crate) struct Icon {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) pixels: Vec<u32>,
}

/// All `icons` on one sheet, wrapped at 512 pixels (wider for a wider icon).
/// Fails if the sheet would pass the size cap.
pub(crate) fn sheet(icons: &[Icon]) -> Result<Image, DecodeError> {
    let sheet_width = icons
        .iter()
        .map(|icon| icon.width + 2 * GAP)
        .fold(SHEET_WIDTH, usize::max);
    let (mut x, mut y, mut row_height) = (GAP, GAP, 0);
    let mut places = Vec::with_capacity(icons.len());
    for icon in icons {
        if x + icon.width + GAP > sheet_width {
            x = GAP;
            y += row_height + GAP;
            row_height = 0;
        }
        places.push((x, y));
        x += icon.width + GAP;
        row_height = row_height.max(icon.height);
    }
    let sheet_height = y + row_height + GAP;
    check_size(sheet_width, sheet_height)?;
    let mut image = Image::new(sheet_width as u32, sheet_height as u32);
    for py in 0..sheet_height {
        for px in 0..sheet_width {
            image.set(px as u32, py as u32, TRANSPARENT_FILL);
        }
    }
    for (icon, &(x, y)) in icons.iter().zip(&places) {
        for (i, &color) in icon.pixels.iter().enumerate() {
            image.set(
                (x + i % icon.width) as u32,
                (y + i / icon.width) as u32,
                color,
            );
        }
    }
    Ok(image)
}
