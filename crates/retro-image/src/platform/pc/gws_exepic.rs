//! Graphic Workshop self-displaying pictures (`.EXE`), made by Alchemy
//! Mindworks' Graphic Workshop.
//!
//! Sources:
//! - Deark `gws.c` and `fmtutil-exe.c` (<https://github.com/jsummers/deark>,
//!   MIT license), which Deark itself calls a best guess at the image
//!   position. The EXE header has no relocations, a stack pointer of 200h, its
//!   relocation table at 34 and an entry point at the start of the code; the
//!   string `GraphicWorkshop` or `GWS/Windows` sits 29 bytes into the code.
//!   Counted from the start of the code: a 16-bit picture offset (rounded up
//!   to a multiple of 16) at 9, then width, height, bytes per row per plane,
//!   depth (1 to 8 bits) and compression (1 stored, 2 run-length) as 16-bit
//!   values, and a palette of `1 << depth` entries of 8-bit red, green and
//!   blue at 54. Depths up to 4 are bit planes, stored one row after another
//!   with the lowest plane first; larger depths are one byte per pixel. The
//!   run-length coding is that of PCX (`pcx.rs`): a byte below `C0h` is
//!   itself, otherwise the low six bits repeat the next byte.
//! - Checked on 14 sample files (Sembiance's `graphicWorkshopSelfDisplayingImage`
//!   folder).
//!
//! No extension is claimed: `.com` and `.exe` belong to every DOS program, and
//! the shared MIME package would send them all to the image viewer. The check
//! is strict enough to find these files by content under any name.
//!
//! Verification: no RECOIL oracle for this format; output matches Deark's
//! `gws_exepic` module pixel for pixel on the sample files.

// Parts of this file follow Deark's modules/gws.c and src/fmtutil-exe.c
// (Deark, https://github.com/jsummers/deark):
//
// Copyright (C) 2024 Jason Summers
// <jason1@pobox.com>
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

use super::pcx::unpack;
use crate::bytes::le16;
use crate::image::{check_size, planar_pixels};
use crate::{DecodeError, Image};

const FAIL: DecodeError = DecodeError::Unrecognized;
const STORED: usize = 1;
const RUN_LENGTH: usize = 2;
const PALETTE_AT: usize = 54;

/// The values the header must hold, and the strings that name the program.
fn is_gws_exe(data: &[u8], code: usize) -> bool {
    let word = |at| le16(data, at);
    let marker = data.get(code + 29..);
    data.starts_with(b"MZ")
        && word(6) == Some(0) // no relocations
        && word(16) == Some(0x200) // stack pointer
        && word(24) == Some(34) // relocation table
        && word(20) == Some(0) // entry point: the first byte of the code
        && word(22) == Some(0)
        && marker.is_some_and(|m| m.starts_with(b"GraphicWorkshop") || m.starts_with(b"GWS/Windows"))
}

pub(super) fn decode_gws_exepic(data: &[u8]) -> Result<Image, DecodeError> {
    let code = usize::from(le16(data, 8).ok_or(FAIL)?) * 16;
    if !is_gws_exe(data, code) {
        return Err(FAIL);
    }
    let word = |at: usize| le16(data, code + at).map(usize::from).ok_or(FAIL);
    let picture = code + word(9)?.next_multiple_of(16);
    let (width, height, row_len, depth, compression) =
        (word(11)?, word(13)?, word(15)?, word(17)?, word(19)?);
    if !(1..=8).contains(&depth) || !matches!(compression, STORED | RUN_LENGTH) {
        return Err(FAIL);
    }
    check_size(width, height)?;
    let planes = if depth < 5 { depth } else { 1 };
    // A row holds `width` bytes, or `width` bits per plane, and at most one
    // pad byte (all 14 samples have none).
    let row_min = if depth > 4 { width } else { width.div_ceil(8) };
    if !(row_min..=row_min + 1).contains(&row_len) {
        return Err(FAIL);
    }
    // Expanding planes makes a value per bit of the padded rows.
    check_size(if depth > 4 { row_len } else { row_len * 8 }, height)?;
    let len = row_len * planes * height;
    let stored = data.get(picture..).ok_or(FAIL)?;
    let rows = if compression == RUN_LENGTH {
        unpack(stored, len)?.0
    } else {
        stored.get(..len).ok_or(FAIL)?.to_vec()
    };

    let palette_at = code + PALETTE_AT;
    let mut palette: Vec<u32> = data
        .get(palette_at..palette_at + (3 << depth))
        .ok_or(FAIL)?
        .as_chunks::<3>()
        .0
        .iter()
        .map(|c| u32::from(c[0]) << 16 | u32::from(c[1]) << 8 | u32::from(c[2]))
        .collect();
    palette.resize(256, 0);
    let pixels: Vec<u8> = if depth > 4 {
        rows.chunks_exact(row_len)
            .flat_map(|row| row[..width].iter().copied())
            .collect()
    } else {
        planar_pixels(&rows, width, height, row_len, planes, |plane, y| {
            (y * planes + plane) * row_len
        })?
        .into_iter()
        .map(|v| v as u8)
        .collect()
    };
    Image::from_indexed(width as u32, height as u32, &pixels, &palette)
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;

    /// A one-row picture behind the header, palette entries 1 to 3 set to
    /// grays 1 to 3, the picture 16-byte aligned after the palette.
    fn exe(width: u16, depth: u16, compression: u16, picture: &[u8]) -> Vec<u8> {
        let row_len = if depth > 4 { width } else { width.div_ceil(8) };
        exe_with_rows(width, 1, row_len, depth, compression, picture)
    }

    fn exe_with_rows(
        width: u16,
        height: u16,
        row_len: u16,
        depth: u16,
        compression: u16,
        picture: &[u8],
    ) -> Vec<u8> {
        let code = 32;
        let picture_at = (PALETTE_AT + (3 << depth)).next_multiple_of(16);
        let mut file = vec![0u8; code + picture_at];
        file[..2].copy_from_slice(b"MZ");
        file[8..10].copy_from_slice(&2u16.to_le_bytes());
        file[16..18].copy_from_slice(&0x200u16.to_le_bytes());
        file[24..26].copy_from_slice(&34u16.to_le_bytes());
        file[code + 29..code + 44].copy_from_slice(b"GraphicWorkshop");
        for (at, value) in [
            (9, picture_at as u16),
            (11, width),
            (13, height),
            (15, row_len),
            (17, depth),
            (19, compression),
        ] {
            file[code + at..code + at + 2].copy_from_slice(&value.to_le_bytes());
        }
        for gray in 1..(1usize << depth).min(4) {
            file[code + PALETTE_AT + gray * 3..][..3].fill(gray as u8);
        }
        file.extend_from_slice(picture);
        file
    }

    #[test]
    fn eight_bit_pictures_are_one_byte_per_pixel_and_may_be_run_length_coded() {
        let file = exe(2, 8, STORED as u16, &[1, 3]);
        let stored = decode_gws_exepic(&file).unwrap();
        assert_eq!([stored.get(0, 0), stored.get(1, 0)], [0x010101, 0x030303]);
        assert_eq!(crate::decode("picture.exe", &file), Ok(stored));
        // C2 01: two pixels of color 1.
        let packed = decode_gws_exepic(&exe(2, 8, RUN_LENGTH as u16, &[0xc2, 1])).unwrap();
        assert_eq!([packed.get(0, 0), packed.get(1, 0)], [0x010101; 2]);
    }

    #[test]
    fn planes_follow_each_other_in_a_row_with_the_lowest_first() {
        // Plane 0 bits 1010, plane 1 bits 0110: colors 1, 2, 3, 0.
        let picture = [0b1010_0000, 0b0110_0000];
        let image = decode_gws_exepic(&exe(4, 2, STORED as u16, &picture)).unwrap();
        let colors: Vec<u32> = (0..4).map(|x| image.get(x, 0)).collect();
        assert_eq!(colors, [0x010101, 0x020202, 0x030303, 0]);
    }

    #[test]
    fn rows_far_wider_than_the_picture_are_rejected_before_expanding_them() {
        // 8 pixels wide, but 129-byte rows of 1-bit pixels, 65535 rows: the
        // stored raster passes the size cap, the 1032-pixel rows expanded to
        // a value each (about 270 MB) would not.
        let picture = vec![0u8; 129 * 65535];
        let file = exe_with_rows(8, 65535, 129, 1, STORED as u16, &picture);
        assert!(decode_gws_exepic(&file).is_err());
    }

    #[test]
    fn the_marker_string_is_required() {
        let mut file = exe(2, 8, STORED as u16, &[1, 0]);
        file[32 + 29] = b'X';
        assert!(decode_gws_exepic(&file).is_err());
    }
}
