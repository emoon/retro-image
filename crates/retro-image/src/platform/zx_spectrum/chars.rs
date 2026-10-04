//! Character-based files: fonts, `chr$` cell arrays, CHX big fonts and
//! SevenuP sprites.
//!
//! Sources:
//! - CH4/CH6/CH8 fonts (2048 bytes, 256 characters of 8 bytes, MSB left):
//!   ZX-Paintbrush page, <https://zx-modules.jimdofree.com/zx-modules-start/zx-paintbrush/>.
//! - `chr$` header and cell layout: SpectraLab
//!   `ZX_SPECTRUM_GRAPHICS_GUIDE.md` (MIT), section chr$,
//!   <https://github.com/Bedazzle/SpectraLab/blob/main/ZX_SPECTRUM_GRAPHICS_GUIDE.md>.
//! - Font sheet layout (32 characters per row, white on black): observed
//!   from `recoil2png` output.
//! - CHX (ZX-Editor / ZX-Paintbrush big fonts): `CHX` signature, characters
//!   of 1x1 to 4x4 cells, coloured or not: ZX-Paintbrush page above. Byte
//!   layout (256 little-endian file offsets at 5, 0 for a missing
//!   character; each character: a flag, 0 with an attribute after each
//!   cell or 1 without, width and height in cells, then the cells row by
//!   row), the sheet of 16 characters per row in slots of the largest size,
//!   black on white (non-bright) without attributes, and the checkerboard
//!   behind and between characters: reverse engineered from samples and
//!   `recoil2png` output.
//! - SevenuP `.SEV` sprites: the header (`Sev\0`, a u16 that must be 1 at 6,
//!   width and height in pixels at 10 and 12), the 9-byte cells (8 bitmap
//!   bytes, then the attribute) row by row, sizes that aren't a multiple of
//!   8 cropped from whole cells, and that only the first frame is shown:
//!   reverse engineered from samples (Sword of Ianna sources, Apache-2.0,
//!   <https://github.com/fjpena/sword-of-ianna-zx>)
//!   and `recoil2png` probes. SevenuP's own code (GPL) was not read.

use alloc::vec::Vec;

use super::screen::{Frame, attribute_color, blend};
use crate::bytes::le16;
use crate::image::check_size;
use crate::{DecodeError, Image};

const FONT_LEN: usize = 2048;
const SHEET_COLUMNS: usize = 32;

/// 256 characters of 8 bytes each, drawn as a 32x8 character sheet.
pub(super) fn decode_font(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != FONT_LEN {
        return Err(DecodeError::Unrecognized);
    }
    let rows = FONT_LEN / 8 / SHEET_COLUMNS;
    let mut frame = Frame::new(SHEET_COLUMNS * 8, rows * 8);
    for (index, glyph) in data.as_chunks::<8>().0.iter().enumerate() {
        let (left, top) = (index % SHEET_COLUMNS * 8, index / SHEET_COLUMNS * 8);
        draw_cell(&mut frame, left, top, glyph, 0xffffff, 0);
    }
    Ok(frame.into_image())
}

const CHR_HEADER_LEN: usize = 7;

/// `chr$`: magic, width and height in cells, bytes per cell (9: 8 bitmap
/// bytes and an attribute; 18: two such cells shown as gigascreen), then
/// the cells row by row.
pub(super) fn decode_chr(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() < CHR_HEADER_LEN || !data.starts_with(b"chr$") {
        return Err(DecodeError::Unrecognized);
    }
    let (columns, rows) = (usize::from(data[4]), usize::from(data[5]));
    let frame_count = match data[6] {
        9 => 1,
        18 => 2,
        _ => return Err(DecodeError::Unrecognized),
    };
    let cells = &data[CHR_HEADER_LEN..];
    if columns == 0 || rows == 0 || cells.len() != columns * rows * 9 * frame_count {
        return Err(DecodeError::Unrecognized);
    }
    let frames: Vec<Frame> = (0..frame_count)
        .map(|f| {
            let mut frame = Frame::new(columns * 8, rows * 8);
            for (index, cell) in cells.chunks_exact(9 * frame_count).enumerate() {
                let cell = &cell[f * 9..][..9];
                let (left, top) = (index % columns * 8, index / columns * 8);
                let ink = attribute_color(cell[8], true);
                let paper = attribute_color(cell[8], false);
                draw_cell(&mut frame, left, top, &cell[..8], ink, paper);
            }
            frame
        })
        .collect();
    Ok(blend(&frames))
}

const SEV_HEADER_LEN: usize = 14;

/// SevenuP sprite: `Sev\0`, 2 ignored bytes, a u16 that must be 1, a
/// frame count (ignored), width and height in pixels as u16, then each
/// frame's 8x8 cells row by row (8 bitmap bytes and an attribute). Only the
/// first frame is shown, cropped to the size.
pub(super) fn decode_sev(data: &[u8]) -> Result<Image, DecodeError> {
    if !data.starts_with(b"Sev\0") || le16(data, 6) != Some(1) {
        return Err(DecodeError::Unrecognized);
    }
    let (Some(width), Some(height)) = (le16(data, 10), le16(data, 12)) else {
        return Err(DecodeError::Unrecognized);
    };
    let (width, height) = (usize::from(width), usize::from(height));
    check_size(width, height)?;
    let columns = width.div_ceil(8);
    let cells = data
        .get(SEV_HEADER_LEN..SEV_HEADER_LEN + columns * height.div_ceil(8) * 9)
        .ok_or(DecodeError::Unrecognized)?;
    let mut frame = Frame::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let cell = &cells[(y / 8 * columns + x / 8) * 9..][..9];
            let ink = cell[y % 8] & (0x80 >> (x % 8)) != 0;
            frame.set(x, y, attribute_color(cell[8], ink));
        }
    }
    Ok(frame.into_image())
}

fn draw_cell(frame: &mut Frame, left: usize, top: usize, rows: &[u8], ink: u32, paper: u32) {
    for (y, &byte) in rows.iter().enumerate() {
        for bit in 0..8 {
            let color = if byte & (0x80 >> bit) != 0 {
                ink
            } else {
                paper
            };
            frame.set(left + bit, top + y, color);
        }
    }
}

const CHX_TABLE: usize = 5;
const CHX_CHARACTERS: usize = 256;
const CHX_PER_ROW: usize = 16;

/// One CHX character: its code, size in cells and its cells.
struct BigChar<'a> {
    code: usize,
    width: usize,
    height: usize,
    colored: bool,
    cells: &'a [u8],
}

impl BigChar<'_> {
    fn cell_len(&self) -> usize {
        if self.colored { 9 } else { 8 }
    }
}

/// CHX: big characters drawn on a 16-wide sheet over a checkerboard.
pub(super) fn decode_chx(data: &[u8]) -> Result<Image, DecodeError> {
    let table = data
        .get(CHX_TABLE..CHX_TABLE + 2 * CHX_CHARACTERS)
        .filter(|_| data.starts_with(b"CHX"))
        .ok_or(DecodeError::Unrecognized)?;
    let mut chars = Vec::new();
    for (code, entry) in table.as_chunks::<2>().0.iter().enumerate() {
        let offset = usize::from(u16::from_le_bytes([entry[0], entry[1]]));
        if offset != 0 {
            chars.push(big_char(data, code, offset).ok_or(DecodeError::Unrecognized)?);
        }
    }
    let slot_width = 8 * chars
        .iter()
        .map(|c| c.width)
        .max()
        .ok_or(DecodeError::Unrecognized)?;
    let slot_height = 8 * chars.iter().map(|c| c.height).max().unwrap_or(1);
    let (width, height) = (
        CHX_PER_ROW * slot_width,
        CHX_CHARACTERS / CHX_PER_ROW * slot_height,
    );
    let mut frame = Frame::new(width, height);
    let white = attribute_color(0x38, false);
    for y in 0..height {
        for x in 0..width {
            frame.set(x, y, if (x + y) % 2 == 1 { white } else { 0 });
        }
    }
    for c in &chars {
        let left = c.code % CHX_PER_ROW * slot_width;
        let top = c.code / CHX_PER_ROW * slot_height;
        for (i, cell) in c.cells.chunks_exact(c.cell_len()).enumerate() {
            // Without attributes: black ink on white paper.
            let attribute = if c.colored { cell[8] } else { 0x38 };
            let ink = attribute_color(attribute, true);
            let paper = attribute_color(attribute, false);
            let (x, y) = (left + i % c.width * 8, top + i / c.width * 8);
            draw_cell(&mut frame, x, y, &cell[..8], ink, paper);
        }
    }
    Ok(frame.into_image())
}

/// The character at `offset`: flag (0 coloured, 1 not), width and height in
/// cells (1-4), then the cells.
fn big_char(data: &[u8], code: usize, offset: usize) -> Option<BigChar<'_>> {
    let header = data.get(offset..offset + 3)?;
    let colored = match header[0] {
        0 => true,
        1 => false,
        _ => return None,
    };
    let (width, height) = (usize::from(header[1]), usize::from(header[2]));
    if !(1..=4).contains(&width) || !(1..=4).contains(&height) {
        return None;
    }
    let mut c = BigChar {
        code,
        width,
        height,
        colored,
        cells: &[],
    };
    c.cells = data.get(offset + 3..offset + 3 + width * height * c.cell_len())?;
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sev_crops_whole_cells_to_its_size() {
        // 12x4: two cells side by side; the second cell's attribute is
        // bright red ink on blue paper.
        let mut sev = b"Sev\0\0\x08\x01\0\0\0\x0c\0\x04\0".to_vec();
        sev.extend_from_slice(&[0; 9]);
        sev.extend_from_slice(&[0x80, 0, 0, 0, 0, 0, 0, 0, 0x4a]);
        let image = decode_sev(&sev).unwrap();
        assert_eq!((image.width(), image.height()), (12, 4));
        assert_eq!(image.get(8, 0), 0xff0000);
        assert_eq!(image.get(9, 0), 0x0000ff);
        assert!(decode_sev(&sev[..sev.len() - 1]).is_err());
        sev[6] = 2;
        assert!(decode_sev(&sev).is_err());
    }
}
