//! Shared Spectrum screen pieces: bitmap interleave, attribute colours,
//! frames and frame blending.

use alloc::vec::Vec;

use crate::Image;

pub(super) const WIDTH: usize = 256;
pub(super) const HEIGHT: usize = 192;
pub(super) const COLUMNS: usize = WIDTH / 8;
pub(super) const BITMAP_LEN: usize = 6144;
pub(super) const ATTRIBUTES_LEN: usize = 768;
pub(super) const SCR_LEN: usize = BITMAP_LEN + ATTRIBUTES_LEN;

/// Byte offset of pixel row `y` in an interleaved bitmap: the row number's
/// bits are third (7-6), character row (5-3) and pixel line (2-0), laid out
/// in memory as third, pixel line, character row.
pub(super) fn bitmap_offset(y: usize) -> usize {
    let third = (y >> 6) & 3;
    let char_row = (y >> 3) & 7;
    let line = y & 7;
    (third << 11) | (line << 8) | (char_row << 5)
}

/// Byte holding pixels `column * 8 ..` of row `y` in an interleaved bitmap.
pub(super) fn bitmap_byte(bitmap: &[u8], column: usize, y: usize) -> u8 {
    bitmap[bitmap_offset(y) + column]
}

/// Spectrum colour: index bits 0 blue, 1 red, 2 green; normal intensity
/// 0xCD, bright 0xFF (observed from `recoil2png` output).
pub(super) fn color(index: u8, bright: bool) -> u32 {
    let level = if bright { 0xff } else { 0xcd };
    rgb_bits(index, level)
}

/// GRB colour index scaled to `level` per set bit.
pub(super) fn rgb_bits(index: u8, level: u32) -> u32 {
    let channel = |bit: u8| if index & bit != 0 { level } else { 0 };
    channel(2) << 16 | channel(4) << 8 | channel(1)
}

/// Attribute byte: bit 7 flash (ignored), bit 6 bright, bits 5-3 paper, bits 2-0 ink.
pub(super) fn attribute_color(attribute: u8, ink: bool) -> u32 {
    let index = if ink {
        attribute & 7
    } else {
        (attribute >> 3) & 7
    };
    color(index, attribute & 0x40 != 0)
}

/// One displayed frame of `0xRRGGBB` pixels; several frames shown in
/// alternation (gigascreen, tricolor) are averaged into the final image.
pub(super) struct Frame {
    width: usize,
    height: usize,
    pixels: Vec<u32>,
}

impl Frame {
    pub(super) fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: alloc::vec![0; width * height],
        }
    }

    pub(super) fn set(&mut self, x: usize, y: usize, color: u32) {
        self.pixels[y * self.width + x] = color;
    }

    /// Fills a `width` x `height` rectangle.
    pub(super) fn fill(&mut self, x: usize, y: usize, width: usize, height: usize, color: u32) {
        for row in y..y + height {
            let start = row * self.width + x;
            self.pixels[start..start + width].fill(color);
        }
    }

    /// Draws a 256x192 attribute screen at (`left`, `top`). `pixels(column, y)`
    /// gives the bitmap byte and `colors(column, y, ink)` the colour of set
    /// (`ink`) or clear bits for that 8x1 cell.
    pub(super) fn draw_screen(
        &mut self,
        left: usize,
        top: usize,
        pixels: impl Fn(usize, usize) -> u8,
        colors: impl Fn(usize, usize, bool) -> u32,
    ) {
        for y in 0..HEIGHT {
            for column in 0..COLUMNS {
                let byte = pixels(column, y);
                let ink = colors(column, y, true);
                let paper = colors(column, y, false);
                for bit in 0..8 {
                    let color = if byte & (0x80 >> bit) != 0 {
                        ink
                    } else {
                        paper
                    };
                    self.set(left + column * 8 + bit, top + y, color);
                }
            }
        }
    }

    pub(super) fn into_image(self) -> Image {
        blend(&[self])
    }
}

/// Averages equally sized frames channel by channel (rounding down), the way
/// alternating frames are shown (observed from `recoil2png` output).
pub(super) fn blend(frames: &[Frame]) -> Image {
    let (width, height) = (frames[0].width, frames[0].height);
    let mut image = Image::new(width as u32, height as u32);
    let count = frames.len() as u32;
    for y in 0..height {
        for x in 0..width {
            let i = y * width + x;
            let mut sum = [0u32; 3];
            for frame in frames {
                let [_, r, g, b] = frame.pixels[i].to_be_bytes();
                sum[0] += u32::from(r);
                sum[1] += u32::from(g);
                sum[2] += u32::from(b);
            }
            let [r, g, b] = sum.map(|s| s / count);
            image.set(x as u32, y as u32, r << 16 | g << 8 | b);
        }
    }
    image
}

/// Draws a standard 6912-byte screen (bitmap then 32x24 attributes).
pub(super) fn draw_scr(frame: &mut Frame, left: usize, top: usize, scr: &[u8]) {
    let (bitmap, attributes) = scr.split_at(BITMAP_LEN);
    frame.draw_screen(
        left,
        top,
        |column, y| bitmap_byte(bitmap, column, y),
        |column, y, ink| attribute_color(attributes[y / 8 * COLUMNS + column], ink),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitmap_offset_follows_screen_interleave() {
        assert_eq!(bitmap_offset(0), 0);
        assert_eq!(bitmap_offset(1), 0x100);
        assert_eq!(bitmap_offset(8), 0x20);
        assert_eq!(bitmap_offset(64), 0x800);
        assert_eq!(bitmap_offset(191), 0x17e0);
    }

    #[test]
    fn colors_use_grb_bits_and_bright_level() {
        assert_eq!(color(0, true), 0x000000);
        assert_eq!(color(1, false), 0x0000cd);
        assert_eq!(color(2, false), 0xcd0000);
        assert_eq!(color(4, true), 0x00ff00);
        assert_eq!(color(7, false), 0xcdcdcd);
    }

    #[test]
    fn blend_averages_rounding_down() {
        let mut a = Frame::new(1, 1);
        let b = Frame::new(1, 1);
        a.set(0, 0, 0xff_cd_01);
        let image = blend(&[a, b]);
        assert_eq!(image.rgb(), &[0x7f, 0x66, 0x00]);
    }
}
