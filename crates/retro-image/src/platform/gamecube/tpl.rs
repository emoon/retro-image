//! TPL texture palette libraries (`.tpl`) of GameCube and Wii games, shown as
//! the first texture.
//!
//! Sources:
//! - File layout, big-endian throughout: the mkwiiki page for TPL
//!   (<https://wiki.tockdom.com/wiki/TPL_(File_Format)>) and YAGCD section
//!   15.35 (<https://hitmen.c02.at/files/yagcd/yagcd/chap15.html>), both facts
//!   only. Magic `00 20 AF 30`, texture count at 4, table offset at 8 (usually
//!   `0x0C`); each table entry is the offsets of an image header and of a
//!   palette header (0 if the texture has none). An image header is height,
//!   width (16 bits each), the format and the data offset (32 bits each),
//!   followed by wrap, filter and LOD fields; a palette header is the entry
//!   count (16 bits), 2 bytes of flags, the palette format and the data offset
//!   (32 bits each).
//! - The pixel formats are the GX formats of `codec::gx`, whose numbers TPL
//!   stores as they are. Mipmaps are ignored.
//!
//! No TPL sample was available. The decoder is checked by unit tests built
//! from the documented layout, and the corpus holds synthetic files from a
//! scratch encoder, so it is unverified against real game files.
//!
//! Transparent pixels are composited onto the shared fill color. An indexed
//! texture without a palette header shows a gray ramp.

use alloc::vec::Vec;

use crate::bytes::{be16, be32};
use crate::codec::gx::{self, PaletteFormat, PixelFormat};
use crate::image::{check_size, gray_ramp, over_fill};
use crate::{DecodeError, Image};

const MAGIC: u32 = 0x0020_af30;

pub(super) fn decode_tpl(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    if be32(data, 0) != Some(MAGIC) || be32(data, 4).ok_or(fail)? == 0 {
        return Err(fail);
    }
    // Offsets come from the file: add to them without overflowing.
    let word = |base: usize, offset: usize| be32(data, base.checked_add(offset)?);
    let half = |base: usize, offset: usize| be16(data, base.checked_add(offset)?);
    let table = be32(data, 8).ok_or(fail)? as usize;
    let image_header = word(table, 0).ok_or(fail)? as usize;
    let palette_header = word(table, 4).ok_or(fail)? as usize;

    let height = usize::from(half(image_header, 0).ok_or(fail)?);
    let width = usize::from(half(image_header, 2).ok_or(fail)?);
    let format = PixelFormat::from_code(word(image_header, 4).ok_or(fail)?).ok_or(fail)?;
    let pixels_at = word(image_header, 8).ok_or(fail)? as usize;
    check_size(width, height)?;

    let palette = match format.palette_len() {
        None => Vec::new(),
        Some(len) if palette_header == 0 => gray_ramp(len),
        Some(len) => {
            let count = usize::from(half(palette_header, 0).ok_or(fail)?).min(len);
            let palette_format =
                PaletteFormat::from_code(word(palette_header, 4).ok_or(fail)?).ok_or(fail)?;
            let colors_at = word(palette_header, 8).ok_or(fail)? as usize;
            let colors = data.get(colors_at..).ok_or(fail)?;
            gx::decode_palette(palette_format, colors, count).ok_or(fail)?
        }
    };
    let pixels = data.get(pixels_at..).ok_or(fail)?;
    let argb = gx::decode(format, width, height, pixels, &palette).ok_or(fail)?;
    Ok(Image::from_colors(
        width as u32,
        height as u32,
        argb.into_iter().map(over_fill),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A one-texture library: header, one table entry at 0x0C, an image
    /// header at 0x14, an optional palette header at 0x38, then the data.
    fn tpl(
        width: u16,
        height: u16,
        format: u32,
        palette: Option<(u32, &[u8])>,
        pixels: &[u8],
    ) -> Vec<u8> {
        let mut data = Vec::new();
        for word in [MAGIC, 1, 0x0c] {
            data.extend_from_slice(&word.to_be_bytes());
        }
        let palette_header = if palette.is_some() { 0x38u32 } else { 0 };
        data.extend_from_slice(&0x14u32.to_be_bytes());
        data.extend_from_slice(&palette_header.to_be_bytes());
        // Image header: 36 bytes; the pixels follow the (optional) palette.
        let palette_len = palette.map_or(0, |(_, colors)| colors.len());
        let pixels_at = 0x38
            + if palette.is_some() {
                12 + palette_len
            } else {
                0
            };
        data.extend_from_slice(&height.to_be_bytes());
        data.extend_from_slice(&width.to_be_bytes());
        data.extend_from_slice(&format.to_be_bytes());
        data.extend_from_slice(&(pixels_at as u32).to_be_bytes());
        data.resize(0x38, 0);
        if let Some((palette_format, colors)) = palette {
            data.extend_from_slice(&((colors.len() / 2) as u16).to_be_bytes());
            data.extend_from_slice(&[0, 0]);
            data.extend_from_slice(&palette_format.to_be_bytes());
            data.extend_from_slice(&(0x38u32 + 12).to_be_bytes());
            data.extend_from_slice(colors);
        }
        data.extend_from_slice(pixels);
        data
    }

    #[test]
    fn decodes_direct_color_with_the_dimensions_height_first() {
        // RGB565 8x4: two 4x4 blocks; the second block's first pixel is blue.
        let mut pixels = [0u8; 64];
        pixels[0..2].copy_from_slice(&0xf800u16.to_be_bytes());
        pixels[32..34].copy_from_slice(&0x001fu16.to_be_bytes());
        let image = decode_tpl(&tpl(8, 4, 4, None, &pixels)).unwrap();
        assert_eq!((image.width(), image.height()), (8, 4));
        assert_eq!((image.get(0, 0), image.get(4, 0)), (0xff0000, 0x0000ff));
    }

    #[test]
    fn indexed_textures_read_their_palette_header() {
        // C8 8x4 with an IA8 palette: index 0 is opaque black, 1 opaque white.
        let mut colors = alloc::vec![0u8; 512];
        colors[0] = 0xff;
        colors[2..4].copy_from_slice(&[0xff, 0xff]);
        let mut pixels = [0u8; 32];
        pixels[1] = 1;
        let image = decode_tpl(&tpl(8, 4, 9, Some((0, &colors)), &pixels)).unwrap();
        assert_eq!((image.get(0, 0), image.get(1, 0)), (0x000000, 0xffffff));
        // Without a palette header the indices show as a gray ramp.
        let image = decode_tpl(&tpl(8, 4, 9, None, &pixels)).unwrap();
        assert_eq!(image.get(1, 0), 0x010101);
    }

    #[test]
    fn rejects_bad_headers_and_truncated_data() {
        let data = tpl(8, 4, 4, None, &[0; 64]);
        assert!(decode_tpl(&data[..data.len() - 1]).is_err());
        assert!(decode_tpl(&tpl(8, 4, 7, None, &[0; 64])).is_err());
        assert!(decode_tpl(&tpl(0, 4, 4, None, &[0; 64])).is_err());
        let mut bad = data.clone();
        bad[0] = 1;
        assert!(decode_tpl(&bad).is_err());
        let mut far = data;
        far[8..12].copy_from_slice(&0xffff_fff0u32.to_be_bytes());
        assert!(decode_tpl(&far).is_err());
    }
}
