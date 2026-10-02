//! Sony PlayStation.
//!
//! Sources:
//! - TIM: psx-spx
//!   (<https://problemkaputt.de/psxspx-cdrom-file-video-texture-image-tim-pxl-clt-sony.htm>)
//!   and the Kaitai `psx_tim.ksy` spec (CC0, <https://formats.kaitai.io/psx_tim/>):
//!   id `$10`, flags (bits 0-2 depth, bit 3 CLUT), blocks of
//!   `[length, x, y, width in halfwords, height, data]`, 15-bit BGR colours.
//! - 5-bit to 8-bit scaling (`v << 3 | v >> 2`): observed from `recoil2png`
//!   output.

use crate::{DecodeError, Format, Image};

pub(super) static FORMATS: &[Format] =
    &[Format::new("PlayStation", "TIM", &["tim"], decode_tim).signature()];

fn le16(b: &[u8]) -> usize {
    usize::from(u16::from_le_bytes([b[0], b[1]]))
}

fn le32(b: &[u8]) -> usize {
    u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize
}

/// A block: returns (width in halfwords, height, data, length field).
fn block(data: &[u8]) -> Option<(usize, usize, &[u8], usize)> {
    let header = data.get(..12)?;
    let len = le32(&header[0..4]);
    let (width, height) = (le16(&header[8..10]), le16(&header[10..12]));
    let body = data.get(12..12 + width * height * 2)?;
    Some((width, height, body, len))
}

/// 15-bit colour: red in bits 0-4, green 5-9, blue 10-14.
fn color15(word: usize) -> u32 {
    let channel = |shift: usize| {
        let v = (word >> shift & 31) as u32;
        v << 3 | v >> 2
    };
    channel(0) << 16 | channel(5) << 8 | channel(10)
}

fn decode_tim(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let header = data.get(..8).ok_or(fail)?;
    if le32(&header[0..4]) != 0x10 {
        return Err(fail);
    }
    let flags = le32(&header[4..8]);
    let depth = flags & 7;
    let has_clut = flags & 8 != 0;
    if flags & !0xf != 0 || depth > 3 || (depth < 2) != has_clut {
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
    if width == 0 || height == 0 {
        return Err(fail);
    }
    let row_len = w * 2;
    let lookup = |i: usize| clut.get(i * 2..i * 2 + 2).map_or(0, |c| color15(le16(c)));
    let mut image = Image::new(width as u32, height as u32);
    for y in 0..height {
        let row = &pixels[y * row_len..(y + 1) * row_len];
        for x in 0..width {
            let color = match depth {
                0 => lookup(usize::from(row[x / 2] >> (x % 2 * 4) & 15)),
                1 => lookup(usize::from(row[x])),
                2 => color15(le16(&row[x * 2..])),
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
