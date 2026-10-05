//! 3DS GPU textures: the pixel formats and the way their pixels are laid out.
//!
//! Sources: GBATEK, "3DS GPU Texture Formats" (format numbers, bit layouts of
//! each pixel), "3DS Video Texture Swizzling" (Z-order inside 8 x 8 tiles) and
//! "3DS GPU Internal Registers - Texturing registers" (texels in Z-order, mip
//! levels after the base level), <https://problemkaputt.de/gbatek.htm> (no
//! licence, facts only). No sample file was available, so the layouts are
//! unverified; they are checked by unit tests built from the documented layouts.
//!
//! A texture is a raster of 8 x 8 pixel tiles, left to right and top to
//! bottom, and inside a tile the pixels are in Z-order: the bits of x take
//! the even bits of the pixel number and the bits of y the odd ones. Pixels
//! are little-endian words, so `RGBA8` is stored as the bytes A, B, G, R and
//! `RGB8` as B, G, R; the two 4-bit pixels of a byte are the low nibble first
//! (GBATEK does not say; it is the order of the DS and of a little-endian
//! GPU). The two formats that carry only one of the colors, `L` and `A`, are
//! drawn as gray and as black with alpha. `HILO8` (two normal-map channels)
//! is red and green with no blue.
//!
//! The GPU's texture origin is its lower left corner, and textures are
//! made for it, so the rows of a texture are stored bottom row first: this
//! decoder turns them upside down. That is the one guess here that no source
//! states for these files (the icons of SMDH files, which are not GPU
//! textures, are stored top row first); it is based on how textures are
//! used on the 3DS and is to be confirmed with a real file.

use alloc::vec::Vec;

use crate::image::{TRANSPARENT_FILL, check_size};
use crate::morton::morton_index;
use crate::{DecodeError, Image};

/// Pixels to a side of a tile.
const TILE: usize = 8;

/// A GPU texture format, by its pixel layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Format {
    Rgba8,
    Rgb8,
    Rgba5551,
    Rgb565,
    Rgba4,
    La8,
    Hilo8,
    L8,
    A8,
    La4,
    L4,
    A4,
}

impl Format {
    /// The format with GPU number `number`.
    pub(super) fn from_gpu(number: u32) -> Option<Self> {
        Some(match number {
            0 => Self::Rgba8,
            1 => Self::Rgb8,
            2 => Self::Rgba5551,
            3 => Self::Rgb565,
            4 => Self::Rgba4,
            5 => Self::La8,
            6 => Self::Hilo8,
            7 => Self::L8,
            8 => Self::A8,
            9 => Self::La4,
            10 => Self::L4,
            11 => Self::A4,
            _ => return None,
        })
    }

    /// The format with the number a CLIM file gives it, which is not the
    /// GPU's: GBATEK's table `07h,08h,09h,05h,06h,03h,01h,02h,04h,00h,0Ch,0Dh,0Ah,0Bh`
    /// maps CLIM numbers to GPU numbers.
    pub(super) fn from_clim(number: u8) -> Option<Self> {
        const GPU: [u32; 14] = [7, 8, 9, 5, 6, 3, 1, 2, 4, 0, 12, 13, 10, 11];
        Self::from_gpu(*GPU.get(usize::from(number))?)
    }

    fn bits(self) -> usize {
        match self {
            Self::Rgba8 => 32,
            Self::Rgb8 => 24,
            Self::Rgba5551 | Self::Rgb565 | Self::Rgba4 | Self::La8 | Self::Hilo8 => 16,
            Self::L8 | Self::A8 | Self::La4 => 8,
            Self::L4 | Self::A4 => 4,
        }
    }

    /// Bytes of a `width` x `height` texture (both multiples of 8).
    pub(super) fn data_len(self, width: usize, height: usize) -> usize {
        width * height * self.bits() / 8
    }

    /// Pixel number `index` of `data`, as red, green, blue and alpha.
    fn pixel(self, data: &[u8], index: usize) -> [u8; 4] {
        let byte = |i: usize| data[i];
        let word = |i: usize| u16::from_le_bytes([data[i * 2], data[i * 2 + 1]]);
        let widen4 = |v: u16| (v * 17) as u8;
        let widen5 = |v: u16| (v << 3 | v >> 2) as u8;
        let widen6 = |v: u16| (v << 2 | v >> 4) as u8;
        let nibble = |i: usize| u16::from(data[i / 2] >> (i % 2 * 4) & 15);
        match self {
            Self::Rgba8 => {
                let at = index * 4;
                [data[at + 3], data[at + 2], data[at + 1], data[at]]
            }
            Self::Rgb8 => {
                let at = index * 3;
                [data[at + 2], data[at + 1], data[at], 255]
            }
            Self::Rgba5551 => {
                let w = word(index);
                [
                    widen5(w >> 11),
                    widen5(w >> 6 & 31),
                    widen5(w >> 1 & 31),
                    if w & 1 != 0 { 255 } else { 0 },
                ]
            }
            Self::Rgb565 => {
                let w = word(index);
                [widen5(w >> 11), widen6(w >> 5 & 63), widen5(w & 31), 255]
            }
            Self::Rgba4 => {
                let w = word(index);
                [
                    widen4(w >> 12),
                    widen4(w >> 8 & 15),
                    widen4(w >> 4 & 15),
                    widen4(w & 15),
                ]
            }
            Self::La8 => {
                let w = word(index);
                let l = (w >> 8) as u8;
                [l, l, l, w as u8]
            }
            Self::Hilo8 => {
                let w = word(index);
                [(w >> 8) as u8, w as u8, 0, 255]
            }
            Self::L8 => {
                let l = byte(index);
                [l, l, l, 255]
            }
            Self::A8 => [0, 0, 0, byte(index)],
            Self::La4 => {
                let b = u16::from(byte(index));
                let l = widen4(b >> 4);
                [l, l, l, widen4(b & 15)]
            }
            Self::L4 => {
                let l = widen4(nibble(index));
                [l, l, l, 255]
            }
            Self::A4 => [0, 0, 0, widen4(nibble(index))],
        }
    }
}

/// A texture of `width` x `height` pixels, both multiples of 8, whose
/// pixels are `data`.
pub(super) struct Texture<'a> {
    pub format: Format,
    pub width: usize,
    pub height: usize,
    pub data: &'a [u8],
}

impl Texture<'_> {
    /// The upper left `visible_width` x `visible_height` pixels of the
    /// picture, which are not more than the texture holds. Pixels with alpha
    /// are composited onto the shared transparent fill.
    pub(super) fn image(
        &self,
        visible_width: usize,
        visible_height: usize,
    ) -> Result<Image, DecodeError> {
        let (width, height) = (self.width, self.height);
        let sound = width % TILE == 0
            && height % TILE == 0
            && visible_width <= width
            && visible_height <= height
            && self.data.len() >= self.format.data_len(width, height);
        if !sound {
            return Err(DecodeError::Unrecognized);
        }
        check_size(visible_width, visible_height)?;
        let tiles_per_row = width / TILE;
        let mut colors = Vec::with_capacity(visible_width * visible_height);
        for y in 0..visible_height {
            // Rows are stored bottom row first.
            let stored_y = height - 1 - y;
            for x in 0..visible_width {
                let tile = stored_y / TILE * tiles_per_row + x / TILE;
                let within = morton_index((x % TILE) as u32, (stored_y % TILE) as u32);
                let index = tile * TILE * TILE + within as usize;
                colors.push(composite(self.format.pixel(self.data, index)));
            }
        }
        Ok(Image::from_colors(
            visible_width as u32,
            visible_height as u32,
            colors.into_iter(),
        ))
    }
}

/// `rgba` laid over the transparent fill, as 0xRRGGBB.
fn composite([r, g, b, a]: [u8; 4]) -> u32 {
    let [_, fill_r, fill_g, fill_b] = TRANSPARENT_FILL.to_be_bytes();
    let alpha = u32::from(a);
    let mix = |color: u8, fill: u8| {
        (u32::from(color) * alpha + u32::from(fill) * (255 - alpha) + 127) / 255
    };
    mix(r, fill_r) << 16 | mix(g, fill_g) << 8 | mix(b, fill_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 16 x 8 texture (two tiles) of `format` from `data`.
    fn texture(format: Format, data: &[u8]) -> Texture<'_> {
        Texture {
            format,
            width: 16,
            height: 8,
            data,
        }
    }

    #[test]
    fn pixels_are_in_z_order_inside_tiles_and_tiles_in_rows() {
        // L8: pixel number n of the stored data has the value n.
        let data: Vec<u8> = (0..128).collect();
        let image = texture(Format::L8, &data).image(16, 8).unwrap();
        let gray = |n: u32| n * 0x01_0101;
        // The stored bottom row (row 0) is the bottom row of the picture,
        // which is picture row 7: x takes the even bits of the number, y
        // the odd ones.
        assert_eq!(image.get(0, 7), gray(0));
        assert_eq!(image.get(1, 7), gray(1));
        assert_eq!(image.get(0, 6), gray(2));
        assert_eq!(image.get(1, 6), gray(3));
        assert_eq!(image.get(2, 7), gray(4));
        assert_eq!(image.get(4, 7), gray(16));
        assert_eq!(image.get(0, 5), gray(8));
        assert_eq!(image.get(7, 0), gray(63));
        // The second tile follows the first.
        assert_eq!(image.get(8, 7), gray(64));
        assert_eq!(image.get(9, 7), gray(65));
    }

    #[test]
    fn pixel_formats_unpack_their_channels() {
        let pixel = |format: Format, data: &[u8]| format.pixel(data, 0);
        // Words and bytes are little-endian: RGBA8 is stored A, B, G, R.
        assert_eq!(
            pixel(Format::Rgba8, &[0x44, 0x33, 0x22, 0x11]),
            [0x11, 0x22, 0x33, 0x44]
        );
        assert_eq!(pixel(Format::Rgb8, &[3, 2, 1]), [1, 2, 3, 255]);
        // 0xF801: red 31, green 0, blue 0, alpha 1.
        assert_eq!(pixel(Format::Rgba5551, &[0x01, 0xf8]), [255, 0, 0, 255]);
        assert_eq!(pixel(Format::Rgba5551, &[0x00, 0xf8])[3], 0);
        assert_eq!(pixel(Format::Rgb565, &[0xe0, 0x07]), [0, 255, 0, 255]);
        assert_eq!(
            pixel(Format::Rgba4, &[0x84, 0x12]),
            [0x11, 0x22, 0x88, 0x44]
        );
        assert_eq!(pixel(Format::La8, &[0x40, 0x80]), [0x80, 0x80, 0x80, 0x40]);
        assert_eq!(pixel(Format::Hilo8, &[0x20, 0x10]), [0x10, 0x20, 0, 255]);
        assert_eq!(pixel(Format::L8, &[7]), [7, 7, 7, 255]);
        assert_eq!(pixel(Format::A8, &[7]), [0, 0, 0, 7]);
        assert_eq!(pixel(Format::La4, &[0x3c]), [0x33, 0x33, 0x33, 0xcc]);
        // Two 4-bit pixels to a byte, low nibble first.
        assert_eq!(Format::L4.pixel(&[0x2f], 0), [0xff, 0xff, 0xff, 255]);
        assert_eq!(Format::L4.pixel(&[0x2f], 1), [0x22, 0x22, 0x22, 255]);
        assert_eq!(Format::A4.pixel(&[0x2f], 1), [0, 0, 0, 0x22]);
    }

    #[test]
    fn clim_numbers_map_to_gpu_formats() {
        assert_eq!(Format::from_clim(0), Some(Format::L8));
        assert_eq!(Format::from_clim(5), Some(Format::Rgb565));
        assert_eq!(Format::from_clim(9), Some(Format::Rgba8));
        assert_eq!(Format::from_clim(13), Some(Format::A4));
        assert_eq!(Format::from_clim(14), None);
        assert_eq!(Format::from_gpu(12), None);
    }

    #[test]
    fn alpha_is_composited_onto_the_fill_and_sizes_are_checked() {
        let transparent = texture(Format::Rgba8, &[0; 16 * 8 * 4])
            .image(16, 8)
            .unwrap();
        assert_eq!(transparent.get(0, 0), TRANSPARENT_FILL);
        let opaque = [0xff; 16 * 8 * 4];
        assert_eq!(
            texture(Format::Rgba8, &opaque)
                .image(16, 8)
                .unwrap()
                .get(5, 5),
            0xff_ffff
        );
        // Short data, sizes that are not tiles, a picture larger than the texture.
        assert!(texture(Format::Rgba8, &opaque[..100]).image(16, 8).is_err());
        assert!(texture(Format::Rgba8, &opaque).image(17, 8).is_err());
        let odd = Texture {
            format: Format::L8,
            width: 12,
            height: 8,
            data: &[0; 96],
        };
        assert!(odd.image(12, 8).is_err());
        // A cropped picture.
        assert_eq!(
            texture(Format::L8, &[9; 128]).image(3, 2).unwrap().width(),
            3
        );
    }
}
