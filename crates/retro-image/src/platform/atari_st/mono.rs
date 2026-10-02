//! Compressed monochrome formats: Public Painter (`CMP`) and Calamus
//! Raster Graphic (`CRG`).
//!
//! Sources:
//! - Public Painter, including Lonny Pursell's public-domain decoder:
//!   <https://temlib.org/AtariForumWiki/index.php/Public_Painter_file_format>
//! - Calamus Raster Graphic:
//!   <https://temlib.org/AtariForumWiki/index.php/Calamus_Raster_Graphic_file_format>

use alloc::vec::Vec;

use super::common::{be32, mono_image};
use crate::{DecodeError, Image};

/// Upper bound on the picture area, so corrupt headers can't make us
/// allocate gigabytes.
const MAX_PIXELS: usize = 1 << 25;

/// Public Painter: escape byte, size byte (0 = 640x400, 200 = 640x800),
/// then literal bytes or `escape, count - 1, value` runs.
pub(super) fn decode_cmp(data: &[u8]) -> Result<Image, DecodeError> {
    let (&escape, rest) = data.split_first().ok_or(DecodeError::Unrecognized)?;
    let (&size, body) = rest.split_first().ok_or(DecodeError::Unrecognized)?;
    let height = match size {
        0 => 400,
        200 => 800,
        _ => return Err(DecodeError::Unrecognized),
    };
    let len = 80 * height;
    let mut bitmap = Vec::with_capacity(len);
    let mut pos = 0;
    while pos < body.len() && bitmap.len() < len {
        let cmd = body[pos];
        pos += 1;
        if cmd == escape {
            let count = usize::from(*body.get(pos).ok_or(DecodeError::Unrecognized)?) + 1;
            let value = *body.get(pos + 1).ok_or(DecodeError::Unrecognized)?;
            pos += 2;
            bitmap.extend(core::iter::repeat_n(value, count));
        } else {
            bitmap.push(cmd);
        }
    }
    if bitmap.len() < len {
        return Err(DecodeError::Unrecognized);
    }
    mono_image(&bitmap, 640, height as u32, 80).ok_or(DecodeError::Unrecognized)
}

/// Calamus Raster Graphic: 42-byte header, byte RLE.
pub(super) fn decode_crg(data: &[u8]) -> Result<Image, DecodeError> {
    decode_crg_inner(data).ok_or(DecodeError::Unrecognized)
}

fn decode_crg_inner(data: &[u8]) -> Option<Image> {
    if data.get(..10)? != b"CALAMUSCRG" {
        return None;
    }
    let width = be32(data, 20)? as usize;
    let height = be32(data, 24)? as usize;
    if width == 0 || height == 0 || width.checked_mul(height)? > MAX_PIXELS {
        return None;
    }
    let row_len = width.div_ceil(8);
    let len = row_len * height;
    let mut bitmap = Vec::with_capacity(len);
    let mut pos = 42;
    while bitmap.len() < len {
        let code = *data.get(pos)?;
        pos += 1;
        if code < 128 {
            let count = usize::from(code) + 1;
            bitmap.extend_from_slice(data.get(pos..pos + count)?);
            pos += count;
        } else {
            let value = *data.get(pos)?;
            pos += 1;
            bitmap.extend(core::iter::repeat_n(value, usize::from(code) - 127));
        }
    }
    mono_image(&bitmap, width as u32, height as u32, row_len)
}
