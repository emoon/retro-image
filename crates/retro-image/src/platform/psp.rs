//! Sony PSP and PS3: GIM pictures (`.gim`), the first picture of a file.
//!
//! Sources:
//! - Layout: PuyoTools' `PuyoTools.Core/Textures/Gim` (`GimTextureDecoder.cs`,
//!   `GimPixelFormat.cs`, `GimPaletteFormat.cs` and the pixel and palette
//!   codecs; <https://github.com/nickworonekin/puyotools>, MIT license, notice
//!   below). The magic is `MIG.00.1PSP` (little-endian, PSP) or `.GIM1.00PSP`
//!   (big-endian, PS3), followed by chunks from offset 16. Each chunk starts
//!   with a 16-bit type, and has its total length at +8: type 2 is the file
//!   root (its +4 is the end-of-file offset minus 16) and type 3 the picture,
//!   both only a 16-byte header in front of their children; type 4 is an
//!   image and type 5 a palette, with 16-bit fields at +0x14 (pixel or
//!   palette format), +0x16 (1 if the pixels are swizzled), +0x18 and +0x1a
//!   (width and height of an image, color count of a palette), +0x1e and
//!   +0x20 (row stride and height alignment) and their data at +0x50.
//!   Swizzled data is stored in blocks of 16 bytes by 8 rows, blocks in
//!   raster order.
//! - Checked on the twelve files of Sembiance's `psxGIM` folder
//!   (<https://sembiance.com/fileFormatSamples/image/psxGIM/>): 256x256
//!   swizzled 8-bit pictures with a 256-color RGBA8888 palette. The other pixel
//!   formats, the big-endian form and unswizzled data follow PuyoTools only.
//!
//! Read: RGB565, RGBA5551, RGBA4444 and RGBA8888 pixels and 4 and 8-bit indices
//! (palette in the file). Not read: 16 and 32-bit indices and the DXT formats.
//! Alpha is kept.

// Parts of this file follow PuyoTools.Core/Textures/Gim
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

use crate::image::{bgr555, check_size, widen_channel};
use crate::{DecodeError, Format, Image};

pub(super) static FORMATS: &[Format] =
    &[Format::new("PSP", "GIM", &["gim"], decode_gim).signature()];

/// The header in front of the chunks, and the most chunks read.
const HEADER_LEN: usize = 16;
const MAX_CHUNKS: usize = 64;
/// Offset of an image or palette chunk's data.
const DATA_AT: usize = 0x50;

#[derive(Clone, Copy)]
enum Endian {
    Little,
    Big,
}

impl Endian {
    fn u16(self, data: &[u8], at: usize) -> Option<u16> {
        let bytes = data.get(at..at.checked_add(2)?)?.try_into().ok()?;
        Some(match self {
            Self::Little => u16::from_le_bytes(bytes),
            Self::Big => u16::from_be_bytes(bytes),
        })
    }

    fn u32(self, data: &[u8], at: usize) -> Option<u32> {
        let bytes = data.get(at..at.checked_add(4)?)?.try_into().ok()?;
        Some(match self {
            Self::Little => u32::from_le_bytes(bytes),
            Self::Big => u32::from_be_bytes(bytes),
        })
    }
}

/// Bytes per pixel or palette entry for formats 0 to 3 (RGB565, RGBA5551,
/// RGBA4444, RGBA8888).
fn color_len(format: u16) -> Option<usize> {
    match format {
        0..=2 => Some(2),
        3 => Some(4),
        _ => None,
    }
}

/// `0xAARRGGBB` from a color of format 0 to 3 at the start of `bytes`.
fn color(endian: Endian, format: u16, bytes: &[u8]) -> Option<u32> {
    if format == 3 {
        let [r, g, b, a] = <[u8; 4]>::try_from(bytes.get(..4)?).ok()?;
        return Some(u32::from_be_bytes([a, r, g, b]));
    }
    let v = endian.u16(bytes, 0)?;
    let channel =
        |shift: u32, bits: u32| widen_channel(u32::from(v >> shift) & ((1 << bits) - 1), bits);
    Some(match format {
        0 => 0xff << 24 | channel(0, 5) << 16 | channel(5, 6) << 8 | channel(11, 5),
        1 => {
            let alpha = if v & 0x8000 != 0 { 0xff } else { 0 };
            alpha << 24 | bgr555(v)
        }
        _ => channel(12, 4) << 24 | channel(0, 4) << 16 | channel(4, 4) << 8 | channel(8, 4),
    })
}

/// A chunk found while walking the file.
struct Chunk<'a> {
    kind: u16,
    /// The chunk from its first byte to the end of the file.
    data: &'a [u8],
}

fn decode_gim(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let endian = match data.get(..8) {
        Some(b"MIG.00.1") => Endian::Little,
        Some(b".GIM1.00") => Endian::Big,
        _ => return Err(fail),
    };
    // The root chunk holds the end of the file; the rest is walked flat.
    if endian.u16(data, HEADER_LEN) != Some(2) {
        return Err(fail);
    }
    let end = (endian.u32(data, HEADER_LEN + 4).ok_or(fail)? as usize)
        .saturating_add(HEADER_LEN)
        .min(data.len());
    let mut chunks = Vec::new();
    let mut at = HEADER_LEN;
    while at < end && chunks.len() < MAX_CHUNKS {
        let kind = endian.u16(data, at).ok_or(fail)?;
        let len = endian.u32(data, at + 8).ok_or(fail)? as usize;
        if len == 0 {
            return Err(fail);
        }
        chunks.push(Chunk {
            kind,
            data: &data[at..],
        });
        at = at.checked_add(len).ok_or(fail)?;
    }

    let mut after_image = chunks.iter().skip_while(|c| c.kind != 4);
    let image = after_image.next().ok_or(fail)?.data;
    // The palette belongs to the first image: it lies before the next one.
    let palette = after_image
        .take_while(|c| c.kind != 4)
        .find(|c| c.kind == 5);

    let format = endian.u16(image, 0x14).ok_or(fail)?;
    let swizzled = endian.u16(image, 0x16).ok_or(fail)? == 1;
    let width = usize::from(endian.u16(image, 0x18).ok_or(fail)?);
    let height = usize::from(endian.u16(image, 0x1a).ok_or(fail)?);
    let stride_alignment = usize::from(endian.u16(image, 0x1e).ok_or(fail)?).max(1);
    let height_alignment = usize::from(endian.u16(image, 0x20).ok_or(fail)?).max(1);
    check_size(width, height)?;
    let bits = match format {
        0..=2 => 16,
        3 => 32,
        4 => 4,
        5 => 8,
        _ => return Err(fail),
    };
    let stride = (width * bits)
        .div_ceil(8)
        .next_multiple_of(stride_alignment);
    let rows = height.next_multiple_of(height_alignment);
    if swizzled && (stride % 16 != 0 || rows % 8 != 0) {
        return Err(fail);
    }
    let pixels = image.get(DATA_AT..).ok_or(fail)?;
    pixels
        .get(..stride.checked_mul(rows).ok_or(fail)?)
        .ok_or(fail)?;

    let colors = match (format, palette) {
        (4 | 5, Some(palette)) => Some(palette_colors(endian, palette.data).ok_or(fail)?),
        (4 | 5, None) => return Err(fail),
        _ => None,
    };
    // The byte `bx` of row `y`, undoing the swizzle (16 bytes by 8 rows).
    let byte_at = |bx: usize, y: usize| {
        let at = if swizzled {
            let block = bx / 16 + y / 8 * (stride / 16);
            block * 128 + bx % 16 + y % 8 * 16
        } else {
            y * stride + bx
        };
        pixels[at]
    };
    let argb = (0..height).flat_map(|y| (0..width).map(move |x| (x, y)));
    let argb = argb.map(|(x, y)| match (bits, &colors) {
        (4, Some(colors)) => {
            let byte = byte_at(x / 2, y);
            colors[usize::from(byte >> (x % 2 * 4) & 15)]
        }
        (8, Some(colors)) => colors[usize::from(byte_at(x, y))],
        _ => {
            let len = bits / 8;
            let mut texel = [0u8; 4];
            for (i, slot) in texel[..len].iter_mut().enumerate() {
                *slot = byte_at(x * len + i, y);
            }
            color(endian, format, &texel).unwrap_or(0)
        }
    });
    Ok(Image::from_argb(width as u32, height as u32, argb))
}

/// The colors of a palette chunk, padded to 256 entries so that any index of
/// an 8-bit picture finds one (missing entries are transparent).
fn palette_colors(endian: Endian, chunk: &[u8]) -> Option<Vec<u32>> {
    let format = endian.u16(chunk, 0x14)?;
    let len = color_len(format)?;
    let count = usize::from(endian.u16(chunk, 0x18)?);
    let entries = chunk.get(DATA_AT..)?;
    let mut colors = (0..count)
        .map(|i| color(endian, format, entries.get(i * len..)?))
        .collect::<Option<Vec<u32>>>()?;
    colors.resize(colors.len().max(256), 0);
    Some(colors)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What a test picture looks like.
    struct Picture<'a> {
        format: u16,
        width: u16,
        height: u16,
        swizzled: bool,
        stride_alignment: u16,
        pixels: &'a [u8],
        /// Palette format and its 4-byte entries.
        palette: Option<(u16, &'a [u8])>,
    }

    impl<'a> Picture<'a> {
        fn new(format: u16, width: u16, height: u16, pixels: &'a [u8]) -> Self {
            Self {
                format,
                width,
                height,
                swizzled: false,
                stride_alignment: 1,
                pixels,
                palette: None,
            }
        }
    }

    /// A little-endian GIM: root and picture headers, an image chunk and an
    /// optional palette chunk.
    fn gim(picture: &Picture) -> Vec<u8> {
        let chunk = |kind: u16, len: usize| {
            let mut c = Vec::new();
            c.extend_from_slice(&kind.to_le_bytes());
            c.extend_from_slice(&[0; 6]);
            c.extend_from_slice(&(len as u32).to_le_bytes());
            c.extend_from_slice(&[0; 4]);
            c
        };
        let mut image = chunk(4, DATA_AT + picture.pixels.len());
        image.resize(0x14, 0);
        for field in [
            picture.format,
            u16::from(picture.swizzled),
            picture.width,
            picture.height,
            0,
            picture.stride_alignment,
            1,
        ] {
            image.extend_from_slice(&field.to_le_bytes());
        }
        image.resize(DATA_AT, 0);
        image.extend_from_slice(picture.pixels);
        let mut rest = chunk(3, 0x10);
        rest.extend(image);
        if let Some((format, colors)) = picture.palette {
            let mut c = chunk(5, DATA_AT + colors.len());
            c.resize(0x14, 0);
            c.extend_from_slice(&format.to_le_bytes());
            c.extend_from_slice(&[0, 0]);
            c.extend_from_slice(&((colors.len() / 4) as u16).to_le_bytes());
            c.resize(DATA_AT, 0);
            c.extend_from_slice(colors);
            rest.extend(c);
        }
        let mut root = chunk(2, 0x10);
        root[4..8].copy_from_slice(&((16 + rest.len()) as u32).to_le_bytes());
        let mut data = b"MIG.00.1PSP\0\0\0\0\0".to_vec();
        data.extend(root);
        data.extend(rest);
        data
    }

    #[test]
    fn colors_use_the_psp_bit_layouts() {
        let le = Endian::Little;
        assert_eq!(color(le, 0, &0x001fu16.to_le_bytes()), Some(0xffff_0000));
        assert_eq!(color(le, 0, &0xf800u16.to_le_bytes()), Some(0xff00_00ff));
        assert_eq!(color(le, 1, &0x83e0u16.to_le_bytes()), Some(0xff00_ff00));
        assert_eq!(color(le, 2, &0x800fu16.to_le_bytes()), Some(0x88ff_0000));
        assert_eq!(color(le, 3, &[1, 2, 3, 4]), Some(0x0401_0203));
    }

    #[test]
    fn decodes_direct_and_indexed_pictures() {
        let rgb565: Vec<u8> = [0x001fu16, 0xf800, 0x07e0, 0xffff]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        let image = decode_gim(&gim(&Picture::new(0, 2, 2, &rgb565))).unwrap();
        assert_eq!((image.get(0, 0), image.get(1, 0)), (0xff0000, 0x0000ff));
        assert_eq!((image.get(0, 1), image.get(1, 1)), (0x00ff00, 0xffffff));
        // 8-bit indices with an RGBA8888 palette.
        let mut colors = alloc::vec![0u8; 8];
        colors[4..8].copy_from_slice(&[0x10, 0x20, 0x30, 0xff]);
        let mut indexed = Picture::new(5, 2, 1, &[1, 0]);
        indexed.palette = Some((3, &colors));
        let image = decode_gim(&gim(&indexed)).unwrap();
        assert_eq!(image.get(0, 0), 0x102030);
        // Index 0 is transparent black in this palette.
        assert_eq!(image.get_argb(1, 0), crate::image::CLEAR);
        indexed.palette = None;
        assert!(decode_gim(&gim(&indexed)).is_err());
    }

    #[test]
    fn swizzled_pictures_use_16_by_8_byte_blocks() {
        // 32x8 8-bit: two blocks of 16 bytes by 8 rows side by side. File byte
        // 16 is row 1 of the first block, pixel (0, 1); byte 128 starts the
        // second block, pixel (16, 0). Row-major data would put them at pixels
        // (16, 0) and (0, 4).
        let mut pixels = alloc::vec![0u8; 256];
        pixels[16] = 1;
        pixels[128] = 2;
        let mut colors = alloc::vec![0u8; 12];
        colors[4..8].copy_from_slice(&[1, 2, 3, 0xff]);
        colors[8..12].copy_from_slice(&[4, 5, 6, 0xff]);
        let mut picture = Picture::new(5, 32, 8, &pixels);
        picture.swizzled = true;
        picture.stride_alignment = 16;
        picture.palette = Some((3, &colors));
        let image = decode_gim(&gim(&picture)).unwrap();
        assert_eq!(image.get(0, 1), 0x010203);
        assert_eq!(image.get(16, 0), 0x040506);
        assert_eq!(image.get_argb(16, 1), crate::image::CLEAR);
    }

    #[test]
    fn rejects_other_data_and_unknown_formats() {
        assert!(decode_gim(b"MIG.00.1PSP\0").is_err());
        assert!(decode_gim(&gim(&Picture::new(9, 2, 2, &[0; 16]))).is_err());
        let good = gim(&Picture::new(0, 2, 2, &[0; 8]));
        assert!(decode_gim(&good[..good.len() - 1]).is_err());
    }
}
