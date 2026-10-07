//! Art Studio window (`.mwin`, `.mwi`): a multicolor clip of an Art Studio
//! picture.
//!
//! Sources: reverse engineered from the five `artstudio2-*.mwin` samples by
//! black-box probing of `recoil2png`; no format description was found.
//! - Header of 5 bytes: one ignored byte, the window's x position in
//!   multicolor pixels (a multiple of 4) and y position (a multiple of 8),
//!   the width in multicolor pixels and the height in lines. Neither
//!   position is used and no bounds are checked.
//! - Then one record per character cell, left to right and top to bottom
//!   (`ceil(width / 4)` by `ceil(height / 8)` cells): screen RAM byte,
//!   color RAM byte (low nibble) and 8 bitmap bytes. The background is black.
//! - Each multicolor pixel is drawn two pixels wide. `recoil2png` wants the
//!   length to be exact and a nonzero width and height.

use super::vic2;
use crate::{DecodeError, Image};
use alloc::vec::Vec;

const HEADER: usize = 5;
const RECORD: usize = 10;

pub(super) fn decode_mwin(data: &[u8]) -> Result<Image, DecodeError> {
    let [_, x, y, width, height, cells @ ..] = data else {
        return Err(DecodeError::Invalid);
    };
    if x % 4 != 0 || y % 8 != 0 || *width == 0 || *height == 0 {
        return Err(DecodeError::Invalid);
    }
    let (width, height) = (usize::from(*width), usize::from(*height));
    let columns = width.div_ceil(4);
    if cells.len() != columns * height.div_ceil(8) * RECORD {
        return Err(DecodeError::Invalid);
    }
    let mut pixels = Vec::with_capacity(width * 2 * height);
    for py in 0..height {
        for px in 0..width {
            let cell = &cells[(py / 8 * columns + px / 4) * RECORD..][..RECORD];
            let pair = cell[2 + py % 8] >> (6 - px % 4 * 2) & 3;
            let color = match pair {
                0 => 0,
                1 => cell[0] >> 4,
                2 => cell[0] & 15,
                _ => cell[1] & 15,
            };
            pixels.extend([color, color]);
        }
    }
    debug_assert_eq!(HEADER + cells.len(), data.len());
    Ok(vic2::image(width * 2, height, pixels))
}
