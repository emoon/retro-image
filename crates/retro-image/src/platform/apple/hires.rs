//! Apple II High-Resolution and Apple IIe Double High-Resolution screen dumps.
//!
//! Sources:
//! - Screen layout (line interleave, 7 pixels per byte with the least
//!   significant bit leftmost, bit 7 ignored, file sizes; Double Hi-Res
//!   auxiliary half first, aux and main bytes alternating across the line):
//!   CiderPress II notes (<https://ciderpress2.com/formatdoc/HiRes-notes.html>,
//!   <https://ciderpress2.com/formatdoc/DoubleHiRes-notes.html>).
//! - Monochrome rendering (set bit = white) and doubled lines for Double
//!   Hi-Res: observed from `recoil2png` output.

use alloc::vec::Vec;

use crate::{DecodeError, Image};

const PAGE_LEN: usize = 0x2000;
const HEIGHT: usize = 192;

/// Offset of line `y` within an 8 KB page.
fn line_offset(y: usize) -> usize {
    (y & 7) << 10 | (y >> 3 & 7) << 7 | ((y >> 6) * 40)
}

pub(super) fn decode_hgr(data: &[u8]) -> Result<Image, DecodeError> {
    if !matches!(data.len(), 0x1ff8 | 0x1ffc | PAGE_LEN) {
        return Err(DecodeError::Invalid);
    }
    let indices: Vec<u8> = (0..HEIGHT)
        .flat_map(|y| line_pixels(&data[line_offset(y)..line_offset(y) + 40]))
        .collect();
    Image::from_indexed(280, HEIGHT as u32, &indices, &BLACK_WHITE)
}

const BLACK_WHITE: [u32; 2] = [0, 0xffffff];

/// The 7 pixels of each byte, least significant bit leftmost; bit 7 ignored.
fn line_pixels(bytes: &[u8]) -> impl Iterator<Item = u8> + '_ {
    bytes.iter().flat_map(|&b| (0..7).map(move |i| b >> i & 1))
}

pub(super) fn decode_dhgr(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 2 * PAGE_LEN {
        return Err(DecodeError::Invalid);
    }
    let (aux, main) = data.split_at(PAGE_LEN);
    let mut indices = Vec::with_capacity(560 * HEIGHT);
    for y in 0..HEIGHT {
        let at = line_offset(y);
        for (&aux, &main) in aux[at..at + 40].iter().zip(&main[at..at + 40]) {
            indices.extend(line_pixels(&[aux, main]));
        }
    }
    Image::from_indexed(560, HEIGHT as u32, &indices, &BLACK_WHITE)?.scaled(1, 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_offsets_interleave() {
        assert_eq!(line_offset(0), 0);
        assert_eq!(line_offset(1), 0x400);
        assert_eq!(line_offset(8), 0x80);
        assert_eq!(line_offset(64), 0x28);
        assert_eq!(line_offset(191), 0x1fd0);
    }
}
