//! Rendering of ANTIC/GTIA bitmap memory.
//!
//! Sources: De Re Atari ch. 2 (ANTIC mode table) and App. E (GTIA modes);
//! the output scaling (a 320-pixel-wide canvas, so 160-pixel modes are drawn
//! 2 pixels wide and GTIA modes 4 pixels wide) is observed from `recoil2png` output.

use crate::Image;

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

    /// Draws every pixel as a `pixel_width` x `pixel_height` block, colour
    /// from `color(line, value)`. The caller checks `data` holds every line.
    pub fn render(
        &self,
        pixel_width: u32,
        pixel_height: u32,
        color: impl Fn(usize, u8) -> u32,
    ) -> Image {
        let width = self.width() as u32 * pixel_width;
        let mut image = Image::new(width, self.lines as u32 * pixel_height);
        for y in 0..self.lines {
            for x in 0..self.width() {
                let rgb = color(y, self.pixel(x, y));
                fill(
                    &mut image,
                    x as u32 * pixel_width,
                    y as u32 * pixel_height,
                    pixel_width,
                    pixel_height,
                    rgb,
                );
            }
        }
        image
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
