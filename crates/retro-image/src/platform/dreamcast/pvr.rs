//! PVR textures (`.pvr`) and PVM archives of them (`.pvm`), the texture files
//! of the Dreamcast's PowerVR chip; a PVM shows its first texture, and
//! mipmapped textures their largest level.
//!
//! Sources:
//! - Layout: an optional `GBIX` chunk, then `PVRT` with the pixel format
//!   (byte 8), the data format (byte 9) and little-endian width and height at
//!   12 and 14, the data from 16 on; and the data format codes, the padding
//!   before the smallest mipmap, the codebook sizes of the small VQ formats,
//!   and the `PVPL` palette file (pixel format at byte 8, little-endian color
//!   count at 14, colors from 16): PuyoTools, the wiki page
//!   <https://github.com/nickworonekin/puyotools/wiki/PVR-Texture> and
//!   `PuyoTools.Core/Textures/Pvr` and `Archives/Formats/Pvm`
//!   (<https://github.com/nickworonekin/puyotools>, MIT license, notice below).
//!   A PVM starts `PVMH` with the little-endian offset of the first texture
//!   minus 8 at byte 4.
//! - Reverse engineered from samples, where documents disagree or are silent:
//!   the twiddle puts `x` on the odd bits of the texel index and `y` on the
//!   even ones (the picture is upright with that order and transposed with
//!   the Ikaruga guide's, <https://ikaruga.dashgl.com/guides/pvr/>; checked on
//!   `0GDTEX.PVR`); a VQ codebook entry stores its 2x2 texels in column order
//!   (upper left, lower left, upper right, lower right; checked on
//!   `Font.pvr`); with mipmaps the levels run from 1x1 up and the largest
//!   comes last (checked on five samples, among them VQ at 128 and 256
//!   pixels, whose base level starts 1366 and 5462 bytes after the codebook).
//!   The `GBIX` and `PVRT` length fields are zero in KallistiOS's `star.pvr`,
//!   so they are not used: the data is sized from the dimensions and format.
//!   Samples: Sembiance's `pvrTexture` and `pvmhPVM` folders
//!   (<https://sembiance.com/fileFormatSamples/image/pvrTexture/>) and
//!   KallistiOS's `glass.pvr` and `star.pvr`
//!   (<https://github.com/KallistiOS/KallistiOS>, `examples/dreamcast/gldc`).
//!   The index formats (4 and 8-bit, palette in a `.pvp` companion, gray ramp
//!   without it), small VQ, rectangle twiddled, stride and the alternate
//!   twiddled mipmap format have no sample; they follow the documents only.
//!
//! Not read: YUV422 and bump pixel formats, RLE-compressed files. Alpha is
//! kept; alpha-only textures (fonts) show as solid squares.

// Parts of this file follow PuyoTools.Core/Textures/Pvr and
// PuyoTools.Core/Archives/Formats/Pvm
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

use crate::bytes::{le16, le32};
use crate::image::{check_size, gray_ramp, rgb565, widen_channel, xrgb1555};
use crate::morton::morton_index;
use crate::{Companions, DecodeError, Image};

/// Most chunks a PVM may have before its first texture.
const MAX_SKIPPED_CHUNKS: usize = 16;

pub(super) fn decode_pvr(data: &[u8], companions: &dyn Companions) -> Result<Image, DecodeError> {
    decode_texture(data, companions)
}

pub(super) fn decode_pvm(data: &[u8], companions: &dyn Companions) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    if data.get(..4) != Some(b"PVMH") || le16(data, 10).ok_or(fail)? == 0 {
        return Err(fail);
    }
    let mut at = (le32(data, 4).ok_or(fail)? as usize)
        .checked_add(8)
        .filter(|&at| at >= 12)
        .ok_or(fail)?;
    // Textures are `PVRT` chunks; skip any other chunk in front of the first.
    for _ in 0..MAX_SKIPPED_CHUNKS {
        let chunk = data.get(at..).ok_or(fail)?;
        if chunk.get(..4) == Some(b"PVRT") {
            return decode_texture(chunk, companions);
        }
        let len = le32(chunk, 4).ok_or(fail)? as usize;
        at = len
            .checked_add(8)
            .and_then(|len| at.checked_add(len))
            .ok_or(fail)?;
    }
    Err(fail)
}

/// How a texture's texels are stored, from its data format code.
#[derive(Clone, Copy)]
enum Storage {
    /// Texels in rows.
    Raster,
    /// Texels of one or more square tiles in Z-order; mipmaps have `pad`
    /// bytes in front of the 1x1 level.
    Twiddled { mipmaps: Option<usize> },
    /// 2x2 blocks of texels from a codebook of `codes` entries, indexed in
    /// Z-order, with mipmap index grids in front of the base level if
    /// `mipmaps`.
    Vq { codes: usize, mipmaps: bool },
    /// 4 or 8-bit palette indices in Z-order tiles.
    Indexed { bits: usize, mipmaps: bool },
}

impl Storage {
    fn from_code(code: u8, width: usize) -> Option<Self> {
        let small_codes = |sizes: [usize; 3]| match width {
            ..=16 => sizes[0],
            17..=32 => sizes[1],
            _ => sizes[2],
        };
        Some(match code {
            0x01 => Self::Twiddled { mipmaps: None },
            // The 1x1 level of a mipmapped texture is stored as a 2x1 one.
            0x02 => Self::Twiddled { mipmaps: Some(1) },
            0x12 => Self::Twiddled { mipmaps: Some(3) },
            0x03 => Self::Vq {
                codes: 256,
                mipmaps: false,
            },
            0x04 => Self::Vq {
                codes: 256,
                mipmaps: true,
            },
            0x10 => Self::Vq {
                codes: if width <= 64 {
                    small_codes([16, 32, 128])
                } else {
                    256
                },
                mipmaps: false,
            },
            0x11 => Self::Vq {
                codes: small_codes([16, 64, 256]),
                mipmaps: true,
            },
            0x05 => Self::Indexed {
                bits: 4,
                mipmaps: false,
            },
            0x06 => Self::Indexed {
                bits: 4,
                mipmaps: true,
            },
            0x07 => Self::Indexed {
                bits: 8,
                mipmaps: false,
            },
            0x08 => Self::Indexed {
                bits: 8,
                mipmaps: true,
            },
            0x09 | 0x0b => Self::Raster,
            // Rectangles are square tiles in a row or column.
            0x0d => Self::Twiddled { mipmaps: None },
            _ => return None,
        })
    }
}

/// How many bytes one texel of the pixel format takes, `None` for formats
/// that are not read.
fn texel_len(pixel_format: u8) -> Option<usize> {
    match pixel_format {
        0..=2 => Some(2),
        6 => Some(4),
        _ => None,
    }
}

/// `0xAARRGGBB` from the texel at the start of `bytes`.
fn color(pixel_format: u8, bytes: &[u8]) -> Option<u32> {
    let channel = |value: u16, shift: u32, bits: u32| {
        widen_channel(u32::from(value >> shift) & ((1 << bits) - 1), bits)
    };
    if pixel_format == 6 {
        return le32(bytes, 0);
    }
    let v = le16(bytes, 0)?;
    Some(match pixel_format {
        0 => {
            let alpha = if v & 0x8000 != 0 { 0xff } else { 0 };
            alpha << 24 | xrgb1555(v)
        }
        1 => 0xff << 24 | rgb565(v),
        _ => {
            channel(v, 12, 4) << 24
                | channel(v, 8, 4) << 16
                | channel(v, 4, 4) << 8
                | channel(v, 0, 4)
        }
    })
}

/// The position of texel (`x`, `y`) in a twiddled image made of square tiles
/// (one tile unless the image is a rectangle): `x` takes the odd bits.
fn twiddled_index(width: usize, height: usize, x: usize, y: usize) -> usize {
    let size = width.min(height);
    let tile = x / size + y / size * (width / size);
    tile * size * size + morton_index((y % size) as u32, (x % size) as u32) as usize
}

/// Bytes of a mipmap level `size` texels wide (square), the levels in front
/// of the base level run 1, 2, 4 ... `width / 2`.
fn level_len(storage: Storage, size: usize, texel_len: usize) -> usize {
    match storage {
        Storage::Raster => 0,
        Storage::Twiddled { .. } => size * size * texel_len,
        Storage::Vq { .. } => (size * size / 4).max(1),
        Storage::Indexed { bits: 4, .. } => (size * size / 2).max(1),
        Storage::Indexed { .. } => size * size,
    }
}

/// Bytes between the codebook or palette and the base level of a mipmapped
/// texture of `width` texels: padding, then every smaller level.
fn mipmap_skip(storage: Storage, width: usize, texel_len: usize) -> usize {
    let padding = match storage {
        Storage::Twiddled {
            mipmaps: Some(texels),
        } => texels * texel_len,
        Storage::Indexed { bits: 4, .. } => 1,
        Storage::Indexed { .. } => 3,
        _ => 0,
    };
    let levels = core::iter::successors(Some(1usize), |size| Some(size * 2))
        .take_while(|&size| size < width);
    padding
        + levels
            .map(|size| level_len(storage, size, texel_len))
            .sum::<usize>()
}

fn decode_texture(data: &[u8], companions: &dyn Companions) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    // The `GBIX` chunk is 16 bytes in practice, whatever its length says.
    let start = if data.get(..4) == Some(b"GBIX") {
        16
    } else {
        0
    };
    let chunk = data.get(start..).ok_or(fail)?;
    if chunk.get(..4) != Some(b"PVRT") {
        return Err(fail);
    }
    let pixel_format = *chunk.get(8).ok_or(fail)?;
    let texel_len = texel_len(pixel_format).ok_or(fail)?;
    let data_format = *chunk.get(9).ok_or(fail)?;
    let width = usize::from(le16(chunk, 12).ok_or(fail)?);
    let height = usize::from(le16(chunk, 14).ok_or(fail)?);
    let storage = Storage::from_code(data_format, width).ok_or(fail)?;
    check_size(width, height)?;
    let twiddled = !matches!(storage, Storage::Raster);
    if twiddled && !(width.is_power_of_two() && height.is_power_of_two()) {
        return Err(fail);
    }
    // Rectangles are only the plain twiddled texels (0x0d) and indices.
    let square = width == height;
    let mipmapped = matches!(
        storage,
        Storage::Twiddled { mipmaps: Some(_) }
            | Storage::Vq { mipmaps: true, .. }
            | Storage::Indexed { mipmaps: true, .. }
    );
    let needs_square = mipmapped || matches!(storage, Storage::Vq { .. }) || data_format == 0x01;
    // A VQ block grid needs at least one block.
    let too_small = matches!(storage, Storage::Vq { .. }) && width < 2;
    if (needs_square && !square) || too_small {
        return Err(fail);
    }

    let mut body = chunk.get(16..).ok_or(fail)?;
    // The codebook of a VQ texture follows the header.
    let codebook = match storage {
        Storage::Vq { codes, .. } => {
            let len = codes * 4 * texel_len;
            let book = body.get(..len).ok_or(fail)?;
            body = &body[len..];
            Some(book)
        }
        _ => None,
    };
    if mipmapped {
        body = body
            .get(mipmap_skip(storage, width, texel_len)..)
            .ok_or(fail)?;
    }

    // The base level takes this much of the data; a header that claims more
    // gets no buffer.
    let texels = width * height;
    let base_len = match storage {
        Storage::Raster | Storage::Twiddled { .. } => texels * texel_len,
        Storage::Vq { .. } => texels / 4,
        Storage::Indexed { bits, .. } => (texels * bits).div_ceil(8),
    };
    if body.len() < base_len {
        return Err(fail);
    }

    let mut argb = Vec::with_capacity(texels);
    match storage {
        Storage::Raster => {
            for y in 0..height {
                for x in 0..width {
                    let at = (y * width + x) * texel_len;
                    argb.push(color(pixel_format, body.get(at..).ok_or(fail)?).ok_or(fail)?);
                }
            }
        }
        Storage::Twiddled { .. } => {
            for y in 0..height {
                for x in 0..width {
                    let at = twiddled_index(width, height, x, y) * texel_len;
                    argb.push(color(pixel_format, body.get(at..).ok_or(fail)?).ok_or(fail)?);
                }
            }
        }
        Storage::Vq { .. } => {
            let book = codebook.ok_or(fail)?;
            for y in 0..height {
                for x in 0..width {
                    let grid = twiddled_index(width / 2, height / 2, x / 2, y / 2);
                    let code = usize::from(*body.get(grid).ok_or(fail)?);
                    // Column order within a block.
                    let texel = code * 4 + (x % 2) * 2 + y % 2;
                    let at = texel * texel_len;
                    argb.push(color(pixel_format, book.get(at..).ok_or(fail)?).ok_or(fail)?);
                }
            }
        }
        Storage::Indexed { bits, .. } => {
            let palette = palette(companions, 1 << bits);
            for y in 0..height {
                for x in 0..width {
                    let texel = twiddled_index(width, height, x, y);
                    let index = if bits == 4 {
                        // The even texel is the low nibble.
                        *body.get(texel / 2).ok_or(fail)? >> (texel % 2 * 4) & 15
                    } else {
                        *body.get(texel).ok_or(fail)?
                    };
                    argb.push(palette[usize::from(index)]);
                }
            }
        }
    }
    Image::from_argb(width as u32, height as u32, argb.into_iter())
}

/// The first `len` colors of the `.pvp` file next to the texture, or a ramp of
/// grays.
fn palette(companions: &dyn Companions, len: usize) -> Vec<u32> {
    let from_file = || -> Option<Vec<u32>> {
        let file = companions.get("pvp")?;
        if file.get(..4) != Some(b"PVPL") {
            return None;
        }
        let pixel_format = *file.get(8)?;
        let texel_len = texel_len(pixel_format)?;
        if usize::from(le16(&file, 14)?) < len {
            return None;
        }
        (0..len)
            .map(|i| color(pixel_format, file.get(16 + i * texel_len..)?))
            .collect()
    };
    from_file().unwrap_or_else(|| gray_ramp(len))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NoCompanions;

    /// A texture: `GBIX` chunk (with a zero length field), `PVRT` header
    /// (also with a zero length) and `body`.
    fn pvr(pixel_format: u8, data_format: u8, width: u16, height: u16, body: &[u8]) -> Vec<u8> {
        let mut data = b"GBIX\0\0\0\0\0\0\0\0\0\0\0\0PVRT\0\0\0\0".to_vec();
        data.extend_from_slice(&[pixel_format, data_format, 0, 0]);
        data.extend_from_slice(&width.to_le_bytes());
        data.extend_from_slice(&height.to_le_bytes());
        data.extend_from_slice(body);
        data
    }

    fn texel_bytes(values: &[u16]) -> Vec<u8> {
        values.iter().flat_map(|v| v.to_le_bytes()).collect()
    }

    #[test]
    fn pixel_formats_expand_with_their_alpha() {
        let argb = |format, value: u16| color(format, &value.to_le_bytes()).unwrap();
        assert_eq!(argb(1, 0xf800), 0xffff_0000);
        assert_eq!(argb(0, 0x801f), 0xff00_00ff);
        assert_eq!(argb(0, 0x001f), 0x0000_00ff);
        assert_eq!(argb(2, 0x8f0f), 0x88ff_00ff);
        assert_eq!(color(6, &[0x03, 0x02, 0x01, 0x80]), Some(0x8001_0203));
        assert_eq!(texel_len(3), None);
    }

    #[test]
    fn twiddling_puts_x_on_the_odd_bits() {
        // 2x2 RGB565 texels in file order: (0,0), (0,1), (1,0), (1,1) as (x, y).
        // File index 1 is x = 0, y = 1; index 2 is x = 1, y = 0.
        let body = texel_bytes(&[0xf800, 0x07e0, 0x001f, 0xffff]);
        let image = decode_pvr(&pvr(1, 1, 2, 2, &body), &NoCompanions).unwrap();
        assert_eq!(image.get(0, 1), 0x00ff00, "second texel is below the first");
        assert_eq!(
            image.get(1, 0),
            0x0000ff,
            "third texel is right of the first"
        );
        assert_eq!(image.get(0, 0), 0xff0000);
    }

    #[test]
    fn vq_blocks_store_texels_in_column_order() {
        // 2x2 texture, 256-entry codebook of 2x2 blocks, one index byte.
        let mut body = alloc::vec![0u8; 256 * 8 + 1];
        let block = texel_bytes(&[0xf800, 0x07e0, 0x001f, 0xffff]);
        body[8..16].copy_from_slice(&block);
        body[256 * 8] = 1;
        let image = decode_pvr(&pvr(1, 3, 2, 2, &body), &NoCompanions).unwrap();
        // Upper left, lower left, upper right, lower right.
        assert_eq!(image.get(0, 0), 0xff0000);
        assert_eq!(image.get(0, 1), 0x00ff00);
        assert_eq!(image.get(1, 0), 0x0000ff);
        assert_eq!(image.get(1, 1), 0xffffff);
    }

    #[test]
    fn mipmapped_textures_show_the_base_level_stored_last() {
        // 2x2 with mipmaps: 2 bytes of padding, the 1x1 level, then the base.
        let mut body = alloc::vec![0u8; 2 + 2];
        body.extend(texel_bytes(&[0xf800, 0xf800, 0xf800, 0xf800]));
        let image = decode_pvr(&pvr(1, 2, 2, 2, &body), &NoCompanions).unwrap();
        assert_eq!(image.get(1, 1), 0xff0000);
        // The 4x4 VQ base level follows 1 + 1 + 4... bytes: 1x1, 2x2.
        let mut vq = alloc::vec![0u8; 256 * 8 + 1 + 1];
        vq.push(0); // the 4x4 base: an index grid of 2x2 codes
        vq.extend_from_slice(&[0, 0, 0]);
        assert!(decode_pvr(&pvr(1, 4, 4, 4, &vq), &NoCompanions).is_ok());
        assert!(decode_pvr(&pvr(1, 4, 4, 4, &vq[..vq.len() - 1]), &NoCompanions).is_err());
    }

    #[test]
    fn rectangle_textures_are_raster_and_twiddled_rectangles_are_tiled() {
        let body = texel_bytes(&[0xf800, 0x07e0, 0x001f, 0xffff]);
        let image = decode_pvr(&pvr(1, 9, 4, 1, &body), &NoCompanions).unwrap();
        assert_eq!((image.get(0, 0), image.get(3, 0)), (0xff0000, 0xffffff));
        // 4x2 twiddled: two 2x2 tiles side by side.
        let body = texel_bytes(&[0xf800, 0, 0, 0, 0x07e0, 0, 0, 0]);
        let image = decode_pvr(&pvr(1, 0x0d, 4, 2, &body), &NoCompanions).unwrap();
        assert_eq!((image.get(0, 0), image.get(2, 0)), (0xff0000, 0x00ff00));
    }

    #[test]
    fn index_textures_use_the_pvp_palette_or_a_gray_ramp() {
        // 8x8 4-bit, one tile in Z-order: texel 0 (x 0, y 0) low nibble 1,
        // texel 1 (x 0, y 1) high nibble 15.
        let mut body = alloc::vec![0u8; 32];
        body[0] = 0xf1;
        let texture = pvr(1, 5, 8, 8, &body);
        let alone = decode_pvr(&texture, &NoCompanions).unwrap();
        assert_eq!((alone.get(0, 0), alone.get(0, 1)), (0x111111, 0xffffff));

        struct Pvp;
        impl Companions for Pvp {
            fn get(&self, extension: &str) -> Option<Vec<u8>> {
                let mut file = b"PVPL\0\0\0\0\x01\0\0\0\0\0\x10\0".to_vec();
                for i in 0..16u16 {
                    file.extend_from_slice(&(i << 11).to_le_bytes());
                }
                (extension == "pvp").then_some(file)
            }
            fn get_named(&self, _: &str) -> Option<Vec<u8>> {
                None
            }
        }
        let with = decode_pvr(&texture, &Pvp).unwrap();
        // Index 15 is RGB565 red 15, which stretches to 0x7b.
        assert_eq!(with.get(0, 1), 0x7b0000);
    }

    #[test]
    fn declared_sizes_need_their_data_before_anything_is_allocated() {
        // Headers for 8192x8192 textures, 64 megatexels, with no data at all:
        // refused without reserving a buffer for them.
        let storages: [(u8, u8); 5] = [(1, 0x09), (1, 0x01), (2, 0x05), (1, 0x07), (6, 0x0d)];
        for (pixel_format, data_format) in storages {
            let file = pvr(pixel_format, data_format, 8192, 8192, &[]);
            assert!(
                decode_pvr(&file, &NoCompanions).is_err(),
                "{data_format:#x}"
            );
        }
        let vq = pvr(1, 0x03, 8192, 8192, &alloc::vec![0; 2048]);
        assert!(decode_pvr(&vq, &NoCompanions).is_err());
    }

    #[test]
    fn pvm_shows_its_first_texture_and_bad_data_is_rejected() {
        let body = texel_bytes(&[0xf800, 0, 0, 0]);
        let texture = pvr(1, 1, 2, 2, &body)[16..].to_vec();
        let mut archive = b"PVMH\x18\0\0\0\0\0\x01\0".to_vec();
        archive.resize(0x20, 0);
        archive.extend_from_slice(&texture);
        assert_eq!(
            decode_pvm(&archive, &NoCompanions).unwrap().get(0, 0),
            0xff0000
        );
        assert!(decode_pvr(&pvr(3, 1, 2, 2, &body), &NoCompanions).is_err());
        assert!(decode_pvr(&pvr(1, 1, 3, 3, &body), &NoCompanions).is_err());
        assert!(decode_pvr(&pvr(1, 0x7f, 2, 2, &body), &NoCompanions).is_err());
        assert!(decode_pvr(&pvr(1, 1, 2, 2, &body[..7]), &NoCompanions).is_err());
        assert!(decode_pvm(&pvr(1, 1, 2, 2, &body), &NoCompanions).is_err());
    }
}
