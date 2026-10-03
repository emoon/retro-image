//! Dynamic Publisher (MSX2 desktop publishing) screens, fonts and stamps.
//!
//! Sources:
//! - MarMSX, "Dynamic Publisher Screen (.PCT)", "Font (.FNT)" and "Shape (.STP)"
//!   (<https://marmsx.msxall.com/projetos/sketch/pct_en.php>,
//!   <https://marmsx.msxall.com/projetos/sketch/fnt_en.php>,
//!   <https://marmsx.msxall.com/projetos/sketch/stp_en.php>): headers, the RLE scheme, nibble-swapped 8x1 patterns with
//!   1 = black, and stamps of 4 two-bit pixels per byte.
//! - Observed from `recoil2png` output: pictures are shown black on white with
//!   doubled lines (Screen 6 pixels are twice as tall as wide); any non-zero
//!   stamp pixel is black.

use alloc::vec::Vec;

use crate::image::check_size;
use crate::{DecodeError, Image};

const BLACK: u32 = 0x000000;
const WHITE: u32 = 0xffffff;

/// Unpacks rows of 8x1 patterns: a counter with bit 7 set repeats the next
/// byte (counter & 0x7f) + 1 times, otherwise (counter + 1) literal bytes follow.
fn unpack(packed: &[u8], len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(len);
    let mut i = 0;
    while out.len() < len && i < packed.len() {
        let counter = packed[i];
        let count = (counter & 0x7f) as usize + 1;
        if counter & 0x80 != 0 {
            let Some(&value) = packed.get(i + 1) else {
                break;
            };
            out.extend(core::iter::repeat_n(value, count));
            i += 2;
        } else {
            let end = (i + 1 + count).min(packed.len());
            out.extend_from_slice(&packed[i + 1..end]);
            i = end;
        }
    }
    out.resize(len, 0);
    out
}

/// Black-on-white picture with doubled lines; `pixel(x, y)` tells if a dot is set.
fn mono(
    width: usize,
    height: usize,
    pixel: impl Fn(usize, usize) -> bool,
) -> Result<Image, DecodeError> {
    let indices: Vec<u8> = (0..width * height)
        .map(|i| u8::from(pixel(i % width, i / width)))
        .collect();
    Ok(Image::from_indexed(width as u32, height as u32, &indices, &[WHITE, BLACK])?.scaled(1, 2))
}

/// 512-wide picture of RLE-packed, nibble-swapped patterns at `offset`.
fn decode_packed(
    data: &[u8],
    signature: &[u8],
    offset: usize,
    height: usize,
) -> Result<Image, DecodeError> {
    if !data.starts_with(signature) || data.len() <= offset {
        return Err(DecodeError::Unrecognized);
    }
    let patterns = unpack(&data[offset..], 64 * height);
    mono(512, height, |x, y| {
        let byte = patterns[y * 64 + x / 8].rotate_left(4);
        byte & (0x80 >> (x % 8)) != 0
    })
}

/// 512x704 screen.
pub(super) fn decode_pct(data: &[u8]) -> Result<Image, DecodeError> {
    decode_packed(data, b"DYNAMIC PUBLISHER SCREEN", 0x180, 704)
}

/// 512x160 font sheet of 32x8 characters.
pub(super) fn decode_fnt(data: &[u8]) -> Result<Image, DecodeError> {
    decode_packed(data, b"DYNAMIC PUBLISHER FONT", 0x200, 160)
}

/// Stamp: width and height (LE16), then 4 two-bit pixels per byte, rows not padded.
pub(super) fn decode_stp(data: &[u8]) -> Result<Image, DecodeError> {
    let header = data.get(..4).ok_or(DecodeError::Unrecognized)?;
    let width = u16::from_le_bytes([header[0], header[1]]) as usize;
    let height = u16::from_le_bytes([header[2], header[3]]) as usize;
    let pixels = &data[4..];
    if width == 0 || height == 0 || pixels.len() < (width * height).div_ceil(4) {
        return Err(DecodeError::Unrecognized);
    }
    check_size(width, height)?;
    mono(width, height, |x, y| {
        let i = y * width + x;
        (pixels[i / 4] >> (6 - 2 * (i % 4))) & 3 != 0
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unpack_follows_marmsx_example() {
        let packed = [0x01, 0x08, 0x04, 0xbd, 0x00, 0xbf, 0x00];
        let out = unpack(&packed, 128);
        assert_eq!(&out[..3], &[0x08, 0x04, 0]);
        assert_eq!(out.len(), 128);
    }

    #[test]
    fn stamp_needs_all_pixels() {
        let data = [3, 0, 2, 0, 0b0100_0000, 0b0000_0011];
        let image = decode_stp(&data).unwrap();
        assert_eq!((image.width(), image.height()), (3, 4));
        assert_eq!(&image.rgb()[..6], &[0, 0, 0, 0xff, 0xff, 0xff]);
        assert!(decode_stp(&data[..5]).is_err());
    }

    #[test]
    fn stamp_larger_than_the_pixel_cap_is_rejected() {
        let (width, height) = (8192usize, 8200usize);
        let mut data = alloc::vec![0u8; 4 + (width * height).div_ceil(4)];
        data[..2].copy_from_slice(&(width as u16).to_le_bytes());
        data[2..4].copy_from_slice(&(height as u16).to_le_bytes());
        assert!(decode_stp(&data).is_err());
    }
}
