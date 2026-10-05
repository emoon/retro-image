//! OS/2 icons and pointers: `IC` and `PT` (two-color), `CI` and `CP` (color),
//! alone or inside a `BA` bitmap array.
//!
//! Sources:
//! - Deark `os2bmp.c` (<https://github.com/jsummers/deark>, MIT license): the
//!   layout and the meaning of the masks, also used as the oracle.
//! - Just Solve the File Format Problem, "OS/2 Icon", "OS/2 Pointer" and "OS/2
//!   Bitmap Array": <http://justsolve.archiveteam.org/wiki/OS/2_Icon> (CC0;
//!   the markers, the BA container, and the OS/2 Presentation Manager
//!   Programming Reference as the specification).
//! - Reverse engineered from 22 sample files (Sembiance's `icoOS2` and
//!   `os2Pointer` folders) and 15 icons from the Hobbes OS/2 archive CD
//!   (<http://cd.textfiles.com/hobbesos29804/disk1/ICONS/>).
//!
//! A bitmap record is a 14-byte file header (marker, size, hotspot, and the
//! offset of the pixels from the start of the file), a bitmap header (12-byte
//! OS/2 1.x core header or a 40 or 64-byte one) and the palette (3-byte
//! entries after a core header, 4-byte ones after the others). Rows are
//! padded to 4 bytes and stored bottom-up. `IC` and `PT` hold one two-color
//! bitmap of double height; `CI` and `CP` hold that bitmap as the mask, then
//! the color bitmap. In a mask the upper half is the AND mask and the lower
//! half the XOR mask. A pixel with AND clear shows its color (black or white
//! for `IC` and `PT`: XOR clear is black); with AND set, XOR clear leaves the
//! screen alone (transparent) and XOR set inverts it.
//!
//! `Image` has no alpha: transparent pixels are drawn as `TRANSPARENT_FILL`
//! and inverting ones as its inverse. A `BA` array shows its largest picture
//! (the first of equal size); an array entry that is a plain OS/2 bitmap
//! (`BM`) is not read. Hotspots are ignored.
//!
//! Verification: no RECOIL oracle for this format; output matches Deark's PNG
//! output (alpha composited onto the same fill) on the sample files.

use alloc::vec::Vec;

use crate::bytes::{le16, le32};
use crate::image::{TRANSPARENT_FILL, check_size};
use crate::{DecodeError, Image};

const FAIL: DecodeError = DecodeError::Unrecognized;
const FILE_HEADER_LEN: usize = 14;
/// `BA` entries followed in one array: far more than any icon file holds.
const MAX_ENTRIES: usize = 64;
const BLACK: u32 = 0x000000;
const WHITE: u32 = 0xffffff;
const INVERTED_FILL: u32 = TRANSPARENT_FILL ^ 0xff_ffff;

/// One bitmap of a record: its geometry, palette and pixel rows.
struct Bitmap<'a> {
    width: usize,
    height: usize,
    bits_per_pixel: usize,
    palette: Vec<u32>,
    rows: &'a [u8],
    row_len: usize,
    /// Where the next record starts, after this one's headers and palette.
    end: usize,
}

impl Bitmap<'_> {
    /// Pixel `x` of row `row` of the file (the bottom row first): a palette
    /// index, or the color itself for 24-bit pixels.
    fn value(&self, x: usize, row: usize) -> u32 {
        let line = &self.rows[row * self.row_len..][..self.row_len];
        match self.bits_per_pixel {
            24 => u32::from_be_bytes([0, line[x * 3 + 2], line[x * 3 + 1], line[x * 3]]),
            bpp => {
                let shift = 8 - bpp - (x * bpp) % 8;
                u32::from(line[x * bpp / 8] >> shift) & ((1 << bpp) - 1)
            }
        }
    }

    /// The color of a pixel.
    fn color(&self, x: usize, row: usize) -> u32 {
        let value = self.value(x, row);
        match self.bits_per_pixel {
            24 => value,
            _ => self.palette.get(value as usize).copied().unwrap_or(BLACK),
        }
    }
}

/// Parses the record at `at`, which must start with `marker`.
fn parse_bitmap<'a>(data: &'a [u8], at: usize, marker: [u8; 2]) -> Result<Bitmap<'a>, DecodeError> {
    if !data.get(at..).is_some_and(|rest| rest.starts_with(&marker)) {
        return Err(FAIL);
    }
    let pixels_at = le32(data, at + 10).ok_or(FAIL)? as usize;
    let info = at + FILE_HEADER_LEN;
    let header_len = le32(data, info).ok_or(FAIL)? as usize;
    let word = |offset: usize| le16(data, info + offset).map(usize::from).ok_or(FAIL);
    let long = |offset: usize| le32(data, info + offset).map(|v| v as usize).ok_or(FAIL);
    let (width, height, planes, bits_per_pixel, entry_len, colors_used) = match header_len {
        12 => (word(4)?, word(6)?, word(8)?, word(10)?, 3, 0),
        40 | 64 => {
            // Uncompressed only.
            if long(16)? != 0 {
                return Err(FAIL);
            }
            (long(4)?, long(8)?, word(12)?, word(14)?, 4, long(32)?)
        }
        _ => return Err(FAIL),
    };
    if planes != 1 || !matches!(bits_per_pixel, 1 | 4 | 8 | 24) {
        return Err(FAIL);
    }
    check_size(width, height)?;
    let palette_len = match (bits_per_pixel, colors_used) {
        (24, _) => 0,
        (bpp, 0) => 1 << bpp,
        (bpp, used) => used.min(1 << bpp),
    };
    let palette_at = info + header_len;
    let end = palette_at + palette_len * entry_len;
    let palette = data
        .get(palette_at..end)
        .ok_or(FAIL)?
        .chunks_exact(entry_len)
        .map(|e| u32::from_be_bytes([0, e[2], e[1], e[0]]))
        .collect();
    let row_len = (width * bits_per_pixel).div_ceil(32) * 4;
    let rows = pixels_at
        .checked_add(row_len * height)
        .and_then(|end| data.get(pixels_at..end))
        .ok_or(FAIL)?;
    Ok(Bitmap {
        width,
        height,
        bits_per_pixel,
        palette,
        rows,
        row_len,
        end,
    })
}

/// A single `IC`, `CI`, `PT` or `CP` record starting at `at`.
fn decode_record(data: &[u8], at: usize) -> Result<Image, DecodeError> {
    let (marker, colored) = match data.get(at..at + 2) {
        Some(b"IC") => (*b"IC", false),
        Some(b"PT") => (*b"PT", false),
        Some(b"CI") => (*b"CI", true),
        Some(b"CP") => (*b"CP", true),
        _ => return Err(FAIL),
    };
    let mask = parse_bitmap(data, at, marker)?;
    if mask.bits_per_pixel != 1 || mask.height % 2 != 0 {
        return Err(FAIL);
    }
    let (width, height) = (mask.width, mask.height / 2);
    let picture = if colored {
        let picture = parse_bitmap(data, mask.end, marker)?;
        if (picture.width, picture.height) != (width, height) {
            return Err(FAIL);
        }
        Some(picture)
    } else {
        None
    };
    let color = (0..height).flat_map(|y| (0..width).map(move |x| (x, y)));
    Ok(Image::from_colors(
        width as u32,
        height as u32,
        color.map(|(x, y)| {
            // Rows are stored bottom-up; the AND mask is the upper half of
            // the mask picture, so it comes last in the file.
            let and = mask.value(x, 2 * height - 1 - y) != 0;
            let xor = mask.value(x, height - 1 - y) != 0;
            match (and, xor, &picture) {
                (false, _, Some(picture)) => picture.color(x, height - 1 - y),
                (false, xor, None) => [BLACK, WHITE][usize::from(xor)],
                (true, false, _) => TRANSPARENT_FILL,
                (true, true, _) => INVERTED_FILL,
            }
        }),
    ))
}

pub(super) fn decode_os2_icon(data: &[u8]) -> Result<Image, DecodeError> {
    if !data.starts_with(b"BA") {
        return decode_record(data, 0);
    }
    // A chain of array headers, each followed by one record; the offset of
    // the next header is absolute and zero in the last one.
    let mut best: Option<Image> = None;
    let mut at = 0;
    for _ in 0..MAX_ENTRIES {
        // A chain that runs off into something else ends the array.
        if !data.get(at..).is_some_and(|rest| rest.starts_with(b"BA")) {
            break;
        }
        if let Ok(image) = decode_record(data, at + 14) {
            let area = |i: &Image| i.width() * i.height();
            if best.as_ref().is_none_or(|b| area(&image) > area(b)) {
                best = Some(image);
            }
        }
        let next = le32(data, at + 6).ok_or(FAIL)? as usize;
        if next <= at {
            break;
        }
        at = next;
    }
    best.ok_or(FAIL)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 2x1 `IC` record: the mask is 2x2 bits in 4-byte rows, XOR row first.
    fn icon(xor: u8, and: u8) -> Vec<u8> {
        let mut file = b"IC".to_vec();
        file.extend_from_slice(&[0; 8]);
        file.extend_from_slice(&32u32.to_le_bytes()); // pixels at 32
        file.extend_from_slice(&12u32.to_le_bytes());
        file.extend_from_slice(&[2, 0, 2, 0, 1, 0, 1, 0]);
        file.extend_from_slice(&[0, 0, 0, 255, 255, 255]);
        file.extend_from_slice(&[xor, 0, 0, 0, and, 0, 0, 0]);
        file
    }

    #[test]
    fn mask_halves_choose_black_white_transparent_and_inverse() {
        // Left pixel: AND 0, XOR 1 is white. Right: AND 1, XOR 0 is transparent.
        let image = decode_os2_icon(&icon(0b1000_0000, 0b0100_0000)).unwrap();
        assert_eq!((image.width(), image.height()), (2, 1));
        assert_eq!(
            [image.get(0, 0), image.get(1, 0)],
            [WHITE, TRANSPARENT_FILL]
        );
        let inverted = decode_os2_icon(&icon(0b0100_0000, 0b0100_0000)).unwrap();
        assert_eq!(inverted.get(1, 0), INVERTED_FILL);
        assert_eq!(inverted.get(0, 0), BLACK);
    }

    #[test]
    fn an_array_wraps_a_record_and_a_broken_chain_ends_it() {
        let mut array = b"BA".to_vec();
        array.extend_from_slice(&40u32.to_le_bytes());
        array.extend_from_slice(&[0; 8]);
        // Pixel offsets are absolute: the record moves by 14 bytes.
        let mut record = icon(0, 0);
        record[10..14].copy_from_slice(&(32u32 + 14).to_le_bytes());
        array.extend_from_slice(&record);
        assert_eq!(decode_os2_icon(&array).unwrap().width(), 2);
        // The next array header points at something else: the first record stays.
        array[6..10].copy_from_slice(&20u32.to_le_bytes());
        assert_eq!(decode_os2_icon(&array).unwrap().width(), 2);
        // A first record that is no icon leaves nothing to show.
        array[14] = b'X';
        assert!(decode_os2_icon(&array).is_err());
    }

    #[test]
    fn truncated_records_are_rejected() {
        let record = icon(0, 0);
        assert!(decode_os2_icon(&record[..record.len() - 1]).is_err());
        assert!(decode_os2_icon(b"IC").is_err());
    }
}
