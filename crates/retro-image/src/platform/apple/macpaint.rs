//! MacPaint (`PNTG`).
//!
//! Sources:
//! - Layout (512-byte header, then 720 PackBits rows of 72 bytes; bit set =
//!   black): Apple Technical Note PT24
//!   (<https://leopard-adc.pepas.com/technotes/pt/pt_24.html>) and
//!   CiderPress II notes (<https://ciderpress2.com/formatdoc/MacPaint-notes.html>).
//! - Optional 128-byte MacBinary header (name length at +1, file type at
//!   +65): MacBinary specification, recognised by type `PNTG`.

use alloc::vec::Vec;

use crate::codec::packbits;
use crate::{DecodeError, Image};

const WIDTH: usize = 576;
const HEIGHT: usize = 720;
const HEADER_LEN: usize = 512;
const MAC_BINARY_LEN: usize = 128;

/// A MacPaint file with a MacBinary header of file type `PNTG`.
pub(super) fn decode_mac_binary(data: &[u8]) -> Result<Image, DecodeError> {
    if !is_mac_binary(data) {
        return Err(DecodeError::Unrecognized);
    }
    decode(&data[MAC_BINARY_LEN..])
}

/// A bare MacPaint file.
pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    // The header starts with a version number: 0, 2 or 3.
    if data.len() <= HEADER_LEN || data[..3] != [0, 0, 0] || data[3] > 3 {
        return Err(DecodeError::Unrecognized);
    }
    let (bitmap, _) = packbits::unpack(&data[HEADER_LEN..], WIDTH / 8 * HEIGHT)
        .ok_or(DecodeError::Unrecognized)?;
    // Most significant bit leftmost, set bit black.
    let indices: Vec<u8> = bitmap
        .iter()
        .flat_map(|&b| (0..8).rev().map(move |i| b >> i & 1))
        .collect();
    Image::from_indexed(WIDTH as u32, HEIGHT as u32, &indices, &[0xffffff, 0])
}

fn is_mac_binary(data: &[u8]) -> bool {
    data.len() > MAC_BINARY_LEN
        && data[0] == 0
        && (1..=63).contains(&data[1])
        && &data[65..69] == b"PNTG"
}
