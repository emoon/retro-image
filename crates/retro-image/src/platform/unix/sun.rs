//! Sun raster (`.ras`, `.sun`, `.im1`, `.im8`, `.im24`, `.im32`).
//!
//! Sources:
//! - Encyclopedia of Graphics File Formats, "Sun Raster"
//!   (<https://www.fileformat.info/format/sunraster/egff.htm>): 32-byte
//!   big-endian header (magic `59 A6 6A 95`, width, height, depth, data length,
//!   type, color map type, color map length), the color map (three planes of
//!   red, green and blue), then the raster, whose scan lines are padded to
//!   16 bits; types 0 and 1 are uncompressed, 2 is byte-encoded, 3 is RGB
//!   order; 24 and 32-bit pixels are BGR unless the type is 3, and 32-bit
//!   pixels have a byte in front; the byte-encoded stream ignores scan line
//!   boundaries, `80 00` is a literal `80`, and `80 count value` is a run.
//!   The page describes a run as `count` bytes long, but the run is
//!   `count + 1` bytes: samples and Pillow agree on that.
//! - A set bit in a 1-bit raster without a color map is black (as in the
//!   PBM samples of the same picture); found by comparing the `input.im1` and
//!   `input_p1.pbm` samples, not stated by the page.
//!
//! Types 4 (TIFF), 5 (IFF) and the experimental type are not accepted, nor
//! are depths other than 1, 8, 24 and 32. The first byte of a 32-bit pixel
//! is alpha when any pixel has it non-zero, and unused when all are 0. The
//! `abydos.im32` sample (a converted RGBA picture) has real alpha there,
//! while older files leave the pad byte 0; Deark makes the same choice.
//! Alpha is composited onto the shared transparent-fill gray. A color map
//! on a 24 or 32-bit raster is skipped, and a raw color map on a 1 or 8-bit
//! raster is ignored (8-bit pictures then show their values as grays).
//! Palette entries missing from a short map are black.
//!
//! Verification: no RECOIL oracle. Output matches Pillow's Sun reader pixel
//! for pixel on the ten samples without alpha and Deark's on all twelve (the
//! 32-bit pictures after compositing); Pillow reads the 32-bit pad byte last
//! and so differs on `abydos.im32` and `img.ras`. See the divergence file
//! `unix-rasters.tsv`.

use alloc::vec::Vec;

use crate::bytes::be32;
use crate::image::check_size;
use crate::{BitOrder, DecodeError, Image};

const FAIL: DecodeError = DecodeError::Unrecognized;
const MAGIC: u32 = 0x59a6_6a95;
const HEADER_LEN: usize = 32;
/// Type 3: pixels in RGB order instead of BGR.
const TYPE_RGB: u32 = 3;
const TYPE_BYTE_ENCODED: u32 = 2;

pub(super) fn decode_sun(data: &[u8]) -> Result<Image, DecodeError> {
    if be32(data, 0) != Some(MAGIC) {
        return Err(FAIL);
    }
    let field = |i: usize| be32(data, 4 * i).ok_or(FAIL);
    let (width, height, depth) = (field(1)? as usize, field(2)? as usize, field(3)?);
    let (kind, map_kind, map_len) = (field(5)?, field(6)?, field(7)? as usize);
    if kind > TYPE_RGB || map_kind > 2 || !matches!(depth, 1 | 8 | 24 | 32) {
        return Err(FAIL);
    }
    check_size(width, height)?;
    let map = data
        .get(HEADER_LEN..)
        .and_then(|d| d.get(..map_len))
        .ok_or(FAIL)?;
    let packed = &data[HEADER_LEN + map_len..];

    // Scan lines are a whole number of 16-bit words.
    let bits_per_row = width.checked_mul(depth as usize).ok_or(FAIL)?;
    let row_len = bits_per_row.div_ceil(16) * 2;
    let total = row_len.checked_mul(height).ok_or(FAIL)?;
    let unpacked;
    let raster: &[u8] = if kind == TYPE_BYTE_ENCODED {
        unpacked = unpack(packed, total)?;
        &unpacked
    } else {
        packed.get(..total).ok_or(FAIL)?
    };

    let (w, h) = (width as u32, height as u32);
    match depth {
        1 => {
            let colors = if map_kind == 1 && map_len >= 6 {
                let palette = palette(map);
                [palette[0], palette[1]]
            } else {
                [0xff_ffff, 0]
            };
            Image::from_bits(w, h, raster, row_len, BitOrder::MsbFirst, colors)
        }
        8 => {
            let palette = if map_kind == 1 {
                palette(map)
            } else {
                (0..256).map(|v| v * 0x01_0101).collect()
            };
            let mut indices = Vec::with_capacity(width * height);
            for row in raster.chunks_exact(row_len) {
                indices.extend_from_slice(&row[..width]);
            }
            Image::from_indexed(w, h, &indices, &palette)
        }
        _ => {
            let bytes = depth as usize / 8;
            let rgb_order = kind == TYPE_RGB;
            let pixels = || {
                raster
                    .chunks_exact(row_len)
                    .flat_map(|row| row[..width * bytes].chunks_exact(bytes))
            };
            // A 32-bit pixel starts with an alpha or pad byte. Files that
            // leave it all zero are taken as having no alpha.
            let has_alpha = bytes == 4 && pixels().any(|pixel| pixel[0] != 0);
            let colors = pixels().map(|pixel| {
                let [a, b, c] = [pixel[bytes - 3], pixel[bytes - 2], pixel[bytes - 1]];
                let (r, g, b) = if rgb_order { (a, b, c) } else { (c, b, a) };
                let color = u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b);
                let alpha = if has_alpha { pixel[0] } else { 255 };
                u32::from(alpha) << 24 | color
            });
            Ok(Image::from_argb(w, h, colors))
        }
    }
}

/// A 256-entry palette from a color map of three planes (red, green, blue);
/// entries the map does not reach are black.
fn palette(map: &[u8]) -> Vec<u32> {
    let count = map.len() / 3;
    let plane = |p: usize, i: usize| u32::from(map.get(p * count + i).copied().unwrap_or(0));
    (0..256)
        .map(|i| plane(0, i) << 16 | plane(1, i) << 8 | plane(2, i))
        .collect()
}

/// Expands the byte-encoded stream into exactly `len` bytes. A run that
/// reaches past `len` is cut off; running out of input fails.
fn unpack(data: &[u8], len: usize) -> Result<Vec<u8>, DecodeError> {
    // A three-byte packet yields at most 256 bytes.
    if len / 86 > data.len() {
        return Err(FAIL);
    }
    let mut out = Vec::with_capacity(len);
    let mut pos = 0;
    while out.len() < len {
        let byte = *data.get(pos).ok_or(FAIL)?;
        pos += 1;
        if byte != 0x80 {
            out.push(byte);
            continue;
        }
        let count = *data.get(pos).ok_or(FAIL)?;
        pos += 1;
        if count == 0 {
            out.push(0x80);
            continue;
        }
        let value = *data.get(pos).ok_or(FAIL)?;
        pos += 1;
        let run = (usize::from(count) + 1).min(len - out.len());
        out.resize(out.len() + run, value);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(width: u32, height: u32, depth: u32, kind: u32, map_len: u32) -> Vec<u8> {
        let map_kind = u32::from(map_len > 0);
        [MAGIC, width, height, depth, 0, kind, map_kind, map_len]
            .iter()
            .flat_map(|v| v.to_be_bytes())
            .collect()
    }

    #[test]
    fn rows_are_padded_to_16_bits_and_one_is_black() {
        // 3x2 1-bit: each row is one 16-bit word.
        let mut file = header(3, 2, 1, 1, 0);
        file.extend_from_slice(&[0b1010_0000, 0, 0b0100_0000, 0]);
        let image = decode_sun(&file).unwrap();
        assert_eq!(image.get(0, 0), 0);
        assert_eq!(image.get(1, 0), 0xffffff);
        assert_eq!(image.get(1, 1), 0);
    }

    #[test]
    fn byte_encoding_runs_ignore_row_boundaries() {
        // 2x2 8-bit gray: a run of 3 crossing the first row, then a
        // literal 0x80.
        let mut file = header(2, 2, 8, TYPE_BYTE_ENCODED, 0);
        file.extend_from_slice(&[0x80, 2, 7, 0x80, 0]);
        let image = decode_sun(&file).unwrap();
        assert_eq!(image.get(0, 0), 0x070707);
        assert_eq!(image.get(1, 0), 0x070707);
        assert_eq!(image.get(0, 1), 0x070707);
        assert_eq!(image.get(1, 1), 0x808080);
    }

    #[test]
    fn color_map_planes_and_pixel_orders() {
        // 1x1 8-bit with a 2-entry map: reds 1 2, greens 3 4, blues 5 6.
        let mut file = header(1, 1, 8, 1, 6);
        file.extend_from_slice(&[1, 2, 3, 4, 5, 6, 1, 0]);
        assert_eq!(decode_sun(&file).unwrap().get(0, 0), 0x020406);
        // 1x1 24-bit: BGR for standard, RGB for type 3.
        let mut bgr = header(1, 1, 24, 1, 0);
        bgr.extend_from_slice(&[1, 2, 3, 0]);
        assert_eq!(decode_sun(&bgr).unwrap().get(0, 0), 0x030201);
        let mut rgb = header(1, 1, 32, TYPE_RGB, 0);
        rgb.extend_from_slice(&[0, 1, 2, 3]);
        assert_eq!(decode_sun(&rgb).unwrap().get(0, 0), 0x010203);
    }

    #[test]
    fn rejects_unsupported_and_truncated_files() {
        let mut file = header(2, 2, 8, 1, 0);
        file.extend_from_slice(&[0; 3]);
        assert!(decode_sun(&file).is_err());
        file.push(0);
        assert!(decode_sun(&file).is_ok());
        assert!(decode_sun(&header(2, 2, 4, 1, 0)).is_err());
        assert!(decode_sun(&header(2, 2, 8, 4, 0)).is_err());
        assert!(decode_sun(&header(1 << 20, 1 << 20, 24, 2, 0)).is_err());
    }
}
