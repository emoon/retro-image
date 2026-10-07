//! CFAST (`.cft`): Disney Animation Studio animations, shown as their first
//! frame.
//!
//! Sources:
//! - Just Solve the File Format Problem, "CFAST Disney Animation Studio"
//!   (CC0, <http://justsolve.archiveteam.org/wiki/CFAST_Disney_Animation_Studio>):
//!   the signature (`GUCF`, or `LOCK` and `STDY` followed by a text), the
//!   header (picture size, screen size, planes, the first two colors, an
//!   extra block, the frame count), the per-frame bitplanes with their sizes,
//!   the per-frame palette, and the run-length code (the words of each
//!   column of a bitplane coded separately: a negative code repeats the next
//!   word `-code + 1` times, otherwise `code + 1` words follow).
//! - Checked on the ten `.cft` files of Sembiance's `video/disneyCFAST`: the
//!   first plane of `PATH.CFT` is twenty columns of `ff39 0000` (200 rows of
//!   zero words each) before the drawing, which fits the code as described.
//!
//! The palette bytes are read as 8-bit components, as the first two header
//! colors (`ff ff ff` and `00 00 00` in `PATH.CFT`) suggest. The `.sec`
//! files in the same collection have a different, undocumented header
//! (`SSFFANM0`) and are not decoded.
//!
//! RECOIL has no CFAST support.

use alloc::vec::Vec;

use crate::bytes::{be16, be32};
use crate::image::{check_size, planar_pixels};
use crate::{DecodeError, Image};

const TAGS: [&[u8; 4]; 3] = [b"GUCF", b"LOCK", b"STDY"];

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let tag = data.get(..4).ok_or(fail)?;
    if !TAGS.iter().any(|t| tag == &t[..]) {
        return Err(fail);
    }
    // `LOCK` and `STDY` files carry a message before the header.
    let mut at = 4;
    if tag != b"GUCF" {
        at += 2 + usize::from(be16(data, at).ok_or(fail)?);
    }
    let width = be32(data, at).ok_or(fail)? as usize;
    let height = be32(data, at + 4).ok_or(fail)? as usize;
    let planes = usize::from(*data.get(at + 16).ok_or(fail)?);
    if !(1..=5).contains(&planes) || width > 0xffff || height > 0xffff {
        return Err(fail);
    }
    check_size(width, height)?;
    // The first two colors, then the extra block (a length byte and bytes).
    let rgb = |at: usize| -> Option<u32> {
        Some(u32::from_be_bytes([
            0,
            *data.get(at)?,
            *data.get(at + 1)?,
            *data.get(at + 2)?,
        ]))
    };
    let mut palette = alloc::vec![0u32; 1 << planes];
    palette[0] = rgb(at + 17).ok_or(fail)?;
    palette[1] = rgb(at + 20).ok_or(fail)?;
    at += 17 + 6;
    at += 1 + usize::from(*data.get(at).ok_or(fail)?);
    let frames = be32(data, at).ok_or(fail)?;
    at += 4;
    if frames == 0 {
        return Err(fail);
    }

    let row_len = width.div_ceil(16) * 2;
    let plane_len = row_len * height;
    let mut bitmap = alloc::vec![0u8; plane_len * planes];
    for plane in bitmap.chunks_exact_mut(plane_len) {
        let size = be32(data, at).ok_or(fail)? as usize;
        at += 4;
        let packed = data
            .get(at..at.checked_add(size).ok_or(fail)?)
            .ok_or(fail)?;
        at += size;
        unpack_plane(packed, plane, row_len, height).ok_or(fail)?;
    }

    // The palette of this frame: a count, then red, green and blue bytes,
    // which replace the header's colors from index 0 on (a count of 0 keeps
    // them).
    let count = usize::from(*data.get(at).ok_or(fail)?);
    let colors = data.get(at + 1..at + 1 + count * 3).ok_or(fail)?;
    for (entry, c) in palette.iter_mut().zip(colors.as_chunks::<3>().0) {
        *entry = u32::from_be_bytes([0, c[0], c[1], c[2]]);
    }
    let pixels = planar_pixels(&bitmap, width, height, row_len, planes, |plane, y| {
        (plane * height + y) * row_len
    })?;
    let indices: Vec<u8> = pixels.into_iter().map(|v| v as u8).collect();
    Image::from_indexed(width as u32, height as u32, &indices, &palette)
}

/// Fills `plane` (rows of `row_len` bytes) from its column-wise run-length
/// coding.
fn unpack_plane(packed: &[u8], plane: &mut [u8], row_len: usize, height: usize) -> Option<()> {
    let mut at = 0;
    let word = |at: &mut usize| {
        let value = be16(packed, *at)?;
        *at += 2;
        Some(value)
    };
    for column in 0..row_len / 2 {
        let mut row = 0;
        while row < height {
            let code = word(&mut at)? as i16;
            // A negative code repeats the next word `-code + 1` times; otherwise
            // `code + 1` words follow. The count is `|code| + 1` either way.
            let count = usize::from(code.unsigned_abs()) + 1;
            let repeated = if code < 0 { Some(word(&mut at)?) } else { None };
            if count > height - row {
                return None;
            }
            for _ in 0..count {
                let value = match repeated {
                    Some(value) => value,
                    None => word(&mut at)?,
                };
                let offset = row * row_len + column * 2;
                plane[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
                row += 1;
            }
        }
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A one-plane `GUCF` file of 32x2 pixels, white and black, with one
    /// frame holding `plane` (already run-length coded) and `palette`.
    fn file(tag: &[u8; 4], plane: &[u8], palette: &[u8]) -> Vec<u8> {
        let mut out = tag.to_vec();
        if tag != b"GUCF" {
            out.extend_from_slice(&[0, 3]);
            out.extend_from_slice(b"hey");
        }
        for value in [32u32, 2, 320, 200] {
            out.extend_from_slice(&value.to_be_bytes());
        }
        out.push(1); // planes
        out.extend_from_slice(&[255, 255, 255, 0, 0, 0]);
        out.extend_from_slice(&[2, 9, 9]); // an extra block of two bytes
        out.extend_from_slice(&1u32.to_be_bytes());
        out.extend_from_slice(&(plane.len() as u32).to_be_bytes());
        out.extend_from_slice(plane);
        out.push((palette.len() / 3) as u8);
        out.extend_from_slice(palette);
        out.push(0); // no color cycling ranges
        out
    }

    /// Column 0 repeats the word 0x8000 over both rows; column 1 holds two
    /// literal words, 0 and 0x4000.
    const PLANE: [u8; 10] = [0xff, 0xff, 0x80, 0x00, 0x00, 0x01, 0x00, 0x00, 0x40, 0x00];

    #[test]
    fn columns_of_repeated_and_literal_words() {
        let image = decode(&file(b"GUCF", &PLANE, &[])).unwrap();
        let ink = |x: u32, y: u32| image.get(x, y) == 0;
        // Pixel 0 of both rows, and pixel 17 of row 1, are set (color 1 is black).
        assert!(ink(0, 0) && ink(0, 1) && ink(17, 1));
        assert!(!ink(1, 0) && !ink(17, 0) && !ink(16, 1));
        assert_eq!(image.get(5, 0), 0xffffff, "color 0 is white");
    }

    #[test]
    fn frame_palette_replaces_the_header_colors_and_messages_are_skipped() {
        let red_green = [255, 0, 0, 0, 255, 0];
        let image = decode(&file(b"LOCK", &PLANE, &red_green)).unwrap();
        assert_eq!(image.get(5, 0), 0xff0000);
        assert_eq!(image.get(0, 0), 0x00ff00);
        assert_eq!(decode(&file(b"STDY", &PLANE, &[])).unwrap().width(), 32);
    }

    #[test]
    fn damaged_columns_are_rejected() {
        // A run longer than the column, and a plane that ends early.
        let long_run = [0xff, 0xfe, 0x80, 0x00, 0x00, 0x01, 0x00, 0x00, 0x40, 0x00];
        assert!(decode(&file(b"GUCF", &long_run, &[])).is_err());
        assert!(decode(&file(b"GUCF", &PLANE[..8], &[])).is_err());
        let mut data = file(b"GUCF", &PLANE, &[]);
        data[20] = 6; // six planes
        assert!(decode(&data).is_err());
        assert!(decode(&data[..30]).is_err());
    }
}
