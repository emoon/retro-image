//! Truevision Targa (TGA).
//!
//! Sources:
//! - Truevision TGA File Format Specification, version 2.0 (1991), as
//!   summarised at <https://en.wikipedia.org/wiki/Truevision_TGA> and
//!   <http://www.paulbourke.net/dataformats/tga/>: 18-byte header, image ID,
//!   colour map, pixel data (types 1, 2, 3 raw; 9, 10, 11 run-length coded),
//!   16-bit pixels as 1:5:5:5, bit 5 of the descriptor = top-to-bottom, bit 4
//!   = right-to-left; the low 4 bits of the descriptor count the alpha
//!   (attribute) bits per pixel.
//!
//! Alpha is read from a 32-bit pixel (or color-map entry) whose descriptor
//! says 8 attribute bits, and from bit 15 of a 16-bit one whose descriptor
//! says 1. An alpha that leaves every pixel clear is taken as unused, since
//! many writers keep that byte or bit at 0.
//!
//! Version 1 files have no footer, so the format is chosen by extension only
//! and the header is validated tightly (type, depth and colour-map
//! consistency, sizes, enough data). Version 2 files end in
//! `TRUEVISION-XFILE.`; the footer and extension area are not needed.
//!
//! Verification: no oracle exists (RECOIL has no TGA). Output was rendered to
//! PNG and viewed, and compared pixel for pixel with Python PIL's TGA reader
//! (used as a black box) on the sample files.

use alloc::vec::Vec;

use crate::bytes::le16;
use crate::image::{check_size, xrgb1555};
use crate::{DecodeError, Image};

const HEADER_LEN: usize = 18;

/// The alpha of a color-map entry or pixel of `pixel.len()` bytes, given the
/// descriptor's attribute bit count: 255 where the layout has none.
fn alpha_of(pixel: &[u8], attribute_bits: u8) -> u8 {
    match (pixel, attribute_bits) {
        ([.., a], 8) if pixel.len() == 4 => *a,
        ([_, hi], 1) => 255 * (hi >> 7),
        _ => 255,
    }
}

/// One colour-map entry or pixel of 2 to 4 bytes as 0xRRGGBB.
fn rgb_of(pixel: &[u8]) -> u32 {
    match *pixel {
        [b, g, r] | [b, g, r, _] => u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b),
        [lo, hi] => xrgb1555(u16::from_le_bytes([lo, hi])),
        _ => 0,
    }
}

pub(super) fn decode_tga(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let h = data.get(..HEADER_LEN).ok_or(fail)?;
    let (id_len, map_type, kind) = (usize::from(h[0]), h[1], h[2]);
    let word = |at| le16(h, at).map(usize::from).ok_or(fail);
    let (map_first, map_len, map_bits) = (word(3)?, word(5)?, usize::from(h[7]));
    let (width, height) = (word(12)?, word(14)?);
    let depth = usize::from(h[16]);
    let descriptor = h[17];

    let rle = kind & 8 != 0;
    let mapped = kind & 7 == 1;
    let depth_ok = match kind & 7 {
        1 | 3 => depth == 8,
        2 => matches!(depth, 15 | 16 | 24 | 32),
        _ => false,
    };
    if !matches!(kind, 1..=3 | 9..=11) || !depth_ok || descriptor & 0xc0 != 0 {
        return Err(fail);
    }
    if map_type > 1 || (mapped && map_type != 1) {
        return Err(fail);
    }
    let map_entry = match map_bits {
        15 | 16 => 2,
        24 => 3,
        32 => 4,
        0 if map_type == 0 || map_len == 0 => 0,
        _ => return Err(fail),
    };
    check_size(width, height)?;

    // Image ID, then the colour map, then the pixels.
    let map_start = HEADER_LEN + id_len;
    let map_end = map_start + map_len * map_entry;
    let map_bytes = data.get(map_start..map_end).ok_or(fail)?;
    let attribute_bits = descriptor & 0xf;
    let (palette, palette_alpha): (Vec<u32>, Vec<u8>) = if mapped {
        let entries = map_bytes.chunks_exact(map_entry);
        entries
            .map(|entry| (rgb_of(entry), alpha_of(entry, attribute_bits)))
            .unzip()
    } else {
        (Vec::new(), Vec::new())
    };
    let bpp = depth.div_ceil(8);
    let count = width * height;
    let body = &data[map_end..];

    // Raw bytes of every pixel, in file order.
    let raw: Vec<u8> = if rle {
        unpack(body, count, bpp)?
    } else {
        body.get(..count * bpp).ok_or(fail)?.to_vec()
    };

    let top_down = descriptor & 0x20 != 0;
    let right_to_left = descriptor & 0x10 != 0;
    // Mapped pixels get the alpha of their entry, so look at the entry size.
    let has_alpha = match (mapped, kind & 7) {
        (true, _) => {
            (map_entry == 4 && attribute_bits == 8) || (map_entry == 2 && attribute_bits == 1)
        }
        (false, 2) => (bpp == 4 && attribute_bits == 8) || (bpp == 2 && attribute_bits == 1),
        _ => false,
    };
    let mut alpha = has_alpha.then(|| alloc::vec![255u8; count]);
    let mut image = Image::new(width as u32, height as u32);
    for (i, pixel) in raw.chunks_exact(bpp).enumerate() {
        let (color, a) = if mapped {
            let index = usize::from(pixel[0]).checked_sub(map_first).ok_or(fail)?;
            (*palette.get(index).ok_or(fail)?, palette_alpha[index])
        } else if kind & 7 == 3 {
            (u32::from(pixel[0]) * 0x01_01_01, 255)
        } else {
            (rgb_of(pixel), alpha_of(pixel, attribute_bits))
        };
        let (row, col) = (i / width, i % width);
        let y = if top_down { row } else { height - 1 - row };
        let x = if right_to_left { width - 1 - col } else { col };
        image.set(x as u32, y as u32, color);
        if let Some(plane) = &mut alpha {
            plane[y * width + x] = a;
        }
    }
    // An alpha that leaves every pixel clear is not used.
    match alpha.filter(|plane| plane.iter().any(|&a| a != 0)) {
        Some(plane) => Ok(image.with_alpha(plane)),
        None => Ok(image),
    }
}

/// Expands run-length packets into `count` pixels of `bpp` bytes.
fn unpack(data: &[u8], count: usize, bpp: usize) -> Result<Vec<u8>, DecodeError> {
    let fail = DecodeError::Unrecognized;
    // A packet yields at most 128 pixels for at least 2 bytes.
    if count > data.len().saturating_mul(64) {
        return Err(fail);
    }
    let mut out = Vec::with_capacity(count * bpp);
    let mut pos = 0;
    while out.len() < count * bpp {
        let head = *data.get(pos).ok_or(fail)?;
        pos += 1;
        let n = usize::from(head & 0x7f) + 1;
        if n * bpp > count * bpp - out.len() {
            return Err(fail);
        }
        if head & 0x80 != 0 {
            let pixel = data.get(pos..pos + bpp).ok_or(fail)?;
            pos += bpp;
            for _ in 0..n {
                out.extend_from_slice(pixel);
            }
        } else {
            out.extend_from_slice(data.get(pos..pos + n * bpp).ok_or(fail)?);
            pos += n * bpp;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(kind: u8, depth: u8, width: u8, height: u8, descriptor: u8) -> Vec<u8> {
        let mut h = alloc::vec![0u8; HEADER_LEN];
        h[2] = kind;
        h[12] = width;
        h[14] = height;
        h[16] = depth;
        h[17] = descriptor;
        h
    }

    #[test]
    fn true_colour_origin_flags() {
        // 2x1 BGR; bit 4 flips horizontally.
        let mut data = header(2, 24, 2, 1, 0x10);
        data.extend_from_slice(&[1, 2, 3, 4, 5, 6]);
        let image = decode_tga(&data).unwrap();
        assert_eq!(&image.rgb()[..6], &[6, 5, 4, 3, 2, 1]);
    }

    #[test]
    fn rle_16_bit() {
        // 1x2 pixels: a run of two white (0x7fff) pixels.
        let mut data = header(10, 16, 1, 2, 0);
        data.extend_from_slice(&[0x81, 0xff, 0x7f]);
        let image = decode_tga(&data).unwrap();
        assert_eq!(image.rgb(), &[255; 6]);
    }

    #[test]
    fn eight_attribute_bits_make_a_32_bit_alpha() {
        // 2x1, top-down, 8 attribute bits; BGRA.
        let mut data = header(2, 32, 2, 1, 0x28);
        data.extend_from_slice(&[3, 2, 1, 255, 6, 5, 4, 0x40]);
        let image = decode_tga(&data).unwrap();
        assert_eq!(image.get_argb(0, 0), 0xff01_0203);
        assert_eq!(image.get_argb(1, 0), 0x4004_0506);
        // With no attribute bits the fourth byte is padding.
        let mut padded = header(2, 32, 2, 1, 0x20);
        padded.extend_from_slice(&[3, 2, 1, 0, 6, 5, 4, 0]);
        assert!(!decode_tga(&padded).unwrap().has_alpha());
        // An alpha that is 0 everywhere is unused too.
        let mut unused = header(2, 32, 2, 1, 0x28);
        unused.extend_from_slice(&[3, 2, 1, 0, 6, 5, 4, 0]);
        assert!(!decode_tga(&unused).unwrap().has_alpha());
    }

    #[test]
    fn bit_15_is_the_alpha_of_a_16_bit_pixel_with_one_attribute_bit() {
        let mut data = header(2, 16, 2, 1, 0x21);
        data.extend_from_slice(&[0xff, 0xff, 0xff, 0x7f]);
        let image = decode_tga(&data).unwrap();
        assert_eq!(image.get_argb(0, 0), 0xffff_ffff);
        assert_eq!(image.get_argb(1, 0), crate::image::CLEAR);
    }

    #[test]
    fn rows_are_bottom_up_by_default() {
        let mut data = header(3, 8, 1, 2, 0);
        data.extend_from_slice(&[10, 20]);
        let image = decode_tga(&data).unwrap();
        assert_eq!(image.rgb(), &[20, 20, 20, 10, 10, 10]);
    }

    #[test]
    fn colour_map_and_validation() {
        let mut data = header(1, 8, 2, 1, 0x20);
        data[1] = 1;
        data[5] = 2; // two entries
        data[7] = 24;
        data.extend_from_slice(&[1, 2, 3, 4, 5, 6, 1, 0]);
        let image = decode_tga(&data).unwrap();
        assert_eq!(image.rgb(), &[6, 5, 4, 3, 2, 1]);
        data[18 + 6 + 1] = 2; // index outside the map
        assert!(decode_tga(&data).is_err());
        assert!(decode_tga(&data[..20]).is_err());
        assert!(decode_tga(&header(7, 8, 1, 1, 0)).is_err());
    }
}
