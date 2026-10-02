//! TVPaint DEEP: chunky true-colour IFF (FORM DEEP and FORM TVPP).
//!
//! Source: <https://wiki.amigaos.net/wiki/DEEP_IFF_Chunky_Pixel_Image>
//! (DGBL, DPEL, DLOC, DBOD chunks; TVDC delta decompression). How RLE
//! (ByteRun1 counting whole pixels) and TVDC (one stream per element per line)
//! data is arranged was reverse engineered from samples and verified against
//! `recoil2png` output.

use alloc::vec;
use alloc::vec::Vec;

use super::iff::find;
use crate::bytes::{be16, be32};
use crate::{DecodeError, Image};

const RED: u16 = 1;
const GREEN: u16 = 2;
const BLUE: u16 = 3;

pub(super) fn decode(contents: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let dgbl = find(contents, b"DGBL")
        .filter(|c| c.len() >= 8)
        .ok_or(fail)?;
    let dpel = find(contents, b"DPEL")
        .filter(|c| c.len() >= 4)
        .ok_or(fail)?;
    let body = find(contents, b"DBOD").ok_or(fail)?;
    let (width, height) = match find(contents, b"DLOC").filter(|c| c.len() >= 4) {
        Some(dloc) => (be16(dloc, 0), be16(dloc, 2)),
        None => (be16(dgbl, 0), be16(dgbl, 2)),
    };
    let (width, height) = (
        usize::from(width.ok_or(fail)?),
        usize::from(height.ok_or(fail)?),
    );
    let compression = be16(dgbl, 4).ok_or(fail)?;
    // Element types; every element must be 8 bits deep.
    let count = be32(dpel, 0).ok_or(fail)? as usize;
    let elements: Vec<u16> = dpel[4..]
        .as_chunks::<4>()
        .0
        .iter()
        .take(count)
        .map(|e| be16(e, 2).filter(|&depth| depth == 8).and(be16(e, 0)))
        .collect::<Option<_>>()
        .ok_or(fail)?;
    if width == 0 || height == 0 || elements.len() != count || count == 0 {
        return Err(fail);
    }
    let channel = |kind| elements.iter().position(|&k| k == kind).ok_or(fail);
    let (r, g, b) = (channel(RED)?, channel(GREEN)?, channel(BLUE)?);
    let pixel_len = count;
    let row_len = width * pixel_len;
    // Neither compression expands a byte to more than 128 bytes; checking
    // this first keeps corrupt sizes from allocating huge buffers.
    if row_len * height > body.len().saturating_mul(128) {
        return Err(fail);
    }
    let pixels = match compression {
        0 => body.get(..row_len * height).ok_or(fail)?.to_vec(),
        1 => unpack_pixel_rle(body, pixel_len, width * height).ok_or(fail)?,
        5 => unpack_tvdc(contents, body, width, height, count).ok_or(fail)?,
        _ => return Err(fail),
    };
    let mut image = Image::new(width as u32, height as u32);
    for (i, p) in pixels.chunks_exact(pixel_len).enumerate() {
        let color = u32::from(p[r]) << 16 | u32::from(p[g]) << 8 | u32::from(p[b]);
        image.set((i % width) as u32, (i / width) as u32, color);
    }
    Ok(image)
}

/// ByteRun1 working on whole pixels instead of bytes.
fn unpack_pixel_rle(src: &[u8], pixel_len: usize, pixels: usize) -> Option<Vec<u8>> {
    let len = pixels * pixel_len;
    let mut out = Vec::with_capacity(len);
    let mut pos = 0;
    while out.len() < len {
        let n = *src.get(pos)? as i8;
        pos += 1;
        match n {
            0..=127 => {
                let count = (n as usize + 1) * pixel_len;
                out.extend_from_slice(src.get(pos..pos + count)?);
                pos += count;
            }
            -127..=-1 => {
                let pixel = src.get(pos..pos + pixel_len)?;
                pos += pixel_len;
                for _ in 0..1 - isize::from(n) {
                    out.extend_from_slice(pixel);
                }
            }
            -128 => {}
        }
    }
    out.truncate(len);
    Some(out)
}

/// TVDC: per line, one delta stream per element, each padded to a byte.
fn unpack_tvdc(
    contents: &[u8],
    body: &[u8],
    width: usize,
    height: usize,
    elements: usize,
) -> Option<Vec<u8>> {
    let table_chunk = find(contents, b"TVDC")?;
    let mut table = [0i16; 16];
    for (t, w) in table
        .iter_mut()
        .zip(table_chunk.get(..32)?.as_chunks::<2>().0)
    {
        *t = i16::from_be_bytes([w[0], w[1]]);
    }
    let mut out = vec![0u8; width * height * elements];
    let mut line = vec![0u8; width];
    let mut pos = 0;
    for y in 0..height {
        for e in 0..elements {
            pos += depack_line(body.get(pos..)?, &table, &mut line)?;
            for (x, &v) in line.iter().enumerate() {
                out[(y * width + x) * elements + e] = v;
            }
        }
    }
    Some(out)
}

/// The spec's `CDepackTVDC`: nibble deltas; a zero delta is followed by a
/// nibble repeat count. Returns the number of source bytes used.
fn depack_line(source: &[u8], table: &[i16; 16], dest: &mut [u8]) -> Option<usize> {
    let mut nibble_pos = 0;
    let mut next_nibble = || {
        let byte = *source.get(nibble_pos >> 1)?;
        let d = if nibble_pos & 1 != 0 {
            byte & 0xf
        } else {
            byte >> 4
        };
        nibble_pos += 1;
        Some(usize::from(d))
    };
    let mut v = 0u8;
    let mut i = 0;
    while i < dest.len() {
        let d = next_nibble()?;
        v = v.wrapping_add(table[d] as u8);
        dest[i] = v;
        if table[d] == 0 {
            for _ in 0..next_nibble()? {
                i += 1;
                if let Some(slot) = dest.get_mut(i) {
                    *slot = v;
                }
            }
        }
        i += 1;
    }
    Some(nibble_pos.div_ceil(2))
}
