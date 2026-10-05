//! Indexed-colour screens of the ZX Evolution TS-Conf (SXG).
//!
//! Sources:
//! - SXG header: hype.retroscene.org sXg article,
//!   <https://hype.retroscene.org/blog/126.html>; palette entry encodings
//!   (bit 15 set: 5-bit RGB, clear: 25-level TS-Conf CLUT indices) from the
//!   CC0 moroz1999/sxg writer, <https://github.com/moroz1999/sxg>.
//! - CLUT level scaling (`level * 255 / 24`, rounded down) and nibble order:
//!   observed from `recoil2png` output.

use super::screen::Frame;
use crate::image::{check_size, widen_channel};
use crate::{DecodeError, Image};

const SXG_HEADER_LEN: usize = 16;

/// SXG: `\x7FSXG`, version, background, packing (0 only), format (1: 16
/// colours, 2: 256 colours), width, height, then the palette and bitmap
/// offsets, each relative to the end of its own field.
pub(super) fn decode_sxg(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() < SXG_HEADER_LEN || !data.starts_with(b"\x7fSXG") || data[6] != 0 {
        return Err(DecodeError::Unrecognized);
    }
    let word = |offset: usize| usize::from(u16::from_le_bytes([data[offset], data[offset + 1]]));
    let bits_per_pixel = match data[7] {
        1 => 4,
        2 => 8,
        _ => return Err(DecodeError::Unrecognized),
    };
    let (width, height) = (word(8), word(10));
    let palette_start = 14 + word(12);
    let bitmap_start = 16 + word(14);
    check_size(width, height)?;
    let bitmap_len = (width * height * bits_per_pixel).div_ceil(8);
    if palette_start > bitmap_start {
        return Err(DecodeError::Unrecognized);
    }
    let palette = &data[palette_start.min(data.len())..bitmap_start.min(data.len())];
    let bitmap = data
        .get(bitmap_start..bitmap_start + bitmap_len)
        .ok_or(DecodeError::Unrecognized)?;
    let mut frame = Frame::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let index = if bits_per_pixel == 8 {
                bitmap[y * width + x]
            } else {
                let byte = bitmap[(y * width + x) / 2];
                if x % 2 == 0 { byte >> 4 } else { byte & 15 }
            };
            let entry = palette
                .get(usize::from(index) * 2..usize::from(index) * 2 + 2)
                .map_or(0, |e| u16::from_le_bytes([e[0], e[1]]));
            frame.set(x, y, tsconf_color(entry));
        }
    }
    Ok(frame.into_image())
}

/// TS-Conf palette entry: 5 bits per channel (red 14-10, green 9-5, blue
/// 4-0). With bit 15 set the values are plain 5-bit levels, otherwise
/// indices 0-24 of the hardware's 25 PWM levels.
fn tsconf_color(entry: u16) -> u32 {
    let direct = entry & 0x8000 != 0;
    let channel = |shift: u16| {
        let v = u32::from((entry >> shift) & 31);
        if direct {
            widen_channel(v, 5)
        } else {
            v.min(24) * 255 / 24
        }
    };
    channel(10) << 16 | channel(5) << 8 | channel(0)
}
