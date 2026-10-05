//! Windows and OS/2 bitmaps (BMP, DIB).
//!
//! Sources:
//! - Microsoft, Bitmap Storage and the BITMAPINFOHEADER, BITMAPV4HEADER and
//!   BITMAPV5HEADER pages:
//!   <https://learn.microsoft.com/en-us/windows/win32/gdi/bitmap-storage>
//!   (14-byte file header, info header, palette, rows padded to 4 bytes and
//!   stored bottom-up unless the height is negative, RLE4/RLE8 escape codes,
//!   BI_BITFIELDS masks after a 40-byte header or inside later headers).
//! - OS/2 BITMAPCOREHEADER (12 bytes, 16-bit sizes, 3-byte palette entries)
//!   and the 16/64-byte OS/2 2.x headers (compression 3 and 4 there mean
//!   Huffman 1D and RLE24, which are rejected): Deark `bmp.c`
//!   (<https://github.com/jsummers/deark>, MIT licence), also used as the
//!   oracle for the sample files.
//!
//! The alpha channel of 32-bit pixels and of V4/V5 masks is ignored, as for
//! Targa. JPEG and PNG payloads (compression 4 and 5) are rejected. A
//! MacBinary wrapper (`crate::macbinary`) is removed first. A headerless DIB starts at the info header and is chosen by extension only.
//!
//! Verification: no RECOIL oracle for this format; output was compared pixel
//! for pixel with Deark's PNG output on the sample files.

// Parts of this file follow Deark's modules/bmp.c
// (Deark, https://github.com/jsummers/deark):
//
// Copyright (C) 2016 Jason Summers
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

use alloc::borrow::Cow;
use alloc::vec;
use alloc::vec::Vec;

use crate::bytes::{le16, le32};
use crate::image::check_size;
use crate::macbinary::data_fork_or_self;
use crate::{DecodeError, Image};

const FILE_HEADER_LEN: usize = 14;
/// Info header sizes: OS/2 core, OS/2 2.x short, Windows 3 to 5, OS/2 2.x.
const HEADER_SIZES: [usize; 8] = [12, 16, 40, 52, 56, 64, 108, 124];
/// Where the colour masks start in a header that carries them.
const MASKS_AT: usize = 40;

#[derive(Clone, Copy, PartialEq)]
enum Compression {
    Rgb,
    Rle8,
    Rle4,
    Bitfields,
}

struct Info {
    header_len: usize,
    width: usize,
    height: usize,
    top_down: bool,
    bpp: usize,
    compression: Compression,
    /// BI_ALPHABITFIELDS: a fourth mask follows a 40-byte header's three.
    alpha_mask: bool,
    colors_used: usize,
    /// Bytes per palette entry.
    entry_len: usize,
    /// An OS/2 2.x header (16 or 64 bytes).
    os2_v2: bool,
}

fn parse_info(data: &[u8]) -> Result<Info, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let header_len = le32(data, 0).ok_or(fail)? as usize;
    if !HEADER_SIZES.contains(&header_len) || data.len() < header_len {
        return Err(fail);
    }
    let (width, height, planes, bpp);
    let (mut top_down, mut compression, mut colors_used) = (false, 0, 0);
    if header_len == 12 {
        width = usize::from(le16(data, 4).ok_or(fail)?);
        height = usize::from(le16(data, 6).ok_or(fail)?);
        planes = le16(data, 8).ok_or(fail)?;
        bpp = usize::from(le16(data, 10).ok_or(fail)?);
    } else {
        let w = le32(data, 4).ok_or(fail)? as i32;
        let h = le32(data, 8).ok_or(fail)? as i32;
        width = usize::try_from(w).map_err(|_| fail)?;
        height = h.unsigned_abs() as usize;
        top_down = h < 0;
        planes = le16(data, 12).ok_or(fail)?;
        bpp = usize::from(le16(data, 14).ok_or(fail)?);
        if header_len >= 20 {
            compression = le32(data, 16).ok_or(fail)?;
        }
        if header_len >= 36 {
            colors_used = le32(data, 32).ok_or(fail)? as usize;
        }
    }
    let os2_v2 = header_len == 16 || header_len == 64;
    let alpha_mask = compression == 6;
    let compression = match (compression, bpp) {
        (0, 1 | 4 | 8 | 16 | 24 | 32) => Compression::Rgb,
        (1, 8) => Compression::Rle8,
        (2, 4) => Compression::Rle4,
        (3 | 6, 16 | 32) if !os2_v2 => Compression::Bitfields,
        _ => return Err(fail),
    };
    if planes != 1 {
        return Err(fail);
    }
    check_size(width, height)?;
    Ok(Info {
        header_len,
        width,
        height,
        top_down,
        bpp,
        compression,
        alpha_mask,
        colors_used,
        entry_len: if header_len == 12 { 3 } else { 4 },
        os2_v2,
    })
}

/// Parses a BMP file's headers: `BM`, a known info header, and a pixel offset
/// that lies inside the file. Returns the info and the pixel offset.
fn parse_file(data: &[u8]) -> Result<(Info, usize), DecodeError> {
    let fail = DecodeError::Unrecognized;
    if !data.starts_with(b"BM") {
        return Err(fail);
    }
    let offset = le32(data, 10).ok_or(fail)? as usize;
    let info = parse_info(&data[FILE_HEADER_LEN..])?;
    if offset < FILE_HEADER_LEN + info.header_len || offset >= data.len() {
        return Err(fail);
    }
    Ok((info, offset))
}

pub(super) fn decode_bmp(data: &[u8]) -> Result<Image, DecodeError> {
    let data = data_fork_or_self(data);
    let (info, offset) = parse_file(data)?;
    decode_pixels(
        &data[FILE_HEADER_LEN..],
        &info,
        Some(offset - FILE_HEADER_LEN),
    )
}

/// The bitmap of an icon or cursor entry: a headerless DIB whose height
/// counts the colour bitmap plus a 1-bit mask of the same size, so only the
/// first half of the rows is decoded.
pub(super) fn decode_icon_dib(data: &[u8]) -> Result<Image, DecodeError> {
    let mut info = parse_info(data)?;
    info.height /= 2;
    if info.height == 0 {
        return Err(DecodeError::Unrecognized);
    }
    decode_pixels(data, &info, None)
}

/// A headerless DIB: the info header, palette and pixels, no file header.
pub(super) fn decode_dib(data: &[u8]) -> Result<Image, DecodeError> {
    let info = parse_info(data)?;
    decode_pixels(data, &info, None)
}

/// A channel mask: where its bits sit and the largest value they hold.
#[derive(Clone, Copy)]
struct Mask {
    shift: u32,
    max: u32,
}

impl Mask {
    fn new(mask: u32) -> Self {
        if mask == 0 {
            return Self { shift: 0, max: 0 };
        }
        let shift = mask.trailing_zeros();
        Self {
            shift,
            max: mask >> shift,
        }
    }

    /// The channel of `pixel` scaled to 0..=255, rounded to nearest (as Deark does).
    fn extract(self, pixel: u32) -> u32 {
        if self.max == 0 {
            return 0;
        }
        let value = (pixel >> self.shift) & self.max;
        let max = u64::from(self.max);
        ((u64::from(value) * 255 + max / 2) / max) as u32
    }
}

/// `dib` starts at the info header; `pixels` is the offset of the pixel data
/// in it, or `None` when it follows the palette directly.
fn decode_pixels(dib: &[u8], info: &Info, pixels: Option<usize>) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let Info {
        header_len,
        width,
        height,
        bpp,
        ..
    } = *info;

    // Masks sit after a 40-byte header, or inside the later ones.
    let mut after_header = header_len;
    let mut masks = match bpp {
        16 => [0x7c00, 0x03e0, 0x001f],
        _ => [0xff0000, 0x00ff00, 0x0000ff],
    };
    if info.compression == Compression::Bitfields {
        for (i, mask) in masks.iter_mut().enumerate() {
            *mask = le32(dib, MASKS_AT + i * 4).ok_or(fail)?;
        }
        if header_len == 40 {
            after_header += if info.alpha_mask { 16 } else { 12 };
        }
    }

    let palette_len = if bpp <= 8 {
        let full = 1usize << bpp;
        match info.colors_used {
            0 => full,
            n => n.min(full),
        }
    } else {
        0
    };
    let palette_bytes = dib.get(after_header..).ok_or(fail)?;
    // The pixel offset can cut a palette short; missing entries stay black.
    let available = match pixels {
        Some(offset) => offset.saturating_sub(after_header),
        None => palette_bytes.len(),
    };
    // OS/2 2.x headers can come with 3-byte entries; the pixel offset tells.
    let entry_len = if info.os2_v2 && available >= palette_len * 3 && available < palette_len * 4 {
        3
    } else {
        info.entry_len
    };
    let stored = palette_len.min(available / entry_len);
    let mut palette = vec![0u32; 1 << bpp.min(8)];
    for (slot, entry) in palette
        .iter_mut()
        .zip(palette_bytes.chunks_exact(entry_len).take(stored))
    {
        *slot = u32::from(entry[2]) << 16 | u32::from(entry[1]) << 8 | u32::from(entry[0]);
    }

    let start = pixels.unwrap_or(after_header + palette_len * entry_len);
    let body = dib.get(start..).ok_or(fail)?;
    let (w, h) = (width as u32, height as u32);

    // Rows of `row_len` bytes in file order to top-first order.
    let top_first = |rows: Vec<u8>, row_len: usize| -> Vec<u8> {
        if info.top_down {
            rows
        } else {
            rows.chunks_exact(row_len)
                .rev()
                .flatten()
                .copied()
                .collect()
        }
    };
    let stride = (width * bpp).div_ceil(32) * 4;

    match info.compression {
        Compression::Rle8 | Compression::Rle4 => {
            let rows = unpack_rle(body, width, height, info.compression == Compression::Rle4)?;
            Image::from_indexed(w, h, &top_first(rows, width), &palette)
        }
        _ if bpp <= 8 => {
            let raw = whole_rows(body, stride, height)?;
            let mut indices = Vec::with_capacity(width * height);
            for row in raw.chunks_exact(stride) {
                indices.extend((0..width).map(|x| {
                    let bit = x * bpp;
                    (row[bit / 8] >> (8 - bpp - (bit & 7))) & ((1u16 << bpp) - 1) as u8
                }));
            }
            Image::from_indexed(w, h, &top_first(indices, width), &palette)
        }
        _ => {
            let raw = whole_rows(body, stride, height)?;
            let [r, g, b] = masks.map(Mask::new);
            let mut image = Image::new(w, h);
            for (row, y) in raw.chunks_exact(stride).zip(0..h) {
                let y = if info.top_down { y } else { h - 1 - y };
                for x in 0..width {
                    let color = if bpp == 24 {
                        let p = &row[x * 3..x * 3 + 3];
                        u32::from(p[2]) << 16 | u32::from(p[1]) << 8 | u32::from(p[0])
                    } else {
                        let p = &row[x * bpp / 8..][..bpp / 8];
                        let v = if bpp == 16 {
                            u32::from(u16::from_le_bytes([p[0], p[1]]))
                        } else {
                            u32::from_le_bytes([p[0], p[1], p[2], p[3]])
                        };
                        r.extract(v) << 16 | g.extract(v) << 8 | b.extract(v)
                    };
                    image.set(x as u32, y, color);
                }
            }
            Ok(image)
        }
    }
}

/// `height` rows of `stride` bytes. Files cut short are common; if at least
/// half the rows are there the rest is zero-filled.
fn whole_rows(body: &[u8], stride: usize, height: usize) -> Result<Cow<'_, [u8]>, DecodeError> {
    let total = stride * height;
    if body.len() >= total {
        Ok(Cow::Borrowed(&body[..total]))
    } else if body.len() >= total / 2 {
        let mut rows = body.to_vec();
        rows.resize(total, 0);
        Ok(Cow::Owned(rows))
    } else {
        Err(DecodeError::Unrecognized)
    }
}

/// Expands RLE8 or RLE4 data into `width` x `height` palette indices, first
/// row in the file first. Pixels no code reaches stay 0; a missing end code
/// is tolerated, as common viewers do.
fn unpack_rle(
    data: &[u8],
    width: usize,
    height: usize,
    nibbles: bool,
) -> Result<Vec<u8>, DecodeError> {
    // A delta moves at most 255 rows and columns for 4 bytes, an encoded run
    // covers at most 255 pixels for 2 bytes; this keeps tiny files from
    // demanding a huge frame.
    if width * height > data.len().saturating_mul(1 << 14) {
        return Err(DecodeError::Unrecognized);
    }
    let mut out = vec![0u8; width * height];
    let (mut x, mut y) = (0usize, 0usize);
    let mut put = |x: usize, y: usize, v: u8| {
        if x < width && y < height {
            out[y * width + x] = v;
        }
    };
    let nibble = |byte: u8, i: usize| {
        if i.is_multiple_of(2) {
            byte >> 4
        } else {
            byte & 15
        }
    };
    let mut at = 0;
    while y < height {
        let (Some(&count), Some(&value)) = (data.get(at), data.get(at + 1)) else {
            break;
        };
        at += 2;
        let count = usize::from(count);
        if count > 0 {
            for i in 0..count {
                put(x + i, y, if nibbles { nibble(value, i) } else { value });
            }
            x += count;
            continue;
        }
        match value {
            0 => {
                x = 0;
                y += 1;
            }
            1 => break,
            2 => {
                let (Some(&dx), Some(&dy)) = (data.get(at), data.get(at + 1)) else {
                    break;
                };
                at += 2;
                x += usize::from(dx);
                y += usize::from(dy);
            }
            n => {
                let n = usize::from(n);
                let bytes = if nibbles { n.div_ceil(2) } else { n };
                let Some(run) = data.get(at..at + bytes) else {
                    break;
                };
                for i in 0..n {
                    let v = if nibbles {
                        nibble(run[i / 2], i)
                    } else {
                        run[i]
                    };
                    put(x + i, y, v);
                }
                x += n;
                // Absolute runs are padded to a 16-bit boundary.
                at += bytes + (bytes & 1);
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dib(width: i32, height: i32, bpp: u16, compression: u32, extra: &[u8]) -> Vec<u8> {
        let mut d = vec![0u8; 40];
        d[0] = 40;
        d[4..8].copy_from_slice(&width.to_le_bytes());
        d[8..12].copy_from_slice(&height.to_le_bytes());
        d[12] = 1;
        d[14..16].copy_from_slice(&bpp.to_le_bytes());
        d[16..20].copy_from_slice(&compression.to_le_bytes());
        d.extend_from_slice(extra);
        d
    }

    fn file(dib: &[u8], palette_len: usize) -> Vec<u8> {
        let offset = (FILE_HEADER_LEN + 40 + palette_len) as u32;
        let mut f = b"BM".to_vec();
        f.extend_from_slice(&[0; 8]);
        f.extend_from_slice(&offset.to_le_bytes());
        f.extend_from_slice(dib);
        f
    }

    #[test]
    fn bottom_up_24_bit_with_row_padding() {
        // 1x2: bottom row (first in file) red, top row green; rows pad to 4.
        let d = dib(1, 2, 24, 0, &[0, 0, 255, 0, 0, 255, 0, 0]);
        let image = decode_bmp(&file(&d, 0)).unwrap();
        assert_eq!(image.rgb(), &[0, 255, 0, 255, 0, 0]);
    }

    #[test]
    fn top_down_flag_keeps_row_order() {
        let d = dib(1, -2, 24, 0, &[0, 0, 255, 0, 0, 255, 0, 0]);
        let image = decode_bmp(&file(&d, 0)).unwrap();
        assert_eq!(image.rgb(), &[255, 0, 0, 0, 255, 0]);
    }

    #[test]
    fn four_bit_rle_with_run_and_absolute_run() {
        // 4x2, palette entry 1 = white. First file row (bottom): encoded run
        // of nibbles 1,0,1,0. Second row: absolute run of 3 nibbles 1,1,1.
        let mut extra = vec![0, 0, 0, 0, 255, 255, 255, 0];
        extra.extend_from_slice(&[4, 0x10, 0, 0, 0, 3, 0x11, 0x10, 0, 1]);
        // The palette is limited to 2 entries by colours-used.
        let mut d = dib(4, 2, 4, 2, &extra);
        d[32] = 2;
        let image = decode_bmp(&file(&d, 8)).unwrap();
        let white: Vec<bool> = image.rgb().chunks(3).map(|p| p[0] == 255).collect();
        assert_eq!(white[..4], [true, true, true, false]); // top row
        assert_eq!(white[4..], [true, false, true, false]); // bottom row
    }

    #[test]
    fn bitfields_565() {
        let mut extra = Vec::new();
        for mask in [0xf800u32, 0x07e0, 0x001f] {
            extra.extend_from_slice(&mask.to_le_bytes());
        }
        extra.extend_from_slice(&[0x00, 0xf8, 0, 0]); // pure red, then padding
        let d = dib(1, 1, 16, 3, &extra);
        let image = decode_bmp(&file(&d, 12)).unwrap();
        assert_eq!(image.rgb(), &[255, 0, 0]);
    }

    #[test]
    fn rejects_garbage() {
        let d = dib(1, 1, 24, 0, &[0, 0, 0, 0]);
        let f = file(&d, 0);
        assert!(decode_bmp(&f).is_ok());
        // Missing rows are zero-filled only when at least half are present.
        let two_rows = file(&dib(1, 4, 24, 0, &[1; 8]), 0);
        assert!(decode_bmp(&two_rows).is_ok());
        assert!(decode_bmp(&two_rows[..two_rows.len() - 5]).is_err());
        assert!(decode_bmp(&f[..20]).is_err());
        let huge = dib(60000, 60000, 24, 0, &[0, 0, 0, 0]);
        assert!(decode_bmp(&file(&huge, 0)).is_err());
        assert!(decode_dib(&dib(1, 1, 7, 0, &[0; 4])).is_err());
    }
}
