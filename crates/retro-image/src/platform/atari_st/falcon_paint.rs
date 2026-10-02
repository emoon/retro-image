//! Falcon-era paint programs with flexible depths: TmS Cranach (`ESM`),
//! Funny Paint (`FUN`), PixArt (`PIX`), Prism Paint / TruePaint (`PNT`,
//! `TPI`), DelmPaint (`DEL`, `DPH`) and RAG-D (`RAG`, `RAGC`).
//!
//! Sources:
//! - TmS Cranach: <https://temlib.org/AtariForumWiki/index.php/TmS_Cranach_file_format>
//! - Funny Paint: <https://temlib.org/AtariForumWiki/index.php/Funny_Paint_file_format>
//! - PixArt: <https://temlib.org/AtariForumWiki/index.php/PixArt_file_format>
//! - Prism Paint / TruePaint:
//!   <https://temlib.org/AtariForumWiki/index.php?title=TruePaint/Prism_Paint_file_format>
//! - DelmPaint: <https://temlib.org/AtariForumWiki/index.php/DelmPaint_file_format>
//! - RAG-D: <https://temlib.org/AtariForumWiki/index.php/Rag-D_file_format>,
//!   <http://cd.textfiles.com/atarilibrary/atari_cd07/GRAPHICS/PAINT/RAGDEE/ENGLISH/FORMAT.TXT>
//!   (Music Compile `RAGC` files use the same container with chunky pixels:
//!   derived from sample files)
//! - Observed from `recoil2png` output: pictures three times as wide as
//!   they are tall (640x200) are shown with doubled lines, Prism Paint
//!   pictures half again as tall as wide (320x480) with doubled pixels.

use alloc::vec::Vec;

use super::common::{
    MAX_PIXELS, line_planes_to_interleaved, palette_words, planar_image, st_palette, vdi_palette,
};
use super::falcon::{rgb565, videl_palette};
use crate::bytes::{be16, be32};
use crate::codec::packbits;
use crate::{DecodeError, Image};

fn ok(image: Option<Image>) -> Result<Image, DecodeError> {
    image.ok_or(DecodeError::Unrecognized)
}

fn check_size(width: usize, height: usize) -> Option<()> {
    (width > 0 && height > 0 && width * height <= MAX_PIXELS).then_some(())
}

/// Lines are doubled for 640x200-like shapes.
fn y_scale(width: usize, height: usize) -> u32 {
    if width >= height * 3 { 2 } else { 1 }
}

/// Pixels are doubled for 320x480-like shapes.
fn widen(image: Image) -> Image {
    if image.height() * 2 < image.width() * 3 {
        return image;
    }
    image.scaled(2, 1)
}

/// Renders chunky pixels (`bytes` per pixel) through `color`.
fn chunky(
    data: &[u8],
    width: usize,
    height: usize,
    bytes: usize,
    color: impl Fn(&[u8]) -> u32,
) -> Option<Image> {
    check_size(width, height)?;
    let data = data.get(..width * height * bytes)?;
    let mut image = Image::new(width as u32, height as u32);
    for (i, pixel) in data.chunks_exact(bytes).enumerate() {
        image.set((i % width) as u32, (i / width) as u32, color(pixel));
    }
    Some(image)
}

fn word565(p: &[u8]) -> u32 {
    rgb565(u16::from_be_bytes([p[0], p[1]]))
}

fn rgb(p: &[u8]) -> u32 {
    u32::from_be_bytes([0, p[0], p[1], p[2]])
}

/// TmS Cranach: `TMS\0`, header size, width, height, planes (1, 8, 24),
/// ..., red/green/blue palette tables at 36.
pub(super) fn decode_esm(data: &[u8]) -> Result<Image, DecodeError> {
    ok(decode_esm_inner(data))
}

fn decode_esm_inner(data: &[u8]) -> Option<Image> {
    if data.get(..4)? != b"TMS\0" {
        return None;
    }
    let header_len = usize::from(be16(data, 4)?);
    let width = usize::from(be16(data, 6)?);
    let height = usize::from(be16(data, 8)?);
    check_size(width, height)?;
    let body = data.get(header_len..)?;
    match be16(data, 10)? {
        1 => {
            let row_len = width.div_ceil(8);
            super::common::mono_image(body, width as u32, height as u32, row_len)
        }
        8 => {
            let tables = data.get(36..36 + 768)?;
            chunky(body, width, height, 1, |p| {
                let i = usize::from(p[0]);
                rgb(&[tables[i], tables[256 + i], tables[512 + i]])
            })
        }
        24 => chunky(body, width, height, 3, rgb),
        _ => None,
    }
}

/// Funny Paint: magic, width, height, planes, frames, one byte; frames;
/// colour count - 1 and two longs; VDI palette. Only the first frame is
/// shown.
pub(super) fn decode_fun(data: &[u8]) -> Result<Image, DecodeError> {
    ok(decode_fun_inner(data))
}

fn decode_fun_inner(data: &[u8]) -> Option<Image> {
    if be32(data, 0)? != 0x000a_cfe2 {
        return None;
    }
    let width = usize::from(be16(data, 4)?);
    let height = usize::from(be16(data, 6)?);
    let planes = usize::from(be16(data, 8)?);
    let frames = usize::from(be16(data, 10)?);
    check_size(width, height)?;
    let body = data.get(13..)?;
    if planes == 16 {
        return chunky(body, width, height, 2, word565);
    }
    if !matches!(planes, 1 | 2 | 4 | 8) || width % 16 != 0 {
        return None;
    }
    let frame_len = width / 8 * planes * height;
    let palette_at = 13 + frame_len.checked_mul(frames)? + 12;
    let palette = if planes == 1 {
        super::common::MONO_PALETTE.to_vec()
    } else {
        vdi_palette(data.get(palette_at..)?, 1 << planes)?
    };
    planar_image(
        body,
        width as u32,
        height as u32,
        planes as u32,
        &palette,
        y_scale(width, height),
    )
}

/// PixArt: `PIXT`, version, type, planes, width, height, unknown word(s),
/// RGB palette for 2-8 planes, then planar or chunky pixels.
pub(super) fn decode_pix(data: &[u8]) -> Result<Image, DecodeError> {
    ok(decode_pix_inner(data))
}

fn decode_pix_inner(data: &[u8]) -> Option<Image> {
    if data.get(..4)? != b"PIXT" {
        return None;
    }
    let header_len = match be16(data, 4)? {
        1 => 14,
        2 => 16,
        _ => return None,
    };
    let kind = *data.get(6)?;
    let planes = usize::from(*data.get(7)?);
    let width = usize::from(be16(data, 8)?);
    let height = usize::from(be16(data, 10)?);
    check_size(width, height)?;
    let colors = if matches!(planes, 2 | 4 | 8) {
        1 << planes
    } else {
        0
    };
    let palette: Vec<u32> = data
        .get(header_len..header_len + colors * 3)?
        .chunks_exact(3)
        .map(rgb)
        .collect();
    let body = &data[header_len + colors * 3..];
    match (planes, kind) {
        (1, _) => planar_image(
            body,
            width as u32,
            height as u32,
            1,
            &super::common::MONO_PALETTE,
            y_scale(width, height),
        ),
        (2 | 4 | 8, 1) => planar_image(
            body,
            width as u32,
            height as u32,
            planes as u32,
            &palette,
            y_scale(width, height),
        ),
        (8, 0) => chunky(body, width, height, 1, |p| palette[usize::from(p[0])]),
        (16, 1) => chunky(body, width, height, 2, word565),
        (24, 1) => chunky(body, width, height, 3, rgb),
        (24, 0) => chunky(body, width, height, 3, |p| rgb(&[p[2], p[1], p[0]])),
        (32, 0) => chunky(body, width, height, 4, |p| rgb(&p[1..])),
        _ => None,
    }
}

/// Prism Paint / TruePaint: `PNT\0`, version, palette size, width, height,
/// bits per pixel, compression, data size, VDI palette, then interleaved
/// planes (below 16 bits) or chunky pixels, optionally PackBits per plane
/// row.
pub(super) fn decode_pnt(data: &[u8]) -> Result<Image, DecodeError> {
    ok(decode_pnt_inner(data).map(widen))
}

fn decode_pnt_inner(data: &[u8]) -> Option<Image> {
    if data.get(..4)? != b"PNT\0" {
        return None;
    }
    let palette_len = usize::from(be16(data, 6)?);
    let width = usize::from(be16(data, 8)?);
    let height = usize::from(be16(data, 10)?);
    let bits = usize::from(be16(data, 12)?);
    let compressed = be16(data, 14)? != 0;
    check_size(width, height)?;
    if !matches!(bits, 1 | 2 | 4 | 8 | 16 | 24) {
        return None;
    }
    let padded = width.next_multiple_of(16);
    let line_len = padded / 8 * bits;
    let body = data.get(128 + palette_len * 6..)?;
    let bitmap = if compressed {
        let (unpacked, _) = packbits::unpack(body, line_len * height)?;
        if bits < 16 {
            line_planes_to_interleaved(&unpacked, padded as u32, height as u32, bits as u32)?
        } else {
            // Chunky lines were packed as if made of `bits` planes.
            unpacked
        }
    } else {
        body.get(..line_len * height)?.to_vec()
    };
    let image = match bits {
        1 | 2 | 4 | 8 => {
            let palette = if bits == 1 {
                super::common::MONO_PALETTE.to_vec()
            } else {
                vdi_palette(data.get(128..)?, 1 << bits)?
            };
            planar_image(
                &bitmap,
                padded as u32,
                height as u32,
                bits as u32,
                &palette,
                y_scale(width, height),
            )?
        }
        16 => chunky(&bitmap, padded, height, 2, word565)?,
        24 => chunky(&bitmap, padded, height, 3, rgb)?,
        _ => return None,
    };
    Some(if padded == width {
        image
    } else {
        super::common::crop(&image, width as u32, image.height())
    })
}

/// DelmPaint: CrackArt-packed 32000-byte blocks (3 for `DEL`; 10, all
/// with their lengths given, for `DPH`: derived from sample files) holding a VIDEL palette and 320x240 8-plane pictures (four
/// quadrants for `DPH`).
pub(super) fn decode_del(data: &[u8]) -> Result<Image, DecodeError> {
    let unpacked = delm_blocks(data, 2, 3).ok_or(DecodeError::Unrecognized)?;
    let palette = videl_palette(&unpacked).ok_or(DecodeError::Unrecognized)?;
    ok(planar_image(&unpacked[1024..], 320, 240, 8, &palette, 1))
}

pub(super) fn decode_dph(data: &[u8]) -> Result<Image, DecodeError> {
    ok(decode_dph_inner(data))
}

fn decode_dph_inner(data: &[u8]) -> Option<Image> {
    let unpacked = delm_blocks(data, 10, 10)?;
    let palette = videl_palette(&unpacked)?;
    // Quadrants: 320-byte lines of pictures 1 and 2 alternate, then 3 and 4.
    let quadrant = 76800;
    let mut bitmap = Vec::with_capacity(4 * quadrant);
    for half in 0..2 {
        for line in 0..240 {
            for side in 0..2 {
                let start = 1024 + (half * 2 + side) * quadrant + line * 320;
                bitmap.extend_from_slice(unpacked.get(start..start + 320)?);
            }
        }
    }
    planar_image(&bitmap, 640, 480, 8, &palette, 1)
}

/// Unpacks `blocks` CrackArt blocks preceded by `lengths` block lengths
/// (the last block, if its length is not given, runs to the end).
fn delm_blocks(data: &[u8], lengths: usize, blocks: usize) -> Option<Vec<u8>> {
    let mut pos = lengths * 4;
    let mut out = Vec::with_capacity(blocks * 32000);
    for i in 0..blocks {
        let block = if i < lengths {
            let len = be32(data, i * 4)? as usize;
            let block = data.get(pos..pos.checked_add(len)?)?;
            pos += len;
            block
        } else {
            data.get(pos..)?
        };
        out.extend_from_slice(&super::crackart::unpack(block, 32000)?);
    }
    Some(out)
}

/// Music Compile: the RAG-D container with 8-bit chunky pixels (derived
/// from sample files).
pub(super) fn decode_ragc(data: &[u8]) -> Result<Image, DecodeError> {
    ok(decode_ragc_inner(data))
}

fn decode_ragc_inner(data: &[u8]) -> Option<Image> {
    if data.get(..6)? != b"RAG-D!" || be16(data, 16)? != 8 || be32(data, 18)? != 1024 {
        return None;
    }
    let width = usize::from(be16(data, 12)?);
    let height = usize::from(be16(data, 14)?) + 1;
    let palette = videl_palette(data.get(30..)?)?;
    chunky(data.get(30 + 1024..)?, width, height, 1, |p| {
        palette[usize::from(p[0])]
    })
}

/// RAG-D: `RAG-D!`, pack word (0), data length, columns, rows - 1, planes,
/// palette length, four control words, palette (ST words or VIDEL
/// entries), then interleaved planes or RGB565.
pub(super) fn decode_rag(data: &[u8]) -> Result<Image, DecodeError> {
    ok(decode_rag_inner(data))
}

fn decode_rag_inner(data: &[u8]) -> Option<Image> {
    if data.get(..6)? != b"RAG-D!" || be16(data, 6)? != 0 {
        return None;
    }
    let width = usize::from(be16(data, 12)?);
    let height = usize::from(be16(data, 14)?) + 1;
    let planes = usize::from(be16(data, 16)?);
    let palette_len = be32(data, 18)? as usize;
    check_size(width, height)?;
    let body = data.get(30usize.checked_add(palette_len)?..)?;
    let palette = match palette_len {
        32 => st_palette(&palette_words(data, 30, 16)?),
        1024 => videl_palette(&data[30..])?,
        _ => return None,
    };
    match planes {
        1 | 2 | 4 | 8 if width % 16 == 0 => planar_image(
            body,
            width as u32,
            height as u32,
            planes as u32,
            &palette,
            y_scale(width, height),
        ),
        16 => chunky(body, width, height, 2, word565),
        _ => None,
    }
}
