//! Dr. Halo `PIC` pictures (not the `CUT` format of `halo.rs`).
//!
//! Sources:
//! - Just Solve the File Format Problem, Dr. Halo PIC:
//!   <http://fileformats.archiveteam.org/wiki/Dr._Halo_PIC> (CC0; "partly
//!   reverse engineered": `AH` signature, version byte 2, graphics board and
//!   mode, one run-length stream per plane aligned to 512-byte blocks).
//! - Deark `drhalo.c`, `de_run_drhalopic` (<https://github.com/jsummers/deark>,
//!   MIT license): the table of board and mode combinations with their
//!   dimensions and header sizes, the CGA palette choice, the block rules and
//!   the de-interlacing of CGA and Hercules screens. Also the oracle for the
//!   sample files.
//!
//! Each plane is a stream of control bytes: `0x00` ends the plane (and skips to
//! the next 512-byte boundary of the file), `0x80` skips to the boundary and
//! goes on, a byte with bit 7 set repeats the next byte `control & 0x7f`
//! times, any other byte is followed by that many literals.
//!
//! The corpus has four CGA 320x200 samples with four colors (board `0x01`)
//! and twenty EGA 640x350 samples (board `0x15`, mode 4, four planes, default
//! 16-color EGA palette). The other rows of the mode table (Hercules, VGA,
//! the remaining EGA modes) follow Deark and are not tested against any file.
//! A separate `PAL` file is not read. Modes outside the table are rejected.
//!
//! Verification: no RECOIL oracle for this format; the CGA output was compared
//! pixel for pixel with Deark's PNG output, the EGA output was checked by eye.

// Parts of this file follow Deark's modules/drhalo.c
// (Deark, https://github.com/jsummers/deark):
//
// Copyright (C) 2017 Jason Summers
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

use super::cga::{BANK_STRIDE, deinterlace};
use super::{CGA_PALETTE, cga_set};
use crate::bytes::le16;
use crate::image::planar_pixels;
use crate::{BitOrder, DecodeError, Image};

const FAIL: DecodeError = DecodeError::Invalid;
const BLOCK: usize = 512;
const HERCULES: u8 = 0x07;

#[derive(Clone, Copy, PartialEq)]
enum Interlace {
    None,
    /// Even rows, then odd rows from 8192 bytes on, 80 bytes per row.
    Cga,
    /// Rows `4k + n` in the 8192-byte bank `n`, 90 bytes per row.
    Hercules,
}

/// One board and video mode combination.
struct Mode {
    board: u8,
    mode: u16,
    width: usize,
    height: usize,
    planes: usize,
    bits_per_pixel: usize,
    /// Header bytes before the first plane.
    header: usize,
    /// Bytes each plane occupies once unpacked.
    plane_len: usize,
    interlace: Interlace,
}

const fn mode(
    board: u8,
    mode: u16,
    (width, height): (usize, usize),
    (planes, bits_per_pixel): (usize, usize),
    header: usize,
    plane_len: usize,
    interlace: Interlace,
) -> Mode {
    Mode {
        board,
        mode,
        width,
        height,
        planes,
        bits_per_pixel,
        header,
        plane_len,
        interlace,
    }
}

use Interlace::{Cga, Hercules as Herc, None as Flat};

static MODES: [Mode; 16] = [
    mode(0x01, 0, (320, 200), (1, 2), 16, 16384, Cga),
    mode(0x01, 1, (640, 200), (1, 1), 16, 16384, Cga),
    mode(0x07, 0, (720, 348), (1, 1), 10, 32768, Herc),
    mode(0x15, 2, (320, 200), (4, 1), 12, 8192, Flat),
    mode(0x15, 3, (640, 200), (4, 1), 12, 16384, Flat),
    mode(0x15, 4, (640, 350), (4, 1), 12, 28000, Flat),
    mode(0x15, 5, (640, 800), (4, 1), 12, 64000, Flat),
    mode(0x15, 10, (640, 350), (4, 1), 12, 28000, Flat),
    mode(0x3c, 0, (320, 200), (1, 2), 16, 16384, Cga),
    mode(0x3c, 1, (640, 200), (1, 1), 16, 16384, Cga),
    mode(0x3c, 2, (640, 480), (1, 1), 16, 40960, Flat),
    mode(0x3c, 3, (320, 200), (1, 8), 16, 65536, Flat),
    mode(0x47, 4, (320, 200), (4, 1), 12, 8192, Flat),
    mode(0x47, 5, (640, 200), (4, 1), 12, 16384, Flat),
    mode(0x47, 6, (640, 350), (4, 1), 12, 28000, Flat),
    mode(0x47, 7, (640, 480), (4, 1), 12, 38400, Flat),
];

pub(super) fn decode_pic(data: &[u8]) -> Result<Image, DecodeError> {
    if data.get(..2) != Some(b"AH") || data.get(6) != Some(&2) {
        return Err(FAIL);
    }
    let board = *data.get(7).ok_or(FAIL)?;
    let file_mode = if board == HERCULES {
        0
    } else {
        le16(data, 10).ok_or(FAIL)?
    };
    // Header sizes of 12 and more carry the mode; the Hercules header doesn't.
    let found = MODES
        .iter()
        .find(|m| m.board == board && (m.header < 12 || m.mode == file_mode))
        .ok_or(FAIL)?;
    if data.len() < found.header {
        return Err(FAIL);
    }
    let mut planes = Vec::with_capacity(found.plane_len * found.planes);
    let mut pos = found.header;
    for _ in 0..found.planes {
        pos = unpack_plane(data, pos, found.plane_len, &mut planes).ok_or(FAIL)?;
    }
    let pixels = match found.interlace {
        Interlace::None => planes,
        Interlace::Cga => deinterlace(&planes, 200, 80, 2, BANK_STRIDE),
        Interlace::Hercules => deinterlace(&planes, 348, 90, 4, BANK_STRIDE),
    };
    let row_len = found.width * found.bits_per_pixel / 8;
    let (width, height) = (found.width as u32, found.height as u32);
    if found.planes == 4 {
        let values = planar_pixels(
            &pixels,
            found.width,
            found.height,
            row_len,
            4,
            |plane, y| plane * found.plane_len + y * row_len,
        )?;
        let indices: Vec<u8> = values.into_iter().map(|v| v as u8).collect();
        return Image::from_indexed(width, height, &indices, &CGA_PALETTE);
    }
    match found.bits_per_pixel {
        1 => Image::from_bits(
            width,
            height,
            &pixels,
            row_len,
            BitOrder::MsbFirst,
            [0x000000, 0xffffff],
        ),
        2 => {
            let palette = cga_palette(data, found.header);
            let indices: Vec<u8> = pixels
                .iter()
                .flat_map(|b| (0..4).map(move |i| b >> (6 - 2 * i) & 3))
                .collect();
            Image::from_indexed(
                width,
                height,
                &indices[..found.width * found.height],
                &palette,
            )
        }
        _ => Image::from_indexed(
            width,
            height,
            &pixels[..found.width * found.height],
            &RGB_332,
        ),
    }
}

/// Appends one plane of exactly `len` bytes (cut or zero-padded) decoded from
/// `data` at `pos`; returns where the next plane starts.
fn unpack_plane(data: &[u8], mut pos: usize, len: usize, out: &mut Vec<u8>) -> Option<usize> {
    let start = out.len();
    let room = |out: &Vec<u8>| (start + len).saturating_sub(out.len());
    let to_block = |pos: usize| pos.next_multiple_of(BLOCK);
    while pos < data.len() {
        let control = data[pos];
        pos += 1;
        match control {
            0x00 => {
                pos = to_block(pos);
                break;
            }
            0x80 => pos = to_block(pos),
            _ if control & 0x80 != 0 => {
                let value = *data.get(pos)?;
                pos += 1;
                let count = usize::from(control & 0x7f).min(room(out));
                out.resize(out.len() + count, value);
            }
            _ => {
                let count = usize::from(control);
                let literal = data.get(pos..pos + count)?;
                pos += count;
                out.extend_from_slice(&literal[..count.min(room(out))]);
            }
        }
    }
    out.resize(start + len, 0);
    Some(pos)
}

/// The CGA palette from header bytes 12 and 14: bit 4 of the first selects
/// high intensity, bit 0 of the second the cyan/magenta set, and the low
/// nibble of the first is the background color.
fn cga_palette(data: &[u8], header: usize) -> [u32; 4] {
    let (b12, b14) = if header >= 16 {
        (
            data.get(12).copied().unwrap_or(0),
            data.get(14).copied().unwrap_or(0),
        )
    } else {
        (0, 0)
    };
    let set = match (b12 & 0x10 != 0, b14 & 1 != 0) {
        (true, true) => cga_set([11, 13, 15]),
        (false, true) => cga_set([3, 5, 7]),
        (true, false) => cga_set([10, 12, 14]),
        (false, false) => cga_set([2, 4, 6]),
    };
    let mut palette = set;
    palette[0] = CGA_PALETTE[usize::from(b12 & 15)];
    palette
}

/// The default 256-color palette, a whiteless RGB 3-3-2: red and green take
/// eight levels, blue four.
static RGB_332: [u32; 256] = {
    const LEVELS_8: [u32; 8] = [0, 35, 67, 99, 131, 163, 195, 227];
    const LEVELS_4: [u32; 4] = [0, 67, 131, 195];
    let mut table = [0; 256];
    let mut i = 0;
    while i < 256 {
        table[i] = LEVELS_8[i % 8] << 16 | LEVELS_8[i % 64 / 8] << 8 | LEVELS_4[i / 64];
        i += 1;
    }
    table
};

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn plane_ends_at_a_zero_control_byte_and_pads_to_the_block() {
        // Run of 3 x 0xaa, literal 0x01, end; the next plane starts at 512.
        let mut data = vec![0u8; 520];
        data[..7].copy_from_slice(&[0x83, 0xaa, 1, 0x55, 0, 0, 0]);
        let mut out = Vec::new();
        assert_eq!(unpack_plane(&data, 0, 6, &mut out), Some(BLOCK));
        assert_eq!(out, [0xaa, 0xaa, 0xaa, 0x55, 0, 0]);
    }

    #[test]
    fn a_truncated_literal_is_rejected() {
        let mut out = Vec::new();
        assert_eq!(unpack_plane(&[5, 1, 2], 0, 8, &mut out), None);
    }

    #[test]
    fn default_256_palette_matches_the_documented_levels() {
        assert_eq!(RGB_332[0], 0);
        assert_eq!(RGB_332[255], 227 << 16 | 227 << 8 | 195);
    }
}
