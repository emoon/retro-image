//! Paintworks / N-Vision pictures (`SC0`-`SC2`, `CL0`-`CL2`, `PG0`-`PG2`).
//!
//! Source: <https://temlib.org/AtariForumWiki/index.php/Paintworks_file_format>
//! (header, flags byte, and Lonny Pursell's public-domain RLE decoder).

use alloc::vec::Vec;

use super::common::{
    Resolution, palette_words, planar_image, screen_palette, separate_planes_to_interleaved,
};
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
        separate_planes_to_interleaved(&planes, resolution.planes() as usize)
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
