//! ColoRIX VGA Paint "RIX3" pictures (`.RIX`, `.SCI`, `.SCX`, ...).
//!
//! Sources:
//! - Header, palette types, storage flags: Encyclopedia of Graphics File
//!   Formats, <https://www.fileformat.info/format/rix/egff.htm>, and
//!   <http://fileformats.archiveteam.org/wiki/ColoRIX>.
//! - Compression (Huffman node table, segments, run-length codes, XOR
//!   filter) and the 4-bit planar image type: Deark `colorix.c`
//!   (<https://github.com/jsummers/deark>, MIT license, notice below). The compressed path is checked against
//!   the `colorix-compressed-sci` and `colorix-ega-scr` samples.
//! - Palette type 0 (a 256-colour palette whose last entries are the only
//!   non-black ones) and the file sizes: reverse engineered from the sample
//!   `HELPKEYS.SCI`, whose size is exactly `10 + 768 + width * height`.
//!
//! Layout: `RIX3`, width and height (u16 LE), palette type (`AF` and `00`:
//! 256 VGA entries, `AB`: 16), storage byte (bit 7 compressed, bit 6
//! extension block follows the header, bit 5 encrypted, low nibble image
//! type 0 = 8-bit chunky or 4 = 4-bit planar), then the palette (6-bit RGB)
//! and the pixels.
//!
//! The old headerless EGA `.SCR` (always 640x350, 16 EGA palette indices,
//! bit 7 of the first one flagging compression, then 4 whole-image planes) follows
//! Deark `colorix.c` for the size and flag, and the 112016-byte sample files
//! in `corpus/extra/colorix-ega-scr` for the plane order (checked by eye).
//!
//! Compressed data is a table of 16-bit nodes describing a Huffman tree
//! (most significant bit first; an even value `2..0x1000` is a branch whose
//! 1 child follows it and whose 0 child is that many bytes further on, a
//! value `0x1000..=0x10ff` is a leaf), then segments of `u16` length plus
//! codes. The decoded bytes are run-length coded (`00` and `FF` are followed
//! by a repeat count minus one) and, for 8-bit pictures, each pixel is XORed
//! onto the previous one, restarting at 0 in every segment.

// Parts of this file follow Deark's colorix.c
// (Deark, https://github.com/jsummers/deark):
//
// Copyright (C) 2023 Jason Summers
// <jason1@pobox.com>
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

use alloc::vec::Vec;

use super::{ega_64, vga_rgb};
use crate::bytes::le16;
use crate::image::{check_size, planar_pixels};
use crate::{DecodeError, Image};

const COMPRESSED: u8 = 0x80;
const EXTENSION: u8 = 0x40;
const ENCRYPTED: u8 = 0x20;

pub(super) fn decode_rix(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    if data.get(..4) != Some(b"RIX3") {
        return Err(fail);
    }
    let width = usize::from(le16(data, 4).ok_or(fail)?);
    let height = usize::from(le16(data, 6).ok_or(fail)?);
    let (palette_type, storage) = (*data.get(8).ok_or(fail)?, *data.get(9).ok_or(fail)?);
    if storage & ENCRYPTED != 0 {
        return Err(fail);
    }
    check_size(width, height)?;
    let planar = match storage & 0x0f {
        0 => false,
        4 => true,
        _ => return Err(fail),
    };
    let mut pos = 10;
    if storage & EXTENSION != 0 {
        pos += 2 + usize::from(le16(data, pos).ok_or(fail)?);
    }
    let colors = if palette_type == 0xab { 16 } else { 256 };
    let palette_bytes = data.get(pos..pos + colors * 3).ok_or(fail)?;
    pos += colors * 3;
    let mut palette = alloc::vec![0; 256];
    for (entry, rgb) in palette.iter_mut().zip(palette_bytes.as_chunks::<3>().0) {
        *entry = vga_rgb(*rgb);
    }

    let row_len = if planar {
        width.next_multiple_of(8) / 2
    } else {
        width
    };
    let len = row_len * height;
    let pixels = if storage & COMPRESSED != 0 {
        unpack(data.get(pos..).ok_or(fail)?, len, row_len, !planar).ok_or(fail)?
    } else {
        data.get(pos..pos + len).ok_or(fail)?.to_vec()
    };
    let indices = if planar {
        planar_rows(&pixels, width, height, row_len)
    } else {
        pixels
    };
    Image::from_indexed(width as u32, height as u32, &indices, &palette)
}

const EGA_WIDTH: usize = 640;
const EGA_HEIGHT: usize = 350;
const EGA_PALETTE_LEN: usize = 16;

/// The old headerless EGA `.SCR`: 16 EGA palette indices (bit 7 of the first
/// one flags compression), then 640x350 pixels as 4 whole-image planes, raw or
/// packed like the `RIX3` pictures (each packed segment is one plane).
pub(super) fn decode_ega_scr(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let compressed = data.first().ok_or(fail)? & COMPRESSED != 0;
    let body = data.get(EGA_PALETTE_LEN..).ok_or(fail)?;
    let row_len = EGA_WIDTH / 8;
    let plane_len = row_len * EGA_HEIGHT;
    let len = plane_len * 4;
    let pixels = if compressed {
        unpack(body, len, row_len, false).ok_or(fail)?
    } else if body.len() == len {
        body.to_vec()
    } else {
        return Err(fail);
    };
    let mut palette = [0u32; 16];
    for (entry, &index) in palette.iter_mut().zip(data) {
        *entry = ega_64(index & 0x3f);
    }
    let indices: Vec<u8> = planar_pixels(&pixels, EGA_WIDTH, EGA_HEIGHT, row_len, 4, |plane, y| {
        plane * plane_len + y * row_len
    })
    .into_iter()
    .map(|v| v as u8)
    .collect();
    Image::from_indexed(EGA_WIDTH as u32, EGA_HEIGHT as u32, &indices, &palette)
}

/// Pixel indices of 4-bit rows holding their 4 planes one after the other.
fn planar_rows(pixels: &[u8], width: usize, height: usize, row_len: usize) -> Vec<u8> {
    let plane_len = row_len / 4;
    planar_pixels(pixels, width, height, plane_len, 4, |plane, y| {
        y * row_len + plane * plane_len
    })
    .into_iter()
    .map(|v| v as u8)
    .collect()
}

/// A node of the Huffman table is a branch.
fn is_branch(item: u16) -> bool {
    (2..0x1000).contains(&item) && item.is_multiple_of(2)
}

/// Run-length decoder state for one segment.
struct Runs {
    previous: u8,
    pending: Option<u8>,
    xor: bool,
}

impl Runs {
    fn push(&mut self, out: &mut Vec<u8>, code: u8) {
        let (value, count) = match self.pending.take() {
            Some(value) => (value, usize::from(code) + 1),
            None if code == 0x00 || code == 0xff => {
                self.pending = Some(code);
                return;
            }
            None => (code, 1),
        };
        for _ in 0..count {
            self.previous = if self.xor {
                self.previous ^ value
            } else {
                value
            };
            out.push(self.previous);
        }
    }
}

/// Unpacks `len` bytes of pixels in rows of `row_len` bytes from the
/// Huffman node table and segments at the start of `data`.
fn unpack(data: &[u8], len: usize, row_len: usize, xor: bool) -> Option<Vec<u8>> {
    let table_len = usize::from(le16(data, 0)?) * 2;
    let table = data.get(2..2 + table_len)?;
    let mut pos = 2 + table_len;
    let mut out = Vec::with_capacity(len + 256);
    // A segment needs at least a length word, and every one decodes to at
    // least a row.
    while out.len() < len && pos + 2 < data.len() {
        let size = usize::from(le16(data, pos)?);
        if size == 0 {
            break;
        }
        let segment = data.get(pos + 2..)?.get(..size.min(data.len() - pos - 2))?;
        let start = out.len();
        let mut runs = Runs {
            previous: 0,
            pending: None,
            xor,
        };
        decode_codes(table, segment, |code| {
            if out.len() < len {
                runs.push(&mut out, code);
            }
        });
        if out.len() - start < row_len {
            return None;
        }
        // The padding bits of a segment can decode to garbage, which shows
        // up as a partial row: drop it (Deark's heuristic).
        if out.len() < len {
            out.truncate(out.len() - out.len() % row_len);
        }
        pos += 2 + size;
    }
    (out.len() >= len).then(|| {
        out.truncate(len);
        out
    })
}

/// Walks the Huffman `table` over the bits of `segment`, passing every
/// decoded byte to `emit`. Stops quietly at the end of the data or at a
/// table entry that is neither a branch nor a leaf.
fn decode_codes(table: &[u8], segment: &[u8], mut emit: impl FnMut(u8)) {
    let mut node = 0;
    for bit in segment
        .iter()
        .flat_map(|&byte| (0..8).rev().map(move |i| byte >> i & 1))
    {
        let Some(item) = le16(table, node) else {
            return;
        };
        if !is_branch(item) {
            return;
        }
        node = if bit == 1 {
            node + 2
        } else {
            node + 2 + usize::from(item)
        };
        match le16(table, node) {
            Some(leaf @ 0x1000..=0x10ff) => {
                emit((leaf - 0x1000) as u8);
                node = 0;
            }
            Some(item) if is_branch(item) => {}
            _ => return,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(width: u16, height: u16, palette_type: u8, storage: u8) -> Vec<u8> {
        let mut data = b"RIX3".to_vec();
        data.extend_from_slice(&width.to_le_bytes());
        data.extend_from_slice(&height.to_le_bytes());
        data.extend_from_slice(&[palette_type, storage]);
        data
    }

    fn palette256() -> Vec<u8> {
        (0..=255u8).flat_map(|i| [i & 63, 0, 0]).collect()
    }

    #[test]
    fn uncompressed_chunky_picture() {
        let mut data = header(2, 1, 0xaf, 0);
        data.extend(palette256());
        data.extend_from_slice(&[1, 63]);
        let image = decode_rix(&data).unwrap();
        assert_eq!((image.width(), image.height()), (2, 1));
        assert_eq!(image.rgb(), &[4, 0, 0, 255, 0, 0]);
    }

    #[test]
    fn planar_picture_holds_four_planes_per_row() {
        let mut data = header(8, 1, 0xab, 4);
        data.extend((0..16u8).flat_map(|i| [i * 4 + 3, 0, 0]));
        // Plane 0 sets pixel 0, plane 3 sets pixels 0 and 7.
        data.extend_from_slice(&[0x80, 0, 0, 0x81]);
        let image = decode_rix(&data).unwrap();
        let red = |index: u8| vga_rgb([index * 4 + 3, 0, 0]);
        assert_eq!(image.get(0, 0), red(9));
        assert_eq!(image.get(7, 0), red(8));
        assert_eq!(image.get(3, 0), red(0));
    }

    #[test]
    fn compressed_picture_uses_huffman_runs_and_xor() {
        let mut data = header(4, 1, 0xaf, COMPRESSED);
        data.extend(palette256());
        // Table: branch (0 child 2 bytes after the 1 child), leaf 5, leaf 7.
        data.extend_from_slice(&3u16.to_le_bytes());
        for item in [2u16, 0x1005, 0x1007] {
            data.extend_from_slice(&item.to_le_bytes());
        }
        // One segment holding the codes 1, 0, 1, 1: bytes 5, 7, 5, 5.
        data.extend_from_slice(&[1, 0, 0b1011_0000]);
        let image = decode_rix(&data).unwrap();
        let expected = [5u8, 5 ^ 7, 5 ^ 7 ^ 5, 5 ^ 7 ^ 5 ^ 5];
        for (x, index) in expected.into_iter().enumerate() {
            assert_eq!(image.get(x as u32, 0), vga_rgb([index, 0, 0]));
        }
    }

    #[test]
    fn truncated_and_encrypted_files_are_rejected() {
        let mut data = header(2, 2, 0xaf, 0);
        data.extend(palette256());
        data.extend_from_slice(&[0, 1, 2]);
        assert!(decode_rix(&data).is_err());
        let mut encrypted = header(2, 1, 0xaf, ENCRYPTED);
        encrypted.extend(palette256());
        encrypted.extend_from_slice(&[0, 0]);
        assert!(decode_rix(&encrypted).is_err());
    }
}
