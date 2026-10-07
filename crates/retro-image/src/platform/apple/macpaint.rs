//! MacPaint (`PNTG`).
//!
//! Sources:
//! - Layout (512-byte header, then 720 PackBits rows of 72 bytes; bit set =
//!   black): Apple Technical Note PT24
//!   (<https://leopard-adc.pepas.com/technotes/pt/pt_24.html>) and
//!   CiderPress II notes (<https://ciderpress2.com/formatdoc/MacPaint-notes.html>).
//! - Optional MacBinary header, recognised by file type `PNTG`: see
//!   `crate::macbinary`.

use crate::codec::packbits;
use crate::macbinary::MacBinary;
use crate::{BitOrder, DecodeError, Image};

const WIDTH: usize = 576;
const HEIGHT: usize = 720;
const HEADER_LEN: usize = 512;

/// A MacPaint file with a MacBinary header of file type `PNTG`.
pub(super) fn decode_mac_binary(data: &[u8]) -> Result<Image, DecodeError> {
    match MacBinary::parse(data) {
        Some(file) if file.file_type == *b"PNTG" => decode(file.data_fork),
        _ => Err(DecodeError::Invalid),
    }
}

/// A bare MacPaint file.
pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    // The header starts with a version number: 0, 2 or 3.
    if data.len() <= HEADER_LEN || data[..3] != [0, 0, 0] || data[3] > 3 {
        return Err(DecodeError::Invalid);
    }
    let (bitmap, _) =
        packbits::unpack(&data[HEADER_LEN..], WIDTH / 8 * HEIGHT).ok_or(DecodeError::Invalid)?;
    // Most significant bit leftmost, set bit black.
    let colors = [0xffffff, 0];
    Image::from_bits(
        WIDTH as u32,
        HEIGHT as u32,
        &bitmap,
        WIDTH / 8,
        BitOrder::MsbFirst,
        colors,
    )
}
