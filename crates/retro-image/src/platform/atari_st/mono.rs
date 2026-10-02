//! Monochrome formats: Public Painter (`CMP`), STAD (`PAC`), MegaPaint
//! (`BLD`), DEGAS Elite fonts (`FNT`) and brushes (`BRU`), and Calamus
//! Raster Graphic (`CRG`).
//!
//! Sources:
//! - STAD: <https://temlib.org/AtariForumWiki/index.php/STAD_file_format>
//! - MegaPaint: <https://temlib.org/AtariForumWiki/index.php/MegaPaint_file_format>,
//!   <http://fileformats.archiveteam.org/wiki/MegaPaint_BLD> (sizes stored minus one)
//! - Public Painter, including Lonny Pursell's public-domain decoder:
//!   <https://temlib.org/AtariForumWiki/index.php/Public_Painter_file_format>
//! - Calamus Raster Graphic:
//!   <https://temlib.org/AtariForumWiki/index.php/Calamus_Raster_Graphic_file_format>

use alloc::vec::Vec;

use super::common::{MAX_PIXELS, be32, mono_image};
use crate::{DecodeError, Image};

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

/// STAD: `pM85` (row order) or `pM86` (column order), id byte, pack byte,
/// special byte, then literal bytes, `id, n` (pack byte n + 1 times) or
/// `special, d, n` (d n + 1 times). Always 640x400.
pub(super) fn decode_pac(data: &[u8]) -> Result<Image, DecodeError> {
    decode_pac_inner(data).ok_or(DecodeError::Unrecognized)
}

fn decode_pac_inner(data: &[u8]) -> Option<Image> {
    let vertical = match data.get(..4)? {
        b"pM85" => false,
        b"pM86" => true,
        _ => return None,
    };
    let (id, pack, special) = (*data.get(4)?, *data.get(5)?, *data.get(6)?);
    let len = 80 * 400;
    let mut stream = Vec::with_capacity(len);
    let mut pos = 7;
    while stream.len() < len {
        let x = *data.get(pos)?;
        pos += 1;
        if x == id {
            let n = *data.get(pos)?;
            pos += 1;
            stream.extend(core::iter::repeat_n(pack, usize::from(n) + 1));
        } else if x == special {
            let (d, n) = (*data.get(pos)?, *data.get(pos + 1)?);
            pos += 2;
            stream.extend(core::iter::repeat_n(d, usize::from(n) + 1));
        } else {
            stream.push(x);
        }
    }
    stream.truncate(len);
    let bitmap = if vertical {
        let mut bitmap = alloc::vec![0; len];
        for (i, &b) in stream.iter().enumerate() {
            bitmap[i % 400 * 80 + i / 400] = b;
        }
        bitmap
    } else {
        stream
    };
    mono_image(&bitmap, 640, 400, 80)
}

/// MegaPaint: width - 1 (negative when compressed), height - 1, then a
/// 1-bit bitmap; compressed data expands `0, n` and `255, n` to n + 1
/// copies of 0 or 255.
pub(super) fn decode_bld(data: &[u8]) -> Result<Image, DecodeError> {
    decode_bld_inner(data).ok_or(DecodeError::Unrecognized)
}

fn decode_bld_inner(data: &[u8]) -> Option<Image> {
    let raw_width = super::common::be16(data, 0)? as i16;
    let height = usize::from(super::common::be16(data, 2)?) + 1;
    let compressed = raw_width < 0;
    let width = usize::from(raw_width.unsigned_abs()) + 1;
    if width > 4096 || height > 4096 {
        return None;
    }
    let row_len = width.div_ceil(8);
    let len = row_len * height;
    let body = &data[4..];
    let bitmap = if compressed {
        let mut out = Vec::with_capacity(len);
        let mut pos = 0;
        while out.len() < len {
            let x = *body.get(pos)?;
            pos += 1;
            if x == 0 || x == 255 {
                let n = *body.get(pos)?;
                pos += 1;
                out.extend(core::iter::repeat_n(x, usize::from(n) + 1));
            } else {
                out.push(x);
            }
        }
        out.truncate(len);
        out
    } else {
        body.get(..len)?.to_vec()
    };
    mono_image(&bitmap, width as u32, height as u32, row_len)
}

/// DEGAS Elite 8x16 font: 128 (or 256) characters of 16 bytes, optionally
/// followed by a flag word; shown as a sheet 32 characters wide.
/// Source: <https://temlib.org/AtariForumWiki/index.php/DEGAS_Elite_Font_file_format>;
/// the 256-character size and the sheet layout are observed from
/// `recoil2png` output.
pub(super) fn decode_fnt(data: &[u8]) -> Result<Image, DecodeError> {
    let chars = match data.len() {
        2048 | 2050 => 128,
        4096 | 4098 => 256,
        _ => return Err(DecodeError::Unrecognized),
    };
    let rows = chars / 32;
    let mut bitmap = alloc::vec![0u8; chars * 16];
    for (c, glyph) in data[..chars * 16].chunks_exact(16).enumerate() {
        for (line, &bits) in glyph.iter().enumerate() {
            // Set bits are white (observed from `recoil2png` output).
            bitmap[(c / 32 * 16 + line) * 32 + c % 32] = !bits;
        }
    }
    mono_image(&bitmap, 256, (rows * 16) as u32, 32).ok_or(DecodeError::Unrecognized)
}

/// DEGAS Elite brush: 8x8 pixels, one byte (0 or 1) each.
/// Source: <http://fileformats.archiveteam.org/wiki/DEGAS_Elite_brush> (size);
/// the byte-per-pixel layout is derived from the sample file.
pub(super) fn decode_bru(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 64 || data.iter().any(|&b| b > 1) {
        return Err(DecodeError::Unrecognized);
    }
    let mut image = Image::new(8, 8);
    for (i, &b) in data.iter().enumerate() {
        // Set pixels are white (observed from `recoil2png` output).
        image.set(i as u32 % 8, i as u32 / 8, u32::from(b) * 0xffffff);
    }
    Ok(image)
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
