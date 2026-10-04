//! Loadstar SHP pictures (C64): hires or multicolour bitmaps packed in
//! chunks with a different escape byte each.
//!
//! Sources:
//! - GoDot Loadstar loader page, <https://www.godot64.de/german/l_loadstar.htm>:
//!   load `$4000`; new format (mode `$80` hires / `$00` multicolour, bitmap
//!   escape byte, background; bitmap, screen with escape `$00`, colour with
//!   escape `$FF`) and old format (mode `$A8` hires / `$E8` multicolour,
//!   height in cells, background, bitmap escape; bitmap, screen with escape
//!   `$00`, `$FF`, colour with escape `$D8`); runs are `ESC count value`.
//! - Old format details found in sample files and checked against
//!   `recoil2png` output: the low 6 bits of the mode byte are the width in
//!   cells, and the `$FF` separator comes right before the colour chunk.
//!   Some `.shp` samples are Advanced Art Studio pictures and decode as such.

use super::unpack::{Run, escape_rle_counted};
use super::vic2::{Bitmap, Frame};
use crate::{DecodeError, Image};
use alloc::vec::Vec;

/// Reads consecutive RLE chunks.
struct Chunks<'a>(&'a [u8]);

impl Chunks<'_> {
    fn byte(&mut self) -> Result<u8, DecodeError> {
        let (&byte, rest) = self.0.split_first().ok_or(DecodeError::Unrecognized)?;
        self.0 = rest;
        Ok(byte)
    }

    fn unpack(&mut self, escape: u8, len: usize) -> Result<Vec<u8>, DecodeError> {
        let (out, used) = escape_rle_counted(self.0, escape, Run::CountValue, len)
            .ok_or(DecodeError::Unrecognized)?;
        if out.len() != len {
            return Err(DecodeError::Unrecognized);
        }
        self.0 = &self.0[used..];
        Ok(out)
    }
}

pub(super) fn decode_shp(data: &[u8]) -> Result<Image, DecodeError> {
    // Some `.shp` files are Advanced Art Studio pictures.
    if let Ok(image) = super::bitmap::decode_advanced_art_studio(data) {
        return Ok(image);
    }
    let mut chunks = match data {
        [0x00, 0x40, rest @ ..] => Chunks(rest),
        _ => return Err(DecodeError::Unrecognized),
    };
    let mode = chunks.byte()?;
    let (hires, columns, rows) = match mode {
        0x80 => (true, 40, 25),
        0x00 => (false, 40, 25),
        _ if mode & 0x80 != 0 => (
            mode & 0x40 == 0,
            usize::from(mode & 0x3f),
            usize::from(chunks.byte()?),
        ),
        _ => return Err(DecodeError::Unrecognized),
    };
    if columns == 0 || columns > 40 || rows == 0 || rows > 25 {
        return Err(DecodeError::Unrecognized);
    }
    let old = mode & 0x3f != 0;
    let cells = columns * rows;
    let (background, escape) = match (old, hires) {
        // Hires has no background, only the escape.
        (_, true) => (0, chunks.byte()?),
        // New format: escape, then background.
        (false, false) => {
            let escape = chunks.byte()?;
            (chunks.byte()?, escape)
        }
        // Old format: background, then escape.
        (true, false) => (chunks.byte()?, chunks.byte()?),
    };
    let bitmap = chunks.unpack(escape, cells * 8)?;
    let screen = chunks.unpack(0x00, cells)?;
    let color = match (old, hires) {
        (_, true) => Vec::new(),
        (false, false) => chunks.unpack(0xff, cells)?,
        (true, false) => {
            if chunks.byte()? != 0xff {
                return Err(DecodeError::Unrecognized);
            }
            chunks.unpack(0xd8, cells)?
        }
    };
    render(hires, columns, rows, background, &bitmap, &screen, &color)
}

/// Renders `columns`×`rows` cells stored row by row.
fn render(
    hires: bool,
    columns: usize,
    rows: usize,
    background: u8,
    bitmap: &[u8],
    screen: &[u8],
    color: &[u8],
) -> Result<Image, DecodeError> {
    // Spread the cells over a full 40-column screen.
    let mut full_bitmap = alloc::vec![0; 8000];
    let mut full_screen = alloc::vec![0; 1000];
    let mut full_color = alloc::vec![0; 1000];
    for row in 0..rows {
        for column in 0..columns {
            let (from, to) = (row * columns + column, row * 40 + column);
            full_bitmap[to * 8..to * 8 + 8].copy_from_slice(&bitmap[from * 8..from * 8 + 8]);
            full_screen[to] = screen[from];
            full_color[to] = color.get(from).copied().unwrap_or(0);
        }
    }
    let height = rows * 8;
    let frame = if hires {
        Frame::hires(&Bitmap::hires(&full_bitmap, &full_screen), height)
    } else {
        let bitmap = Bitmap::multicolor(&full_bitmap, &full_screen, &full_color, background);
        Frame::multicolor(&bitmap, height)
    };
    let frame = frame.ok_or(DecodeError::Unrecognized)?;
    Ok(frame.to_image_width(columns * 8))
}
