//! AMOS banks: sprite (`AmSp`) and icon (`AmIc`) banks.
//!
//! Sources:
//! - Bank layout: <http://alvyn.sourceforge.net/amos_file_formats.html>
//!   (count, per-object width in words, height, depth, hot spot, planar data,
//!   then 32 `0RGB` palette words).
//! - Objects drawn side by side, top-aligned, on colour 0: observed from
//!   `recoil2png` output.

use alloc::vec::Vec;

use super::ilbm::rgb12;
use crate::bytes::be16;
use crate::{DecodeError, Image};

struct Object<'a> {
    width: usize,
    height: usize,
    depth: usize,
    planes: &'a [u8],
}

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let magic = data.get(..4).ok_or(fail)?;
    if magic != b"AmSp" && magic != b"AmIc" {
        return Err(fail);
    }
    let count = usize::from(be16(data, 4).ok_or(fail)?);
    let mut pos = 6;
    let mut objects = Vec::new();
    for _ in 0..count {
        let header = data.get(pos..pos + 10).ok_or(fail)?;
        let word = |at| be16(header, at).map(usize::from).ok_or(fail);
        let (width, height, depth) = (word(0)? * 16, word(2)?, word(4)?);
        if depth > 6 {
            return Err(fail);
        }
        let len = width / 8 * height * depth;
        let planes = data.get(pos + 10..pos + 10 + len).ok_or(fail)?;
        pos += 10 + len;
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
        .ok_or(fail)?;
    // 6-plane objects use Extra Half-Brite: colours 32-63 are colours 0-31
    // with each 4-bit component halved (Amiga Hardware Reference Manual,
    // "Extra Half Brite Mode"). RECOIL rejects such banks.
    let colors: Vec<u32> = (0..64)
        .map(|i| {
            let word = palette[i % 32];
            rgb12(if i >= 32 { (word >> 1) & 0x777 } else { word })
        })
        .collect();
    let width: usize = objects.iter().map(|o| o.width).sum();
    let height = objects.iter().map(|o| o.height).max().unwrap_or(0);
    if width == 0 || height == 0 || width > 0xffff {
        return Err(fail);
    }
    // Colour 0 where no object reaches.
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
