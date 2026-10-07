//! Psion Series 5 (EPOC) bitmaps: multi-bitmap files (`.mbm`, first bitmap),
//! Sketch files and application info files (`.aif`, first icon with its mask).
//!
//! Sources:
//! - Deark's `modules/epocimage.c` (<https://github.com/jsummers/deark>, MIT
//!   license, notice below): the file header (UIDs `$10000037`, then `$10000042`
//!   for an MBM, `$1000008a` for an exported MBM, `$1000006d` and `$1000007d`
//!   for a Sketch, `$1000006a` or `$10003a38` for an AIF; a 4-byte checksum;
//!   the offset of the bitmap table at 16), the MBM jump table (a count and
//!   offsets), the Sketch section table (a byte holding twice the entry count,
//!   then `id`, `offset` pairs; the section with id `$1000007d` has an 18-byte
//!   header before its bitmap), the AIF table (a caption count byte, three
//!   bytes per caption, a bitmap count, the offset of the first bitmap; bitmaps
//!   follow one another, each followed by its mask), and the paint data
//!   section (size, header length 40, width and height at 8 and 12, bits per
//!   pixel at 24, color flag at 28, compression at 36; rows padded to 32 bits,
//!   the first pixel in the lowest bits; compression 1 is byte-wise RLE:
//!   a control of 0 to 127 repeats the next byte `control + 1` times, 128 to
//!   255 copies `256 - control` bytes). Deark is also the oracle for the
//!   samples.
//! - Checked on the dexvert samples: 11 MBM, 10 Sketch and 10 AIF files. All
//!   are grayscale bitmaps of 1, 2 or 4 bits, stored raw or RLE-compressed.
//!
//! Choices of this crate: only grayscale bitmaps are decoded (0 is black, the
//! ramp is even), because no color sample was available to check Deark's
//! palettes against; a color bitmap or another compression is rejected. A
//! mask makes black opaque and white transparent, and pixels outside a smaller
//! mask are transparent, as in Deark. Exported MBM files share the MBM layout
//! and are unverified, as is a mask of 8 bits. The first bitmap stands for an
//! MBM, which can hold several unrelated ones.

// Parts of this file follow Deark's modules/epocimage.c
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
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use alloc::borrow::Cow;
use alloc::vec::Vec;

use crate::bytes::le32;
use crate::image::check_size;
use crate::{DecodeError, Image};

const FAIL: DecodeError = DecodeError::Invalid;
/// The first UID of every file store.
const FILE_STORE: u32 = 0x1000_0037;
const SKETCH_SECTION: u32 = 0x1000_007d;
const SKETCH_HEADER_LEN: usize = 18;
/// The paint data section header, and so where its pixels start.
const BITMAP_HEADER_LEN: usize = 40;
/// Most bytes the padded rows of a bitmap may take, as many as the pixels of
/// the largest picture: a narrow bitmap pads each row to 4 bytes, and RLE
/// can unpack that from a small file.
const MAX_ROWS_LEN: usize = 1 << 26;

/// A decoded grayscale bitmap: `levels` are 0 (black) to 255, row-major.
struct Gray {
    width: usize,
    height: usize,
    levels: Vec<u8>,
    /// Bytes the paint data section takes in the file.
    section_len: usize,
}

/// Whether the second and third UIDs are the ones given (`None` is any).
fn has_uids(data: &[u8], second: &[u32], third: Option<u32>) -> bool {
    le32(data, 0) == Some(FILE_STORE)
        && le32(data, 4).is_some_and(|uid| second.contains(&uid))
        && third.is_none_or(|uid| le32(data, 8) == Some(uid))
}

/// The first bitmap of a multi-bitmap file.
pub(super) fn decode_mbm(data: &[u8]) -> Result<Image, DecodeError> {
    if !has_uids(data, &[0x1000_0042, 0x1000_008a], None) {
        return Err(FAIL);
    }
    let table = le32(data, 16).ok_or(FAIL)? as usize;
    if le32(data, table).ok_or(FAIL)? == 0 {
        return Err(FAIL);
    }
    let first = le32(data, table.checked_add(4).ok_or(FAIL)?).ok_or(FAIL)? as usize;
    opaque(&read_bitmap(data, first)?)
}

/// The picture of a Sketch file.
pub(super) fn decode_sketch(data: &[u8]) -> Result<Image, DecodeError> {
    if !has_uids(data, &[0x1000_006d], Some(SKETCH_SECTION)) {
        return Err(FAIL);
    }
    let table = le32(data, 16).ok_or(FAIL)? as usize;
    let entries = usize::from(*data.get(table).ok_or(FAIL)?) / 2;
    let section = (0..entries)
        .map(|i| table.saturating_add(1 + 8 * i))
        .find(|&at| le32(data, at) == Some(SKETCH_SECTION))
        .and_then(|at| le32(data, at.checked_add(4)?))
        .ok_or(FAIL)? as usize;
    let at = section.checked_add(SKETCH_HEADER_LEN).ok_or(FAIL)?;
    opaque(&read_bitmap(data, at)?)
}

/// The first icon of an application info file, drawn through its mask.
pub(super) fn decode_aif(data: &[u8]) -> Result<Image, DecodeError> {
    if !has_uids(data, &[0x1000_006a, 0x1000_3a38], None) {
        return Err(FAIL);
    }
    let table = le32(data, 16).ok_or(FAIL)? as usize;
    let captions = usize::from(*data.get(table).ok_or(FAIL)?);
    let count_at = table.checked_add(1 + 3 * captions).ok_or(FAIL)?;
    let count = *data.get(count_at).ok_or(FAIL)?;
    let first = le32(data, count_at + 1).ok_or(FAIL)? as usize;
    if count == 0 {
        return Err(FAIL);
    }
    let icon = read_bitmap(data, first)?;
    // The bitmap after the icon is its mask.
    let mask = (count > 1)
        .then(|| read_bitmap(data, first.checked_add(icon.section_len)?).ok())
        .flatten();
    let Some(mask) = mask else {
        return opaque(&icon);
    };
    let colors = (0..icon.width * icon.height).map(|i| {
        let (x, y) = (i % icon.width, i / icon.width);
        let alpha = if x < mask.width && y < mask.height {
            255 - mask.levels[y * mask.width + x]
        } else {
            0
        };
        (u32::from(alpha) << 24) | (u32::from(icon.levels[i]) * 0x01_0101)
    });
    Image::from_argb(icon.width as u32, icon.height as u32, colors)
}

fn opaque(bitmap: &Gray) -> Result<Image, DecodeError> {
    let colors = bitmap.levels.iter().map(|&v| u32::from(v) * 0x01_0101);
    Image::from_colors(bitmap.width as u32, bitmap.height as u32, colors)
}

/// The grayscale bitmap whose paint data section starts at `at`.
fn read_bitmap(data: &[u8], at: usize) -> Result<Gray, DecodeError> {
    let field = |offset: usize| le32(data, at.checked_add(offset)?).map(|value| value as usize);
    let section_len = field(0).ok_or(FAIL)?;
    let (width, height) = (field(8).ok_or(FAIL)?, field(12).ok_or(FAIL)?);
    let (bits, color, compression) = (field(24).ok_or(FAIL)?, field(28), field(36));
    if field(4) != Some(BITMAP_HEADER_LEN)
        || section_len < BITMAP_HEADER_LEN
        || !matches!(bits, 1 | 2 | 4 | 8)
        || color != Some(0)
    {
        return Err(FAIL);
    }
    check_size(width, height)?;
    let row_len = (width * bits).div_ceil(32) * 4;
    let size = row_len * height;
    if size > MAX_ROWS_LEN {
        return Err(FAIL);
    }
    let pixels_at = at + BITMAP_HEADER_LEN;
    let rows: Cow<[u8]> = match compression {
        Some(0) => {
            let end = pixels_at.checked_add(size).ok_or(FAIL)?;
            Cow::Borrowed(data.get(pixels_at..end).ok_or(FAIL)?)
        }
        Some(1) => {
            let end = at.checked_add(section_len).ok_or(FAIL)?;
            let packed = data.get(pixels_at..end).ok_or(FAIL)?;
            Cow::Owned(unpack_rle8(packed, size).ok_or(FAIL)?)
        }
        _ => return Err(FAIL),
    };
    // The first pixel is in the lowest bits; `bits` is a power of 2.
    let mask = (1usize << bits) - 1;
    let levels = (0..width * height)
        .map(|i| {
            let bit = i % width * bits;
            let byte = rows[i / width * row_len + bit / 8];
            (usize::from(byte >> (bit % 8)) & mask) * 255 / mask
        })
        .map(|level| level as u8)
        .collect();
    Ok(Gray {
        width,
        height,
        levels,
        section_len,
    })
}

/// `size` bytes of RLE8 data: a control below 128 repeats the next byte
/// `control + 1` times, otherwise `256 - control` bytes are copied.
fn unpack_rle8(packed: &[u8], size: usize) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut at = 0;
    while out.len() < size {
        let control = *packed.get(at)?;
        at += 1;
        if control < 0x80 {
            let value = *packed.get(at)?;
            at += 1;
            out.resize(out.len() + usize::from(control) + 1, value);
        } else {
            let count = 256 - usize::from(control);
            out.extend_from_slice(packed.get(at..at + count)?);
            at += count;
        }
    }
    out.truncate(size);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A paint data section: `bits` per pixel, `compression`, `pixels`.
    fn bitmap(width: u32, height: u32, bits: u32, compression: u32, pixels: &[u8]) -> Vec<u8> {
        let mut section = Vec::new();
        let size = (BITMAP_HEADER_LEN + pixels.len()) as u32;
        for word in [size, 40, width, height, 0, 0, bits, 0, 0, compression] {
            section.extend_from_slice(&word.to_le_bytes());
        }
        section.extend_from_slice(pixels);
        section
    }

    fn header(second: u32, third: u32, table: u32) -> Vec<u8> {
        let mut data = Vec::new();
        for word in [FILE_STORE, second, third, 0, table] {
            data.extend_from_slice(&word.to_le_bytes());
        }
        data
    }

    #[test]
    fn mbm_pixels_start_in_the_low_bits_and_rows_pad_to_32_bits() {
        // 3x2 at 2 bits: levels 0, 1, 2 then 3, 0, 0; each row takes 4 bytes.
        let section = bitmap(3, 2, 2, 0, &[0b10_01_00, 0, 0, 0, 0b11, 0, 0, 0]);
        let mut data = header(0x1000_0042, 0, 20 + section.len() as u32);
        let first = data.len() as u32;
        data.extend_from_slice(&section);
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&first.to_le_bytes());
        let image = decode_mbm(&data).unwrap();
        assert_eq!((image.width(), image.height()), (3, 2));
        let row = |y| [0, 1, 2].map(|x| image.get(x, y) & 0xff);
        assert_eq!(row(0), [0, 85, 170]);
        assert_eq!(row(1), [255, 0, 0]);
    }

    #[test]
    fn rle8_repeats_and_copies() {
        // 4 bytes: 0x11 three times, then one copied byte.
        assert_eq!(
            unpack_rle8(&[2, 0x11, 0xff, 0x22], 4),
            Some(alloc::vec![0x11, 0x11, 0x11, 0x22])
        );
        // Truncated data is refused, extra output is cut.
        assert_eq!(unpack_rle8(&[2, 0x11], 4), None);
        assert_eq!(unpack_rle8(&[7, 0x33], 2), Some(alloc::vec![0x33, 0x33]));
    }

    #[test]
    fn aif_icon_is_drawn_through_its_mask() {
        // A 2x1 icon with levels 255, 0 and a mask that is black (opaque) on
        // the left and white (transparent) on the right.
        let icon = bitmap(2, 1, 1, 0, &[0b01, 0, 0, 0]);
        let mask = bitmap(2, 1, 1, 0, &[0b10, 0, 0, 0]);
        let mut data = header(0x1000_006a, 0, 20);
        // Table: no captions, two bitmaps, the first at 26.
        data.extend_from_slice(&[0, 2]);
        data.extend_from_slice(&26u32.to_le_bytes());
        data.extend_from_slice(&icon);
        data.extend_from_slice(&mask);
        let image = decode_aif(&data).unwrap();
        assert_eq!(image.get_argb(0, 0), 0xffff_ffff);
        assert_eq!(image.get_argb(1, 0), crate::image::CLEAR);
    }

    #[test]
    fn padded_rows_are_bounded_before_they_are_unpacked() {
        // 1 x 16777217 at 8 bits: the 4-byte rows hold 64 MiB of padding, more
        // than a picture of that many pixels may take; the RLE data would
        // unpack to exactly that, 128 bytes per pair.
        let height = (1u32 << 24) + 1;
        let size = 4 * height as usize;
        let packed: Vec<u8> = (0..size.div_ceil(128)).flat_map(|_| [0x7f, 0]).collect();
        let section = bitmap(1, height, 8, 1, &packed);
        let mut data = header(0x1000_0042, 0, 20 + section.len() as u32);
        data.extend_from_slice(&section);
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&20u32.to_le_bytes());
        assert!(decode_mbm(&data).is_err());
    }

    #[test]
    fn color_and_unknown_compression_are_rejected() {
        let section = bitmap(1, 1, 1, 0, &[0, 0, 0, 0]);
        let mut data = header(0x1000_0042, 0, 20 + section.len() as u32);
        let table = data.len() + section.len();
        data.extend_from_slice(&section);
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&20u32.to_le_bytes());
        assert!(decode_mbm(&data).is_ok());
        // Color flag, then compression 2 (RLE12).
        data[20 + 28] = 1;
        assert!(decode_mbm(&data).is_err());
        data[20 + 28] = 0;
        data[20 + 36] = 2;
        assert!(decode_mbm(&data).is_err());
        assert!(decode_mbm(&data[..table]).is_err());
    }
}
