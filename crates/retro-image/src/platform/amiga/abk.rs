//! AMOS banks: sprite (`AmSp`) and icon (`AmIc`) banks.
//!
//! Sources:
//! - Bank layout: <http://alvyn.sourceforge.net/amos_file_formats.html>
//!   (count, per-object width in words, height, depth, hot spot, planar data,
//!   then 32 `0RGB` palette words).
//! - Objects drawn side by side, top-aligned, on colour 0: observed from
//!   `recoil2png` output.

use alloc::vec::Vec;

use super::iff::be16;
use super::ilbm::rgb12;
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
    let count = usize::from(be16(data.get(4..6).ok_or(fail)?));
    let mut pos = 6;
    let mut objects = Vec::new();
    for _ in 0..count {
        let header = data.get(pos..pos + 10).ok_or(fail)?;
        let width = usize::from(be16(&header[0..2])) * 16;
        let height = usize::from(be16(&header[2..4]));
        let depth = usize::from(be16(&header[4..6]));
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
    let palette = data.get(pos..pos + 64).ok_or(fail)?;
    // 6-plane objects use Extra Half-Brite: colours 32-63 are colours 0-31
    // with each 4-bit component halved (Amiga Hardware Reference Manual,
    // "Extra Half Brite Mode"). RECOIL rejects such banks.
    let color = |i: usize| {
        let word = be16(&palette[i % 32 * 2..i % 32 * 2 + 2]);
        rgb12(if i >= 32 { (word >> 1) & 0x777 } else { word })
    };
    let width: usize = objects.iter().map(|o| o.width).sum();
    let height = objects.iter().map(|o| o.height).max().unwrap_or(0);
    if width == 0 || height == 0 || width > 0xffff {
        return Err(fail);
    }
    let mut image = Image::new(width as u32, height as u32);
    let background = color(0);
    for y in 0..height {
        for x in 0..width {
            image.set(x as u32, y as u32, background);
        }
    }
    let mut left = 0;
    for object in &objects {
        let row_len = object.width / 8;
        let plane_len = row_len * object.height;
        for y in 0..object.height {
            for x in 0..object.width {
                let index = (0..object.depth).fold(0, |index, plane| {
                    let byte = object.planes[plane * plane_len + y * row_len + x / 8];
                    index | usize::from(byte >> (7 - x % 8) & 1) << plane
                });
                image.set((left + x) as u32, y as u32, color(index));
            }
        }
        left += object.width;
    }
    Ok(image)
}
