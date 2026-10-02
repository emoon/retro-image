//! Dali compressed (`LPK`, `MPK`, `HPK`) and ZZ_ROUGH (`RGH`) pictures.
//!
//! Sources:
//! - <https://temlib.org/AtariForumWiki/index.php/Dali_Compressed_file_format>
//!   (including Lonny Pursell's public-domain decode loop)
//! - <https://temlib.org/AtariForumWiki/index.php/ZZ_ROUGH_file_format>

use super::common::{Resolution, SCREEN_LEN, decode_screen, palette_words};
use crate::{DecodeError, Image};

pub(super) fn decode_lpk(data: &[u8]) -> Result<Image, DecodeError> {
    decode_pk(data, Resolution::Low).ok_or(DecodeError::Unrecognized)
}

pub(super) fn decode_mpk(data: &[u8]) -> Result<Image, DecodeError> {
    decode_pk(data, Resolution::Medium).ok_or(DecodeError::Unrecognized)
}

pub(super) fn decode_hpk(data: &[u8]) -> Result<Image, DecodeError> {
    decode_pk(data, Resolution::High).ok_or(DecodeError::Unrecognized)
}

/// Palette, ASCII byte-table size, ASCII long-table size, tables.
fn decode_pk(data: &[u8], resolution: Resolution) -> Option<Image> {
    let words = palette_words(data, 0, 16)?;
    let (byte_len, pos) = ascii_number(data, 32)?;
    let (long_len, pos) = ascii_number(data, pos)?;
    let bitmap = expand(data, pos, byte_len, long_len)?;
    decode_screen(resolution, &bitmap, &words)
}

/// `(c)F.MARCHAL`, ASCII byte-table size, palette, tables.
pub(super) fn decode_rgh(data: &[u8]) -> Result<Image, DecodeError> {
    decode_rgh_inner(data).ok_or(DecodeError::Unrecognized)
}

fn decode_rgh_inner(data: &[u8]) -> Option<Image> {
    if data.get(..12)? != b"(c)F.MARCHAL" {
        return None;
    }
    let (byte_len, pos) = ascii_number(data, 12)?;
    let words = palette_words(data, pos, 16)?;
    let bitmap = expand(data, pos + 32, byte_len, byte_len.checked_mul(4)?)?;
    decode_screen(Resolution::Low, &bitmap, &words)
}

/// Parses decimal digits terminated by CR LF; returns the value and the
/// position after the line end.
fn ascii_number(data: &[u8], mut pos: usize) -> Option<(usize, usize)> {
    let start = pos;
    let mut value = 0usize;
    while let Some(digit @ b'0'..=b'9') = data.get(pos).copied() {
        value = value
            .checked_mul(10)?
            .checked_add(usize::from(digit - b'0'))?;
        pos += 1;
    }
    if pos == start || data.get(pos..pos + 2)? != b"\r\n" {
        return None;
    }
    Some((value, pos + 2))
}

/// Fills the screen four bytes at a time, column by column (each column
/// 200 lines tall), from runs: byte table entry = run length, long table
/// entry = the four bytes.
fn expand(
    data: &[u8],
    pos: usize,
    byte_len: usize,
    long_len: usize,
) -> Option<alloc::vec::Vec<u8>> {
    let counts = data.get(pos..pos.checked_add(byte_len)?)?;
    let longs = data.get(pos + byte_len..pos + byte_len + long_len)?;
    let mut bitmap = alloc::vec![0u8; SCREEN_LEN];
    let mut index = 0;
    let mut remaining = 0u8;
    let mut value: &[u8] = &[0; 4];
    for column in (0..160).step_by(4) {
        for line in 0..200 {
            if remaining == 0 {
                remaining = *counts.get(index)?;
                value = longs.get(index * 4..index * 4 + 4)?;
                index += 1;
            }
            let offset = line * 160 + column;
            bitmap[offset..offset + 4].copy_from_slice(value);
            remaining = remaining.wrapping_sub(1);
        }
    }
    Some(bitmap)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ascii_sizes() {
        assert_eq!(ascii_number(b"x123\r\n", 1), Some((123, 6)));
        assert_eq!(ascii_number(b"12\n", 0), None);
        assert_eq!(ascii_number(b"\r\n", 0), None);
    }
}
