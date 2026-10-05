//! GameCube and Wii GX texture formats: images stored as blocks in raster
//! order, and the 16-bit palettes of the indexed formats.
//!
//! Sources:
//! - Block sizes, bit layouts and the CMPR sub-block order: PuyoTools'
//!   `PuyoTools.Core/Textures/Gvr` pixel and palette codecs
//!   (<https://github.com/nickworonekin/puyotools>, MIT licence, notice
//!   below), cross-checked against the mkwiiki "Image Formats" page for TPL
//!   textures (<https://wiki.tockdom.com/wiki/Image_Formats>, facts only) and
//!   YAGCD chapter 15 (<https://hitmen.c02.at/files/yagcd/yagcd/chap15.html>,
//!   facts only).
//! - Expanding 3, 4, 5 and 6-bit channels by repeating the high bits (so
//!   `0x1f` becomes `0xff`) matches the 5-to-8-bit scaling `playstation.rs`
//!   observed from `recoil2png`; the GX documents leave the rounding open.
//!
//! The formats are the ones Sega's GVR textures and Nintendo's TPL libraries
//! share, and the numbers [`Format::from_code`] accepts are the codes both
//! files store. Pixels come out as `0xAARRGGBB`; flattening the alpha is the
//! caller's business (`image::over_fill`). Mipmaps are not read.

// Parts of this file follow PuyoTools.Core/Textures/Gvr
// (PuyoTools, https://github.com/nickworonekin/puyotools):
//
// MIT License
//
// Copyright (c) 2020 Nick Woronekin
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use alloc::vec::Vec;

use crate::bytes::be16;

/// How a texture's pixels are stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Format {
    /// 4-bit gray, 8x8 blocks.
    I4,
    /// 8-bit gray, 8x4 blocks.
    I8,
    /// 4-bit alpha (high nibble) and 4-bit gray, 8x4 blocks.
    IA4,
    /// 8-bit alpha then 8-bit gray, 4x4 blocks.
    IA8,
    /// 5-6-5 color, 4x4 blocks.
    Rgb565,
    /// 15-bit color, or 3-bit alpha and 12-bit color, 4x4 blocks.
    Rgb5A3,
    /// 8-bit alpha, red, green and blue, 4x4 blocks.
    Rgba32,
    /// 4-bit palette index, 8x8 blocks.
    C4,
    /// 8-bit palette index, 8x4 blocks.
    C8,
    /// 14-bit palette index, 4x4 blocks.
    C14X2,
    /// Two-bit DXT1 blocks, 8x8 pixels of four 4x4 sub-blocks.
    Cmpr,
}

impl Format {
    /// The format a TPL or GVR file names by `code`.
    pub(crate) fn from_code(code: u32) -> Option<Self> {
        Some(match code {
            0 => Self::I4,
            1 => Self::I8,
            2 => Self::IA4,
            3 => Self::IA8,
            4 => Self::Rgb565,
            5 => Self::Rgb5A3,
            6 => Self::Rgba32,
            8 => Self::C4,
            9 => Self::C8,
            10 => Self::C14X2,
            14 => Self::Cmpr,
            _ => return None,
        })
    }

    /// Colors in the palette an indexed format reads, `None` for the others.
    pub(crate) fn palette_len(self) -> Option<usize> {
        match self {
            Self::C4 => Some(16),
            Self::C8 => Some(256),
            Self::C14X2 => Some(1 << 14),
            _ => None,
        }
    }

    /// Width and height of one block, in pixels.
    fn block(self) -> (usize, usize) {
        match self {
            Self::I4 | Self::C4 | Self::Cmpr => (8, 8),
            Self::I8 | Self::IA4 | Self::C8 => (8, 4),
            Self::IA8 | Self::Rgb565 | Self::Rgb5A3 | Self::Rgba32 | Self::C14X2 => (4, 4),
        }
    }

    /// Bytes of one block.
    fn block_len(self) -> usize {
        let (width, height) = self.block();
        let bits = match self {
            Self::I4 | Self::C4 | Self::Cmpr => 4,
            Self::I8 | Self::IA4 | Self::C8 => 8,
            Self::IA8 | Self::Rgb565 | Self::Rgb5A3 | Self::C14X2 => 16,
            Self::Rgba32 => 32,
        };
        width * height * bits / 8
    }

    /// Bytes of a `width` x `height` image, which is stored in whole blocks.
    pub(crate) fn data_len(self, width: usize, height: usize) -> usize {
        let (block_width, block_height) = self.block();
        width.div_ceil(block_width) * height.div_ceil(block_height) * self.block_len()
    }
}

/// How the entries of a palette are stored: always 16 bits, big-endian.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PaletteFormat {
    IA8,
    Rgb565,
    Rgb5A3,
}

impl PaletteFormat {
    /// The palette format a TPL or GVR file names by `code`.
    pub(crate) fn from_code(code: u32) -> Option<Self> {
        match code {
            0 => Some(Self::IA8),
            1 => Some(Self::Rgb565),
            2 => Some(Self::Rgb5A3),
            _ => None,
        }
    }
}

/// `0xAARRGGBB` from four 8-bit channels.
fn argb(alpha: u32, r: u32, g: u32, b: u32) -> u32 {
    alpha << 24 | r << 16 | g << 8 | b
}

/// A `bits`-bit value stretched to 8 bits by repeating its high bits.
fn widen(value: u32, bits: u32) -> u32 {
    let mut wide = value << (8 - bits);
    let mut have = bits;
    while have < 8 {
        wide |= wide >> have;
        have *= 2;
    }
    wide
}

/// The color of a 16-bit palette entry or RGB565/RGB5A3 pixel.
fn color16(format: PaletteFormat, word: u16) -> u32 {
    let word = u32::from(word);
    match format {
        PaletteFormat::IA8 => {
            let gray = word & 0xff;
            argb(word >> 8, gray, gray, gray)
        }
        PaletteFormat::Rgb565 => argb(
            255,
            widen(word >> 11, 5),
            widen(word >> 5 & 63, 6),
            widen(word & 31, 5),
        ),
        PaletteFormat::Rgb5A3 if word & 0x8000 != 0 => argb(
            255,
            widen(word >> 10 & 31, 5),
            widen(word >> 5 & 31, 5),
            widen(word & 31, 5),
        ),
        PaletteFormat::Rgb5A3 => argb(
            widen(word >> 12 & 7, 3),
            widen(word >> 8 & 15, 4),
            widen(word >> 4 & 15, 4),
            widen(word & 15, 4),
        ),
    }
}

/// The first `count` entries of a palette stored as `format`, or `None` if
/// `data` is too short.
pub(crate) fn decode_palette(format: PaletteFormat, data: &[u8], count: usize) -> Option<Vec<u32>> {
    (0..count)
        .map(|i| be16(data, i * 2).map(|word| color16(format, word)))
        .collect()
}

/// A palette of `len` grays from black to white, for indexed textures whose
/// palette is not at hand.
pub(crate) fn gray_ramp(len: usize) -> Vec<u32> {
    (0..len)
        .map(|i| {
            let gray = (i * 255 / (len - 1).max(1)) as u32;
            argb(255, gray, gray, gray)
        })
        .collect()
}

/// A gray `value` of 8 bits, opaque.
fn gray(value: u32) -> u32 {
    argb(255, value, value, value)
}

/// The pixels of a `width` x `height` image stored as `format`, row-major.
/// `palette` is read by the indexed formats and must hold every index the
/// data uses. `None` if `data` is too short or an index is outside `palette`.
pub(crate) fn decode(
    format: Format,
    width: usize,
    height: usize,
    data: &[u8],
    palette: &[u32],
) -> Option<Vec<u32>> {
    let (block_width, block_height) = format.block();
    let blocks_per_row = width.div_ceil(block_width);
    let data = data.get(..format.data_len(width, height))?;
    let mut pixels = alloc::vec![0; width * height];
    let mut tile = [0u32; 64];
    for (index, block) in data.chunks_exact(format.block_len()).enumerate() {
        decode_block(format, block, palette, &mut tile)?;
        let left = index % blocks_per_row * block_width;
        let top = index / blocks_per_row * block_height;
        for y in 0..block_height.min(height - top) {
            let row = &tile[y * block_width..][..block_width.min(width - left)];
            pixels[(top + y) * width + left..][..row.len()].copy_from_slice(row);
        }
    }
    Some(pixels)
}

/// One block's pixels, row-major in `tile`.
fn decode_block(format: Format, block: &[u8], palette: &[u32], tile: &mut [u32; 64]) -> Option<()> {
    let look_up = |index: usize| palette.get(index).copied();
    match format {
        Format::I4 => {
            for (i, pixel) in tile.iter_mut().enumerate() {
                *pixel = gray(nibble(block, i) * 17);
            }
        }
        Format::C4 => {
            for (i, pixel) in tile.iter_mut().enumerate() {
                *pixel = look_up(nibble(block, i) as usize)?;
            }
        }
        Format::I8 => {
            for (pixel, &v) in tile.iter_mut().zip(block) {
                *pixel = gray(u32::from(v));
            }
        }
        Format::IA4 => {
            for (pixel, &v) in tile.iter_mut().zip(block) {
                let v = u32::from(v);
                *pixel = argb((v >> 4) * 17, (v & 15) * 17, (v & 15) * 17, (v & 15) * 17);
            }
        }
        Format::C8 => {
            for (pixel, &v) in tile.iter_mut().zip(block) {
                *pixel = look_up(usize::from(v))?;
            }
        }
        Format::IA8 => decode_words(PaletteFormat::IA8, block, tile),
        Format::Rgb565 => decode_words(PaletteFormat::Rgb565, block, tile),
        Format::Rgb5A3 => decode_words(PaletteFormat::Rgb5A3, block, tile),
        Format::C14X2 => {
            for (pixel, word) in tile.iter_mut().zip(block.as_chunks::<2>().0) {
                *pixel = look_up(usize::from(u16::from_be_bytes(*word) & 0x3fff))?;
            }
        }
        Format::Rgba32 => {
            // Sixteen alpha and red bytes, then sixteen green and blue bytes.
            let (ar, gb) = block.split_at(32);
            for ((pixel, ar), gb) in tile.iter_mut().zip(ar.chunks(2)).zip(gb.chunks(2)) {
                *pixel = argb(ar[0].into(), ar[1].into(), gb[0].into(), gb[1].into());
            }
        }
        Format::Cmpr => {
            // Four 4x4 sub-blocks (left to right, top to bottom) in an 8x8 block.
            for (sub, bytes) in block.chunks(8).enumerate() {
                let colors = cmpr_colors(bytes);
                let (left, top) = (sub % 2 * 4, sub / 2 * 4);
                for y in 0..4 {
                    for x in 0..4 {
                        let code = bytes[4 + y] >> (6 - 2 * x) & 3;
                        tile[(top + y) * 8 + left + x] = colors[usize::from(code)];
                    }
                }
            }
        }
    }
    Some(())
}

/// The `index`th 4-bit value of `bytes`, high nibble first.
fn nibble(bytes: &[u8], index: usize) -> u32 {
    u32::from(bytes[index / 2] >> (4 - index % 2 * 4) & 15)
}

/// Pixels stored as big-endian 16-bit words.
fn decode_words(format: PaletteFormat, block: &[u8], tile: &mut [u32; 64]) {
    for (pixel, word) in tile.iter_mut().zip(block.as_chunks::<2>().0) {
        *pixel = color16(format, u16::from_be_bytes(*word));
    }
}

/// The four colors of a CMPR sub-block: two RGB565 endpoints, then either two
/// thirds blends, or (when the first endpoint is not the larger) their
/// average and transparent black.
fn cmpr_colors(sub_block: &[u8]) -> [u32; 4] {
    let (word0, word1) = (
        be16(sub_block, 0).unwrap_or(0),
        be16(sub_block, 2).unwrap_or(0),
    );
    let (c0, c1) = (
        color16(PaletteFormat::Rgb565, word0),
        color16(PaletteFormat::Rgb565, word1),
    );
    let mix = |weight0: u32, weight1: u32, divisor: u32| {
        let channel =
            |shift: u32| ((c0 >> shift & 255) * weight0 + (c1 >> shift & 255) * weight1) / divisor;
        argb(255, channel(16), channel(8), channel(0))
    };
    if word0 > word1 {
        [c0, c1, mix(2, 1, 3), mix(1, 2, 3)]
    } else {
        [c0, c1, mix(1, 1, 2), 0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widening_repeats_the_high_bits() {
        assert_eq!(widen(0x1f, 5), 0xff);
        assert_eq!(widen(0, 5), 0);
        assert_eq!(widen(0b100, 3), 0b1001_0010);
        assert_eq!(widen(0x3f, 6), 0xff);
        assert_eq!(widen(0xa, 4), 0xaa);
    }

    #[test]
    fn rgb5a3_has_an_opaque_and_a_translucent_branch() {
        // Opaque: red 31, green 0, blue 15.
        assert_eq!(color16(PaletteFormat::Rgb5A3, 0xfc0f), 0xffff_007b);
        // Alpha 3 bits (4 = 0x92), red 15, green 0, blue 8.
        assert_eq!(color16(PaletteFormat::Rgb5A3, 0x4f08), 0x92ff_0088);
    }

    #[test]
    fn rgba32_stores_alpha_red_then_green_blue_in_a_block() {
        let mut block = [0u8; 64];
        block[0..2].copy_from_slice(&[0x80, 0x11]); // pixel 0: alpha, red
        block[32..34].copy_from_slice(&[0x22, 0x33]); // pixel 0: green, blue
        block[2..4].copy_from_slice(&[0xff, 0x44]); // pixel 1
        block[34..36].copy_from_slice(&[0x55, 0x66]);
        let pixels = decode(Format::Rgba32, 4, 4, &block, &[]).unwrap();
        assert_eq!(pixels[0], 0x8011_2233);
        assert_eq!(pixels[1], 0xff44_5566);
    }

    #[test]
    fn blocks_run_in_raster_order_and_images_are_cropped() {
        // I8 uses 8x4 blocks: a 10x5 image takes 2 x 2 blocks of 32 bytes.
        let mut data = alloc::vec![0u8; 4 * 32];
        data[0] = 1; // block 0 (left), pixel (0, 0)
        data[32] = 2; // block 1 (right), pixel (8, 0)
        data[64 + 7] = 3; // block 2 (below the left one), pixel (7, 4)
        data[96 + 8] = 4; // block 3, pixel (0, 1) of the right block: (8, 5), cropped
        let pixels = decode(Format::I8, 10, 5, &data, &[]).unwrap();
        assert_eq!(pixels.len(), 50);
        assert_eq!(pixels[0] & 0xff, 1);
        assert_eq!(pixels[8] & 0xff, 2);
        assert_eq!(pixels[4 * 10 + 7] & 0xff, 3);
        assert_eq!(decode(Format::I8, 10, 5, &data[..127], &[]), None);
    }

    #[test]
    fn four_bit_formats_put_the_left_pixel_in_the_high_nibble() {
        let mut data = [0u8; 32];
        data[0] = 0x1f;
        let pixels = decode(Format::I4, 8, 8, &data, &[]).unwrap();
        assert_eq!((pixels[0] & 0xff, pixels[1] & 0xff), (0x11, 0xff));
        let palette = gray_ramp(16);
        let pixels = decode(Format::C4, 8, 8, &data, &palette).unwrap();
        assert_eq!((pixels[0] & 0xff, pixels[1] & 0xff), (17, 255));
    }

    #[test]
    fn ia4_has_alpha_in_the_high_nibble_and_c8_checks_the_palette() {
        let mut data = [0u8; 32];
        data[0] = 0x8f;
        assert_eq!(
            decode(Format::IA4, 8, 4, &data, &[]).unwrap()[0],
            0x88ff_ffff
        );
        data[1] = 3;
        assert!(decode(Format::C8, 8, 4, &data, &[0, 0]).is_none());
        assert!(decode(Format::C8, 8, 4, &data, &gray_ramp(256)).is_some());
    }

    #[test]
    fn c14x2_ignores_the_top_two_bits() {
        let mut data = [0u8; 32];
        data[0..2].copy_from_slice(&0xc002u16.to_be_bytes());
        let palette = [1, 2, 3];
        assert_eq!(decode(Format::C14X2, 4, 4, &data, &palette).unwrap()[0], 3);
    }

    #[test]
    fn cmpr_sub_blocks_use_four_colors_or_three_and_transparent() {
        // Sub-block 0: white (0xffff) and black, so four colors: indices
        // 0, 1, 2, 3 along the first row; the other sub-blocks stay zero.
        let mut data = [0u8; 32];
        data[0..4].copy_from_slice(&[0xff, 0xff, 0x00, 0x00]);
        data[4] = 0b00_01_10_11;
        // Sub-block 1 (right): black first, then white: three colors and
        // transparent. Index 3 on its first row's last pixel.
        data[8..12].copy_from_slice(&[0x00, 0x00, 0xff, 0xff]);
        data[12] = 0b10_10_10_11;
        let pixels = decode(Format::Cmpr, 8, 8, &data, &[]).unwrap();
        assert_eq!(
            &pixels[0..4],
            &[0xffff_ffff, 0xff00_0000, 0xffaa_aaaa, 0xff55_5555]
        );
        // Average of black and white: 255 / 2 rounded down.
        assert_eq!(pixels[4], 0xff7f_7f7f);
        assert_eq!(pixels[6], 0xff7f_7f7f);
        assert_eq!(
            pixels[7], 0,
            "index 3 is transparent in the three-color mode"
        );
    }
}
