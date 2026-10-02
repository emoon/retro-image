//! Atari Falcon formats: high-colour (RGB565), 8-plane, greyscale and
//! true-colour pictures.
//!
//! Sources:
//! - Falcon RGB565 words and VIDEL palette entries (`R, G, 0, B` bytes):
//!   Atari Compendium chapter 5, <http://cd.textfiles.com/ataricompendium/BOOK/HTML/CHAP5.HTM>,
//!   and the hardware register listing,
//!   <https://temlib.org/AtariForumWiki/index.php/Atari_ST/STe/MSTe/TT/F030_Hardware_Register_Listing>.
//! - ImageLab: <https://temlib.org/AtariForumWiki/index.php/ImageLab_file_format>
//! - DuneGraph: <https://temlib.org/AtariForumWiki/index.php/DuneGraph_file_format>
//! - GodPaint: <https://temlib.org/AtariForumWiki/index.php/GodPaint_file_format>
//! - Print-Technik: <https://temlib.org/AtariForumWiki/index.php/Print-Technik_Raw_Data_file_format>
//! - InShape: <https://temlib.org/AtariForumWiki/index.php/InShape_file_format>
//! - FuckPaint: <https://temlib.org/AtariForumWiki/index.php/FuckPaint_file_format>,
//!   <http://fileformats.archiveteam.org/wiki/Extended_DEGAS_image> (`PI7`)
//! - IMG Scan: <https://temlib.org/AtariForumWiki/index.php/IMG_Scan_file_format>
//! - Rembrandt: <https://temlib.org/AtariForumWiki/index.php/Rembrandt_file_format>
//! - COKE: <https://temlib.org/AtariForumWiki/index.php/COKE_file_format>
//! - EggPaint: <https://temlib.org/AtariForumWiki/index.php/EggPaint_file_format>
//! - Spooky Sprites (`TRU`, RLE `TRE`): <https://temlib.org/AtariForumWiki/index.php/Spooky_Sprites_file_format>,
//!   <http://cd.textfiles.com/atarilibrary/atari_cd07/GRAPHICS/PAINT/SPOOKY4/SPOOKY.TXT>
//! - ICDRAW icons: <http://fileformats.archiveteam.org/wiki/ICDRAW_icon> and
//!   the ICDRAW 1.4 distribution, <http://cd.textfiles.com/suzybatari1/falcon/icdraw14/>;
//!   the byte layout is derived from sample files and `recoil2png` output.
//! - IndyPaint: <https://temlib.org/AtariForumWiki/index.php/IndyPaint_file_format>
//! - Falcon True Color: <http://fileformats.archiveteam.org/wiki/Falcon_True_Color>
//! - XGA: <http://fileformats.archiveteam.org/wiki/XGA_(Falcon)>
//! - Observed from `recoil2png` output: RGB565 components are scaled by
//!   bit replication, 7-bit greys doubled; palette bytes are used as they
//!   are; InShape 8-bit greys are inverted (0 = white); 384-pixel-wide XGA
//!   pictures are shown with doubled pixels.

use alloc::vec::Vec;

use super::common::{MAX_PIXELS, planar_image, separate_planes_to_interleaved};
use crate::bytes::{be16, be32};
use crate::{DecodeError, Image};

fn ok(image: Option<Image>) -> Result<Image, DecodeError> {
    image.ok_or(DecodeError::Unrecognized)
}

fn check_size(width: usize, height: usize) -> Option<()> {
    (width > 0 && height > 0 && width * height <= MAX_PIXELS).then_some(())
}

/// Falcon high-colour word `RRRRRGGG GGGBBBBB` to `0xRRGGBB`.
pub(super) fn rgb565(word: u16) -> u32 {
    let word = u32::from(word);
    let (r, g, b) = (word >> 11, word >> 5 & 0x3f, word & 0x1f);
    (r << 3 | r >> 2) << 16 | (g << 2 | g >> 4) << 8 | (b << 3 | b >> 2)
}

/// Renders big-endian RGB565 pixels, each repeated `x_scale` times.
fn high_color(data: &[u8], width: usize, height: usize, x_scale: usize) -> Option<Image> {
    check_size(width, height)?;
    let data = data.get(..width * height * 2)?;
    let mut image = Image::new(width as u32, height as u32);
    for (i, pixel) in data.as_chunks::<2>().0.iter().enumerate() {
        let color = rgb565(u16::from_be_bytes([pixel[0], pixel[1]]));
        image.set((i % width) as u32, (i / width) as u32, color);
    }
    Some(if x_scale == 1 {
        image
    } else {
        image.scaled(x_scale as u32, 1)
    })
}

/// Renders one byte per pixel through `level` (grey from byte value).
fn grey(data: &[u8], width: usize, height: usize, level: impl Fn(u8) -> u32) -> Option<Image> {
    check_size(width, height)?;
    let data = data.get(..width * height)?;
    let mut image = Image::new(width as u32, height as u32);
    for (i, &v) in data.iter().enumerate() {
        image.set((i % width) as u32, (i / width) as u32, level(v) * 0x010101);
    }
    Some(image)
}

/// Reads a 256-entry VIDEL palette (`R, G, 0, B` bytes per entry).
pub(super) fn videl_palette(data: &[u8]) -> Option<Vec<u32>> {
    let data = data.get(..1024)?;
    Some(
        data.as_chunks::<4>()
            .0
            .iter()
            .map(|e| u32::from_be_bytes([0, e[0], e[1], e[3]]))
            .collect(),
    )
}

/// ImageLab: `B&W256`, width, height, 8-bit grey (0 = black).
pub(super) fn decode_bw(data: &[u8]) -> Result<Image, DecodeError> {
    if data.get(..6) != Some(b"B&W256") {
        return Err(DecodeError::Unrecognized);
    }
    let width = be16(data, 6).ok_or(DecodeError::Unrecognized)?.into();
    let height = be16(data, 8).ok_or(DecodeError::Unrecognized)?.into();
    ok(grey(&data[10..], width, height, u32::from))
}

/// Print-Technik: `0x0F0F 0x0001`, width, height, word, 7-bit grey.
pub(super) fn decode_hir(data: &[u8]) -> Result<Image, DecodeError> {
    if be32(data, 0) != Some(0x0f0f_0001) {
        return Err(DecodeError::Unrecognized);
    }
    let width = be16(data, 4).ok_or(DecodeError::Unrecognized)?.into();
    let height = be16(data, 6).ok_or(DecodeError::Unrecognized)?.into();
    ok(grey(&data[10.min(data.len())..], width, height, |v| {
        u32::from(v & 0x7f) << 1
    }))
}

/// IMG Scan raw greyscale (0 = white): size picks the resolution.
pub(super) fn decode_img_scan(data: &[u8]) -> Result<Image, DecodeError> {
    let (width, height) = match data.len() {
        64000 => (320, 200),
        128000 => (640, 200),
        256000 => (640, 400),
        _ => return Err(DecodeError::Unrecognized),
    };
    ok(grey(data, width, height, |v| u32::from(!v)))
}

/// DuneGraph uncompressed: `DGU`, version, width, height, palette, 8 planes.
pub(super) fn decode_dg1(data: &[u8]) -> Result<Image, DecodeError> {
    if data.get(..3) != Some(b"DGU") || data.len() != 8 + 1024 + 64000 {
        return Err(DecodeError::Unrecognized);
    }
    let palette = videl_palette(&data[8..]).ok_or(DecodeError::Unrecognized)?;
    ok(planar_image(&data[1032..], 320, 200, 8, &palette, 1))
}

/// DuneGraph compressed: `DGC`, method, width, height, word, palette,
/// then raw interleaved planes (method 0) or separate planes run-length
/// coded in bytes, words or longs (methods 1-3).
pub(super) fn decode_dc1(data: &[u8]) -> Result<Image, DecodeError> {
    ok(decode_dc1_inner(data))
}

fn decode_dc1_inner(data: &[u8]) -> Option<Image> {
    if data.get(..3)? != b"DGC" {
        return None;
    }
    let method = *data.get(3)?;
    let palette = videl_palette(data.get(10..)?)?;
    let body = &data[1034..];
    let bitmap = match method {
        0 => body.get(..64000)?.to_vec(),
        1..=3 => {
            let size = be32(body, 0)? as usize;
            let packed = body.get(4..size)?;
            let (count_len, value_len) = match method {
                1 => (1, 1),
                2 => (2, 2),
                _ => (2, 4),
            };
            let planes = unpack_dc1(packed, count_len, value_len);
            separate_planes_to_interleaved(&planes, 8)
        }
        _ => return None,
    };
    planar_image(&bitmap, 320, 200, 8, &palette, 1)
}

/// Runs of `count + 1` copies until the data ends; planes not reached stay
/// zero (derived from sample files, which may store only the used planes).
fn unpack_dc1(data: &[u8], count_len: usize, value_len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(64000);
    for record in data.chunks_exact(count_len + value_len) {
        if out.len() >= 64000 {
            break;
        }
        let count = match count_len {
            1 => usize::from(record[0]),
            _ => usize::from(u16::from_be_bytes([record[0], record[1]])),
        };
        for _ in 0..=count {
            out.extend_from_slice(&record[count_len..]);
        }
    }
    out.resize(64000, 0);
    out
}

/// FuckPaint: VIDEL palette, 8 interleaved planes; the size gives the
/// height (`PI4`/`PI9` 320x240 or 320x200, `PI7` 640x480).
pub(super) fn decode_fuckpaint(data: &[u8]) -> Result<Image, DecodeError> {
    let (width, height) = match data.len() {
        77824 => (320, 240),
        65024 => (320, 200),
        308224 => (640, 480),
        _ => return Err(DecodeError::Unrecognized),
    };
    let palette = videl_palette(data).ok_or(DecodeError::Unrecognized)?;
    ok(planar_image(&data[1024..], width, height, 8, &palette, 1))
}

/// GodPaint: id word, width, height, RGB565.
pub(super) fn decode_god(data: &[u8]) -> Result<Image, DecodeError> {
    let width: usize = be16(data, 2).ok_or(DecodeError::Unrecognized)?.into();
    let height: usize = be16(data, 4).ok_or(DecodeError::Unrecognized)?.into();
    if check_size(width, height).is_none() || data.len() != 6 + width * height * 2 {
        return Err(DecodeError::Unrecognized);
    }
    ok(high_color(&data[6..], width, height, 1))
}

/// COKE: `COKE format.`, width, height, data offset, RGB565.
pub(super) fn decode_tg1(data: &[u8]) -> Result<Image, DecodeError> {
    if data.get(..12) != Some(b"COKE format.") {
        return Err(DecodeError::Unrecognized);
    }
    let width = be16(data, 12).ok_or(DecodeError::Unrecognized)?.into();
    let height = be16(data, 14).ok_or(DecodeError::Unrecognized)?.into();
    let offset = be16(data, 16).ok_or(DecodeError::Unrecognized)?.into();
    ok(high_color(
        data.get(offset..).unwrap_or(&[]),
        width,
        height,
        1,
    ))
}

/// EggPaint (`TRUP`) and Spooky Sprites (`tru?`): id, width, height, RGB565.
pub(super) fn decode_trp(data: &[u8]) -> Result<Image, DecodeError> {
    // EggPaint files are often Pack-Ice packed (EggPaint page above).
    if super::pack_ice::is_packed(data) {
        let unpacked = super::pack_ice::unpack(data).ok_or(DecodeError::Unrecognized)?;
        return decode_trp(&unpacked);
    }
    match data.get(..4) {
        Some(b"TRUP" | b"tru?") => {}
        _ => return Err(DecodeError::Unrecognized),
    }
    let width = be16(data, 4).ok_or(DecodeError::Unrecognized)?.into();
    let height = be16(data, 6).ok_or(DecodeError::Unrecognized)?.into();
    ok(high_color(&data[8..], width, height, 1))
}

/// IndyPaint: `Indy`, width, height, 248 zero bytes, RGB565.
pub(super) fn decode_tru(data: &[u8]) -> Result<Image, DecodeError> {
    if data.get(..4) != Some(b"Indy") {
        return Err(DecodeError::Unrecognized);
    }
    let width = be16(data, 4).ok_or(DecodeError::Unrecognized)?.into();
    let height = be16(data, 6).ok_or(DecodeError::Unrecognized)?.into();
    ok(high_color(data.get(256..).unwrap_or(&[]), width, height, 1))
}

/// Rembrandt: `TRUECOLR` header, then a `PICT` header and RGB565 data.
pub(super) fn decode_tcp(data: &[u8]) -> Result<Image, DecodeError> {
    ok(decode_tcp_inner(data))
}

fn decode_tcp_inner(data: &[u8]) -> Option<Image> {
    if data.get(..8)? != b"TRUECOLR" {
        return None;
    }
    let pict = usize::from(be16(data, 12)?);
    if data.get(pict..pict + 4)? != b"PICT" {
        return None;
    }
    let header_len = usize::from(be16(data, pict + 8)?);
    let width = usize::from(be16(data, pict + 10)?);
    let height = usize::from(be16(data, pict + 12)?);
    high_color(data.get(pict + header_len..)?, width, height, 1)
}

/// Spooky Sprites RLE: `tre1`, width, height, chunk count, then
/// alternating raw and repeat chunks of RGB565 pixels; counts of 255
/// continue in a following word.
pub(super) fn decode_tre(data: &[u8]) -> Result<Image, DecodeError> {
    ok(decode_tre_inner(data))
}

fn decode_tre_inner(data: &[u8]) -> Option<Image> {
    if data.get(..4)? != b"tre1" {
        return None;
    }
    let width = usize::from(be16(data, 4)?);
    let height = usize::from(be16(data, 6)?);
    check_size(width, height)?;
    let total = width * height;
    let mut pixels: Vec<u16> = Vec::with_capacity(total);
    let mut pos = 12;
    let mut raw = true;
    while pixels.len() < total {
        let mut n = usize::from(*data.get(pos)?);
        pos += 1;
        if n == 255 {
            n += usize::from(be16(data, pos)?);
            pos += 2;
        }
        if raw {
            for _ in 0..n {
                pixels.push(be16(data, pos)?);
                pos += 2;
            }
        } else {
            let last = *pixels.last()?;
            let n = n.min(total - pixels.len());
            pixels.extend(core::iter::repeat_n(last, n));
        }
        raw = !raw;
    }
    let mut image = Image::new(width as u32, height as u32);
    for (i, &word) in pixels.iter().take(total).enumerate() {
        image.set((i % width) as u32, (i / width) as u32, rgb565(word));
    }
    Some(image)
}

/// ICDRAW icons (`IBI` one icon, `IB3` three): `ICBI`/`ICB3` header of 64
/// bytes with the size and plane count, then the (first) icon as 32x32
/// word-interleaved planes in the default VDI colours. Sources: survey
/// notes (`docs/research/atari-st-tt-falcon.md`); the layout is derived from
/// sample files and `recoil2png` output.
pub(super) fn decode_icdraw(data: &[u8]) -> Result<Image, DecodeError> {
    match data.get(..4) {
        Some(b"ICBI" | b"ICB3") => {}
        _ => return Err(DecodeError::Unrecognized),
    }
    if (be16(data, 8), be16(data, 10), be16(data, 12)) != (Some(32), Some(32), Some(4)) {
        return Err(DecodeError::Unrecognized);
    }
    let palette = super::common::default_vdi_palette(16);
    ok(planar_image(
        data.get(64..).unwrap_or(&[]),
        32,
        32,
        4,
        &palette,
        1,
    ))
}

/// Falcon True Color: raw 384x240 RGB565.
pub(super) fn decode_ftc(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 384 * 240 * 2 {
        return Err(DecodeError::Unrecognized);
    }
    ok(high_color(data, 384, 240, 1))
}

/// XGA: raw RGB565, 320x240 or 384x480 (shown 768x480).
pub(super) fn decode_xga(data: &[u8]) -> Result<Image, DecodeError> {
    match data.len() {
        153600 => ok(high_color(data, 320, 240, 1)),
        368640 => ok(high_color(data, 384, 480, 2)),
        _ => Err(DecodeError::Unrecognized),
    }
}

/// InShape: `IS_IMAGE`, type, planes, width, height, then 1-bit, 8-bit
/// grey, RGB or ARGB pixels.
pub(super) fn decode_iim(data: &[u8]) -> Result<Image, DecodeError> {
    ok(decode_iim_inner(data))
}

fn decode_iim_inner(data: &[u8]) -> Option<Image> {
    if data.get(..8)? != b"IS_IMAGE" {
        return None;
    }
    let kind = be16(data, 8)?;
    let width = usize::from(be16(data, 12)?);
    let height = usize::from(be16(data, 14)?);
    check_size(width, height)?;
    let body = &data[16..];
    match kind {
        0 => {
            let row_len = width.div_ceil(8);
            super::common::mono_image(body, width as u32, height as u32, row_len)
        }
        1 => grey(body, width, height, |v| u32::from(!v)),
        4 | 5 => {
            let bytes = if kind == 4 { 3 } else { 4 };
            let body = body.get(..width * height * bytes)?;
            let mut image = Image::new(width as u32, height as u32);
            for (i, p) in body.chunks_exact(bytes).enumerate() {
                let rgb = &p[bytes - 3..];
                let color = u32::from_be_bytes([0, rgb[0], rgb[1], rgb[2]]);
                image.set((i % width) as u32, (i / width) as u32, color);
            }
            Some(image)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb565_replicates_bits() {
        assert_eq!(rgb565(0xffff), 0xffffff);
        assert_eq!(rgb565(0x0001), 0x000008);
        assert_eq!(rgb565(0x0020), 0x000400);
        assert_eq!(rgb565(0x0004), 0x000021);
    }

    #[test]
    fn dc1_runs() {
        let out = unpack_dc1(&[1, 7, 0xff, 9], 1, 1);
        assert_eq!(&out[..3], &[7, 7, 9]);
        assert_eq!(out.len(), 64000);
        let mut data = Vec::new();
        for _ in 0..250 {
            data.extend_from_slice(&[255, 0]);
        }
        assert!(unpack_dc1(&data, 1, 1).iter().all(|&b| b == 0));
    }
}
