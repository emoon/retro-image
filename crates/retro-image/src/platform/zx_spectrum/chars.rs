//! Character-based files: fonts and `chr$` cell arrays.
//!
//! Sources:
//! - CH4/CH6/CH8 fonts (2048 bytes, 256 characters of 8 bytes, MSB left):
//!   ZX-Paintbrush page, <https://zx-modules.jimdofree.com/zx-modules-start/zx-paintbrush/>.
//! - `chr$` header and cell layout: SpectraLab
//!   `ZX_SPECTRUM_GRAPHICS_GUIDE.md` (MIT), section chr$.
//! - Font sheet layout (32 characters per row, white on black): observed
//!   from `recoil2png` output.

use alloc::vec::Vec;

use super::screen::{Frame, attribute_color, blend};
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
    for (index, glyph) in data.chunks_exact(8).enumerate() {
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
