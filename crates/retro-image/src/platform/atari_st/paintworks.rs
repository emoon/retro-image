//! Paintworks / N-Vision pictures (`SC0`-`SC2`, `CL0`-`CL2`, `PG0`-`PG2`).
//!
//! Source: <https://temlib.org/AtariForumWiki/index.php/Paintworks_file_format>
//! (header, flags byte, and Lonny Pursell's public-domain RLE decoder).

use alloc::vec::Vec;

use super::common::{Resolution, palette_words, planar_image, screen_palette};
use crate::{DecodeError, Image};

const HEADER_LEN: usize = 128;

pub(super) fn decode_paintworks(data: &[u8]) -> Result<Image, DecodeError> {
    decode(data).ok_or(DecodeError::Unrecognized)
}

fn decode(data: &[u8]) -> Option<Image> {
    if data.get(0x36..0x3f)? != b"ANvisionA" {
        return None;
    }
    let flags = *data.get(0x3f)?;
    let resolution = Resolution::from_index(u16::from(flags >> 4 & 3))?;
    let lines = match flags & 0xf {
        0 => resolution.height() * 2,
        1 | 2 => resolution.height(),
        _ => return None,
    };
    let len = lines as usize * (resolution.width() / 8 * resolution.planes()) as usize;
    let body = &data[HEADER_LEN..];
    let bitmap = if flags & 0x80 != 0 {
        let planes = unpack(body, len)?;
        separate_to_interleaved(&planes, resolution.planes() as usize)
    } else {
        body.get(..len)?.to_vec()
    };
    let words = palette_words(data, 4, 16)?;
    planar_image(
        &bitmap,
        resolution.width(),
        lines,
        resolution.planes(),
        &screen_palette(resolution, &words),
        resolution.y_scale(),
    )
}

/// RLE: bit 7 set = literal of `x & 0x7f` bytes, else a run of `x` copies.
fn unpack(data: &[u8], len: usize) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(len);
    let mut pos = 0;
    while out.len() < len {
        let cmd = *data.get(pos)?;
        pos += 1;
        let count = usize::from(cmd & 0x7f);
        if cmd & 0x80 != 0 {
            out.extend_from_slice(data.get(pos..pos + count)?);
            pos += count;
        } else {
            let value = *data.get(pos)?;
            pos += 1;
            out.extend(core::iter::repeat_n(value, count));
        }
    }
    out.truncate(len);
    Some(out)
}

/// Whole planes one after another become word-interleaved lines.
fn separate_to_interleaved(data: &[u8], planes: usize) -> Vec<u8> {
    let plane_len = data.len() / planes;
    let mut out = alloc::vec![0; data.len()];
    for plane in 0..planes {
        for word in 0..plane_len / 2 {
            let from = plane * plane_len + word * 2;
            let to = (word * planes + plane) * 2;
            out[to..to + 2].copy_from_slice(&data[from..from + 2]);
        }
    }
    out
}
