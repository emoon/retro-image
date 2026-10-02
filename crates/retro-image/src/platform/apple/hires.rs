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

use crate::{DecodeError, Image};

const PAGE_LEN: usize = 0x2000;
const HEIGHT: usize = 192;

/// Offset of line `y` within an 8 KB page.
fn line_offset(y: usize) -> usize {
    (y & 7) << 10 | (y >> 3 & 7) << 7 | ((y >> 6) * 40)
}

pub(super) fn decode_hgr(data: &[u8]) -> Result<Image, DecodeError> {
    if !matches!(data.len(), 0x1ff8 | 0x1ffc | PAGE_LEN) {
        return Err(DecodeError::Unrecognized);
    }
    let mut image = Image::new(280, HEIGHT as u32);
    for y in 0..HEIGHT {
        let line = &data[line_offset(y)..line_offset(y) + 40];
        for x in 0..280 {
            let white = line[x / 7] >> (x % 7) & 1 != 0;
            image.set(x as u32, y as u32, if white { 0xffffff } else { 0 });
        }
    }
    Ok(image)
}

pub(super) fn decode_dhgr(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 2 * PAGE_LEN {
        return Err(DecodeError::Unrecognized);
    }
    let (aux, main) = data.split_at(PAGE_LEN);
    let mut image = Image::new(560, HEIGHT as u32 * 2);
    for y in 0..HEIGHT {
        let offset = line_offset(y);
        for x in 0..560 {
            let (column, bit) = (x / 7, x % 7);
            let page = if column % 2 == 0 { aux } else { main };
            let white = page[offset + column / 2] >> bit & 1 != 0;
            let color = if white { 0xffffff } else { 0 };
            image.set(x as u32, y as u32 * 2, color);
            image.set(x as u32, y as u32 * 2 + 1, color);
        }
    }
    Ok(image)
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
