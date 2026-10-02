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

use crate::bytes::{le16, le32};
use crate::{BitOrder, DecodeError, Format, Image};

pub(super) static FORMATS: &[Format] =
    &[Format::new("Psion Series 3", "mono", &["pic", "icn"], decode_pic).signature()];

fn decode_pic(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    // "PIC" $DC, then format version "00".
    if data.len() < 20 || data[..6] != *b"PIC\xdc00" || le16(data, 6) == Some(0) {
        return Err(fail);
    }
    // The first picture record.
    let word = |at| le16(data, at).map(usize::from).ok_or(fail);
    let (width, height) = (word(10)?, word(12)?);
    let offset = le32(data, 16).ok_or(fail)? as usize;
    let row_len = width.div_ceil(16) * 2;
    let start = 20usize.checked_add(offset).ok_or(fail)?;
    let pixels = data.get(start..start + row_len * height).ok_or(fail)?;
    if width == 0 || height == 0 {
        return Err(fail);
    }
    let colors = [0xffffff, 0];
    Image::from_bits(
        width as u32,
        height as u32,
        pixels,
        row_len,
        BitOrder::LsbFirst,
        colors,
    )
}
