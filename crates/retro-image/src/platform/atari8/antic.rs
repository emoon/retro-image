//! Rendering of ANTIC/GTIA bitmap memory.
//!
//! Sources:
//! - De Re Atari ch. 2, ANTIC mode table
//!   (<https://www.atariarchives.org/dere/chapt02.php>), and App. E, GTIA
//!   modes (<https://www.atariarchives.org/dere/chaptE.php>).
//! - The output scaling (a 320-pixel-wide canvas, so 160-pixel modes are
//!   drawn 2 pixels wide and GTIA modes 4 pixels wide) is observed from
//!   `recoil2png` output (black box).

use crate::{DecodeError, Image};

/// Packed bitmap: `lines` rows of `bytes_per_line` bytes, `bits` (1, 2 or 4)
/// per pixel, most significant bits leftmost.
#[derive(Clone, Copy)]
pub(super) struct Bitmap<'a> {
    pub data: &'a [u8],
    pub bytes_per_line: usize,
    pub lines: usize,
    pub bits: u8,
}

impl Bitmap<'_> {
    /// Pixels per line.
    pub fn width(&self) -> usize {
        self.bytes_per_line * 8 / usize::from(self.bits)
    }

    /// Value of pixel (`x`, `y`). Both must be in range.
    pub fn pixel(&self, x: usize, y: usize) -> u8 {
        let per_byte = 8 / usize::from(self.bits);
        let byte = self.data[y * self.bytes_per_line + x / per_byte];
        let shift = 8 - usize::from(self.bits) * (x % per_byte + 1);
        (byte >> shift) & ((1 << self.bits) - 1)
    }

    /// Draws every pixel as a `pixel_width` x `pixel_height` block, color
    /// from `color(line, value)`. The caller checks `data` holds every line.
    pub fn render(
        &self,
        pixel_width: u32,
        pixel_height: u32,
        color: impl Fn(usize, u8) -> u32,
    ) -> Result<Image, DecodeError> {
        let mut image = Image::new(self.width() as u32, self.lines as u32);
        for y in 0..self.lines {
            for x in 0..self.width() {
                image.set(x as u32, y as u32, color(y, self.pixel(x, y)));
            }
        }
        if (pixel_width, pixel_height) == (1, 1) {
            Ok(image)
        } else {
            image.scaled(pixel_width, pixel_height)
        }
    }
}

/// Pixel `x` of a 4-bit row, high nibble first.
pub(super) fn nibble(row: &[u8], x: usize) -> u8 {
    let byte = row[x / 2];
    if x.is_multiple_of(2) {
        byte >> 4
    } else {
        byte & 0x0f
    }
}

/// Fills a `width` x `height` block at (`x`, `y`).
pub(super) fn fill(image: &mut Image, x: u32, y: u32, width: u32, height: u32, rgb: u32) {
    for dy in 0..height {
        for dx in 0..width {
            image.set(x + dx, y + dy, rgb);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unpacks_msb_first() {
        let data = [0b1101_0010];
        let two = Bitmap {
            data: &data,
            bytes_per_line: 1,
            lines: 1,
            bits: 2,
        };
        assert_eq!(
            [two.pixel(0, 0), two.pixel(1, 0), two.pixel(3, 0)],
            [3, 1, 2]
        );
        let four = Bitmap { bits: 4, ..two };
        assert_eq!([four.pixel(0, 0), four.pixel(1, 0)], [0xd, 0x2]);
    }
}
