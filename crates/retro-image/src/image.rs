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

/// Most pixels a decoder may allocate for one picture (192 MiB of RGB).
/// Dimensions come from untrusted headers, so a few bytes of file must not
/// be able to demand gigabytes.
const MAX_PIXELS: usize = 1 << 26;

/// Fails if a `width` x `height` picture is empty or exceeds [`MAX_PIXELS`].
/// The one size gate: call before allocating anything sized from header
/// dimensions. A format with a smaller hard limit states it locally.
pub(crate) fn check_size(width: usize, height: usize) -> Result<(), DecodeError> {
    match width.checked_mul(height) {
        Some(pixels) if pixels != 0 && pixels <= MAX_PIXELS => Ok(()),
        _ => Err(DecodeError::Unrecognized),
    }
}

/// Colour shown where a picture is transparent. `Image` has no alpha channel,
/// so every decoder whose format carries transparency composites onto this
/// light grey, which stays visible against both white and black artwork.
pub(crate) const TRANSPARENT_FILL: u32 = 0xc0_c0c0;

/// A color with alpha (red, green, blue, alpha, each 0 to 255) laid over the
/// [`TRANSPARENT_FILL`], as `0xRRGGBB`: opaque colors stay, transparent ones
/// become the fill, in between they are mixed.
pub(crate) fn over_fill([r, g, b, alpha]: [u8; 4]) -> u32 {
    let [_, fill_r, fill_g, fill_b] = TRANSPARENT_FILL.to_be_bytes();
    let alpha = u32::from(alpha);
    let mix = |color: u8, fill: u8| {
        (u32::from(color) * alpha + u32::from(fill) * (255 - alpha) + 127) / 255
    };
    mix(r, fill_r) << 16 | mix(g, fill_g) << 8 | mix(b, fill_b)
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
        let len = (width as usize)
            .checked_mul(height as usize)
            .and_then(|pixels| pixels.checked_mul(3))
            .expect("image size overflows; decoders must call check_size first");
        Self {
            width,
            height,
            rgb: alloc::vec![0; len],
        }
    }

    /// An image from `0xRRGGBB` colours in row-major order; pixels the
    /// iterator doesn't reach stay black.
    pub(crate) fn from_colors(width: u32, height: u32, colors: impl Iterator<Item = u32>) -> Self {
        let mut image = Self::new(width, height);
        for (pixel, color) in image.rgb.as_chunks_mut::<3>().0.iter_mut().zip(colors) {
            let [_, r, g, b] = color.to_be_bytes();
            *pixel = [r, g, b];
        }
        image
    }

    /// Width in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Height in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Pixel data, 3 bytes (R, G, B) per pixel.
    pub fn rgb(&self) -> &[u8] {
        &self.rgb
    }

    /// Takes the pixel data, 3 bytes (R, G, B) per pixel.
    pub fn into_rgb(self) -> Vec<u8> {
        self.rgb
    }

    /// Sets the pixel at (`x`, `y`) to `0xRRGGBB`.
    #[inline]
    pub(crate) fn set(&mut self, x: u32, y: u32, color: u32) {
        let i = y as usize * self.width as usize + x as usize;
        let [_, r, g, b] = color.to_be_bytes();
        self.rgb.as_chunks_mut::<3>().0[i] = [r, g, b];
    }

    /// Row `y` as RGB bytes. Panics if `y` is outside the image.
    pub(crate) fn row_mut(&mut self, y: u32) -> &mut [u8] {
        let row_len = self.width as usize * 3;
        &mut self.rgb[y as usize * row_len..][..row_len]
    }

    /// The pixel at (`x`, `y`) as `0xRRGGBB`.
    #[inline]
    pub(crate) fn get(&self, x: u32, y: u32) -> u32 {
        let i = y as usize * self.width as usize + x as usize;
        let [r, g, b] = self.rgb.as_chunks::<3>().0[i];
        u32::from_be_bytes([0, r, g, b])
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
        let outside = max_byte(indices).is_some_and(|max| usize::from(max) >= palette.len());
        let pixels = (width as usize).checked_mul(height as usize);
        if pixels != Some(indices.len()) || outside {
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
        // The rows form one bit stream, so a single `expand_plane` call
        // covers the image; the padding bits are cropped afterwards.
        let bits = &bitmap[..row_len * height as usize];
        let reversed: Vec<u8>;
        let bits = match order {
            BitOrder::MsbFirst => bits,
            BitOrder::LsbFirst => {
                reversed = bits.iter().map(|b| b.reverse_bits()).collect();
                &reversed
            }
        };
        let mut indices = alloc::vec![0; bits.len() * 8];
        simd::expand_plane(bits, 0, &mut indices);
        crop_rows(&mut indices, row_len * 8, width as usize);
        Self::from_indexed(width, height, &indices, &colors)
    }

    /// Every pixel repeated `sx` times horizontally and `sy` times vertically.
    /// Fails if the result would be empty or exceed the [`check_size`] cap.
    pub(crate) fn scaled(&self, sx: u32, sy: u32) -> Result<Self, DecodeError> {
        let width = (self.width as usize).saturating_mul(sx as usize);
        let height = (self.height as usize).saturating_mul(sy as usize);
        check_size(width, height)?;
        let (width, height) = (width as u32, height as u32);
        let out_row = width as usize * 3;
        let mut rgb = Vec::with_capacity(out_row * height as usize);
        for row in self.rgb.chunks_exact(self.width as usize * 3) {
            let start = rgb.len();
            if sx == 1 {
                rgb.extend_from_slice(row);
            } else {
                for pixel in row.as_chunks::<3>().0 {
                    for _ in 0..sx {
                        rgb.extend_from_slice(pixel);
                    }
                }
            }
            for _ in 1..sy {
                rgb.extend_from_within(start..start + out_row);
            }
        }
        Ok(Self { width, height, rgb })
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

/// Pixel values of a `width` x `height` image stored as `planes`
/// bitplanes of `row_len`-byte rows, most significant bit first:
/// `row_start(plane, y)` is where a row starts in `data`, and plane `p` gives
/// bit `p` of each value. At most 32 planes. Panics if a row is outside
/// `data` or shorter than `width` bits.
pub(crate) fn planar_pixels(
    data: &[u8],
    width: usize,
    height: usize,
    row_len: usize,
    planes: usize,
    row_start: impl Fn(usize, usize) -> usize,
) -> Vec<u32> {
    assert!(planes <= 32, "more than 32 planes");
    assert!(row_len * 8 >= width, "rows too short");
    // Regrouped plane by plane, each plane is one bit stream covering the
    // whole image, so it takes a single `expand_plane` call.
    let plane_len = row_len * height;
    if plane_len == 0 {
        return Vec::new();
    }
    let mut grouped = Vec::with_capacity(plane_len * planes);
    for plane in 0..planes {
        for y in 0..height {
            grouped.extend_from_slice(&data[row_start(plane, y)..][..row_len]);
        }
    }
    let stride = row_len * 8;
    let mut values = alloc::vec![0u32; stride * height];
    let mut bits = alloc::vec![0u8; values.len()];
    for (shift, group) in (0..).step_by(8).zip(grouped.chunks(plane_len * 8)) {
        bits.fill(0);
        for (bit, plane) in group.chunks_exact(plane_len).enumerate() {
            simd::expand_plane(plane, bit as u32, &mut bits);
        }
        for (value, &byte) in values.iter_mut().zip(&bits) {
            *value |= u32::from(byte) << shift;
        }
    }
    crop_rows(&mut values, stride, width);
    values
}

/// The largest byte of `bytes`. Lane-wise maxima over fixed-size chunks
/// vectorise; `Iterator::max` compiles to one compare-and-select per byte.
fn max_byte(bytes: &[u8]) -> Option<u8> {
    let (chunks, tail) = bytes.as_chunks::<32>();
    let mut lanes = [0u8; 32];
    for chunk in chunks {
        for (lane, &byte) in lanes.iter_mut().zip(chunk) {
            *lane = (*lane).max(byte);
        }
    }
    let max = lanes.iter().chain(tail).copied().max();
    max.filter(|_| !bytes.is_empty())
}

/// Keeps the first `width` (at most `stride`) items of every `stride`-item
/// row of `values`.
fn crop_rows<T: Copy>(values: &mut Vec<T>, stride: usize, width: usize) {
    if stride == width {
        return;
    }
    let rows = values.len() / stride;
    for y in 1..rows {
        values.copy_within(y * stride..y * stride + width, y * width);
    }
    values.truncate(rows * width);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn over_fill_mixes_by_alpha() {
        assert_eq!(over_fill([1, 2, 3, 255]), 0x01_0203);
        assert_eq!(over_fill([1, 2, 3, 0]), TRANSPARENT_FILL);
        // Half transparent white over the gray 0xc0: (255 * 128 + 192 * 127) / 255 rounds to 0xe0.
        assert_eq!(over_fill([255, 255, 255, 128]), 0xe0_e0e0);
    }

    #[test]
    fn get_returns_what_set_stored() {
        let mut image = Image::new(2, 2);
        image.set(1, 1, 0x123456);
        assert_eq!(image.get(1, 1), 0x123456);
        assert_eq!(image.get(0, 1), 0);
    }

    #[test]
    #[should_panic(expected = "check_size")]
    fn new_refuses_sizes_that_overflow() {
        Image::new(u32::MAX, u32::MAX);
    }

    #[test]
    fn from_indexed_size_overflow_is_an_error() {
        assert!(Image::from_indexed(u32::MAX, u32::MAX, &[0], &[0]).is_err());
    }

    #[test]
    fn from_indexed_maps_through_palette_and_validates() {
        let image = Image::from_indexed(2, 1, &[1, 0], &[0x000000, 0xff8000]).unwrap();
        assert_eq!(image.rgb(), &[0xff, 0x80, 0, 0, 0, 0]);
        assert!(Image::from_indexed(2, 1, &[2, 0], &[0, 0]).is_err());
        assert!(Image::from_indexed(2, 2, &[0, 0], &[0]).is_err());
    }

    #[test]
    fn from_colors_fills_row_major_and_pads_black() {
        let image = Image::from_colors(2, 2, [0x010203, 0x040506, 0x070809].into_iter());
        assert_eq!(image.rgb(), &[1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 0, 0]);
    }

    #[test]
    fn max_byte_covers_chunks_and_tail() {
        assert_eq!(max_byte(&[]), None);
        assert_eq!(max_byte(&[0; 5]), Some(0));
        let mut bytes = [1u8; 70];
        assert_eq!(max_byte(&bytes), Some(1));
        bytes[33] = 9;
        assert_eq!(max_byte(&bytes), Some(9));
        bytes[69] = 200;
        assert_eq!(max_byte(&bytes), Some(200));
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
        let big = image.scaled(2, 3).unwrap();
        assert_eq!((big.width(), big.height()), (4, 3));
        assert_eq!(big.get(1, 2), 0x000000);
        assert_eq!(big.get(2, 0), 0xffffff);
    }

    #[test]
    fn size_gate_rejects_empty_and_oversized() {
        assert!(check_size(0, 5).is_err());
        assert!(check_size(5, 0).is_err());
        assert!(check_size(1 << 14, 1 << 14).is_err());
        let image = Image::from_indexed(2, 1, &[0, 1], &[0, 1]).unwrap();
        assert!(image.scaled(1 << 14, 1 << 14).is_err());
        assert!(image.scaled(0, 1).is_err());
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
    fn planar_pixels_combines_up_to_32_planes() {
        // 24 planes of two 1-byte rows, cropped to 3 pixels.
        let data: Vec<u8> = (0..48).map(|i| 0x80 >> (i / 2 % 3)).collect();
        let values = planar_pixels(&data, 3, 2, 1, 24, |plane, y| plane * 2 + y);
        let row = [0x24_9249, 0x49_2492, 0x92_4924];
        assert_eq!(values, [row, row].concat());
    }
}
