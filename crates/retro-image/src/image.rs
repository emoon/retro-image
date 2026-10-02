use alloc::vec::Vec;

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
}
