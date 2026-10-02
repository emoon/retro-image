//! MacPaint (`PNTG`).
//!
//! Sources:
//! - Layout (512-byte header, then 720 PackBits rows of 72 bytes; bit set =
//!   black): Apple Technical Note PT24
//!   (<https://leopard-adc.pepas.com/technotes/pt/pt_24.html>) and
//!   CiderPress II notes (<https://ciderpress2.com/formatdoc/MacPaint-notes.html>).
//! - Optional 128-byte MacBinary header (name length at +1, file type at
//!   +65): MacBinary specification, recognised by type `PNTG`.

use crate::codec::packbits;
use crate::{DecodeError, Image};

const WIDTH: usize = 576;
const HEIGHT: usize = 720;
const HEADER_LEN: usize = 512;
const MAC_BINARY_LEN: usize = 128;

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    let data = if is_mac_binary(data) {
        &data[MAC_BINARY_LEN..]
    } else {
        data
    };
    // The header starts with a version number: 0, 2 or 3.
    if data.len() <= HEADER_LEN || data[..3] != [0, 0, 0] || data[3] > 3 {
        return Err(DecodeError::Unrecognized);
    }
    let (bitmap, _) = packbits::unpack(&data[HEADER_LEN..], WIDTH / 8 * HEIGHT)
        .ok_or(DecodeError::Unrecognized)?;
    Ok(super::mono_image(&bitmap, WIDTH, HEIGHT, true))
}

fn is_mac_binary(data: &[u8]) -> bool {
    data.len() > MAC_BINARY_LEN
        && data[0] == 0
        && (1..=63).contains(&data[1])
        && &data[65..69] == b"PNTG"
}
