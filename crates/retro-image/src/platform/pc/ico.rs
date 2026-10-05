//! Windows icons (ICO) and cursors (CUR).
//!
//! Sources:
//! - Microsoft, "Icons" (ICONDIR and ICONDIRENTRY, DIB-based and PNG-based
//!   images):
//!   <https://learn.microsoft.com/en-us/previous-versions/ms997538(v=msdn.10)>
//!   and the Wikipedia article <https://en.wikipedia.org/wiki/ICO_(file_format)>
//!   (6-byte directory: reserved 0, type 1 icon or 2 cursor, count; 16-byte
//!   entries: width, height (0 means 256), colors, reserved, planes or
//!   hotspot x, bits per pixel or hotspot y, data size, data offset).
//! - Each DIB image is a headerless bitmap whose stored height is twice the
//!   picture height: the color bitmap, then a 1-bit AND mask. See `bmp.rs`
//!   for the DIB layouts.
//! - Verified against Pillow's ICO and CUR decoders on icons found on the
//!   development machine (see the divergence file `pc-ico.tsv`).
//!
//! Only the largest DIB image is decoded (ties go to the higher bit depth).
//! Transparency comes from the alpha channel of a 32-bit image if any pixel in
//! it is visible, and otherwise from the AND mask: a set bit makes the pixel
//! transparent. That includes the cursor pixels that invert the screen (AND and
//! XOR both set), which have no color of their own. Images stored as PNG
//! (Vista-style 256x256) are not supported; a file with only PNG images is
//! rejected.

use alloc::vec::Vec;

use super::bmp::decode_icon_dib;
use crate::bytes::{le16, le32};
use crate::{DecodeError, Image};

const FAIL: DecodeError = DecodeError::Unrecognized;
const DIR_LEN: usize = 6;
const ENTRY_LEN: usize = 16;
/// Size of the `BITMAPINFOHEADER` every DIB-based icon image starts with.
const INFO_HEADER_LEN: u32 = 40;

struct Entry<'a> {
    width: u32,
    height: u32,
    bpp: u16,
    image: &'a [u8],
}

pub(super) fn decode_ico(data: &[u8]) -> Result<Image, DecodeError> {
    let best = entries(data)?
        .into_iter()
        .filter(|e| le32(e.image, 0) == Some(INFO_HEADER_LEN))
        .max_by_key(|e| (e.width * e.height, e.bpp))
        .ok_or(FAIL)?;
    decode_icon_dib(best.image)
}

/// The directory entries, each with its image data. Fails unless the
/// directory is well formed and every image lies inside the file and starts
/// with a DIB or PNG header, which is what makes this a reliable signature.
fn entries(data: &[u8]) -> Result<Vec<Entry<'_>>, DecodeError> {
    if le16(data, 0) != Some(0) || !matches!(le16(data, 2), Some(1 | 2)) {
        return Err(FAIL);
    }
    let count = usize::from(le16(data, 4).ok_or(FAIL)?);
    if count == 0 {
        return Err(FAIL);
    }
    (0..count)
        .map(|i| {
            let at = DIR_LEN + i * ENTRY_LEN;
            let field = data.get(at..at + ENTRY_LEN).ok_or(FAIL)?;
            let size = le32(field, 8).ok_or(FAIL)? as usize;
            let offset = le32(field, 12).ok_or(FAIL)? as usize;
            let end = offset.checked_add(size).ok_or(FAIL)?;
            let image = data.get(offset..end).ok_or(FAIL)?;
            if le32(image, 0) != Some(INFO_HEADER_LEN) && !image.starts_with(b"\x89PNG") {
                return Err(FAIL);
            }
            // A size byte of 0 means 256.
            let dimension = |b: u8| if b == 0 { 256 } else { u32::from(b) };
            Ok(Entry {
                width: dimension(field[0]),
                height: dimension(field[1]),
                bpp: le16(field, 6).ok_or(FAIL)?,
                image,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::CLEAR;

    /// A 2 x 1 icon whose DIB has `bpp` bits per pixel; `pixels` is the one
    /// padded row and `mask` the first byte of the AND mask's row.
    fn icon(bpp: u16, pixels: &[u8], mask: u8) -> Vec<u8> {
        let mut dib = INFO_HEADER_LEN.to_le_bytes().to_vec();
        dib.extend_from_slice(&2i32.to_le_bytes());
        dib.extend_from_slice(&2i32.to_le_bytes()); // the color rows and the mask's
        dib.extend_from_slice(&1u16.to_le_bytes());
        dib.extend_from_slice(&bpp.to_le_bytes());
        dib.extend_from_slice(&[0; 24]);
        dib.extend_from_slice(pixels);
        dib.extend_from_slice(&[mask, 0, 0, 0]);
        let mut file = alloc::vec![0, 0, 1, 0, 1, 0, 2, 1, 0, 0, 1, 0];
        file.extend_from_slice(&bpp.to_le_bytes());
        file.extend_from_slice(&(dib.len() as u32).to_le_bytes());
        file.extend_from_slice(&22u32.to_le_bytes());
        file.extend_from_slice(&dib);
        file
    }

    #[test]
    fn the_and_mask_clears_pixels_of_24_bit_icons() {
        let file = icon(24, &[3, 2, 1, 6, 5, 4, 0, 0], 0b0100_0000);
        let image = decode_ico(&file).unwrap();
        assert_eq!(image.get_argb(0, 0), 0xff01_0203);
        assert_eq!(image.get_argb(1, 0), CLEAR);
    }

    #[test]
    fn alpha_of_a_32_bit_icon_wins_over_the_mask() {
        let file = icon(32, &[3, 2, 1, 255, 6, 5, 4, 0x40], 0b1000_0000);
        let image = decode_ico(&file).unwrap();
        assert_eq!(image.get_argb(0, 0), 0xff01_0203);
        assert_eq!(image.get_argb(1, 0), 0x4004_0506);
    }

    #[test]
    fn a_32_bit_icon_without_visible_alpha_uses_the_mask() {
        let file = icon(32, &[3, 2, 1, 0, 6, 5, 4, 0], 0b0100_0000);
        let image = decode_ico(&file).unwrap();
        assert_eq!(image.get_argb(0, 0), 0xff01_0203);
        assert_eq!(image.get_argb(1, 0), CLEAR);
    }
}
