//! uConvert's UIMG bitmaps (`BP1`-`BP8`, `C01`-`C32`).
//!
//! Source: the "UIMG Bitmap format" section of the uConvert README,
//! <https://github.com/mikrosk/uconvert/blob/master/README.md> (prose only;
//! the repository's code is GPL and was not read).
//! Observed from `recoil2png` output: ST palettes follow the usual ST/STE
//! detection, TT palettes are 4-bit components times 0x11, Falcon palette
//! bytes are used as they are.

use alloc::vec::Vec;

use super::common::{MAX_PIXELS, planar_image, st_palette, vdi_palette, words};
use super::falcon::rgb565;
use crate::bytes::be16;
use crate::{DecodeError, Image};

const HEADER_LEN: usize = 14;

pub(super) fn decode_uimg(data: &[u8]) -> Result<Image, DecodeError> {
    decode(data).ok_or(DecodeError::Unrecognized)
}

fn decode(data: &[u8]) -> Option<Image> {
    if data.get(..4)? != b"UIMG" {
        return None;
    }
    let flags = be16(data, 6)?;
    let bits = usize::from(*data.get(8)?);
    let chunk = *data.get(9)? as i8;
    let width = usize::from(be16(data, 10)?);
    let height = usize::from(be16(data, 12)?);
    if width == 0 || height == 0 || width * height > MAX_PIXELS {
        return None;
    }
    let (palette, body) = palette(data, flags & 7, bits)?;
    match (bits, chunk) {
        (1 | 2 | 4 | 6 | 8, 0) => {
            planar_image(body, width as u32, height as u32, bits as u32, &palette, 1)
        }
        (1 | 2 | 4 | 6 | 8, 1) => chunky(body, width, height, 1, |p| {
            palette.get(usize::from(p[0])).copied()
        }),
        (1 | 2 | 4, -1) => packed(body, width, height, bits, &palette),
        (16, 2) => chunky(body, width, height, 2, |p| {
            Some(rgb565(u16::from_be_bytes([p[0], p[1]])))
        }),
        (24, 3) => chunky(body, width, height, 3, |p| {
            Some(u32::from_be_bytes([0, p[0], p[1], p[2]]))
        }),
        (32, 4) => chunky(body, width, height, 4, |p| {
            Some(u32::from_be_bytes([0, p[1], p[2], p[3]]))
        }),
        _ => None,
    }
}

/// Reads the palette (if any) and returns it with the remaining data.
fn palette(data: &[u8], kind: u16, bits: usize) -> Option<(Vec<u32>, &[u8])> {
    let entries = if bits <= 8 { 1usize << bits } else { 0 };
    let entry_len = match kind {
        0 => return Some((Vec::new(), &data[HEADER_LEN..])),
        1 | 2 => 2,
        3 => 4,
        4 => 6,
        _ => return None,
    };
    let len = entries * entry_len;
    let table = data.get(HEADER_LEN..HEADER_LEN + len)?;
    let palette = match kind {
        1 => st_palette(&words(table)),
        2 => words(table).into_iter().map(super::tt::tt_rgb).collect(),
        3 => table
            .chunks_exact(4)
            .map(|e| u32::from_be_bytes([0, e[0], e[1], e[3]]))
            .collect(),
        _ => vdi_palette(table, entries)?,
    };
    Some((palette, &data[HEADER_LEN + len..]))
}

fn chunky(
    data: &[u8],
    width: usize,
    height: usize,
    bytes: usize,
    color: impl Fn(&[u8]) -> Option<u32>,
) -> Option<Image> {
    let data = data.get(..width * height * bytes)?;
    let mut image = Image::new(width as u32, height as u32);
    for (i, pixel) in data.chunks_exact(bytes).enumerate() {
        image.set((i % width) as u32, (i / width) as u32, color(pixel)?);
    }
    Some(image)
}

/// Densely packed pixels, `bits` per pixel, most significant first; lines
/// start on byte boundaries.
fn packed(data: &[u8], width: usize, height: usize, bits: usize, palette: &[u32]) -> Option<Image> {
    let line_len = (width * bits).div_ceil(8);
    let data = data.get(..line_len * height)?;
    let mut image = Image::new(width as u32, height as u32);
    for (y, line) in data.chunks_exact(line_len).enumerate() {
        for x in 0..width {
            let bit = x * bits;
            let value = line[bit / 8] >> (8 - bits - bit % 8) & ((1 << bits) - 1);
            image.set(x as u32, y as u32, *palette.get(usize::from(value))?);
        }
    }
    Some(image)
}
