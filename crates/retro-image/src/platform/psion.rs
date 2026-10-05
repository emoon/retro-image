//! Psion Series 3 pictures, and the Series 5 (EPOC) bitmaps of `psion/epoc.rs`.
//!
//! Sources:
//! - PIC/ICN: Psionics files, `bitmap.fmt`
//!   (<https://www.davros.org/psion/psionics/bitmap.fmt>): header "PIC" `$DC`,
//!   version, bitmap count, then 12-byte records (CRC, width, height, data
//!   size, offset from the end of the record); rows padded to even length,
//!   bit 0 leftmost.
//! - Only the first bitmap is shown, set bit = black: observed from
//!   `recoil2png` output.

mod epoc;

use crate::bytes::{le16, le32};
use crate::image::check_size;
use crate::{BitOrder, DecodeError, Format, Image};

pub(super) static FORMATS: &[Format] = &[
    Format::new("Psion Series 3", "mono", &["pic", "icn"], decode_pic).signature(),
    Format::new("Psion Series 5", "Multi-bitmap", &["mbm"], epoc::decode_mbm).signature(),
    // Sketch files have no extension, only their UIDs.
    Format::new("Psion Series 5", "Sketch", &[], epoc::decode_sketch).signature(),
    Format::new(
        "Psion Series 5",
        "Application info",
        &["aif"],
        epoc::decode_aif,
    )
    .signature(),
];

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
    check_size(width, height)?;
    let row_len = width.div_ceil(16) * 2;
    let start = 20usize.checked_add(offset).ok_or(fail)?;
    let end = row_len
        .checked_mul(height)
        .and_then(|len| start.checked_add(len))
        .ok_or(fail)?;
    let pixels = data.get(start..end).ok_or(fail)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pictures_over_the_pixel_cap_are_rejected() {
        let (width, height) = (65535usize, 1025usize);
        let row_len = width.div_ceil(16) * 2;
        let mut data = alloc::vec![0u8; 20 + row_len * height];
        data[..6].copy_from_slice(b"PIC\xdc00");
        data[6] = 1;
        data[10..12].copy_from_slice(&(width as u16).to_le_bytes());
        data[12..14].copy_from_slice(&(height as u16).to_le_bytes());
        assert!(matches!(decode_pic(&data), Err(DecodeError::Unrecognized)));
    }
}
