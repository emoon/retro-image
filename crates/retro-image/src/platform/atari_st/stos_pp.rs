//! STOS Picture Packer screens (`.PP1`, `.PP2`, `.PP3`, `.DAJ`) and plain
//! STOS packed screens (`.PAC`, `.SZ1`).
//!
//! Sources:
//! - Header layout, the three-stream compression (see
//!   `codec::stos_pictbank`) and the per-variant pixel orders: Deark `mbk.c`
//!   (<https://github.com/jsummers/deark>, MIT license, notice below), whose notes say the Picture Packer variants were worked
//!   out from sample files.
//! - Background: <https://snisurset.net/code/abydos/picturepacker.html> and
//!   <https://temlib.org/AtariForumWiki/index.php?title=Picture_Packer_file_format>
//!   (prose only: they describe the files but not the compression).
//!
//! The header is `06 07 19 63`, a resolution word (0 low, 1 medium, 2 high),
//! width in words at 10, height in lumps at 12, lines per lump at 16, the
//! offsets of the RLE and "points" streams at 20 and 24 (relative to the
//! start of the file), a 16-word ST palette at 38 and the picture stream at
//! 70. The pixel data is stored plane by plane; inside a plane it is cut into
//! lumps of `lines per lump` lines, each lump column by column. The resolution
//! word cannot tell the Picture Packer variants apart (medium res covers PP1,
//! PP2 and PP3), so the file extension chooses:
//! - PP1: 320x200, 16 colors, but only 2 planes are stored per 8 pixels:
//!   planes 0-1 of columns 0, 1, 4, 5, ... and planes 2-3 two columns on.
//! - PP2: ordinary medium resolution, 2 planes.
//! - PP3: 640x400 monochrome stored as 2 planes of 200 lines; the first half
//!   of each plane holds the even lines, the second half the odd lines, in
//!   32-pixel groups (8 pixels from each of 4 column/plane combinations).
//! - DAJ: resolution 0 means medium resolution with 4 planes stored; planes
//!   0-1 hold pixels `8k..8k+8` and planes 2-3 pixels `8k+16..8k+24` of each
//!   16-pixel column pair.
//!
//! Medium resolution lines are doubled, like the other ST decoders. A
//! monochrome picture is black on white or white on black depending on
//! whether palette entry 0 is black (Deark's rule).

// Parts of this file follow Deark's mbk.c
// (Deark, https://github.com/jsummers/deark):
//
// Copyright (C) 2016 Jason Summers
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

use super::common::{palette_words, st_palette};
use crate::bytes::{be16, be32};
use crate::codec::stos_pictbank;
use crate::image::check_size;
use crate::{DecodeError, Image};

const MAGIC: &[u8] = &[0x06, 0x07, 0x19, 0x63];
const PALETTE_AT: usize = 38;
const PICTURE_AT: usize = 70;

/// How the stored planes become pixels.
#[derive(Clone, Copy)]
enum Layout {
    /// `planes` planes of 8-pixel columns, one palette bit per plane.
    Standard {
        planes: usize,
    },
    Pp1,
    Pp3,
    /// 4 planes stored, 2 bits per pixel.
    Daj,
}

impl Layout {
    /// Planes stored in the file.
    fn stored_planes(self) -> usize {
        match self {
            Self::Standard { planes } => planes,
            Self::Pp1 | Self::Pp3 => 2,
            Self::Daj => 4,
        }
    }

    /// Bits per pixel of the output palette.
    fn bits(self) -> usize {
        match self {
            Self::Standard { planes } => planes,
            Self::Pp1 => 4,
            Self::Pp3 => 1,
            Self::Daj => 2,
        }
    }
}

/// What the extension says about a medium-resolution file.
#[derive(Clone, Copy, PartialEq)]
enum Variant {
    /// No hint: medium resolution is ambiguous, so it is refused.
    Plain,
    Pp1,
    Pp2,
    Pp3,
    Daj,
}

fn layout(variant: Variant, resolution: u16) -> Option<Layout> {
    Some(match (variant, resolution) {
        (Variant::Daj, 0) => Layout::Daj,
        (_, 0) => Layout::Standard { planes: 4 },
        (Variant::Pp1, 1) => Layout::Pp1,
        (Variant::Pp2 | Variant::Daj, 1) => Layout::Standard { planes: 2 },
        (Variant::Pp3, 1) => Layout::Pp3,
        (_, 2) => Layout::Standard { planes: 1 },
        _ => return None,
    })
}

pub(super) fn decode_pac(data: &[u8]) -> Result<Image, DecodeError> {
    decode(data, Variant::Plain)
}

pub(super) fn decode_pp1(data: &[u8]) -> Result<Image, DecodeError> {
    decode(data, Variant::Pp1)
}

pub(super) fn decode_pp2(data: &[u8]) -> Result<Image, DecodeError> {
    decode(data, Variant::Pp2)
}

pub(super) fn decode_pp3(data: &[u8]) -> Result<Image, DecodeError> {
    decode(data, Variant::Pp3)
}

pub(super) fn decode_daj(data: &[u8]) -> Result<Image, DecodeError> {
    decode(data, Variant::Daj)
}

fn decode(data: &[u8], variant: Variant) -> Result<Image, DecodeError> {
    const FAIL: DecodeError = DecodeError::Invalid;
    if data.get(..4) != Some(MAGIC) {
        return Err(FAIL);
    }
    let resolution = be16(data, 4).ok_or(FAIL)?;
    let layout = layout(variant, resolution).ok_or(FAIL)?;
    let field = |at| be16(data, at).map(usize::from).ok_or(FAIL);
    let (words, lumps, lump_lines) = (field(10)?, field(12)?, field(16)?);
    let rle_pos = be32(data, 20).ok_or(FAIL)? as usize;
    let points_pos = be32(data, 24).ok_or(FAIL)? as usize;
    if words == 0 || lumps == 0 || lump_lines == 0 {
        return Err(FAIL);
    }
    // PP1 and PP3 regroup columns in pairs of 16-pixel words.
    if matches!(layout, Layout::Pp1 | Layout::Pp3) && words % 2 != 0 {
        return Err(FAIL);
    }
    let row_len = words * 2;
    let stored_height = lumps * lump_lines;
    let (width, height) = match layout {
        Layout::Standard { .. } => (words * 16, stored_height),
        Layout::Pp1 => (words * 8, stored_height),
        Layout::Pp3 => (words * 16, stored_height * 2),
        Layout::Daj => (words * 32, stored_height),
    };
    check_size(width, height)?;
    let plane_len = row_len * stored_height;
    let packed = stos_pictbank::unpack(
        data,
        PICTURE_AT,
        rle_pos,
        points_pos,
        plane_len * layout.stored_planes(),
    )
    .ok_or(FAIL)?;

    let source = Source {
        data: &packed,
        row_len,
        plane_len,
        lump_lines,
    };
    let indices = match layout {
        Layout::Standard { planes } => source.standard(planes, width, height),
        Layout::Pp1 => source.pp1(width, height),
        Layout::Pp3 => source.pp3(width, height, words),
        Layout::Daj => source.daj(width, height),
    };
    let colors = 1 << layout.bits();
    let words = palette_words(data, PALETTE_AT, 16).ok_or(FAIL)?;
    let palette = if layout.bits() == 1 {
        if words[0] & 0xfff == 0 {
            alloc::vec![0x000000, 0xffffff]
        } else {
            alloc::vec![0xffffff, 0x000000]
        }
    } else {
        st_palette(&words[..colors])
    };
    let image = Image::from_indexed(width as u32, height as u32, &indices, &palette)?;
    let medium = matches!(layout, Layout::Standard { planes: 2 } | Layout::Daj);
    if medium {
        image.scaled(1, 2)
    } else {
        Ok(image)
    }
}

/// The unpacked planes: `plane_len` bytes per plane, each cut into lumps of
/// `lump_lines` lines of `row_len` bytes, stored column by column.
struct Source<'a> {
    data: &'a [u8],
    row_len: usize,
    plane_len: usize,
    lump_lines: usize,
}

impl Source<'_> {
    /// Byte of `plane` for 8-pixel column `col` on output line `y` of a
    /// picture whose stored lines are numbered `y`.
    fn byte(&self, plane: usize, col: usize, y: usize) -> u8 {
        let (lump, line) = (y / self.lump_lines, y % self.lump_lines);
        let offset = lump * self.row_len * self.lump_lines + col * self.lump_lines + line;
        self.data[plane * self.plane_len + offset]
    }

    fn standard(&self, planes: usize, width: usize, height: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                let value = (0..planes).fold(0, |v, p| {
                    v | (self.byte(p, x / 8, y) >> (7 - x % 8) & 1) << p
                });
                out.push(value);
            }
        }
        out
    }

    fn pp1(&self, width: usize, height: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                // Stored columns come in groups of 4: pixel columns 0 and 1
                // of a group are 8-pixel runs of planes 0-1, with planes 2-3
                // two stored columns on.
                let col = x / 8 + x / 8 / 2 * 2;
                let bit = 7 - x % 8;
                let value = (0..4).fold(0, |v, p| {
                    let source_col = col + if p >= 2 { 2 } else { 0 };
                    v | (self.byte(p % 2, source_col, y) >> bit & 1) << p
                });
                out.push(value);
            }
        }
        out
    }

    fn pp3(&self, width: usize, height: usize, words: usize) -> Vec<u8> {
        let mut out = alloc::vec![0; width * height];
        for y in 0..height {
            let (odd, line) = (y % 2, y / 2);
            for x in 0..width {
                // 32-pixel groups: bytes 0-1 come from plane 0 columns
                // `2g` and `2g + 1`, bytes 2-3 from plane 1.
                let group = x / 32 + odd * (words / 2);
                let part = x / 8 % 4;
                let byte = self.byte(part / 2, group * 2 + part % 2, line);
                out[y * width + x] = byte >> (7 - x % 8) & 1;
            }
        }
        out
    }

    fn daj(&self, width: usize, height: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                // Column pairs cover 32 pixels: column `c` gives 8 pixels
                // at its own position and 8 more 16 pixels on.
                let (set, pos) = (x % 32 / 16, x % 16);
                let col = x / 32 * 2 + pos / 8;
                let bit = 7 - x % 8;
                let value = (0..2).fold(0, |v, p| {
                    v | (self.byte(set * 2 + p, col, y) >> bit & 1) << p
                });
                out.push(value);
            }
        }
        out
    }
}
