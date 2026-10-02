use alloc::vec::Vec;

use crate::DecodeError;

/// A decoded picture: 8-bit RGB, row-major, top row first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    width: u32,
    height: u32,
    rgb: Vec<u8>,
}

impl Image {
    /// Creates a black image.
    pub(crate) fn new(width: u32, height: u32) -> Self {
        let len = width as usize * height as usize * 3;
        Self {
            width,
            height,
            rgb: alloc::vec![0; len],
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Pixel data, 3 bytes (R, G, B) per pixel.
    pub fn rgb(&self) -> &[u8] {
        &self.rgb
    }

    pub fn into_rgb(self) -> Vec<u8> {
        self.rgb
    }

    /// Sets the pixel at (`x`, `y`) to `0xRRGGBB`.
    pub(crate) fn set(&mut self, x: u32, y: u32, color: u32) {
        let i = (y as usize * self.width as usize + x as usize) * 3;
        let [_, r, g, b] = color.to_be_bytes();
        self.rgb[i..i + 3].copy_from_slice(&[r, g, b]);
    }

    /// The pixel at (`x`, `y`) as `0xRRGGBB`.
    pub(crate) fn get(&self, x: u32, y: u32) -> u32 {
        let i = (y as usize * self.width as usize + x as usize) * 3;
        u32::from(self.rgb[i]) << 16 | u32::from(self.rgb[i + 1]) << 8 | u32::from(self.rgb[i + 2])
    }

    /// An image from one palette index per pixel, row-major.
    ///
    /// Fails if `indices` doesn't hold exactly `width * height` entries or
    /// an index is outside `palette`.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "adopted by platform decoders in the next wave")
    )]
    pub(crate) fn from_indexed(
        width: u32,
        height: u32,
        indices: &[u8],
        palette: &[u32],
    ) -> Result<Self, DecodeError> {
        if indices.len() != width as usize * height as usize {
            return Err(DecodeError::Unrecognized);
        }
        let mut rgb = Vec::with_capacity(indices.len() * 3);
        for &index in indices {
            let color = *palette
                .get(usize::from(index))
                .ok_or(DecodeError::Unrecognized)?;
            let [_, r, g, b] = color.to_be_bytes();
            rgb.extend_from_slice(&[r, g, b]);
        }
        Ok(Self { width, height, rgb })
    }

    /// Every pixel repeated `sx` times horizontally and `sy` times vertically.
    pub(crate) fn scaled(&self, sx: u32, sy: u32) -> Self {
        let mut out = Self::new(self.width * sx, self.height * sy);
        for y in 0..out.height {
            for x in 0..out.width {
                out.set(x, y, self.get(x / sx, y / sy));
            }
        }
        out
    }

    /// The per-channel average of equally sized frames, rounding down: how
    /// interlaced, flickering or gigascreen pictures look on screen.
    ///
    /// Panics if `frames` is empty or the sizes differ (a decoder bug).
    pub(crate) fn blend(frames: &[&Self]) -> Self {
        let first = frames[0];
        assert!(
            frames
                .iter()
                .all(|f| (f.width, f.height) == (first.width, first.height)),
            "blended frames must have equal sizes"
        );
        let count = frames.len() as u32;
        let rgb = (0..first.rgb.len())
            .map(|i| (frames.iter().map(|f| u32::from(f.rgb[i])).sum::<u32>() / count) as u8)
            .collect();
        Self {
            width: first.width,
            height: first.height,
            rgb,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_returns_what_set_stored() {
        let mut image = Image::new(2, 2);
        image.set(1, 1, 0x123456);
        assert_eq!(image.get(1, 1), 0x123456);
        assert_eq!(image.get(0, 1), 0);
    }

    #[test]
    fn from_indexed_maps_through_palette_and_validates() {
        let image = Image::from_indexed(2, 1, &[1, 0], &[0x000000, 0xff8000]).unwrap();
        assert_eq!(image.rgb(), &[0xff, 0x80, 0, 0, 0, 0]);
        assert!(Image::from_indexed(2, 1, &[2, 0], &[0, 0]).is_err());
        assert!(Image::from_indexed(2, 2, &[0, 0], &[0]).is_err());
    }

    #[test]
    fn scaled_repeats_pixels() {
        let image = Image::from_indexed(2, 1, &[0, 1], &[0x000000, 0xffffff]).unwrap();
        let big = image.scaled(2, 3);
        assert_eq!((big.width(), big.height()), (4, 3));
        assert_eq!(big.get(1, 2), 0x000000);
        assert_eq!(big.get(2, 0), 0xffffff);
    }

    #[test]
    fn blend_averages_channels_rounding_down() {
        let a = Image::from_indexed(1, 1, &[0], &[0x01ff10]).unwrap();
        let b = Image::from_indexed(1, 1, &[0], &[0x020011]).unwrap();
        assert_eq!(Image::blend(&[&a, &b]).get(0, 0), 0x017f10);
        let c = Image::from_indexed(1, 1, &[0], &[0x030303]).unwrap();
        // (1+2+3)/3, (255+0+3)/3, (16+17+3)/3
        assert_eq!(Image::blend(&[&a, &b, &c]).get(0, 0), 0x02560c);
    }
}
