//! VIC-II building blocks shared by the C64 formats: palette, hires and
//! multicolour bitmap rendering (optionally with per-line FLI screens and
//! `$D021` tables) and interlace blending.
//!
//! Sources:
//! - Bitmap, screen RAM and colour RAM semantics: Christian Bauer, "The MOS
//!   6567/6569 video controller (VIC-II)", <https://www.cebix.net/VIC-Article.txt>,
//!   and the C64 Programmer's Reference Guide,
//!   <https://archive.org/details/Commodore_64_Programmers_Reference_Guide_1983_Commodore>.
//! - Palette: Pepto's 2001 VIC-II palette,
//!   <https://www.pepto.de/projects/colorvic/2001/>; that `recoil2png` uses it
//!   was observed from its output.
//! - Output conventions, observed from `recoil2png` output: images are
//!   320 pixels wide with multicolour pixels doubled; FLI pictures drop the
//!   leftmost 24 pixels (the FLI bug), giving 296; interlaced pictures are
//!   the per-channel average of both frames.

use crate::Image;
use alloc::vec::Vec;

/// Pepto's PAL VIC-II palette.
const PALETTE: [u32; 16] = [
    0x000000, 0xffffff, 0x68372b, 0x70a4b2, 0x6f3d86, 0x588d43, 0x352879, 0xb8c76f, 0x6f4f25,
    0x433900, 0x9a6759, 0x444444, 0x6c6c6c, 0x9ad284, 0x6c5eb5, 0x959595,
];

pub(super) fn rgb(color: u8) -> u32 {
    PALETTE[usize::from(color & 15)]
}

/// Width of a full C64 bitmap in hires pixels.
pub(super) const WIDTH: usize = 320;
/// Bytes of a 320×200 bitmap.
pub(super) const BITMAP_LEN: usize = 8000;
/// Bytes of a screen or colour RAM.
pub(super) const SCREEN_LEN: usize = 1000;
/// Leftmost pixels hidden by the FLI bug that `recoil2png` crops away.
pub(super) const FLI_BUG: usize = 24;

/// Where the per-line screen RAM comes from.
#[derive(Clone, Copy)]
pub(super) enum Screens<'a> {
    /// One screen RAM for the whole picture.
    Single(&'a [u8]),
    /// FLI: eight screen RAMs, `stride` bytes apart; line `y` uses screen `y % 8`.
    Fli { data: &'a [u8], stride: usize },
}

impl Screens<'_> {
    fn get(&self, y: usize, cell: usize) -> u8 {
        match *self {
            Self::Single(data) => data[cell],
            Self::Fli { data, stride } => data[(y & 7) * stride + cell],
        }
    }

    fn fits(&self) -> bool {
        match *self {
            Self::Single(data) => data.len() >= SCREEN_LEN,
            Self::Fli { data, stride } => {
                stride >= SCREEN_LEN && data.len() >= 7 * stride + SCREEN_LEN
            }
        }
    }
}

/// Background colour (`$D021`) source.
#[derive(Clone, Copy)]
pub(super) enum Background<'a> {
    Fixed(u8),
    /// One entry per pixel line; lines past the end use the last entry.
    PerLine(&'a [u8]),
}

impl Background<'_> {
    pub(super) fn get(&self, y: usize) -> u8 {
        match *self {
            Self::Fixed(color) => color,
            Self::PerLine(table) => table.get(y).or(table.last()).copied().unwrap_or(0),
        }
    }
}

/// The memory a VIC-II bitmap mode reads.
#[derive(Clone, Copy)]
pub(super) struct Bitmap<'a> {
    /// 8 bytes per 8×8 cell, cells in row-major order.
    pub bitmap: &'a [u8],
    pub screens: Screens<'a>,
    /// Colour RAM (multicolour only).
    pub color: &'a [u8],
    pub background: Background<'a>,
}

impl<'a> Bitmap<'a> {
    pub(super) fn hires(bitmap: &'a [u8], screen: &'a [u8]) -> Self {
        Self {
            bitmap,
            screens: Screens::Single(screen),
            color: &[],
            background: Background::Fixed(0),
        }
    }

    pub(super) fn multicolor(
        bitmap: &'a [u8],
        screen: &'a [u8],
        color: &'a [u8],
        background: u8,
    ) -> Self {
        Self {
            bitmap,
            screens: Screens::Single(screen),
            color,
            background: Background::Fixed(background),
        }
    }

    fn fits(&self, height: usize, multicolor: bool) -> bool {
        let cells = height.div_ceil(8) * 40;
        self.bitmap.len() >= cells * 8
            && self.screens.fits()
            && (!multicolor || self.color.len() >= SCREEN_LEN.min(cells))
    }

    fn byte(&self, x: usize, y: usize) -> (u8, usize) {
        let cell = y / 8 * 40 + x / 8;
        (self.bitmap[cell * 8 + y % 8], cell)
    }

    /// Hires colour of pixel (`x`, `y`): set bits use the screen's high nibble.
    fn hires_pixel(&self, x: usize, y: usize) -> u8 {
        let (byte, cell) = self.byte(x, y);
        let screen = self.screens.get(y, cell);
        if byte & (0x80 >> (x % 8)) != 0 {
            screen >> 4
        } else {
            screen & 15
        }
    }

    /// Multicolour colour of hires pixel (`x`, `y`) (each bit pair covers two).
    fn multicolor_pixel(&self, x: usize, y: usize) -> u8 {
        let (byte, cell) = self.byte(x, y);
        match byte >> (6 - (x & 6)) & 3 {
            0 => self.background.get(y),
            1 => self.screens.get(y, cell) >> 4,
            2 => self.screens.get(y, cell) & 15,
            _ => self.color[cell] & 15,
        }
    }
}

/// A picture as C64 colour indices, 320 hires pixels wide.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Frame {
    height: usize,
    pixels: Vec<u8>,
}

impl Frame {
    pub(super) fn new(height: usize) -> Self {
        Self {
            height,
            pixels: alloc::vec![0; WIDTH * height],
        }
    }

    pub(super) fn hires(bitmap: &Bitmap, height: usize) -> Option<Self> {
        bitmap
            .fits(height, false)
            .then(|| Self::from_fn(height, |x, y| bitmap.hires_pixel(x, y)))
    }

    pub(super) fn multicolor(bitmap: &Bitmap, height: usize) -> Option<Self> {
        bitmap
            .fits(height, true)
            .then(|| Self::from_fn(height, |x, y| bitmap.multicolor_pixel(x, y)))
    }

    pub(super) fn from_fn(height: usize, pixel: impl Fn(usize, usize) -> u8) -> Self {
        let mut frame = Self::new(height);
        for y in 0..height {
            for x in 0..WIDTH {
                frame.pixels[y * WIDTH + x] = pixel(x, y);
            }
        }
        frame
    }

    /// Drops the top `lines` pixel lines.
    pub(super) fn skip_lines(mut self, lines: usize) -> Self {
        let lines = lines.min(self.height);
        self.pixels.drain(..lines * WIDTH);
        self.height -= lines;
        self
    }

    /// Moves the picture one pixel right; `fill` enters at the left edge.
    pub(super) fn shift_right(mut self, fill: u8) -> Self {
        for row in self.pixels.chunks_exact_mut(WIDTH) {
            row.copy_within(..WIDTH - 1, 1);
            row[0] = fill;
        }
        self
    }

    /// Paints the leftmost `width` pixels of each line `y` in `color(y)`.
    pub(super) fn fill_left(&mut self, width: usize, color: impl Fn(usize) -> u8) {
        for (y, row) in self.pixels.chunks_exact_mut(WIDTH).enumerate() {
            row[..width].fill(color(y));
        }
    }

    pub(super) fn get(&self, x: usize, y: usize) -> u8 {
        self.pixels[y * WIDTH + x]
    }

    /// Converts to RGB, dropping the leftmost `crop` pixels.
    pub(super) fn to_image(&self, crop: usize) -> Image {
        self.crop(crop, WIDTH - crop, self.height)
    }

    /// Converts the leftmost `width` pixels to RGB.
    pub(super) fn to_image_width(&self, width: usize) -> Image {
        self.crop(0, width.min(WIDTH), self.height)
    }

    /// Blends two interlace frames into one picture, dropping the leftmost `crop` pixels.
    pub(super) fn blend(&self, other: &Frame, crop: usize) -> Image {
        let height = self.height.min(other.height);
        let width = WIDTH - crop;
        Image::blend(&[
            &self.crop(crop, width, height),
            &other.crop(crop, width, height),
        ])
    }

    /// The `width`×`height` pixels starting at column `left`, as RGB.
    fn crop(&self, left: usize, width: usize, height: usize) -> Image {
        let indices: Vec<u8> = self
            .pixels
            .chunks_exact(WIDTH)
            .take(height)
            .flat_map(|row| &row[left..left + width])
            .copied()
            .collect();
        image(width, height, indices)
    }
}

/// An image from one C64 colour per pixel (the high nibble is ignored).
/// `colors` must hold `width * height` entries.
pub(super) fn image(width: usize, height: usize, mut colors: Vec<u8>) -> Image {
    debug_assert_eq!(colors.len(), width * height);
    colors.resize(width * height, 0);
    colors.iter_mut().for_each(|color| *color &= 15);
    Image::from_indexed(width as u32, height as u32, &colors, &PALETTE)
        .unwrap_or_else(|_| unreachable!("sized and masked to the palette above"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multicolor_bit_pairs_select_sources() {
        let mut bitmap = [0u8; BITMAP_LEN];
        bitmap[0] = 0b00_01_10_11;
        let screen = [0x23u8; SCREEN_LEN];
        let color = [0x04u8; SCREEN_LEN];
        let frame =
            Frame::multicolor(&Bitmap::multicolor(&bitmap, &screen, &color, 5), 200).unwrap();
        let row: Vec<u8> = (0..8).map(|x| frame.get(x, 0)).collect();
        assert_eq!(row, [5, 5, 2, 2, 3, 3, 4, 4]);
    }

    #[test]
    fn hires_uses_screen_nibbles() {
        let mut bitmap = [0u8; BITMAP_LEN];
        bitmap[8] = 0x80; // cell 1, first pixel
        let screen = [0x61u8; SCREEN_LEN];
        let frame = Frame::hires(&Bitmap::hires(&bitmap, &screen), 200).unwrap();
        assert_eq!(frame.get(8, 0), 6);
        assert_eq!(frame.get(9, 0), 1);
    }

    #[test]
    fn fli_line_selects_screen() {
        let bitmap = [0xffu8; BITMAP_LEN];
        let mut screens = alloc::vec![0u8; 8 * 1024];
        screens[3 * 1024] = 0x70;
        let bitmap = Bitmap {
            bitmap: &bitmap,
            screens: Screens::Fli {
                data: &screens,
                stride: 1024,
            },
            color: &[],
            background: Background::Fixed(0),
        };
        let frame = Frame::hires(&bitmap, 200).unwrap();
        assert_eq!(frame.get(0, 3), 7);
        assert_eq!(frame.get(0, 2), 0);
    }

    #[test]
    fn short_data_is_rejected() {
        let bitmap = [0u8; 100];
        assert!(Frame::hires(&Bitmap::hires(&bitmap, &bitmap), 200).is_none());
    }
}
