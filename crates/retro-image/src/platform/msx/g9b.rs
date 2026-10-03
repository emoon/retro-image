//! G9B pictures of the GFX9k library for the V9990 (GFX9000).
//!
//! Sources:
//! - Header layout (`G9B`, header size, depth, colour type, colours, width,
//!   height, compression, 24-bit data size, 3-byte palette entries): Team Bomba,
//!   "G9B - GFX9000 Bitmap" (<https://www.teambomba.net/g9b.html>).
//! - V9990 16-bit pixels (`GGGGGRRRRRBBBBB`) and the YUV conversion
//!   (R = Y + J, B = Y + K, G = (5Y - 2J - K) / 4, rounding down): reverse
//!   engineered from corpus samples against `recoil2png` output.
//! - 256 fixed colours use the MSX Graphic 7 `GGGRRRBB` encoding and YJK the
//!   MSX2+ conversion: observed from `recoil2png` output.
//! - BitBuster compression: see `bitbuster.rs`.

use super::bitbuster;
use super::vdp::{self, level5};
use crate::{DecodeError, Image};

/// Largest picture accepted, in pixels.
const MAX_PIXELS: usize = 1 << 22;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Colours {
    Palette,
    Fixed256,
    Yjk,
    Yuv,
}

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    decode_inner(data).ok_or(DecodeError::Unrecognized)
}

fn decode_inner(data: &[u8]) -> Option<Image> {
    if !data.starts_with(b"G9B") {
        return None;
    }
    let header_size = u16::from_le_bytes([*data.get(3)?, *data.get(4)?]) as usize;
    let header = data.get(5..5 + header_size.max(11))?;
    let depth = header[0] as usize;
    let colours = match (depth, header[1]) {
        (_, 0) => Colours::Palette,
        (8, 0x40) => Colours::Fixed256,
        (8, 0x80) => Colours::Yjk,
        (8, 0xc0) => Colours::Yuv,
        _ => return None,
    };
    let palette_len = header[2] as usize * 3;
    let width = u16::from_le_bytes([header[3], header[4]]) as usize;
    let height = u16::from_le_bytes([header[5], header[6]]) as usize;
    let compression = header[7];
    let data_size = u32::from_le_bytes([header[8], header[9], header[10], 0]) as usize;
    if !matches!(depth, 2 | 4 | 8 | 16) || width == 0 || height == 0 || width * height > MAX_PIXELS
    {
        return None;
    }
    let palette_at = 5 + header_size;
    let palette = data.get(palette_at..palette_at + palette_len)?;
    let packed = data.get(palette_at + palette_len..)?;
    let needed = (width * height * depth).div_ceil(8);
    let pixels = match compression {
        0 => packed.get(..data_size.min(packed.len()))?.to_vec(),
        1 => bitbuster::unpack(packed, needed)?,
        _ => return None,
    };
    if pixels.len() < needed {
        return None;
    }
    let palette_colour = |index: usize| {
        palette
            .get(index * 3..index * 3 + 3)
            .map_or(0, |c| level5(c[0]) << 16 | level5(c[1]) << 8 | level5(c[2]))
    };

    let mut image = Image::new(width as u32, height as u32);
    for y in 0..height {
        let row = y * width;
        let mut group_colours = [0; 4];
        for x in 0..width {
            let i = row + x;
            let colour = match (depth, colours) {
                (16, _) => {
                    let v = u16::from_le_bytes([pixels[2 * i], pixels[2 * i + 1]]);
                    let five = |shift: u16| level5((v >> shift) as u8);
                    five(5) << 16 | five(10) << 8 | five(0)
                }
                (_, Colours::Fixed256) => vdp::graphic7(pixels[i]),
                (_, Colours::Yjk | Colours::Yuv) => {
                    if x & 3 == 0 {
                        let bytes = [0, 1, 2, 3].map(|k| pixels.get(i + k).copied().unwrap_or(0));
                        group_colours = if colours == Colours::Yjk {
                            vdp::yjk_group(bytes, false, &[0; 16])
                        } else {
                            yuv(bytes)
                        };
                    }
                    group_colours[x & 3]
                }
                _ => {
                    let bit = i * depth;
                    let value =
                        (pixels[bit / 8] >> (8 - depth - bit % 8)) & ((1u16 << depth) - 1) as u8;
                    palette_colour(value as usize)
                }
            };
            image.set(x as u32, y as u32, colour);
        }
    }
    Some(image)
}

/// V9990 YUV: four Y values sharing K (low bits of bytes 0-1) and J (bytes 2-3).
fn yuv(bytes: [u8; 4]) -> [u32; 4] {
    let signed6 = |v: u8| ((v as i32) << 26) >> 26;
    let k = signed6((bytes[0] & 7) | (bytes[1] & 7) << 3);
    let j = signed6((bytes[2] & 7) | (bytes[3] & 7) << 3);
    bytes.map(|b| {
        let y = (b >> 3) as i32;
        let clamp = |v: i32| v.clamp(0, 31) as u8;
        let r = clamp(y + j);
        let g = clamp((5 * y - 2 * j - k).div_euclid(4));
        let b = clamp(y + k);
        level5(r) << 16 | level5(g) << 8 | level5(b)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    fn g9b(depth: u8, kind: u8, colours: u8, width: u16, height: u16, rest: &[u8]) -> Vec<u8> {
        let mut data = b"G9B\x0b\0".to_vec();
        data.extend([depth, kind, colours]);
        data.extend(width.to_le_bytes());
        data.extend(height.to_le_bytes());
        data.extend([0, rest.len() as u8, 0, 0]);
        data.extend(rest);
        data
    }

    #[test]
    fn uncompressed_16_bit() {
        let data = g9b(16, 0, 0, 1, 1, &[0x1f, 0x7c]);
        // Green = 31, red = 0, blue = 31.
        assert_eq!(decode(&data).unwrap().rgb(), &[0, 0xff, 0xff]);
    }

    #[test]
    fn palette_2_bit() {
        let data = g9b(2, 0, 2, 4, 1, &[0, 0, 0, 31, 0, 0, 0b0001_0000]);
        assert_eq!(&decode(&data).unwrap().rgb()[..6], &[0, 0, 0, 0xff, 0, 0]);
        assert!(decode(&data[..data.len() - 1]).is_err());
    }

    #[test]
    fn yuv_without_chroma() {
        assert_eq!(yuv([0x80; 4]), [0x84a584; 4]);
    }
}
