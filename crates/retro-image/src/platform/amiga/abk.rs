//! AMOS banks: sprite (`AmSp`) and icon (`AmIc`) banks.
//!
//! Sources:
//! - Bank layout: <http://alvyn.sourceforge.net/amos_file_formats.html>
//!   (count, per-object width in words, height, depth, hot spot, planar data,
//!   then 32 `0RGB` palette words).
//! - Objects drawn side by side, top-aligned, on color 0: observed from
//!   `recoil2png` output.

use alloc::vec::Vec;

use super::ilbm::half_brite;
use crate::bytes::be16;
use crate::image::{check_size, rgb444};
use crate::{DecodeError, Image};

struct Object<'a> {
    width: usize,
    height: usize,
    depth: usize,
    planes: &'a [u8],
}

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    const FAIL: DecodeError = DecodeError::Invalid;
    let magic = data.get(..4).ok_or(FAIL)?;
    if magic != b"AmSp" && magic != b"AmIc" {
        return Err(FAIL);
    }
    let count = usize::from(be16(data, 4).ok_or(FAIL)?);
    let mut pos = 6;
    let mut objects = Vec::new();
    for _ in 0..count {
        let header = data.get(pos..pos + 10).ok_or(FAIL)?;
        let word = |at| be16(header, at).map(usize::from).ok_or(FAIL);
        let (width, height, depth) = (word(0)? * 16, word(2)?, word(4)?);
        if depth > 6 {
            return Err(FAIL);
        }
        // The product of three header words overflows a 32-bit usize.
        let len = (width / 8)
            .checked_mul(height)
            .and_then(|n| n.checked_mul(depth))
            .ok_or(FAIL)?;
        let end = (pos + 10).checked_add(len).ok_or(FAIL)?;
        let planes = data.get(pos + 10..end).ok_or(FAIL)?;
        pos = end;
        objects.push(Object {
            width,
            height,
            depth,
            planes,
        });
    }
    let palette: Vec<u16> = (0..32)
        .map(|i| be16(data, pos + i * 2))
        .collect::<Option<_>>()
        .ok_or(FAIL)?;
    // 6-plane objects use Extra Half-Brite: colors 32-63 are colors 0-31
    // at half brightness (Amiga Hardware Reference Manual, "Extra Half Brite
    // Mode"), rounded as in ILBM. RECOIL rejects such banks.
    let colors: Vec<u32> = (0..64)
        .map(|i| {
            let color = rgb444(palette[i % 32]);
            if i >= 32 { half_brite(color) } else { color }
        })
        .collect();
    let width: usize = objects.iter().map(|o| o.width).sum();
    let height = objects.iter().map(|o| o.height).max().unwrap_or(0);
    if width > 0xffff {
        return Err(FAIL);
    }
    check_size(width, height)?;
    // Color 0 where no object reaches.
    let mut indices = alloc::vec![0u8; width * height];
    let mut left = 0;
    for object in &objects {
        let row_len = object.width / 8;
        let plane_len = row_len * object.height;
        for y in 0..object.height {
            for x in 0..object.width {
                indices[y * width + left + x] = (0..object.depth).fold(0, |index, plane| {
                    let byte = object.planes[plane * plane_len + y * row_len + x / 8];
                    index | (byte >> (7 - x % 8) & 1) << plane
                });
            }
        }
        left += object.width;
    }
    Image::from_indexed(width as u32, height as u32, &indices, &colors)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn huge_zero_depth_objects_are_rejected_not_allocated() {
        // Depth 0 makes every object's pixel data empty, so the header
        // alone can claim 4096 x 65535 pixels per object.
        let mut data = b"AmSp".to_vec();
        data.extend_from_slice(&1u16.to_be_bytes());
        for word in [256u16, 0xffff, 0, 0, 0] {
            data.extend_from_slice(&word.to_be_bytes());
        }
        data.extend_from_slice(&[0; 64]);
        assert!(decode(&data).is_err());
    }
}
