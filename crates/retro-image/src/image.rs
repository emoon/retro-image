//! The decoded picture type and pixel helpers shared by the decoders.
//!
//! No external format knowledge, except `Image::blend`: averaging the
//! frames of interlaced, flickering and gigascreen pictures per channel,
//! rounding down, reproduces how RECOIL shows them (observed from
//! `recoil2png` output).

use alloc::vec::Vec;
use core::fmt;

use crate::{DecodeError, limits, simd};

/// Which bit of a byte in a 1-bit bitmap is the leftmost pixel.
#[derive(Debug, Clone, Copy)]
pub(crate) enum BitOrder {
    MsbFirst,
    LsbFirst,
}

/// Fails if a `width` x `height` picture exceeds the [`Limits`](crate::Limits) in force.
/// The one size gate: every `Image` constructor passes through it, so no decoder can
/// build an oversized picture. Decoders still call [`check_size`] first when
/// they allocate buffers sized from header dimensions before the `Image`.
fn within_limit(width: usize, height: usize) -> Result<(), DecodeError> {
    if limits::fits(width, height, limits::max_pixels()) {
        Ok(())
    } else {
        Err(DecodeError::TooLarge)
    }
}

/// Fails if a `width` x `height` picture is empty or exceeds the size limit.
/// Call before allocating anything sized from header dimensions. A format
/// with a smaller hard limit states it locally.
pub(crate) fn check_size(width: usize, height: usize) -> Result<(), DecodeError> {
    if width == 0 || height == 0 {
        return Err(DecodeError::Invalid);
    }
    within_limit(width, height)
}

/// A 15-bit color as the Game Boy Color, the Game Boy Advance, the DS and the
/// PlayStation store it, as `0xRRGGBB`: red in bits 0-4, green in bits 5-9,
/// blue in bits 10-14, bit 15 ignored, each channel widened by
/// [`widen_channel`].
pub(crate) fn bgr555(word: u16) -> u32 {
    let channel = |shift: u32| widen_channel(u32::from(word >> shift & 31), 5);
    channel(0) << 16 | channel(5) << 8 | channel(10)
}

/// A 15-bit color with red high, as `0xRRGGBB`: red in bits 10-14, green in
/// bits 5-9, blue in bits 0-4, bit 15 ignored.
pub(crate) fn xrgb1555(word: u16) -> u32 {
    let channel = |shift: u32| widen_channel(u32::from(word >> shift & 31), 5);
    channel(10) << 16 | channel(5) << 8 | channel(0)
}

/// A 12-bit `0RGB` color as `0xRRGGBB`, as the Amiga, the Atari TT and the
/// Apple IIGS store a palette entry: red in bits 8-11, green in bits 4-7, blue
/// in bits 0-3, bits 12-15 ignored.
pub(crate) fn rgb444(word: u16) -> u32 {
    let channel = |shift: u32| widen_channel(u32::from(word >> shift & 15), 4);
    channel(8) << 16 | channel(4) << 8 | channel(0)
}

/// A 16-bit color as `0xRRGGBB`: red in bits 11-15, green in bits 5-10, blue
/// in bits 0-4.
pub(crate) fn rgb565(word: u16) -> u32 {
    let channel =
        |shift: u32, bits: u32| widen_channel(u32::from(word >> shift) & ((1 << bits) - 1), bits);
    channel(11, 5) << 16 | channel(5, 6) << 8 | channel(0, 5)
}

/// A fully transparent pixel as [`Image::get_argb`] reports it.
pub(crate) const CLEAR: u32 = 0;

/// `color` drawn over `base` (both `0xRRGGBB`) with `alpha`, 0 for fully
/// transparent and 255 for opaque, rounded to the nearest level.
pub(crate) fn over(base: u32, color: u32, alpha: u8) -> u32 {
    let alpha = u32::from(alpha);
    let channel = |shift: u32| {
        let (above, below) = (color >> shift & 0xff, base >> shift & 0xff);
        (above * alpha + below * (255 - alpha) + 127) / 255
    };
    channel(16) << 16 | channel(8) << 8 | channel(0)
}

/// A `bits`-bit channel value (at most 8 bits) stretched to 8 bits by
/// repeating its high bits, so that the largest value becomes 255.
#[inline]
pub(crate) const fn widen_channel(value: u32, bits: u32) -> u32 {
    debug_assert!(matches!(bits, 1..=8), "a channel has 1 to 8 bits");
    let mut wide = value << (8 - bits);
    let mut have = bits;
    while have < 8 {
        wide |= wide >> have;
        have *= 2;
    }
    wide
}

/// `len` opaque grays (`0xAARRGGBB`) from black to white: the palette shown
/// for indexed pictures whose palette file is not at hand.
pub(crate) fn gray_ramp(len: usize) -> Vec<u32> {
    (0..len)
        .map(|i| {
            let gray = (i * 255 / (len - 1).max(1)) as u32;
            0xff00_0000 | gray << 16 | gray << 8 | gray
        })
        .collect()
}

/// A decoded picture: 8-bit color, row-major, top row first, and an alpha
/// plane when some pixel is not opaque.
///
/// [`rgb`](Self::rgb) holds the color channels and ignores alpha;
/// [`has_alpha`](Self::has_alpha) says whether that is the whole picture.
/// Alpha is straight (not premultiplied): 0 is transparent, 255 opaque.
/// Decoded images are canonical: an image whose pixels are all opaque has no
/// alpha plane, and a fully transparent pixel is black.
#[derive(Clone, PartialEq, Eq)]
pub struct Image {
    width: u32,
    height: u32,
    rgb: Vec<u8>,
    alpha: Option<Vec<u8>>,
}

/// Prints the size and whether there is an alpha plane, not the pixels: a
/// picture can be millions of bytes, and `unwrap` and `assert_eq!` print
/// their operands.
impl fmt::Debug for Image {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Image")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("has_alpha", &self.has_alpha())
            .finish_non_exhaustive()
    }
}

impl Image {
    /// Creates a black image. Fails if it exceeds the size limit.
    pub(crate) fn new(width: u32, height: u32) -> Result<Self, DecodeError> {
        within_limit(width as usize, height as usize)?;
        Ok(Self {
            width,
            height,
            rgb: alloc::vec![0; width as usize * height as usize * 3],
            alpha: None,
        })
    }

    /// An image from `0xRRGGBB` colors in row-major order; pixels the
    /// iterator doesn't reach stay black.
    pub(crate) fn from_colors(
        width: u32,
        height: u32,
        colors: impl Iterator<Item = u32>,
    ) -> Result<Self, DecodeError> {
        let mut image = Self::new(width, height)?;
        for (pixel, color) in image.rgb.as_chunks_mut::<3>().0.iter_mut().zip(colors) {
            let [_, r, g, b] = color.to_be_bytes();
            *pixel = [r, g, b];
        }
        Ok(image)
    }

    /// An image from straight `0xAARRGGBB` pixels in row-major order; pixels
    /// the iterator doesn't reach stay opaque black.
    pub(crate) fn from_argb(
        width: u32,
        height: u32,
        pixels: impl Iterator<Item = u32>,
    ) -> Result<Self, DecodeError> {
        let mut image = Self::new(width, height)?;
        let mut alpha = alloc::vec![255; image.rgb.len() / 3];
        let mut opaque = true;
        let targets = image.rgb.as_chunks_mut::<3>().0.iter_mut().zip(&mut alpha);
        for ((color, slot), argb) in targets.zip(pixels) {
            let [a, r, g, b] = argb.to_be_bytes();
            *color = [r, g, b];
            *slot = a;
            opaque &= a == 255;
        }
        image.alpha = (!opaque).then_some(alpha);
        Ok(image)
    }

    /// Width in pixels.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Height in pixels.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// The image with `alpha` as its alpha plane, one byte per pixel in the
    /// same order as the pixels. Panics if its length is not the pixel count
    /// (a decoder bug).
    pub(crate) fn with_alpha(mut self, alpha: Vec<u8>) -> Self {
        assert_eq!(alpha.len() * 3, self.rgb.len(), "one alpha value per pixel");
        self.alpha = Some(alpha);
        self
    }

    /// The color channels, 3 bytes (R, G, B) per pixel, ignoring alpha.
    /// Use [`flattened`](Self::flattened) first for the picture as seen over a
    /// background, or [`rgba`](Self::rgba) to keep the transparency.
    #[must_use]
    pub fn rgb(&self) -> &[u8] {
        &self.rgb
    }

    /// Takes the color channels, 3 bytes (R, G, B) per pixel, dropping alpha.
    #[must_use]
    pub fn into_rgb(self) -> Vec<u8> {
        self.rgb
    }

    /// Whether some pixel is not fully opaque, so that [`rgb`](Self::rgb) is
    /// not the whole picture.
    #[must_use]
    pub fn has_alpha(&self) -> bool {
        self.alpha.is_some()
    }

    /// The pixels as straight RGBA, 4 bytes (R, G, B, A) per pixel; opaque
    /// (255) when the image has no alpha. Allocates a copy.
    #[must_use]
    pub fn rgba(&self) -> Vec<u8> {
        let pixels = self.rgb.as_chunks::<3>().0;
        let mut out = Vec::with_capacity(pixels.len() * 4);
        match &self.alpha {
            Some(alpha) => {
                for (&[r, g, b], &a) in pixels.iter().zip(alpha) {
                    out.extend_from_slice(&[r, g, b, a]);
                }
            }
            None => {
                for &[r, g, b] in pixels {
                    out.extend_from_slice(&[r, g, b, 255]);
                }
            }
        }
        out
    }

    /// A new image: the picture drawn over an opaque `background` (R, G, B),
    /// which has no alpha plane, for consumers that cannot show transparency.
    /// This image is left as it is; an image without alpha comes back as a
    /// copy.
    #[must_use]
    pub fn flattened(&self, background: [u8; 3]) -> Self {
        let Some(alpha) = &self.alpha else {
            return self.clone();
        };
        let [r, g, b] = background;
        let base = u32::from_be_bytes([0, r, g, b]);
        let rgb = (self.rgb.as_chunks::<3>().0.iter().zip(alpha))
            .flat_map(|(&[r, g, b], &a)| {
                let [_, r, g, b] = over(base, u32::from_be_bytes([0, r, g, b]), a).to_be_bytes();
                [r, g, b]
            })
            .collect();
        Self {
            width: self.width,
            height: self.height,
            rgb,
            alpha: None,
        }
    }

    /// The canonical form [`decode`](crate::decode) promises: no alpha plane
    /// when every pixel is opaque, black under fully transparent pixels.
    /// The one place decoders' output is made canonical, so equal pictures
    /// compare equal whatever way a decoder built them.
    pub(crate) fn normalized(mut self) -> Self {
        let Some(alpha) = &self.alpha else {
            return self;
        };
        if alpha.iter().all(|&a| a == 255) {
            self.alpha = None;
            return self;
        }
        for (pixel, &a) in self.rgb.as_chunks_mut::<3>().0.iter_mut().zip(alpha) {
            if a == 0 {
                *pixel = [0; 3];
            }
        }
        self
    }

    /// Sets the pixel at (`x`, `y`) to the opaque color `0xRRGGBB`.
    #[inline]
    pub(crate) fn set(&mut self, x: u32, y: u32, color: u32) {
        self.set_argb(x, y, 0xff00_0000 | color);
    }

    /// Sets the pixel at (`x`, `y`) to straight `0xAARRGGBB`. The alpha plane
    /// appears with the first pixel that is not opaque.
    #[inline]
    pub(crate) fn set_argb(&mut self, x: u32, y: u32, argb: u32) {
        let i = y as usize * self.width as usize + x as usize;
        let [a, r, g, b] = argb.to_be_bytes();
        self.rgb.as_chunks_mut::<3>().0[i] = [r, g, b];
        if a != 255 || self.alpha.is_some() {
            let pixels = self.rgb.len() / 3;
            self.alpha.get_or_insert_with(|| alloc::vec![255; pixels])[i] = a;
        }
    }

    /// Row `y` as RGB bytes. Panics if `y` is outside the image.
    pub(crate) fn row_mut(&mut self, y: u32) -> &mut [u8] {
        let row_len = self.width as usize * 3;
        &mut self.rgb[y as usize * row_len..][..row_len]
    }

    /// Row `y` of the alpha plane, which appears (opaque) on the first call.
    /// Panics if `y` is outside the image.
    pub(crate) fn alpha_row_mut(&mut self, y: u32) -> &mut [u8] {
        let width = self.width as usize;
        let pixels = self.rgb.len() / 3;
        let alpha = self.alpha.get_or_insert_with(|| alloc::vec![255; pixels]);
        &mut alpha[y as usize * width..][..width]
    }

    /// Draws straight `0xAARRGGBB` over the pixel at (`x`, `y`) (the "over"
    /// operator), rounding to the nearest level.
    pub(crate) fn draw(&mut self, x: u32, y: u32, argb: u32) {
        let [a, ..] = argb.to_be_bytes();
        let i = y as usize * self.width as usize + x as usize;
        let below = self.alpha.as_ref().map_or(255, |alpha| u32::from(alpha[i]));
        match (a, below) {
            (0, _) => {}
            (255, _) => self.set_argb(x, y, argb),
            (_, 255) => {
                let [_, r, g, b] = over(self.get(x, y), argb & 0xff_ffff, a).to_be_bytes();
                self.rgb.as_chunks_mut::<3>().0[i] = [r, g, b];
            }
            _ => {
                // Weights are scaled by 255 so that the division is exact.
                let (above, below_weight) = (u32::from(a) * 255, below * (255 - u32::from(a)));
                let total = above + below_weight;
                let [r, g, b] = self.rgb.as_chunks::<3>().0[i];
                let mix = |top: u32, bottom: u8| {
                    (top * above + u32::from(bottom) * below_weight + total / 2) / total
                };
                let [_, tr, tg, tb] = argb.to_be_bytes();
                self.rgb.as_chunks_mut::<3>().0[i] = [
                    mix(u32::from(tr), r) as u8,
                    mix(u32::from(tg), g) as u8,
                    mix(u32::from(tb), b) as u8,
                ];
                self.alpha
                    .get_or_insert_with(|| unreachable!("below < 255"))[i] =
                    ((total + 127) / 255) as u8;
            }
        }
    }

    /// Copies `picture` with its top left at (`left`, `top`), keeping at most
    /// `max_width` x `max_height` of its pixels. What falls outside `self` is
    /// dropped.
    pub(crate) fn paste(
        &mut self,
        picture: &Self,
        left: usize,
        top: usize,
        max_width: usize,
        max_height: usize,
    ) {
        let (width, height) = (self.width as usize, self.height as usize);
        if left >= width {
            return;
        }
        if picture.alpha.is_some() && self.alpha.is_none() {
            self.alpha = Some(alloc::vec![255; width * height]);
        }
        let columns = (picture.width as usize).min(max_width).min(width - left);
        let rows = (picture.height as usize).min(max_height);
        for y in 0..rows.min(height.saturating_sub(top)) {
            let from = y * picture.width as usize;
            let to = (top + y) * width + left;
            self.rgb[to * 3..][..columns * 3]
                .copy_from_slice(&picture.rgb[from * 3..][..columns * 3]);
            if let Some(alpha) = &mut self.alpha {
                match &picture.alpha {
                    Some(source) => {
                        alpha[to..][..columns].copy_from_slice(&source[from..][..columns])
                    }
                    None => alpha[to..][..columns].fill(255),
                }
            }
        }
    }

    /// The pixel at (`x`, `y`) as `0xRRGGBB`.
    #[inline]
    pub(crate) fn get(&self, x: u32, y: u32) -> u32 {
        let i = y as usize * self.width as usize + x as usize;
        let [r, g, b] = self.rgb.as_chunks::<3>().0[i];
        u32::from_be_bytes([0, r, g, b])
    }

    /// The pixel at (`x`, `y`) as straight `0xAARRGGBB`, with every fully
    /// transparent pixel as 0.
    pub(crate) fn get_argb(&self, x: u32, y: u32) -> u32 {
        let i = y as usize * self.width as usize + x as usize;
        let alpha = self.alpha.as_ref().map_or(255, |a| a[i]);
        match alpha {
            0 => CLEAR,
            _ => (u32::from(alpha) << 24) | self.get(x, y),
        }
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
        within_limit(width as usize, height as usize)?;
        let outside = max_byte(indices).is_some_and(|max| usize::from(max) >= palette.len());
        let pixels = (width as usize).checked_mul(height as usize);
        if pixels != Some(indices.len()) || outside {
            return Err(DecodeError::Invalid);
        }
        let mut table = [0; 256];
        let used = palette.len().min(256);
        table[..used].copy_from_slice(&palette[..used]);
        let mut rgb = alloc::vec![0; indices.len() * 3];
        simd::palette_to_rgb(indices, &table, &mut rgb);
        Ok(Self {
            width,
            height,
            rgb,
            alpha: None,
        })
    }

    /// [`Self::from_indexed`] for a palette of straight `0xAARRGGBB` colors.
    pub(crate) fn from_indexed_argb(
        width: u32,
        height: u32,
        indices: &[u8],
        palette: &[u32],
    ) -> Result<Self, DecodeError> {
        let mut image = Self::from_indexed(width, height, indices, palette)?;
        let mut alphas = [255u8; 256];
        for (alpha, color) in alphas.iter_mut().zip(palette) {
            *alpha = (color >> 24) as u8;
        }
        if alphas.iter().any(|&a| a != 255) {
            let plane: Vec<u8> = indices.iter().map(|&i| alphas[usize::from(i)]).collect();
            image.alpha = plane.iter().any(|&a| a != 255).then_some(plane);
        }
        Ok(image)
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
        // The padding bits of each row are expanded too, so they count.
        within_limit(
            row_len.saturating_mul(8).max(width as usize),
            height as usize,
        )?;
        let fits = row_len
            .checked_mul(height as usize)
            .is_some_and(|len| len <= bitmap.len());
        if !fits || row_len.saturating_mul(8) < width as usize {
            return Err(DecodeError::Invalid);
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
        let (sx, sy) = (sx as usize, sy as usize);
        let in_width = self.width as usize;
        Ok(Self {
            width: width as u32,
            height: height as u32,
            rgb: repeat_pixels::<3>(&self.rgb, in_width, sx, sy),
            alpha: (self.alpha.as_deref()).map(|alpha| repeat_pixels::<1>(alpha, in_width, sx, sy)),
        })
    }

    /// The per-channel average of equally sized frames, rounding down: how
    /// interlaced, flickering or gigascreen pictures look on screen.
    /// Frames with alpha average their alpha the same way and weight their
    /// colors by it, so a transparent pixel adds no color.
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
        let (rgb, alpha) = if frames.iter().any(|f| f.alpha.is_some()) {
            let (rgb, alpha) = blend_with_alpha(frames);
            (rgb, Some(alpha))
        } else if let [a, b] = frames {
            let mut rgb = alloc::vec![0; a.rgb.len()];
            simd::average_floor(&a.rgb, &b.rgb, &mut rgb);
            (rgb, None)
        } else {
            let count = frames.len() as u32;
            let rgb = (0..first.rgb.len())
                .map(|i| (frames.iter().map(|f| u32::from(f.rgb[i])).sum::<u32>() / count) as u8)
                .collect();
            (rgb, None)
        };
        Self {
            width: first.width,
            height: first.height,
            rgb,
            alpha,
        }
    }
}

/// `plane` (rows of `width` pixels of `N` bytes) with every pixel repeated
/// `sx` times horizontally and every row `sy` times.
fn repeat_pixels<const N: usize>(plane: &[u8], width: usize, sx: usize, sy: usize) -> Vec<u8> {
    let out_row = width * sx * N;
    let mut out = Vec::with_capacity(out_row * (plane.len() / (width * N)) * sy);
    for row in plane.chunks_exact(width * N) {
        let start = out.len();
        if sx == 1 {
            out.extend_from_slice(row);
        } else {
            for pixel in row.as_chunks::<N>().0 {
                for _ in 0..sx {
                    out.extend_from_slice(pixel);
                }
            }
        }
        for _ in 1..sy {
            out.extend_from_within(start..start + out_row);
        }
    }
    out
}

/// The color and alpha planes of the average of `frames`, which are equally
/// sized and not all opaque. Alpha is the plain average; each color channel
/// is weighted by the frames' alpha. Both round down.
fn blend_with_alpha(frames: &[&Image]) -> (Vec<u8>, Vec<u8>) {
    let count = frames.len() as u32;
    let alpha_at =
        |frame: &Image, pixel: usize| frame.alpha.as_ref().map_or(255, |a| u32::from(a[pixel]));
    let pixels = frames[0].rgb.len() / 3;
    let mut rgb = alloc::vec![0; pixels * 3];
    let mut alpha = alloc::vec![0; pixels];
    for pixel in 0..pixels {
        let weight: u32 = frames.iter().map(|f| alpha_at(f, pixel)).sum();
        alpha[pixel] = (weight / count) as u8;
        if weight == 0 {
            continue;
        }
        for channel in 0..3 {
            let sum: u32 = (frames.iter())
                .map(|f| u32::from(f.rgb[pixel * 3 + channel]) * alpha_at(f, pixel))
                .sum();
            rgb[pixel * 3 + channel] = (sum / weight) as u8;
        }
    }
    (rgb, alpha)
}

/// Pixel values of a `width` x `height` image stored as `planes`
/// bitplanes of `row_len`-byte rows, most significant bit first:
/// `row_start(plane, y)` is where a row starts in `data`, and plane `p` gives
/// bit `p` of each value. At most 32 planes. Fails if the picture exceeds
/// the size limit, a row is outside `data`, or a row is shorter than `width`
/// bits.
pub(crate) fn planar_pixels(
    data: &[u8],
    width: usize,
    height: usize,
    row_len: usize,
    planes: usize,
    row_start: impl Fn(usize, usize) -> usize,
) -> Result<Vec<u32>, DecodeError> {
    let stride = row_len.saturating_mul(8);
    if planes > 32 || stride < width {
        return Err(DecodeError::Invalid);
    }
    // The padding bits of each row are expanded too, so they count.
    within_limit(stride, height)?;
    // Regrouped plane by plane, each plane is one bit stream covering the
    // whole image, so it takes a single `expand_plane` call.
    let plane_len = row_len * height;
    if plane_len == 0 {
        return Ok(Vec::new());
    }
    let mut grouped = Vec::with_capacity(plane_len * planes);
    for plane in 0..planes {
        for y in 0..height {
            let row = (row_start(plane, y).checked_add(row_len))
                .and_then(|end| data.get(end - row_len..end));
            grouped.extend_from_slice(row.ok_or(DecodeError::Invalid)?);
        }
    }
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
    Ok(values)
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
    fn bgr555_puts_red_lowest_and_ignores_bit_15() {
        assert_eq!(bgr555(0x001f), 0xff_0000);
        assert_eq!(bgr555(0x03e0), 0x00_ff00);
        assert_eq!(bgr555(0x7c00), 0x00_00ff);
        // Channel 16 widens to 0x84, and bit 15 changes nothing.
        assert_eq!(bgr555(0x8010), 0x84_0000);
    }

    #[test]
    fn rgb444_repeats_each_nibble_and_ignores_the_top_one() {
        assert_eq!(rgb444(0x0f00), 0xff_0000);
        assert_eq!(rgb444(0x00f0), 0x00_ff00);
        assert_eq!(rgb444(0x000f), 0x00_00ff);
        assert_eq!(rgb444(0xf123), 0x11_2233);
    }

    #[test]
    fn xrgb1555_puts_red_highest_and_ignores_bit_15() {
        assert_eq!(xrgb1555(0x7c00), 0xff_0000);
        assert_eq!(xrgb1555(0x03e0), 0x00_ff00);
        assert_eq!(xrgb1555(0x001f), 0x00_00ff);
        assert_eq!(xrgb1555(0xc010), 0x84_0084);
    }

    #[test]
    fn rgb565_widens_green_from_6_bits() {
        assert_eq!(rgb565(0xffff), 0xff_ffff);
        assert_eq!(rgb565(0xf800), 0xff_0000);
        assert_eq!(rgb565(0x07e0), 0x00_ff00);
        assert_eq!(rgb565(0x001f), 0x00_00ff);
        assert_eq!(rgb565(0x0020), 0x00_0400);
        assert_eq!(rgb565(0x0004), 0x00_0021);
    }

    #[test]
    fn get_returns_what_set_stored() {
        let mut image = Image::new(2, 2).unwrap();
        image.set(1, 1, 0x123456);
        assert_eq!(image.get(1, 1), 0x123456);
        assert_eq!(image.get(0, 1), 0);
    }

    #[test]
    fn set_argb_adds_the_alpha_plane_on_the_first_clear_pixel() {
        let mut image = Image::new(2, 1).unwrap();
        image.set_argb(0, 0, 0xff10_2030);
        assert!(!image.has_alpha());
        image.set_argb(1, 0, 0x4000_ff00);
        assert_eq!(image.rgba(), [0x10, 0x20, 0x30, 255, 0, 255, 0, 0x40]);
        // `set` draws an opaque pixel over whatever alpha was there.
        image.set(1, 0, 0x000001);
        assert_eq!(image.rgba()[7], 255);
    }

    #[test]
    fn new_refuses_sizes_that_overflow() {
        assert!(Image::new(u32::MAX, u32::MAX).is_err());
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
        let image = Image::from_colors(2, 2, [0x010203, 0x040506, 0x070809].into_iter()).unwrap();
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

    /// A 2 x 1 image of red and green with the given alpha values.
    fn red_green(alpha: [u8; 2]) -> Image {
        Image {
            width: 2,
            height: 1,
            rgb: alloc::vec![255, 0, 0, 0, 255, 0],
            alpha: Some(alpha.to_vec()),
        }
    }

    #[test]
    fn debug_shows_the_size_and_not_the_pixels() {
        let shown = alloc::format!("{:?}", red_green([0, 128]));
        assert_eq!(shown, "Image { width: 2, height: 1, has_alpha: true, .. }");
    }

    #[test]
    fn rgba_adds_the_alpha_plane_or_full_opacity() {
        let opaque = Image::from_indexed(2, 1, &[0, 1], &[0x102030, 0x405060]).unwrap();
        assert!(!opaque.has_alpha());
        assert_eq!(
            opaque.rgba(),
            [0x10, 0x20, 0x30, 255, 0x40, 0x50, 0x60, 255]
        );
        let clear = red_green([0, 128]);
        assert!(clear.has_alpha());
        assert_eq!(clear.rgba(), [255, 0, 0, 0, 0, 255, 0, 128]);
        assert_eq!(clear.rgb(), [255, 0, 0, 0, 255, 0]);
    }

    #[test]
    fn flattened_draws_over_the_background() {
        let flat = red_green([0, 255]).flattened([10, 20, 30]);
        assert!(!flat.has_alpha());
        assert_eq!(flat.rgb(), [10, 20, 30, 0, 255, 0]);
        // Without alpha it is a plain copy.
        assert_eq!(flat.flattened([1, 2, 3]), flat);
        // Half of green over black: 255 * 128 / 255 = 128.
        assert_eq!(
            red_green([0, 128]).flattened([0, 0, 0]).rgb()[3..],
            [0, 128, 0]
        );
    }

    #[test]
    fn normalized_drops_an_opaque_plane_and_blackens_clear_pixels() {
        let opaque = red_green([255, 255]).normalized();
        assert!(!opaque.has_alpha());
        assert_eq!(opaque.rgb(), [255, 0, 0, 0, 255, 0]);
        let mixed = red_green([0, 254]).normalized();
        assert_eq!(mixed.rgba(), [0, 0, 0, 0, 0, 255, 0, 254]);
    }

    #[test]
    fn scaled_repeats_alpha_with_the_pixels() {
        let big = red_green([0, 200]).scaled(2, 2).unwrap();
        assert_eq!((big.width(), big.height()), (4, 2));
        assert_eq!(
            big.alpha.as_deref(),
            Some(&[0, 0, 200, 200, 0, 0, 200, 200][..])
        );
        assert_eq!(big.rgb().len(), 4 * 2 * 3);
    }

    #[test]
    fn blend_weights_color_by_alpha() {
        let clear = red_green([0, 255]);
        let solid = Image::from_indexed(2, 1, &[0, 0], &[0x0000ff]).unwrap();
        let both = Image::blend(&[&clear, &solid]);
        // Pixel 0: the transparent red adds nothing, so blue stays blue at
        // half the alpha. Pixel 1: green and blue are both opaque.
        assert_eq!(both.rgba(), [0, 0, 255, 127, 0, 127, 127, 255]);
    }

    #[test]
    fn draw_is_the_over_operator_on_straight_alpha() {
        let mut image = Image::from_indexed(1, 1, &[0], &[0x0000ff]).unwrap();
        image.draw(0, 0, 0x00ff_0000);
        assert_eq!(image.get_argb(0, 0), 0xff00_00ff, "clear changes nothing");
        // Half red over opaque blue is `over`, and stays opaque.
        image.draw(0, 0, 0x80ff_0000);
        assert_eq!(image.get_argb(0, 0), 0xff80_007f);
        image.draw(0, 0, 0xff00_ff00);
        assert_eq!(image.get_argb(0, 0), 0xff00_ff00, "opaque replaces");
        // Over a transparent pixel the source shows through unchanged.
        let mut clear = Image::from_argb(1, 1, core::iter::once(CLEAR)).unwrap();
        clear.draw(0, 0, 0x80ff_0000);
        assert_eq!(clear.get_argb(0, 0), 0x80ff_0000);
        // Half red over half blue: alpha 192, red two thirds of the color.
        let mut half = Image::from_argb(1, 1, core::iter::once(0x8000_00ff)).unwrap();
        half.draw(0, 0, 0x80ff_0000);
        assert_eq!(half.get_argb(0, 0), 0xc0aa_0055);
    }

    #[test]
    fn from_indexed_argb_takes_alpha_from_the_palette() {
        let palette = [0x00ff_0000, 0xff00_ff00];
        let image = Image::from_indexed_argb(2, 1, &[0, 1], &palette).unwrap();
        assert_eq!(
            (image.get_argb(0, 0), image.get_argb(1, 0)),
            (CLEAR, 0xff00_ff00)
        );
        // A transparent entry that no pixel uses leaves no alpha plane.
        let unused = Image::from_indexed_argb(2, 1, &[1, 1], &palette).unwrap();
        assert!(!unused.has_alpha());
    }

    #[test]
    fn paste_clips_and_carries_alpha() {
        let mut canvas = Image::from_indexed(3, 1, &[0, 0, 0], &[0x112233]).unwrap();
        let picture = red_green([0, 200]);
        canvas.paste(&picture, 2, 0, 5, 5);
        // Only the first picture column fits; the plane appears opaque
        // elsewhere.
        assert_eq!(
            canvas.rgba(),
            [0x11, 0x22, 0x33, 255, 0x11, 0x22, 0x33, 255, 255, 0, 0, 0]
        );
        canvas.paste(&picture, 0, 0, 1, 1);
        assert_eq!(canvas.get_argb(0, 0), CLEAR);
        assert_eq!(
            canvas.get_argb(1, 0),
            0xff11_2233,
            "max_width stops the copy"
        );
    }

    #[test]
    fn alpha_row_mut_starts_opaque() {
        let mut image = Image::new(2, 2).unwrap();
        image.alpha_row_mut(1)[0] = 7;
        assert_eq!(
            image.rgba()[3..],
            [255, 0, 0, 0, 255, 0, 0, 0, 7, 0, 0, 0, 255]
        );
    }

    #[test]
    fn over_blends_channel_by_channel() {
        assert_eq!(over(0x102030, 0xf0e0d0, 0), 0x102030);
        assert_eq!(over(0x102030, 0xf0e0d0, 255), 0xf0e0d0);
        // Halfway between 0x10 and 0xf0 is 0x80, and 0x20 and 0xe0 as well.
        assert_eq!(over(0x102030, 0xf0e0d0, 128) >> 16, 0x80);
    }

    #[test]
    fn widen_channel_repeats_the_high_bits() {
        assert_eq!(widen_channel(0x1f, 5), 0xff);
        assert_eq!(widen_channel(0, 5), 0);
        assert_eq!(widen_channel(0b100, 3), 0b1001_0010);
        assert_eq!(widen_channel(0x3f, 6), 0xff);
        assert_eq!(widen_channel(0xa, 4), 0xaa);
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "1 to 8 bits")]
    fn widen_channel_refuses_a_zero_bit_channel() {
        // Without the check this would loop forever.
        widen_channel(0, 0);
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "1 to 8 bits")]
    fn widen_channel_refuses_more_than_eight_bits() {
        widen_channel(0, 9);
    }

    #[test]
    fn gray_ramp_runs_from_black_to_white() {
        assert_eq!(gray_ramp(2), [0xff00_0000, 0xffff_ffff]);
        assert_eq!(gray_ramp(16)[1], 0xff11_1111);
        assert_eq!(gray_ramp(1), [0xff00_0000]);
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
        let values = planar_pixels(&data, 3, 2, 1, 24, |plane, y| plane * 2 + y).unwrap();
        let row = [0x24_9249, 0x49_2492, 0x92_4924];
        assert_eq!(values, [row, row].concat());
    }
}
