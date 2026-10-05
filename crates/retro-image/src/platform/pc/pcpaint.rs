//! PCPaint and PICtor `PIC` pictures and `CLP` clips.
//!
//! Sources:
//! - Encyclopedia of Graphics File Formats, "Pictor PC Paint":
//!   <https://www.fileformat.info/format/pictor/egff.htm> (17-byte header with
//!   the plane and palette descriptors, then a palette block, a block count and
//!   blocks of run-length data; CLP clips are a 16-bit file length, size, and
//!   one packed or raw block).
//! - Just Solve the File Format Problem, PCPaint PIC and CLP:
//!   <http://fileformats.archiveteam.org/wiki/PCPaint_PIC>,
//!   <http://fileformats.archiveteam.org/wiki/PCPaint_CLP> (CC0).
//! - Deark `pcpaint.c` and `fmtutil-rle.c` (<https://github.com/jsummers/deark>,
//!   MIT license) for the palette descriptor types, the CGA palette selection,
//!   the 8-bit palette heuristic and the exact block rules. It is also the
//!   oracle for the sample files (all 25 in the corpus).
//!
//! Decoded: bilevel, 4-color CGA (2 bits per pixel), 16-color EGA/VGA
//! (nibbles or 4 planes), 4-color planar and 256-color pictures, with the
//! palette descriptors 0 (default), 1 (CGA code), 2 and 3 (indices into the
//! 16 and 64 color EGA palettes), 4 and 5 (RGB). Rows are stored bottom-up.
//! Text-mode pictures (video modes `0` to `3`: rows of character and
//! attribute bytes, width counted in bytes) are drawn with the text-mode art
//! fonts; the one sample, `WSSCREEN.PIC`, is an 80x60 screen. 24-bit variants
//! and OVR files (containers of CLP tables) are rejected. CLP files carry no palette, so they use the defaults.
//!
//! The default VGA palette is taken from Deark, whose notice is:
//!
//! ```text
//! Copyright (C) 2016-2026 Jason Summers
//! <jason1@pobox.com>
//!
//! Permission is hereby granted, free of charge, to any person obtaining a copy
//! of this software and associated documentation files (the "Software"), to deal
//! in the Software without restriction, including without limitation the rights
//! to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
//! copies of the Software, and to permit persons to whom the Software is
//! furnished to do so, subject to the following conditions:
//!
//! The above copyright notice and this permission notice shall be included in
//! all copies or substantial portions of the Software.
//!
//! THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//! IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//! FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
//! AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//! LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//! OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
//! THE SOFTWARE.
//! ```
//!
//! Verification: no RECOIL oracle for this format; output was compared pixel
//! for pixel with Deark's PNG output on the sample files.

use alloc::vec::Vec;

use super::super::textmode;
use super::{CGA_PALETTE, cga_set, dac_rounded, ega_64};
use crate::bytes::le16;
use crate::image::{check_size, planar_pixels};
use crate::{DecodeError, Image};

const FAIL: DecodeError = DecodeError::Unrecognized;
const MAGIC: u16 = 0x1234;
/// Palette flag of a version 2 header.
const HAS_PALETTE_BLOCK: u8 = 0xff;
/// Bytes before the palette block, and the size of the version 1 block.
const PALETTE_AT: usize = 17;
const V1_PALETTE_LEN: usize = 10;

/// The CGA 4-color palettes selectable by the palette descriptor; any other
/// code falls back to the third.
pub(super) const CGA_4: [[u32; 4]; 6] = [
    cga_set([3, 5, 7]),
    cga_set([2, 4, 6]),
    cga_set([3, 4, 7]),
    cga_set([11, 13, 15]),
    cga_set([10, 12, 14]),
    cga_set([11, 12, 15]),
];

#[derive(Clone, Copy)]
enum Layout {
    /// Pixels of `bpp` bits packed most significant bit first.
    Packed(usize),
    /// Whole-image bitplanes, plane 0 holding the lowest bit.
    Planar(usize),
}

impl Layout {
    /// From the plane descriptor byte.
    fn from_plane_info(info: u8) -> Option<Self> {
        match info {
            0x01 => Some(Self::Packed(1)),
            0x02 => Some(Self::Packed(2)),
            0x04 => Some(Self::Packed(4)),
            0x08 => Some(Self::Packed(8)),
            0x11 => Some(Self::Planar(2)),
            0x31 => Some(Self::Planar(4)),
            _ => None,
        }
    }

    fn colors(self) -> usize {
        match self {
            Self::Packed(bpp) => 1 << bpp,
            Self::Planar(planes) => 1 << planes,
        }
    }

    fn row_len(self, width: usize) -> usize {
        match self {
            Self::Packed(bpp) => (width * bpp).div_ceil(8),
            Self::Planar(_) => width.div_ceil(8),
        }
    }

    /// Bytes of unpacked pixel data.
    fn data_len(self, width: usize, height: usize) -> usize {
        let planes = match self {
            Self::Packed(_) => 1,
            Self::Planar(planes) => planes,
        };
        self.row_len(width) * height * planes
    }
}

/// How the palette block describes the colors.
struct PaletteInfo<'a> {
    /// The palette descriptor.
    kind: u16,
    block: &'a [u8],
    /// The video mode letter, 0 if unknown.
    video_mode: u8,
}

/// A clip has no palette block.
const NO_PALETTE: PaletteInfo = PaletteInfo {
    kind: 0,
    block: &[],
    video_mode: 0,
};

pub(super) fn decode_pic(data: &[u8]) -> Result<Image, DecodeError> {
    if le16(data, 0) != Some(MAGIC) {
        return Err(FAIL);
    }
    let plane_info = *data.get(10).ok_or(FAIL)?;
    let flag = *data.get(11).ok_or(FAIL)?;
    let (info, blocks_at) = if flag == HAS_PALETTE_BLOCK {
        let video_mode = *data.get(12).ok_or(FAIL)?;
        let size = usize::from(le16(data, 15).ok_or(FAIL)?);
        let block = data.get(PALETTE_AT..PALETTE_AT + size).ok_or(FAIL)?;
        let kind = le16(data, 13).ok_or(FAIL)?;
        let info = PaletteInfo {
            kind,
            block,
            video_mode,
        };
        (info, PALETTE_AT + size)
    } else if flag == 0 && plane_info == 1 {
        // Version 1: no palette information, a 10-byte block of unknown use.
        (NO_PALETTE, PALETTE_AT + V1_PALETTE_LEN)
    } else {
        return Err(FAIL);
    };
    let layout = Layout::from_plane_info(plane_info).ok_or(FAIL)?;
    let (width, height) = dimensions(data)?;
    let block_count = usize::from(le16(data, blocks_at).ok_or(FAIL)?);
    let body = data.get(blocks_at + 2..).ok_or(FAIL)?;
    let need = layout.data_len(width, height);
    let pixels = if block_count == 0 {
        body.get(..need).ok_or(FAIL)?.to_vec()
    } else {
        unpack_blocks(body, block_count, need).ok_or(FAIL)?
    };
    // Character-mode pictures (video modes '0' to '3') hold a text screen of
    // character and attribute bytes; the width is in bytes.
    if (b'0'..=b'3').contains(&info.video_mode) {
        if !matches!(layout, Layout::Packed(8)) || width % 2 != 0 {
            return Err(FAIL);
        }
        return textmode::render_text_screen(&pixels, width / 2);
    }
    render(&pixels, width, height, layout, &info)
}

pub(super) fn decode_clp(data: &[u8]) -> Result<Image, DecodeError> {
    // The only signature: the first word is the file length.
    if usize::from(le16(data, 0).ok_or(FAIL)?) != data.len() {
        return Err(FAIL);
    }
    let (width, height) = dimensions(data)?;
    let flag = *data.get(10).ok_or(FAIL)?;
    let compressed = flag == 0xff;
    let (plane_info, body_at) = if compressed {
        (*data.get(11).ok_or(FAIL)?, 13)
    } else {
        (flag, 11)
    };
    let layout = Layout::from_plane_info(plane_info).ok_or(FAIL)?;
    let need = layout.data_len(width, height);
    let body = data.get(body_at..).ok_or(FAIL)?;
    let pixels = if compressed {
        let marker = *data.get(12).ok_or(FAIL)?;
        let mut out = Vec::new();
        expand_run_length(body, marker, need, &mut out).ok_or(FAIL)?;
        out
    } else {
        body.get(..need).ok_or(FAIL)?.to_vec()
    };
    render(&pixels, width, height, layout, &NO_PALETTE)
}

fn dimensions(data: &[u8]) -> Result<(usize, usize), DecodeError> {
    let width = usize::from(le16(data, 2).ok_or(FAIL)?);
    let height = usize::from(le16(data, 4).ok_or(FAIL)?);
    check_size(width, height)?;
    Ok((width, height))
}

/// Joins the blocks (`u16` packed size including this 5-byte header, `u16`
/// unpacked size, run marker byte, data) into `need` bytes of pixel data.
pub(super) fn unpack_blocks(data: &[u8], count: usize, need: usize) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut pos = 0;
    for _ in 0..count {
        let packed = usize::from(le16(data, pos)?);
        let unpacked = usize::from(le16(data, pos + 2)?);
        let marker = *data.get(pos + 4)?;
        if packed < 5 {
            return None;
        }
        let block = data.get(pos + 5..pos + packed)?;
        if expand_run_length(block, marker, need, &mut out)? != unpacked {
            return None;
        }
        pos += packed;
    }
    (out.len() == need).then_some(out)
}

/// Appends the expansion of `block` to `out`, keeping at most `limit` bytes
/// in total, and returns how many bytes the block expands to. `marker count
/// value` is a run; a count of 0 is followed by a 16-bit count. Any other
/// byte is a literal.
fn expand_run_length(block: &[u8], marker: u8, limit: usize, out: &mut Vec<u8>) -> Option<usize> {
    let mut produced = 0;
    let mut pos = 0;
    while pos < block.len() {
        let byte = block[pos];
        pos += 1;
        let room = limit.saturating_sub(out.len());
        if byte == marker {
            let mut count = usize::from(*block.get(pos)?);
            pos += 1;
            if count == 0 {
                count = usize::from(le16(block, pos)?);
                pos += 2;
            }
            let value = *block.get(pos)?;
            pos += 1;
            out.resize(out.len() + count.min(room), value);
            produced += count;
        } else {
            if room > 0 {
                out.push(byte);
            }
            produced += 1;
        }
    }
    Some(produced)
}

/// Palette indices of the pixels, top row first (the file stores them
/// bottom-up). `pixels` holds `layout.data_len(width, height)` bytes.
fn indices(pixels: &[u8], width: usize, height: usize, layout: Layout) -> Vec<u8> {
    let row_len = layout.row_len(width);
    let stored: Vec<u8> = match layout {
        Layout::Packed(bpp) => {
            let mask = ((1u16 << bpp) - 1) as u8;
            let mut out = Vec::with_capacity(width * height);
            for row in pixels.chunks_exact(row_len) {
                out.extend((0..width).map(|x| {
                    let bit = x * bpp;
                    row[bit / 8] >> (8 - bpp - bit % 8) & mask
                }));
            }
            out
        }
        Layout::Planar(planes) => {
            let stride = row_len * height;
            planar_pixels(pixels, width, height, row_len, planes, |plane, y| {
                plane * stride + y * row_len
            })
            .into_iter()
            .map(|v| v as u8)
            .collect()
        }
    };
    stored
        .chunks_exact(width)
        .rev()
        .flatten()
        .copied()
        .collect()
}

fn render(
    pixels: &[u8],
    width: usize,
    height: usize,
    layout: Layout,
    info: &PaletteInfo,
) -> Result<Image, DecodeError> {
    if pixels.len() != layout.data_len(width, height) {
        return Err(FAIL);
    }
    let colors = layout.colors();
    let palette = palette(colors, info);
    let indices = indices(pixels, width, height, layout);
    Image::from_indexed(width as u32, height as u32, &indices, &palette[..colors])
}

/// The default palette for `colors`, overridden by what the descriptor says.
fn palette(colors: usize, info: &PaletteInfo) -> [u32; 256] {
    let mut pal = [0u32; 256];
    match colors {
        2 => pal[..2].copy_from_slice(&[0x000000, 0xffffff]),
        4 => pal[..4].copy_from_slice(&CGA_4[2]),
        16 => pal[..16].copy_from_slice(&CGA_PALETTE),
        _ => pal.copy_from_slice(&VGA_PALETTE),
    }
    let block = info.block;
    match info.kind {
        1 => {
            // CGA palette code and border color.
            let code = usize::from(block.first().copied().unwrap_or(0));
            let border = usize::from(block.get(1).copied().unwrap_or(0));
            pal[..4].copy_from_slice(CGA_4.get(code).unwrap_or(&CGA_4[2]));
            pal[0] = CGA_PALETTE.get(border).copied().unwrap_or(0);
        }
        2 => {
            for (slot, &index) in pal.iter_mut().zip(block).take(16) {
                *slot = CGA_PALETTE.get(usize::from(index)).copied().unwrap_or(0);
            }
        }
        3 => {
            for (slot, &index) in pal.iter_mut().zip(block).take(colors) {
                *slot = ega_64(index);
            }
        }
        4 | 5 => rgb_palette(&mut pal[..colors], block),
        _ => {}
    }
    // PCPaint drew its CGA and EGA 2-color modes in light gray.
    if colors == 2 && info.kind == 0 && matches!(info.video_mode, 0x43 | 0x45) {
        pal[1] = 0xaaaaaa;
    }
    pal
}

/// 3-byte RGB entries. Values up to 63 are VGA DAC values, scaled to 255
/// with rounding; if any entry exceeds 63 the whole table is 8-bit.
fn rgb_palette(pal: &mut [u32], block: &[u8]) {
    let entries = block.as_chunks::<3>().0;
    let wide = entries.iter().take(pal.len()).flatten().any(|&v| v > 63);
    let scale = |v: u8| {
        if wide {
            u32::from(v.rotate_left(2))
        } else {
            dac_rounded(v)
        }
    };
    for (slot, rgb) in pal.iter_mut().zip(entries) {
        *slot = scale(rgb[0]) << 16 | scale(rgb[1]) << 8 | scale(rgb[2]);
    }
}

/// The default VGA BIOS palette (mode 13h), as listed in Deark's standard
/// palettes (see the notice in the module documentation).
pub(super) const VGA_PALETTE: [u32; 256] = [
    0x000000, 0x0000aa, 0x00aa00, 0x00aaaa, 0xaa0000, 0xaa00aa, 0xaa5500, 0xaaaaaa, 0x555555,
    0x5555ff, 0x55ff55, 0x55ffff, 0xff5555, 0xff55ff, 0xffff55, 0xffffff, 0x000000, 0x141414,
    0x202020, 0x2d2d2d, 0x393939, 0x454545, 0x515151, 0x616161, 0x717171, 0x828282, 0x929292,
    0xa2a2a2, 0xb6b6b6, 0xcacaca, 0xe3e3e3, 0xffffff, 0x0000ff, 0x4100ff, 0x7d00ff, 0xbe00ff,
    0xff00ff, 0xff00be, 0xff007d, 0xff0041, 0xff0000, 0xff4100, 0xff7d00, 0xffbe00, 0xffff00,
    0xbeff00, 0x7dff00, 0x41ff00, 0x00ff00, 0x00ff41, 0x00ff7d, 0x00ffbe, 0x00ffff, 0x00beff,
    0x007dff, 0x0041ff, 0x7d7dff, 0x9e7dff, 0xbe7dff, 0xdf7dff, 0xff7dff, 0xff7ddf, 0xff7dbe,
    0xff7d9e, 0xff7d7d, 0xff9e7d, 0xffbe7d, 0xffdf7d, 0xffff7d, 0xdfff7d, 0xbeff7d, 0x9eff7d,
    0x7dff7d, 0x7dff9e, 0x7dffbe, 0x7dffdf, 0x7dffff, 0x7ddfff, 0x7dbeff, 0x7d9eff, 0xb6b6ff,
    0xc6b6ff, 0xdbb6ff, 0xebb6ff, 0xffb6ff, 0xffb6eb, 0xffb6db, 0xffb6c6, 0xffb6b6, 0xffc6b6,
    0xffdbb6, 0xffebb6, 0xffffb6, 0xebffb6, 0xdbffb6, 0xc6ffb6, 0xb6ffb6, 0xb6ffc6, 0xb6ffdb,
    0xb6ffeb, 0xb6ffff, 0xb6ebff, 0xb6dbff, 0xb6c6ff, 0x000071, 0x1c0071, 0x390071, 0x550071,
    0x710071, 0x710055, 0x710039, 0x71001c, 0x710000, 0x711c00, 0x713900, 0x715500, 0x717100,
    0x557100, 0x397100, 0x1c7100, 0x007100, 0x00711c, 0x007139, 0x007155, 0x007171, 0x005571,
    0x003971, 0x001c71, 0x393971, 0x453971, 0x553971, 0x613971, 0x713971, 0x713961, 0x713955,
    0x713945, 0x713939, 0x714539, 0x715539, 0x716139, 0x717139, 0x617139, 0x557139, 0x457139,
    0x397139, 0x397145, 0x397155, 0x397161, 0x397171, 0x396171, 0x395571, 0x394571, 0x515171,
    0x595171, 0x615171, 0x695171, 0x715171, 0x715169, 0x715161, 0x715159, 0x715151, 0x715951,
    0x716151, 0x716951, 0x717151, 0x697151, 0x617151, 0x597151, 0x517151, 0x517159, 0x517161,
    0x517169, 0x517171, 0x516971, 0x516171, 0x515971, 0x000041, 0x100041, 0x200041, 0x310041,
    0x410041, 0x410031, 0x410020, 0x410010, 0x410000, 0x411000, 0x412000, 0x413100, 0x414100,
    0x314100, 0x204100, 0x104100, 0x004100, 0x004110, 0x004120, 0x004131, 0x004141, 0x003141,
    0x002041, 0x001041, 0x202041, 0x282041, 0x312041, 0x392041, 0x412041, 0x412039, 0x412031,
    0x412028, 0x412020, 0x412820, 0x413120, 0x413920, 0x414120, 0x394120, 0x314120, 0x284120,
    0x204120, 0x204128, 0x204131, 0x204139, 0x204141, 0x203941, 0x203141, 0x202841, 0x2d2d41,
    0x312d41, 0x352d41, 0x3d2d41, 0x412d41, 0x412d3d, 0x412d35, 0x412d31, 0x412d2d, 0x41312d,
    0x41352d, 0x413d2d, 0x41412d, 0x3d412d, 0x35412d, 0x31412d, 0x2d412d, 0x2d4131, 0x2d4135,
    0x2d413d, 0x2d4141, 0x2d3d41, 0x2d3541, 0x2d3141, 0x000000, 0x000000, 0x000000, 0x000000,
    0x000000, 0x000000, 0x000000, 0x000000,
];

#[cfg(test)]
mod tests {
    use super::*;

    /// A 4x2 16-color picture whose palette block maps index `i` to color
    /// `15 - i`, with the given block count and body.
    fn pic(block_count: u16, body: &[u8]) -> Vec<u8> {
        let mut file = alloc::vec![0x34, 0x12, 4, 0, 2, 0, 0, 0, 0, 0, 0x04, 0xff, b'L'];
        file.extend_from_slice(&2u16.to_le_bytes()); // descriptor: 16 indices
        file.extend_from_slice(&16u16.to_le_bytes());
        file.extend((0..16).rev());
        file.extend_from_slice(&block_count.to_le_bytes());
        file.extend_from_slice(body);
        file
    }

    #[test]
    fn rows_are_bottom_up_and_palette_indices_apply() {
        let image = decode_pic(&pic(0, &[0x01, 0x23, 0x45, 0x67])).unwrap();
        assert_eq!((image.width(), image.height()), (4, 2));
        // The top row is the second stored row: 4, 5, 6, 7 -> colors 11 to 8.
        assert_eq!(image.get(0, 0), CGA_PALETTE[11]);
        assert_eq!(image.get(3, 0), CGA_PALETTE[8]);
        assert_eq!(image.get(0, 1), CGA_PALETTE[15]);
    }

    #[test]
    fn a_block_expands_to_the_same_picture_as_raw_data() {
        // Marker 0xfe: a run of two 0x11 bytes, then two literals.
        let block = [0xfe, 2, 0x11, 0x23, 0x45];
        let mut body = ((block.len() + 5) as u16).to_le_bytes().to_vec();
        body.extend_from_slice(&4u16.to_le_bytes());
        body.push(0xfe);
        body.extend_from_slice(&block);
        let packed = decode_pic(&pic(1, &body)).unwrap();
        let raw = decode_pic(&pic(0, &[0x11, 0x11, 0x23, 0x45])).unwrap();
        assert_eq!(packed, raw);
    }

    #[test]
    fn runs_with_a_zero_count_take_a_16_bit_count() {
        let mut out = Vec::new();
        let produced = expand_run_length(&[0xfe, 0, 0, 1, 7], 0xfe, 1000, &mut out);
        assert_eq!(produced, Some(256));
        assert!(out.iter().all(|&b| b == 7));
    }

    #[test]
    fn truncated_files_are_rejected() {
        let file = pic(0, &[0; 4]);
        for len in 0..file.len() {
            assert!(decode_pic(&file[..len]).is_err());
        }
    }

    #[test]
    fn a_clip_that_expands_to_too_little_is_rejected() {
        // 4x4 planar 16-color clip (0x31) whose packed body is a single byte.
        let mut clip = alloc::vec![0, 0, 4, 0, 4, 0, 0, 0, 0, 0, 0xff, 0x31, 0xfe, 1];
        clip[0] = clip.len() as u8;
        assert!(decode_clp(&clip).is_err());
    }

    #[test]
    fn six_bit_palettes_round_like_deark() {
        let mut pal = [0u32; 2];
        rgb_palette(&mut pal, &[1, 32, 63, 0, 10, 62]);
        assert_eq!(pal, [0x0482ff, 0x0028fb]);
    }
}
