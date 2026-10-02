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

use alloc::vec;

use crate::{DecodeError, Format, Image};

pub(super) static FORMATS: &[Format] = &[
    Format::new("PC", "Award BIOS logo", &["epa"], decode_epa),
    Format::new("PC", "Handy Scanner 2000 POSTERING", &["hs2"], decode_hs2),
    Format::new("PC", "Microsoft Paint version 1 or 2", &["msp"], decode_msp),
];

/// The 16 colours of the IBM CGA/EGA text palette.
const CGA_PALETTE: [u32; 16] = [
    0x000000, 0x0000aa, 0x00aa00, 0x00aaaa, 0xaa0000, 0xaa00aa, 0xaa5500, 0xaaaaaa, 0x555555,
    0x5555ff, 0x55ff55, 0x55ffff, 0xff5555, 0xff55ff, 0xffff55, 0xffffff,
];

const MSP_HEADER_LEN: usize = 32;

fn le16(b: &[u8]) -> usize {
    usize::from(u16::from_le_bytes([b[0], b[1]]))
}

fn decode_msp(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let header = data.get(..MSP_HEADER_LEN).ok_or(fail)?;
    let (width, height) = (le16(&header[4..6]), le16(&header[6..8]));
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
            for (y, size) in map.chunks_exact(2).map(le16).enumerate() {
                let line = data.get(pos..pos + size).ok_or(fail)?;
                pos += size;
                unpack_msp_line(line, &mut bitmap[y * row_len..(y + 1) * row_len]);
            }
            bitmap
        }
        _ => return Err(fail),
    };
    let mut image = Image::new(width as u32, height as u32);
    for y in 0..height {
        for x in 0..width {
            let set = bitmap[y * row_len + x / 8] & (0x80 >> (x % 8)) != 0;
            image.set(x as u32, y as u32, if set { 0xffffff } else { 0 });
        }
    }
    Ok(image)
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

fn decode_epa(data: &[u8]) -> Result<Image, DecodeError> {
    if data.starts_with(b"AWBM") {
        decode_awbm(data)
    } else {
        decode_epa_cells(data)
    }
}

/// Version 1: attribute bytes (background in the high nibble), then the
/// 14-byte bitmaps of the cells, row by row.
fn decode_epa_cells(data: &[u8]) -> Result<Image, DecodeError> {
    const CELL_HEIGHT: usize = 14;
    let fail = DecodeError::Unrecognized;
    let (columns, rows) = match data {
        [c, r, ..] => (usize::from(*c), usize::from(*r)),
        _ => return Err(fail),
    };
    let cells = columns * rows;
    let bitmaps = 2 + cells;
    if cells == 0 || data.len() < bitmaps + cells * CELL_HEIGHT {
        return Err(fail);
    }
    let mut image = Image::new((columns * 8) as u32, (rows * CELL_HEIGHT) as u32);
    for cell in 0..cells {
        let attribute = data[2 + cell];
        let (column, row) = (cell % columns, cell / columns);
        for line in 0..CELL_HEIGHT {
            let bits = data[bitmaps + cell * CELL_HEIGHT + line];
            for bit in 0..8 {
                let set = bits & (0x80 >> bit) != 0;
                let index = if set { attribute & 15 } else { attribute >> 4 };
                let (x, y) = (column * 8 + bit, row * CELL_HEIGHT + line);
                image.set(x as u32, y as u32, CGA_PALETTE[usize::from(index)]);
            }
        }
    }
    Ok(image)
}

/// Version 2: width, height, bitmap, then "RGB " and the palette.
fn decode_awbm(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let header = data.get(..8).ok_or(fail)?;
    let (width, height) = (le16(&header[4..6]), le16(&header[6..8]));
    if width == 0 || height == 0 {
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
    let scale = |v: u8| u32::from((v & 63) << 2 | (v & 63) >> 4);
    let palette: alloc::vec::Vec<u32> = data[palette_start..palette_start + colors * 3]
        .chunks_exact(3)
        .map(|c| scale(c[0]) << 16 | scale(c[1]) << 8 | scale(c[2]))
        .collect();
    let bitmap = &data[8..];
    let mut image = Image::new(width as u32, height as u32);
    for y in 0..height {
        for x in 0..width {
            let index = if chunky {
                usize::from(bitmap[y * width + x])
            } else {
                (0..4).fold(0, |v, plane| {
                    let byte = bitmap[(y * 4 + plane) * planar_row + x / 8];
                    v | usize::from(byte >> (7 - x % 8) & 1) << plane
                })
            };
            image.set(x as u32, y as u32, palette[index]);
        }
    }
    Ok(image)
}

fn decode_hs2(data: &[u8]) -> Result<Image, DecodeError> {
    const ROW_LEN: usize = 105;
    if data.is_empty() || !data.len().is_multiple_of(ROW_LEN) {
        return Err(DecodeError::Unrecognized);
    }
    let (width, height) = (ROW_LEN * 8, data.len() / ROW_LEN);
    let mut image = Image::new(width as u32, height as u32);
    for y in 0..height {
        for x in 0..width {
            let set = data[y * ROW_LEN + x / 8] & (0x80 >> (x % 8)) != 0;
            image.set(x as u32, y as u32, if set { 0xffffff } else { 0 });
        }
    }
    Ok(image)
}
