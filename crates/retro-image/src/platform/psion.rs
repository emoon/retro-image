//! Psion Series 3.
//!
//! Sources:
//! - PIC/ICN: Psionics files, `bitmap.fmt`
//!   (<https://www.davros.org/psion/psionics/bitmap.fmt>): header "PIC" `$DC`,
//!   version, bitmap count, then 12-byte records (CRC, width, height, data
//!   size, offset from the end of the record); rows padded to even length,
//!   bit 0 leftmost.
//! - Only the first bitmap is shown, set bit = black: observed from
//!   `recoil2png` output.

use crate::{DecodeError, Format, Image};

pub(super) static FORMATS: &[Format] = &[Format::new(
    "Psion Series 3",
    "mono",
    &["pic", "icn"],
    decode_pic,
)];

fn le16(b: &[u8]) -> usize {
    usize::from(u16::from_le_bytes([b[0], b[1]]))
}

fn decode_pic(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    if data.len() < 20 || data[..4] != *b"PIC\xdc" || le16(&data[6..8]) == 0 {
        return Err(fail);
    }
    let record = &data[8..20];
    let (width, height) = (le16(&record[2..4]), le16(&record[4..6]));
    let offset = u32::from_le_bytes([record[8], record[9], record[10], record[11]]) as usize;
    let row_len = width.div_ceil(16) * 2;
    let start = 20usize.checked_add(offset).ok_or(fail)?;
    let pixels = data.get(start..start + row_len * height).ok_or(fail)?;
    if width == 0 || height == 0 {
        return Err(fail);
    }
    let mut image = Image::new(width as u32, height as u32);
    for y in 0..height {
        for x in 0..width {
            let set = pixels[y * row_len + x / 8] >> (x % 8) & 1 != 0;
            image.set(x as u32, y as u32, if set { 0 } else { 0xffffff });
        }
    }
    Ok(image)
}
