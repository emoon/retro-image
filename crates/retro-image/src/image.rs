//! The decoded picture type and pixel helpers shared by the decoders.
//!
//! No external format knowledge, except `Image::blend`: averaging the
//! frames of interlaced, flickering and gigascreen pictures per channel,
//! rounding down, reproduces how RECOIL shows them (observed from
//! `recoil2png` output).

use alloc::vec::Vec;

use crate::{DecodeError, simd};

/// Which bit of a byte in a 1-bit bitmap is the leftmost pixel.
#[derive(Debug, Clone, Copy)]
pub(crate) enum BitOrder {
    MsbFirst,
    LsbFirst,
}

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
    pub(crate) fn from_indexed(
        width: u32,
        height: u32,
        indices: &[u8],
        palette: &[u32],
    ) -> Result<Self, DecodeError> {
        let outside = indices
            .iter()
            .max()
            .is_some_and(|&max| usize::from(max) >= palette.len());
        if indices.len() != width as usize * height as usize || outside {
            return Err(DecodeError::Unrecognized);
        }
        let mut table = [0; 256];
        let used = palette.len().min(256);
        table[..used].copy_from_slice(&palette[..used]);
        let mut rgb = alloc::vec![0; indices.len() * 3];
        simd::palette_to_rgb(indices, &table, &mut rgb);
        Ok(Self { width, height, rgb })
    }

    /// An image from a 1-bit bitmap of `row_len`-byte rows; a pixel whose
    /// bit is `b` gets `colors[b]`.
    ///
    /// Fails if `bitmap` holds fewer than `height` rows or a row is too
    /// short for `width` pixels.
    pub(crate) fn from_bits(
        width: u32,
        height: u32,
        bitmap: &[u8],
        row_len: usize,
        order: BitOrder,
        colors: [u32; 2],
    ) -> Result<Self, DecodeError> {
        let fits = row_len
            .checked_mul(height as usize)
            .is_some_and(|len| len <= bitmap.len());
        if !fits || row_len.saturating_mul(8) < width as usize {
            return Err(DecodeError::Unrecognized);
        }
        let pixels = width as usize;
        let mut indices = alloc::vec![0; pixels * height as usize];
        let mut reversed = Vec::new();
        for (y, out) in indices.chunks_exact_mut(pixels.max(1)).enumerate() {
            let row = &bitmap[y * row_len..][..pixels.div_ceil(8)];
            let row = match order {
                BitOrder::MsbFirst => row,
                BitOrder::LsbFirst => {
                    reversed.clear();
                    reversed.extend(row.iter().map(|b| b.reverse_bits()));
                    &reversed
                }
            };
            simd::expand_plane(row, 0, out);
        }
        Self::from_indexed(width, height, &indices, &colors)
    }

    /// Every pixel repeated `sx` times horizontally and `sy` times vertically.
    pub(crate) fn scaled(&self, sx: u32, sy: u32) -> Self {
        let (width, height) = (self.width * sx, self.height * sy);
        let out_row = width as usize * 3;
        let mut rgb = Vec::with_capacity(out_row * height as usize);
        if out_row > 0 && sy > 0 {
            for row in self.rgb.chunks_exact(self.width as usize * 3) {
                let start = rgb.len();
                if sx == 1 {
                    rgb.extend_from_slice(row);
                } else {
                    for pixel in row.chunks_exact(3) {
                        for _ in 0..sx {
                            rgb.extend_from_slice(pixel);
                        }
                    }
                }
                for _ in 1..sy {
                    rgb.extend_from_within(start..start + out_row);
                }
            }
        }
        Self { width, height, rgb }
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
        let rgb = if let [a, b] = frames {
            let mut rgb = alloc::vec![0; a.rgb.len()];
            simd::average_floor(&a.rgb, &b.rgb, &mut rgb);
            rgb
        } else {
            let count = frames.len() as u32;
            (0..first.rgb.len())
                .map(|i| (frames.iter().map(|f| u32::from(f.rgb[i])).sum::<u32>() / count) as u8)
                .collect()
        };
        Self {
            width: first.width,
            height: first.height,
            rgb,
        }
    }
}

/// Pixel values of one row of bitplanes: the `p`th row from `planes`
/// (most significant bit first) gives bit `p` of each value. At most 32
/// planes; `scratch` must be as long as `out`.
pub(crate) fn planar_values<'a>(
    planes: impl IntoIterator<Item = &'a [u8]>,
    scratch: &mut [u8],
    out: &mut [u32],
) {
    out.fill(0);
    let mut planes = planes.into_iter();
    for shift in [0, 8, 16, 24] {
        scratch.fill(0);
        let mut count = 0;
        for (bit, plane) in planes.by_ref().take(8).enumerate() {
            simd::expand_plane(plane, bit as u32, scratch);
            count += 1;
        }
        for (value, &byte) in out.iter_mut().zip(&*scratch) {
            *value |= u32::from(byte) << shift;
        }
        if count < 8 {
            return;
        }
    }
    assert!(planes.next().is_none(), "more than 32 planes");
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
    fn from_bits_follows_bit_order_and_validates() {
        let colors = [0x000000, 0xffffff];
        let msb = Image::from_bits(
            3,
            2,
            &[0b1010_0000, 0, 0b0100_0000, 0],
            2,
            BitOrder::MsbFirst,
            colors,
        )
        .unwrap();
        assert_eq!(
            (msb.get(0, 0), msb.get(1, 0), msb.get(2, 0)),
            (0xffffff, 0, 0xffffff)
        );
        assert_eq!(msb.get(1, 1), 0xffffff);
        let lsb = Image::from_bits(2, 1, &[0b10], 1, BitOrder::LsbFirst, colors).unwrap();
        assert_eq!((lsb.get(0, 0), lsb.get(1, 0)), (0, 0xffffff));
        assert!(Image::from_bits(8, 2, &[0], 1, BitOrder::MsbFirst, colors).is_err());
        assert!(Image::from_bits(9, 1, &[0, 0], 1, BitOrder::MsbFirst, colors).is_err());
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

    #[test]
    fn planar_values_combines_up_to_32_planes() {
        let planes: Vec<[u8; 1]> = (0..24).map(|p| [0x80 >> (p % 3)]).collect();
        let mut out = [0; 3];
        planar_values(planes.iter().map(|p| &p[..]), &mut [0; 3], &mut out);
        assert_eq!(out, [0x24_9249, 0x49_2492, 0x92_4924]);
    }
}
