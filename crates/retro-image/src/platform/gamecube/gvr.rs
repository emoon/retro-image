//! Sega GVR textures (`.gvr`) and GVM archives of them (`.gvm`), the
//! GameCube versions of the Dreamcast PVR and PVM files; a GVM shows its
//! first texture.
//!
//! Sources:
//! - Layouts: PuyoTools' `GvrTextureDecoder.cs` (optional `GBIX` or `GCIX`
//!   chunk, then `GVRT`; byte 10 of the `GVRT` chunk holds the palette format
//!   in its high nibble and the flags in its low nibble, byte 11 the data
//!   format, then big-endian width and height at 12 and 14, the internal
//!   palette and the pixels at 16), `GvrPalette.cs` (a `GVPL` palette file:
//!   palette format at byte 9, big-endian color count at 14, colors at 16),
//!   and `Archives/Formats/Gvm/GvmReader.cs` (`GVMH`, little-endian
//!   first-entry offset minus 8 at byte 4, flags and entry count at 8 and
//!   10). PuyoTools is MIT licensed (<https://github.com/nickworonekin/puyotools>,
//!   notice below).
//! - The `GBIX` chunk is read as 16 bytes whatever its length field says:
//!   Sega's tools leave that field unreliable in Dreamcast files, and the
//!   same toolchain wrote these. The `GVRT` length is not used.
//! - The data formats are the shared GX ones, see `codec::gx`.
//! - Indexed textures with an external palette read the `.gvp` companion;
//!   without it they show a gray ramp. A palette flag is not needed to read
//!   an internal palette: flag `0x08` says it follows the header.
//!
//! No GVR or GVM sample was available, so the decoder is checked only by unit
//! tests built from the documented layout and is unverified against real
//! game files.
//!
//! Transparent pixels are composited onto the shared fill color.

// Parts of this file follow PuyoTools.Core/Textures/Gvr and
// PuyoTools.Core/Archives/Formats/Gvm
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

use crate::bytes::{be16, le32};
use crate::codec::gx::{self, PaletteFormat, PixelFormat};
use crate::image::{check_size, gray_ramp};
use crate::{Companions, DecodeError, Image};

/// Flag in the low nibble of byte 10: the palette follows the header.
const INTERNAL_PALETTE: u8 = 0x08;

pub(super) fn decode_gvr(data: &[u8], companions: &dyn Companions) -> Result<Image, DecodeError> {
    decode_texture(data, companions)
}

pub(super) fn decode_gvm(data: &[u8], companions: &dyn Companions) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    if data.get(..4) != Some(b"GVMH") || be16(data, 10).ok_or(fail)? == 0 {
        return Err(fail);
    }
    let first = (le32(data, 4).ok_or(fail)? as usize)
        .checked_add(8)
        .filter(|&at| at >= 12)
        .ok_or(fail)?;
    decode_texture(data.get(first..).ok_or(fail)?, companions)
}

/// The texture at the start of `data`: an optional global index chunk, then
/// the `GVRT` chunk.
fn decode_texture(data: &[u8], companions: &dyn Companions) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let start = match data.get(..4) {
        Some(b"GBIX" | b"GCIX") => 16,
        _ => 0,
    };
    let chunk = data.get(start..).ok_or(fail)?;
    if chunk.get(..4) != Some(b"GVRT") {
        return Err(fail);
    }
    let flags = *chunk.get(10).ok_or(fail)?;
    let code = *chunk.get(11).ok_or(fail)?;
    let format = PixelFormat::from_code(u32::from(code))
        .filter(|&f| f != PixelFormat::C14X2)
        .ok_or(fail)?;
    let width = usize::from(be16(chunk, 12).ok_or(fail)?);
    let height = usize::from(be16(chunk, 14).ok_or(fail)?);
    check_size(width, height)?;

    let mut body = chunk.get(16..).ok_or(fail)?;
    let palette = match format.palette_len() {
        None => Vec::new(),
        Some(len) if flags & INTERNAL_PALETTE != 0 => {
            let palette_format = PaletteFormat::from_code(u32::from(chunk[10] >> 4)).ok_or(fail)?;
            let palette = gx::decode_palette(palette_format, body, len).ok_or(fail)?;
            body = &body[len * 2..];
            palette
        }
        Some(len) => external_palette(companions, len).unwrap_or_else(|| gray_ramp(len)),
    };
    let argb = gx::decode(format, width, height, body, &palette).ok_or(fail)?;
    Ok(Image::from_argb(
        width as u32,
        height as u32,
        argb.into_iter(),
    ))
}

/// The first `len` colors of the `.gvp` palette next to the texture.
fn external_palette(companions: &dyn Companions, len: usize) -> Option<Vec<u32>> {
    let file = companions.get("gvp")?;
    if file.get(..4) != Some(b"GVPL") {
        return None;
    }
    let format = PaletteFormat::from_code(u32::from(*file.get(9)?))?;
    let count = usize::from(be16(&file, 14)?);
    if count < len {
        return None;
    }
    gx::decode_palette(format, file.get(16..)?, len)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NoCompanions;

    /// A texture: `GBIX` chunk, `GVRT` header and `body`.
    fn gvr(palette_and_flags: u8, format: u8, width: u16, height: u16, body: &[u8]) -> Vec<u8> {
        let mut data = b"GBIX\x08\0\0\0\0\0\0\x01\0\0\0\0GVRT".to_vec();
        data.extend_from_slice(&(body.len() as u32 + 8).to_le_bytes());
        data.extend_from_slice(&[0, 0, palette_and_flags, format]);
        data.extend_from_slice(&width.to_be_bytes());
        data.extend_from_slice(&height.to_be_bytes());
        data.extend_from_slice(body);
        data
    }

    struct Palette(Vec<u8>);

    impl Companions for Palette {
        fn get(&self, extension: &str) -> Option<Vec<u8>> {
            (extension == "gvp").then(|| self.0.clone())
        }

        fn get_named(&self, _: &str) -> Option<Vec<u8>> {
            None
        }
    }

    #[test]
    fn reads_direct_color_after_the_global_index_chunk() {
        // RGB565 4x4: red in the first pixel, green in the second.
        let mut body = [0u8; 32];
        body[0..4].copy_from_slice(&[0xf8, 0x00, 0x07, 0xe0]);
        let image = decode_gvr(&gvr(0, 4, 4, 4, &body), &NoCompanions).unwrap();
        assert_eq!((image.width(), image.height()), (4, 4));
        assert_eq!((image.get(0, 0), image.get(1, 0)), (0xff0000, 0x00ff00));
    }

    #[test]
    fn transparent_pixels_keep_their_alpha() {
        // RGB5A3 alpha 0 (top bit clear, alpha bits zero).
        let image = decode_gvr(&gvr(0, 5, 4, 4, &[0; 32]), &NoCompanions).unwrap();
        assert_eq!(image.get_argb(2, 2), crate::image::CLEAR);
    }

    #[test]
    fn internal_palette_comes_before_the_pixels() {
        // C4 8x8 with an RGB5A3 palette (format 2, flag 8): index 0 is opaque
        // black and index 1 red.
        let mut body = alloc::vec![0u8; 32 + 32];
        body[0..2].copy_from_slice(&0x8000u16.to_be_bytes());
        body[2..4].copy_from_slice(&0xfc00u16.to_be_bytes());
        body[32] = 0x10;
        let image = decode_gvr(&gvr(0x28, 8, 8, 8, &body), &NoCompanions).unwrap();
        assert_eq!((image.get(0, 0), image.get(1, 0)), (0xff0000, 0x000000));
    }

    #[test]
    fn external_palette_defaults_to_a_gray_ramp_and_reads_the_gvp() {
        let mut body = [0u8; 32];
        body[0] = 0xf0;
        let texture = gvr(0x02, 8, 8, 8, &body);
        let alone = decode_gvr(&texture, &NoCompanions).unwrap();
        assert_eq!((alone.get(0, 0), alone.get(1, 0)), (0xffffff, 0x000000));

        // GVPL: format RGB565 at byte 9, 16 colors at byte 14; color 15 is blue.
        let mut gvp = b"GVPL\x28\0\0\0\0\x01\0\0\0\0\0\x10".to_vec();
        gvp.extend_from_slice(&[0; 30]);
        gvp.extend_from_slice(&[0x00, 0x1f]);
        let with = decode_gvr(&texture, &Palette(gvp)).unwrap();
        assert_eq!(with.get(0, 0), 0x0000ff);
    }

    #[test]
    fn gvm_shows_its_first_texture() {
        let mut body = [0u8; 32];
        body[0..2].copy_from_slice(&[0xf8, 0x00]);
        let texture = gvr(0, 4, 4, 4, &body);
        // GVMH header: first entry at 0x20 (field = 0x18), 1 entry.
        let mut archive = b"GVMH\x18\0\0\0\0\0\0\x01".to_vec();
        archive.resize(0x20, 0);
        archive.extend_from_slice(&texture);
        let image = decode_gvm(&archive, &NoCompanions).unwrap();
        assert_eq!(image.get(0, 0), 0xff0000);
    }

    #[test]
    fn rejects_other_data_unknown_formats_and_truncation() {
        let texture = gvr(0, 4, 4, 4, &[0; 32]);
        assert!(decode_gvr(&texture[..texture.len() - 1], &NoCompanions).is_err());
        assert!(decode_gvr(&gvr(0, 7, 4, 4, &[0; 32]), &NoCompanions).is_err());
        assert!(decode_gvr(&gvr(0, 4, 0, 4, &[0; 32]), &NoCompanions).is_err());
        assert!(decode_gvr(b"GVRT", &NoCompanions).is_err());
        assert!(decode_gvm(&texture, &NoCompanions).is_err());
    }
}
