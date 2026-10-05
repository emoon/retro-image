//! ZSoft PC Paintbrush PCX.
//!
//! Sources:
//! - ZSoft PCX technical reference (the public "ZSoft PCX File Format
//!   Technical Reference Manual"), as summarised at
//!   <https://moddingwiki.shikadi.net/wiki/PCX_Format> and
//!   <https://en.wikipedia.org/wiki/PCX>: 128-byte header, byte-wise RLE
//!   (`0xC0 | count` then a value), rows of `planes * bytes_per_line` bytes
//!   with the planes stored one after another, an optional 256-color VGA
//!   palette behind a `0x0C` marker in the last 769 bytes.
//! - The EGA header palette is 16 RGB entries; older EGA writers keep the
//!   2-bit channel in the top bits (0, 64, 128, 192), and some VGA writers
//!   store 6-bit values (0..63). Both are scaled up when every value fits
//!   (a common convention, also noted on the ModdingWiki page).
//!
//! Layouts handled: 1, 2, 4 and 8 bits per pixel in one plane; 1 bit per
//! pixel in 2 to 4 planes (EGA); 8 bits in 3 planes (24-bit RGB) or 4
//! planes (RGB, alpha ignored). Version 3 files and others with no usable
//! palette get the CGA/EGA default colors, or grays for 8-bit.
//! A bounding box one pixel wider than the rows is cut to the row width
//! (observed on fax pages, see `dcx.rs`).
//!
//! Verification: no oracle exists (RECOIL has no PCX). Output was rendered
//! to PNG and viewed, and compared pixel for pixel with Python PIL's PCX
//! reader (used as a black box) on the sample files.

use alloc::vec;
use alloc::vec::Vec;

use super::{CGA_PALETTE, cga_set};
use crate::bytes::le16;
use crate::image::{check_size, planar_pixels, widen_channel};
use crate::{DecodeError, Image};

const HEADER_LEN: usize = 128;
const VGA_PALETTE_LEN: usize = 768;

/// CGA 2-bit default (palette 1, high intensity) used when the header
/// carries no palette.
const CGA_FOUR: [u32; 4] = cga_set([11, 13, 15]);

struct Header {
    version: u8,
    bits: usize,
    planes: usize,
    width: usize,
    height: usize,
    row_len: usize,
}

fn parse_header(data: &[u8]) -> Result<Header, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let h = data.get(..HEADER_LEN).ok_or(fail)?;
    // Manufacturer 10, known version, RLE encoding.
    if h[0] != 0x0a || !matches!(h[1], 0 | 2 | 3 | 4 | 5) || h[2] != 1 {
        return Err(fail);
    }
    let bits = usize::from(h[3]);
    let planes = usize::from(h[65]);
    let supported = matches!((bits, planes), (1, 1..=4) | (2 | 4, 1) | (8, 1 | 3 | 4));
    if !supported {
        return Err(fail);
    }
    let word = |at| le16(h, at).map(usize::from).ok_or(fail);
    let (x0, y0, x1, y1) = (word(4)?, word(6)?, word(8)?, word(10)?);
    if x1 < x0 || y1 < y0 {
        return Err(fail);
    }
    let (width, height) = (x1 - x0 + 1, y1 - y0 + 1);
    let row_len = word(66)?;
    // Fax software writes x_max as the width, one pixel more than a row
    // holds (1729 for the 1728 pixels of a fax line): the picture is as wide
    // as its rows.
    let stored = row_len * 8 / bits;
    let width = if width == stored + 1 { stored } else { width };
    if row_len < (width * bits).div_ceil(8) {
        return Err(fail);
    }
    // The padded raster is what gets unpacked and, for EGA planes, expanded
    // to a value per pixel, so it must fit the pixel budget too.
    check_size(width, height)?;
    check_size(row_len * 8 / bits, height)?;
    Ok(Header {
        version: h[1],
        bits,
        planes,
        width,
        height,
        row_len,
    })
}

/// Expands the RLE stream from `data` into exactly `len` bytes.
/// Fails on truncated data; a final run reaching past `len` is cut off.
pub(super) fn unpack(data: &[u8], len: usize) -> Result<(Vec<u8>, usize), DecodeError> {
    // A two-byte run yields at most 63 bytes.
    if len > data.len().saturating_mul(32) {
        return Err(DecodeError::Unrecognized);
    }
    let mut out = Vec::with_capacity(len);
    let mut pos = 0;
    while out.len() < len {
        let b = *data.get(pos).ok_or(DecodeError::Unrecognized)?;
        pos += 1;
        if b >= 0xc0 {
            let value = *data.get(pos).ok_or(DecodeError::Unrecognized)?;
            pos += 1;
            let count = usize::from(b & 0x3f).min(len - out.len());
            out.resize(out.len() + count, value);
        } else {
            out.push(b);
        }
    }
    Ok((out, pos))
}

/// Scales a palette entry list up when it is stored in 2 or 6 bits.
fn scale_levels(rgb: &mut [u8]) {
    if rgb.iter().all(|&v| v & 0x3f == 0) {
        for v in rgb.iter_mut() {
            *v = widen_channel(u32::from(*v >> 6), 2) as u8;
        }
    } else if rgb.iter().all(|&v| v < 64) {
        for v in rgb.iter_mut() {
            // Not `widen_channel`: this differs from bit replication for 30
            // of the 64 values.
            *v = (u32::from(*v) * 255 / 63) as u8;
        }
    }
}

fn colors(rgb: &[u8]) -> Vec<u32> {
    rgb.as_chunks::<3>()
        .0
        .iter()
        .map(|c| u32::from(c[0]) << 16 | u32::from(c[1]) << 8 | u32::from(c[2]))
        .collect()
}

/// The 16-entry header palette, or `fallback` for files that have none
/// (version 3 or an all-zero table).
fn header_palette(data: &[u8], version: u8, count: usize, fallback: &[u32]) -> Vec<u32> {
    let mut table = [0u8; 48];
    table.copy_from_slice(&data[16..64]);
    if version == 3 || table.iter().all(|&v| v == 0) {
        return fallback[..count].to_vec();
    }
    scale_levels(&mut table);
    colors(&table)[..count].to_vec()
}

pub(super) fn decode_pcx(data: &[u8]) -> Result<Image, DecodeError> {
    let head = parse_header(data)?;
    let Header {
        bits,
        planes,
        width,
        height,
        row_len,
        ..
    } = head;
    let total = row_len * planes * height;
    let (rows, used) = unpack(&data[HEADER_LEN..], total)?;
    let (w, h) = (width as u32, height as u32);

    if bits == 8 && planes >= 3 {
        let mut image = Image::new(w, h);
        for y in 0..height {
            let row = &rows[y * row_len * planes..];
            for x in 0..width {
                let c = |p: usize| u32::from(row[p * row_len + x]);
                image.set(x as u32, y as u32, c(0) << 16 | c(1) << 8 | c(2));
            }
        }
        return Ok(image);
    }

    if bits == 8 {
        let tail = &data[HEADER_LEN + used..];
        let palette = match tail.len().checked_sub(VGA_PALETTE_LEN + 1) {
            Some(skip) if tail[skip] == 0x0c => {
                let mut rgb = [0u8; VGA_PALETTE_LEN];
                rgb.copy_from_slice(&tail[skip + 1..skip + 1 + VGA_PALETTE_LEN]);
                scale_levels(&mut rgb);
                colors(&rgb)
            }
            _ => (0..256u32).map(|v| v * 0x01_01_01).collect(),
        };
        let pixels = pack_rows(&rows, row_len, width, height);
        return Image::from_indexed(w, h, &pixels, &palette);
    }

    let count = 1usize << (bits * planes);
    let fallback: &[u32] = if bits == 2 { &CGA_FOUR } else { &CGA_PALETTE };
    let palette = if bits == 1 && planes == 1 {
        vec![0x000000, 0xffffff]
    } else {
        header_palette(data, head.version, count, fallback)
    };

    let pixels: Vec<u8> = if planes > 1 {
        planar_pixels(&rows, width, height, row_len, planes, |plane, y| {
            (y * planes + plane) * row_len
        })
        .into_iter()
        .map(|v| v as u8)
        .collect()
    } else {
        let mut out = Vec::with_capacity(width * height);
        for row in rows.chunks_exact(row_len) {
            for x in 0..width {
                let bit = x * bits;
                let shift = 8 - bits - (bit & 7);
                out.push((row[bit / 8] >> shift) & ((1 << bits) - 1) as u8);
            }
        }
        out
    };
    Image::from_indexed(w, h, &pixels, &palette)
}

/// One byte per pixel from rows padded to `row_len`.
fn pack_rows(rows: &[u8], row_len: usize, width: usize, height: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(width * height);
    for row in rows.chunks_exact(row_len).take(height) {
        out.extend_from_slice(&row[..width]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(bits: u8, planes: u8, width: u16, height: u16, row_len: u16) -> Vec<u8> {
        let mut h = vec![0u8; HEADER_LEN];
        h[0] = 10;
        h[1] = 5;
        h[2] = 1;
        h[3] = bits;
        h[8..10].copy_from_slice(&(width - 1).to_le_bytes());
        h[10..12].copy_from_slice(&(height - 1).to_le_bytes());
        h[65] = planes;
        h[66..68].copy_from_slice(&row_len.to_le_bytes());
        h
    }

    #[test]
    fn rejects_huge_row_padding() {
        // One pixel wide, but 65535-byte rows: the padded raster is far over
        // the pixel budget, so it must not be unpacked or expanded. The body
        // is big enough to pass the unpacker's input-size bound.
        let mut data = header(1, 2, 1, 2048, u16::MAX);
        data.resize(
            HEADER_LEN + (u16::MAX as usize * 2 * 2048).div_ceil(32),
            0xff,
        );
        assert!(decode_pcx(&data).is_err());
    }

    #[test]
    fn eight_bit_with_trailing_palette() {
        let mut data = header(8, 1, 3, 1, 4);
        data.extend_from_slice(&[0xc4, 2]); // run of four 2s: three pixels + pad
        data.push(0x0c);
        let mut pal = vec![0u8; 768];
        pal[6..9].copy_from_slice(&[10, 200, 30]);
        data.extend_from_slice(&pal);
        let image = decode_pcx(&data).unwrap();
        assert_eq!((image.width(), image.height()), (3, 1));
        assert_eq!(&image.rgb()[..3], &[10, 200, 30]);
    }

    #[test]
    fn ega_planes_use_header_palette() {
        // 8x1, planes 0 and 2 set on the leftmost pixel: color 5.
        let mut data = header(1, 4, 8, 1, 2);
        data[16 + 15..16 + 18].copy_from_slice(&[1, 2, 3]);
        data[16 + 5 * 3..16 + 5 * 3 + 3].copy_from_slice(&[7, 8, 9]);
        data.extend_from_slice(&[0x80, 0, 0, 0, 0x80, 0, 0, 0]);
        let image = decode_pcx(&data).unwrap();
        // Values below 64 are 6-bit and get scaled.
        assert_eq!(&image.rgb()[..3], &[28, 32, 36]);
    }

    #[test]
    fn width_one_pixel_past_the_rows_is_cut_to_the_rows() {
        // x_max = 16 gives 17 pixels, but a row holds 16.
        let mut data = header(1, 1, 17, 1, 2);
        data.extend_from_slice(&[0x80, 0x01]);
        let image = decode_pcx(&data).unwrap();
        assert_eq!((image.width(), image.height()), (16, 1));
        // Two pixels too many is still a bad header.
        let mut wide = header(1, 1, 18, 1, 2);
        wide.extend_from_slice(&[0x80, 0x01]);
        assert!(decode_pcx(&wide).is_err());
    }

    #[test]
    fn rejects_bad_headers_and_truncation() {
        let mut data = header(8, 1, 2, 2, 2);
        data.extend_from_slice(&[1, 2, 3]);
        assert!(decode_pcx(&data).is_err());
        assert!(decode_pcx(&data[..100]).is_err());
        let mut bad = header(3, 1, 2, 2, 2);
        bad.extend_from_slice(&[1, 2, 3, 4]);
        assert!(decode_pcx(&bad).is_err());
    }
}
