//! IBM PC.
//!
//! Sources:
//! - Microsoft Paint (MSP): Encyclopedia of Graphics File Formats,
//!   <https://www.fileformat.info/format/mspaint/egff.htm> (32-byte header,
//!   "DanM" uncompressed version 1, "LinS" version 2 with a per-line size
//!   map and RLE). Set bit = white: observed from `recoil2png` output.
//! - Award BIOS logo (EPA): Deark `awbm.c` (<https://github.com/jsummers/deark>,
//!   MIT licence): version 1 is a grid of 8x14 character cells (width and
//!   height in cells, one attribute byte per cell, then the cell bitmaps);
//!   version 2 ("AWBM") is a 4-bit planar or 8-bit chunky bitmap followed by
//!   "RGB " and a 6-bit VGA palette (RGB order, as observed from
//!   `recoil2png` output).
//! - Handy Scanner HS2: Deark `misc2.c` (MIT licence): headerless 1-bit
//!   bitmap, 105 bytes (840 pixels) per row.
//! - CGA palette and the 6-bit to 8-bit palette scaling: observed from
//!   `recoil2png` output.

mod image72;

use alloc::vec;
use alloc::vec::Vec;

use crate::bytes::le16;
use crate::{BitOrder, DecodeError, Format, Image};

pub(super) static FORMATS: &[Format] = &[
    // Version 2 first: its "AWBM" header would also pass as version 1 cells.
    Format::new("PC", "Award BIOS logo version 2", &["epa"], decode_awbm).signature(),
    Format::new("PC", "Award BIOS logo", &["epa"], decode_epa_cells),
    Format::new("PC", "Handy Scanner 2000 POSTERING", &["hs2"], decode_hs2),
    Format::new("PC", "Microsoft Paint version 1 or 2", &["msp"], decode_msp).signature(),
    // Wave 5: Amiga and misc
    Format::new("PC", "Image 72 font", &["fnt"], image72::decode),
];

/// The 16 colours of the IBM CGA/EGA text palette, by attribute value.
/// Also used by the text-mode art formats.
pub(super) const CGA_PALETTE: [u32; 16] = [
    0x000000, 0x0000aa, 0x00aa00, 0x00aaaa, 0xaa0000, 0xaa00aa, 0xaa5500, 0xaaaaaa, 0x555555,
    0x5555ff, 0x55ff55, 0x55ffff, 0xff5555, 0xff55ff, 0xffff55, 0xffffff,
];

const MSP_HEADER_LEN: usize = 32;

fn decode_msp(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let header = data.get(..MSP_HEADER_LEN).ok_or(fail)?;
    let word = |at| le16(header, at).map(usize::from).ok_or(fail);
    let (width, height) = (word(4)?, word(6)?);
    if width == 0 || height == 0 {
        return Err(fail);
    }
    let row_len = width.div_ceil(8);
    let bitmap = match &header[..4] {
        b"DanM" => data
            .get(MSP_HEADER_LEN..MSP_HEADER_LEN + row_len * height)
            .ok_or(fail)?
            .to_vec(),
        b"LinS" => {
            let map_end = MSP_HEADER_LEN + height * 2;
            let map = data.get(MSP_HEADER_LEN..map_end).ok_or(fail)?;
            // A 3-byte run gives at most 255 bytes; this keeps corrupt
            // sizes from allocating huge bitmaps.
            if row_len * height > data.len().saturating_mul(85) {
                return Err(fail);
            }
            let mut bitmap = vec![0u8; row_len * height];
            let mut pos = map_end;
            for (y, size) in map
                .as_chunks::<2>()
                .0
                .iter()
                .map(|w| usize::from(u16::from_le_bytes([w[0], w[1]])))
                .enumerate()
            {
                let line = data.get(pos..pos + size).ok_or(fail)?;
                pos += size;
                unpack_msp_line(line, &mut bitmap[y * row_len..(y + 1) * row_len]);
            }
            bitmap
        }
        _ => return Err(fail),
    };
    mono(&bitmap, width, height, row_len)
}

/// A 1-bit bitmap, most significant bit leftmost, set bit white.
fn mono(bitmap: &[u8], width: usize, height: usize, row_len: usize) -> Result<Image, DecodeError> {
    let colors = [0, 0xffffff];
    Image::from_bits(
        width as u32,
        height as u32,
        bitmap,
        row_len,
        BitOrder::MsbFirst,
        colors,
    )
}

/// `00 count value` is a run; any other byte `n` is followed by `n` literals.
/// Output beyond the line is dropped.
fn unpack_msp_line(src: &[u8], out: &mut [u8]) {
    let mut pos = 0;
    let mut x = 0;
    let mut put = |value: u8| {
        if let Some(slot) = out.get_mut(x) {
            *slot = value;
        }
        x += 1;
    };
    while pos < src.len() {
        let kind = src[pos];
        if kind == 0 {
            let (Some(&count), Some(&value)) = (src.get(pos + 1), src.get(pos + 2)) else {
                return;
            };
            (0..count).for_each(|_| put(value));
            pos += 3;
        } else {
            let literal = &src[pos + 1..src.len().min(pos + 1 + usize::from(kind))];
            literal.iter().for_each(|&b| put(b));
            pos += 1 + usize::from(kind);
        }
    }
}

/// Version 1: attribute bytes (background in the high nibble), then the
/// 14-byte bitmaps of the cells, row by row.
fn decode_epa_cells(data: &[u8]) -> Result<Image, DecodeError> {
    const CELL_HEIGHT: usize = 14;
    let fail = DecodeError::Unrecognized;
    let (columns, rows) = match data {
        [b'A', b'W', b'B', b'M', ..] => return Err(fail),
        [c, r, ..] => (usize::from(*c), usize::from(*r)),
        _ => return Err(fail),
    };
    let cells = columns * rows;
    let bitmaps = 2 + cells;
    if cells == 0 || data.len() < bitmaps + cells * CELL_HEIGHT {
        return Err(fail);
    }
    let (width, height) = (columns * 8, rows * CELL_HEIGHT);
    let indices: Vec<u8> = (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .map(|(x, y)| {
            let cell = y / CELL_HEIGHT * columns + x / 8;
            let attribute = data[2 + cell];
            let bits = data[bitmaps + cell * CELL_HEIGHT + y % CELL_HEIGHT];
            if bits >> (7 - x % 8) & 1 != 0 {
                attribute & 15
            } else {
                attribute >> 4
            }
        })
        .collect();
    Image::from_indexed(width as u32, height as u32, &indices, &CGA_PALETTE)
}

/// Version 2: width, height, bitmap, then "RGB " and the palette.
fn decode_awbm(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let header = data.get(..8).ok_or(fail)?;
    let word = |at| le16(header, at).map(usize::from).ok_or(fail);
    let (width, height) = (word(4)?, word(6)?);
    if &header[..4] != b"AWBM" || width == 0 || height == 0 {
        return Err(fail);
    }
    let palette_at = |bitmap_len: usize, colors: usize| {
        let at = 8 + bitmap_len;
        (data.get(at..at + 4) == Some(b"RGB ") && data.len() >= at + 4 + colors * 3)
            .then_some(at + 4)
    };
    let planar_row = width.div_ceil(8);
    let (colors, palette_start, chunky) = if let Some(at) = palette_at(width * height, 256) {
        (256, at, true)
    } else if let Some(at) = palette_at(planar_row * 4 * height, 16) {
        (16, at, false)
    } else {
        return Err(fail);
    };
    let palette: Vec<u32> = data[palette_start..palette_start + colors * 3]
        .as_chunks::<3>()
        .0
        .iter()
        .map(|c| vga_rgb([c[0], c[1], c[2]]))
        .collect();
    let bitmap = &data[8..];
    let indices: Vec<u8> = if chunky {
        bitmap[..width * height].to_vec()
    } else {
        (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .map(|(x, y)| {
                (0..4).fold(0, |v, plane| {
                    let byte = bitmap[(y * 4 + plane) * planar_row + x / 8];
                    v | (byte >> (7 - x % 8) & 1) << plane
                })
            })
            .collect()
    };
    Image::from_indexed(width as u32, height as u32, &indices, &palette)
}

/// A VGA DAC entry (red, green, blue of 0-63; higher bits ignored) as
/// `0xRRGGBB`, each value scaled `v * 4 + v / 16`. Also used by the
/// text-mode art formats.
pub(super) fn vga_rgb(rgb: [u8; 3]) -> u32 {
    let scale = |v: u8| u32::from((v & 63) << 2 | (v & 63) >> 4);
    scale(rgb[0]) << 16 | scale(rgb[1]) << 8 | scale(rgb[2])
}

fn decode_hs2(data: &[u8]) -> Result<Image, DecodeError> {
    const ROW_LEN: usize = 105;
    if data.is_empty() || !data.len().is_multiple_of(ROW_LEN) {
        return Err(DecodeError::Unrecognized);
    }
    mono(data, ROW_LEN * 8, data.len() / ROW_LEN, ROW_LEN)
}
