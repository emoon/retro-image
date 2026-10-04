//! Turbo Rascal Syntax Error (TRSE) "Fluff" images for the C64 and VIC-20.
//!
//! Sources: reverse engineered from `neo_rider_by_the_diad.flf`,
//! `charset_level.flf`, `fastfood.flf`, `hflogo.flf`, `snowman.flf` and
//! `santa.flf` by black-box probing of `recoil2png` (hand-mutated copies
//! and synthetic cell data); TRSE's GPL-3 source was not read. Container:
//! [`crate::codec::flf`]. Palettes: [`super::vic2`] and [`super::vic20`];
//! character glyphs: the C64 character ROM, see [`super::petscii`].
//!
//! Types, with absolute file offsets:
//! - 1: 15-byte header, then 1000 colour cells (40x25). 4 and 5: the same
//!   with an 18-byte header. Cells are multicolour. 6: as 4 and 5 but hires
//!   (found with synthetic cells only; no sample).
//! - 7: text mode. Byte 15 and 16 are the width and height in characters
//!   (1-255), byte 13 the background colour, then colour RAM and screen
//!   codes (one byte per character each) from offset 29, then 16 bytes that
//!   `recoil2png` ignores. Characters are the C64 upper case/graphics ROM
//!   set; colours use the low 4 bits.
//! - 9 (VIC-20): 20-byte header with the width and height in cells at
//!   offsets 18 and 19, then that many multicolour cells with colours 0-7.

use super::petscii::CHARGEN;
use super::{vic2, vic20};
use crate::codec::flf::{self, CELL_LEN, CellMode, Fluff};
use crate::{DecodeError, Image};
use alloc::vec::Vec;

const COLS: usize = 40;
const ROWS: usize = 25;

pub(super) fn decode_c64(data: &[u8]) -> Result<Image, DecodeError> {
    let fluff = Fluff::parse(data)?;
    let palette: [u32; 16] = vic2::PALETTE;
    let cells = |header: usize, mode| {
        let (cells, rest) = fluff.split(header, COLS * ROWS * CELL_LEN)?;
        if !rest.is_empty() {
            return Err(DecodeError::Unrecognized);
        }
        flf::decode_cells(cells, COLS, ROWS, mode, &palette)
    };
    match fluff.kind {
        1 => cells(15, CellMode::Multicolor),
        4 | 5 => cells(18, CellMode::Multicolor),
        6 => cells(18, CellMode::Hires),
        7 => decode_text(&fluff, &palette),
        _ => Err(DecodeError::Unrecognized),
    }
}

fn decode_text(fluff: &Fluff, palette: &[u32; 16]) -> Result<Image, DecodeError> {
    let background = fluff.byte(13)? & 15;
    let (cols, rows) = (usize::from(fluff.byte(15)?), usize::from(fluff.byte(16)?));
    if cols == 0 || rows == 0 {
        return Err(DecodeError::Unrecognized);
    }
    let count = cols * rows;
    let (colors, rest) = fluff.split(29, count)?;
    let (screen, rest) = rest
        .split_at_checked(count)
        .ok_or(DecodeError::Unrecognized)?;
    if rest.len() != 16 {
        return Err(DecodeError::Unrecognized);
    }
    let width = cols * 8;
    let mut indices: Vec<u8> = alloc::vec![background; width * rows * 8];
    for (n, (&code, &color)) in screen.iter().zip(colors).enumerate() {
        let glyph = &CHARGEN[usize::from(code) * 8..][..8];
        let (x0, y0) = (n % cols * 8, n / cols * 8);
        for (y, &bits) in glyph.iter().enumerate() {
            let row = &mut indices[(y0 + y) * width + x0..][..8];
            for (x, out) in row.iter_mut().enumerate() {
                if bits >> (7 - x) & 1 != 0 {
                    *out = color & 15;
                }
            }
        }
    }
    Image::from_indexed(width as u32, (rows * 8) as u32, &indices, palette)
}

pub(super) fn decode_vic20(data: &[u8]) -> Result<Image, DecodeError> {
    let fluff = Fluff::parse(data)?;
    if fluff.kind != 9 {
        return Err(DecodeError::Unrecognized);
    }
    let (cols, rows) = (usize::from(fluff.byte(18)?), usize::from(fluff.byte(19)?));
    let (cells, rest) = fluff.split(20, cols * rows * CELL_LEN)?;
    if cols == 0 || rows == 0 || !rest.is_empty() {
        return Err(DecodeError::Unrecognized);
    }
    flf::decode_cells(
        cells,
        cols,
        rows,
        CellMode::Multicolor,
        &vic20::PALETTE[..8],
    )
}
