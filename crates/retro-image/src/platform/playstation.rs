//! Sony PlayStation.
//!
//! Sources:
//! - TIM: psx-spx
//!   (<https://problemkaputt.de/psxspx-cdrom-file-video-texture-image-tim-pxl-clt-sony.htm>)
//!   and the Kaitai `psx_tim.ksy` spec (CC0, <https://formats.kaitai.io/psx_tim/>):
//!   id `$10`, flags (bits 0-2 depth, bit 3 CLUT), blocks of
//!   `[length, x, y, width in halfwords, height, data]`, 15-bit BGR colours.
//! - 5-bit to 8-bit scaling (`v << 3 | v >> 2`, which is `image::bgr555`):
//!   observed from `recoil2png` output.

use crate::bytes::{le16, le32};
use crate::image::{bgr555, check_size};
use crate::{DecodeError, Format, Image};

pub(super) static FORMATS: &[Format] =
    &[Format::new("PlayStation", "TIM", &["tim"], decode_tim).signature()];

/// A block: returns (width in halfwords, height, data, length field).
fn block(data: &[u8]) -> Option<(usize, usize, &[u8], usize)> {
    let len = le32(data, 0)? as usize;
    let (width, height) = (usize::from(le16(data, 8)?), usize::from(le16(data, 10)?));
    let body = data.get(12..12 + width * height * 2)?;
    Some((width, height, body, len))
}

fn decode_tim(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    if le32(data, 0) != Some(0x10) {
        return Err(fail);
    }
    let flags = le32(data, 4).ok_or(fail)?;
    let depth = flags & 7;
    let has_clut = flags & 8 != 0;
    // The other flag bits are reserved but not always zero (PSn00bSDK's
    // tiles_256.tim); recoil2png ignores them too.
    if depth > 3 || (depth < 2) != has_clut {
        return Err(fail);
    }
    let mut pos = 8;
    let clut = if has_clut {
        let (w, h, body, len) = block(&data[pos..]).ok_or(fail)?;
        // The length locates the pixel block, so it must match. (The pixel
        // block's own length is wrong in some files, e.g. PSn00bSDK's
        // texture.tim, and is not needed.)
        if len != 12 + body.len() {
            return Err(fail);
        }
        pos += len;
        // The first palette row.
        &body[..(w * 2).min(body.len()).min(w * h * 2)]
    } else {
        &[]
    };
    let (w, height, pixels, _) = block(data.get(pos..).ok_or(fail)?).ok_or(fail)?;
    let width = match depth {
        0 => w * 4,
        1 => w * 2,
        2 => w,
        _ => w * 2 / 3,
    };
    check_size(width, height)?;
    let row_len = w * 2;
    let lookup = |i: usize| le16(clut, i * 2).map_or(0, bgr555);
    let mut image = Image::new(width as u32, height as u32);
    for y in 0..height {
        let row = &pixels[y * row_len..(y + 1) * row_len];
        for x in 0..width {
            let color = match depth {
                0 => lookup(usize::from(row[x / 2] >> (x % 2 * 4) & 15)),
                1 => lookup(usize::from(row[x])),
                2 => bgr555(u16::from_le_bytes([row[x * 2], row[x * 2 + 1]])),
                _ => {
                    let p = &row[x * 3..x * 3 + 3];
                    u32::from(p[0]) << 16 | u32::from(p[1]) << 8 | u32::from(p[2])
                }
            };
            image.set(x as u32, y as u32, color);
        }
    }
    Ok(image)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_bit_pictures_over_the_pixel_cap_are_rejected() {
        // 4 pixels per halfword: 65600 x 1025.
        let (halfwords, height) = (16400usize, 1025usize);
        let mut data = alloc::vec::Vec::new();
        data.extend_from_slice(&0x10u32.to_le_bytes());
        data.extend_from_slice(&8u32.to_le_bytes());
        // One palette row of 16 colours, then the pixel block.
        data.extend_from_slice(&(12 + 32u32).to_le_bytes());
        data.extend_from_slice(&[0; 4]);
        data.extend_from_slice(&16u16.to_le_bytes());
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&[0; 32]);
        // Pixel block: length, x, y.
        data.extend_from_slice(&[0; 8]);
        data.extend_from_slice(&(halfwords as u16).to_le_bytes());
        data.extend_from_slice(&(height as u16).to_le_bytes());
        data.resize(data.len() + halfwords * 2 * height, 0);
        assert!(matches!(decode_tim(&data), Err(DecodeError::Unrecognized)));
    }
}
