//! 3DS GPU textures: the pixel formats and the way their pixels are laid out.
//! The compressed formats are in `etc1.rs`.
//!
//! Sources: GBATEK, "3DS GPU Texture Formats" (format numbers, bit layouts of
//! each pixel), "3DS Video Texture Swizzling" (Z-order inside 8 x 8 tiles) and
//! "3DS GPU Internal Registers - Texturing registers" (texels in Z-order, mip
//! levels after the base level), <https://problemkaputt.de/gbatek.htm> (no
//! license, facts only). No sample file was available, so the layouts are
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
//! Two things here are guesses that no source states for these files, and both
//! are to be confirmed with a real file:
//! - The GPU's texture origin is its lower left corner, and textures are
//!   made for it, so the rows of a texture are stored bottom row first: this
//!   decoder turns them upside down. (The icons of SMDH files, which are not
//!   GPU textures, are stored top row first.) It is based on how textures are
//!   used on the 3DS.
//! - A CLIM whose size is not a power of two stores its picture in a larger,
//!   padded texture, and GBATEK does not say where the picture lies in it.
//!   [`Texture::image`] turns the whole padded texture upside down and then
//!   keeps its upper left corner, so the picture is taken to be the last
//!   rows of the stored data, with the padding in the first.

use super::etc1;
use crate::image::{check_size, over_fill};
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
    Etc1,
    Etc1A4,
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
            12 => Self::Etc1,
            13 => Self::Etc1A4,
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
            Self::L4 | Self::A4 | Self::Etc1 => 4,
            Self::Etc1A4 => 8,
        }
    }

    /// Bytes of a `width` x `height` texture (both multiples of 8), or `None`
    /// if that does not fit in memory.
    fn data_len(self, width: usize, height: usize) -> Option<usize> {
        Some(width.checked_mul(height)?.checked_mul(self.bits())? / 8)
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
            Self::Etc1 | Self::Etc1A4 => unreachable!("compressed blocks decode in `tile`"),
        }
    }

    /// The 64 pixels of tile number `tile` of `data`, row by row.
    fn tile(self, data: &[u8], tile: usize) -> [[u8; 4]; 64] {
        let mut pixels = [[0; 4]; 64];
        if matches!(self, Self::Etc1 | Self::Etc1A4) {
            // Four blocks of 4 x 4 pixels in Z-order.
            let block_len = self.bits() * 16 / 8;
            for block in 0..4 {
                let bytes = &data[(tile * 4 + block) * block_len..][..block_len];
                let words = bytes.as_chunks::<8>().0;
                let word = |i: usize| u64::from_le_bytes(words[i]);
                let decoded = match self {
                    Self::Etc1A4 => etc1::decode_block(word(1), Some(word(0))),
                    _ => etc1::decode_block(word(0), None),
                };
                let (left, top) = (block % 2 * 4, block / 2 * 4);
                for (k, &pixel) in decoded.iter().enumerate() {
                    pixels[(top + k % 4) * TILE + left + k / 4] = pixel;
                }
            }
        } else {
            for y in 0..TILE {
                for x in 0..TILE {
                    let index = tile * TILE * TILE + morton_index(x as u32, y as u32) as usize;
                    pixels[y * TILE + x] = self.pixel(data, index);
                }
            }
        }
        pixels
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
            && self
                .format
                .data_len(width, height)
                .is_some_and(|len| self.data.len() >= len);
        if !sound {
            return Err(DecodeError::Unrecognized);
        }
        check_size(visible_width, visible_height)?;
        let tiles_per_row = width / TILE;
        let mut colors = alloc::vec![0; visible_width * visible_height];
        // Rows are stored bottom row first, so the picture's rows come from
        // the last tile rows.
        for tile_row in (height - visible_height) / TILE..height / TILE {
            for tile_column in 0..visible_width.div_ceil(TILE) {
                let tile = self
                    .format
                    .tile(self.data, tile_row * tiles_per_row + tile_column);
                for (n, &pixel) in tile.iter().enumerate() {
                    let x = tile_column * TILE + n % TILE;
                    let y = height - 1 - (tile_row * TILE + n / TILE);
                    if x < visible_width && y < visible_height {
                        colors[y * visible_width + x] = over_fill(
                            u32::from_be_bytes([0, pixel[0], pixel[1], pixel[2]]),
                            pixel[3],
                        );
                    }
                }
            }
        }
        Ok(Image::from_colors(
            visible_width as u32,
            visible_height as u32,
            colors.into_iter(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::TRANSPARENT_FILL;
    use alloc::vec::Vec;

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
        assert_eq!(Format::from_clim(10), Some(Format::Etc1));
        assert_eq!(Format::from_gpu(13), Some(Format::Etc1A4));
        assert_eq!(Format::from_gpu(14), None);
    }

    #[test]
    fn compressed_blocks_are_in_z_order_inside_a_tile() {
        // One tile of ETC1: four solid blocks, individual mode, base color
        // (n, n, n) widened plus 2 from table 0, for n = 1 to 4.
        let data: Vec<u8> = (1..=4u64)
            .flat_map(|n| {
                let gray = n << 60 | n << 56 | n << 52 | n << 48 | n << 44 | n << 40;
                gray.to_le_bytes()
            })
            .collect();
        let tile = Texture {
            format: Format::Etc1,
            width: 8,
            height: 8,
            data: &data,
        };
        let image = tile.image(8, 8).unwrap();
        let level = |n: u32| n * 17 + 2;
        let gray = |n: u32| level(n) * 0x01_0101;
        // Block 0 is stored at the top left, so it ends at the bottom left
        // of the upside-down picture; block 2 is stored below it.
        assert_eq!(image.get(0, 7), gray(1));
        assert_eq!(image.get(4, 7), gray(2));
        assert_eq!(image.get(0, 0), gray(3));
        assert_eq!(image.get(7, 3), gray(4));
        // ETC1A4 blocks are 16 bytes: the alpha, then the color block.
        let mut with_alpha = Vec::new();
        for block in data.as_chunks::<8>().0 {
            with_alpha.extend_from_slice(&[0; 8]);
            with_alpha.extend_from_slice(block);
        }
        let transparent = Texture {
            format: Format::Etc1A4,
            width: 8,
            height: 8,
            data: &with_alpha,
        };
        assert_eq!(transparent.image(8, 8).unwrap().get(0, 7), TRANSPARENT_FILL);
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
