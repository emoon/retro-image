//! Blazing Paddles shape tables (SHP) and fonts (CHR).
//!
//! Sources:
//! - Blazing Paddles (Baudville) and its Atari supplement manual
//!   (<https://archive.org/details/BlazingPaddlesAtariSupplementManualBaudville>),
//!   Just Solve "Blazing Paddles"
//!   (<http://justsolve.archiveteam.org/wiki/Blazing_Paddles>): the names and
//!   the sizes only. The manual does not describe the files.
//! - Everything else is reverse engineered from the corpus samples BLDGS.SHP
//!   (1024 bytes) and ITALIC8.CHR (3072 bytes), and from `recoil2png` probing
//!   of hand-made files (one stream at a time, each command byte on its own,
//!   then limits and layout). The font and the shape table use the same code
//!   and the same sheet layout, so one decoder serves both.
//!
//! Layout: a table of little-endian pointers ended by a zero word, then
//! streams. A pointer is an address in the machine the file was dumped from:
//! the file is a memory image of 1024 bytes at $7C00 (SHP) or 3072 bytes at
//! $7000 (CHR). Each pointer starts a stream of command bytes that ends at
//! the next `08` byte. The table also ends at the first pointer that is
//! outside the file or whose stream has no `08`. (In a font each glyph is `A 08 commands`: the
//! pointer is to `commands`, so the previous glyph's stream also contains
//! this glyph's `A`, a command that moves the pen, and ends at its `08`.)
//!
//! A command byte draws `count = high nibble + 1` steps in one direction,
//! given by the low two bits: 0 right, 1 left, 2 up, 3 down. Bit 2 moves
//! without plotting. A step plots the pixel at the pen and then moves the
//! pen. Bit 3 is ignored (the byte `08` alone ends the stream). The pen
//! starts each stream at (0, 0).
//!
//! The picture is a sheet of units 2 pixels wide and 1 high, white on black.
//! Streams are laid out like text: each starts where the previous pen
//! reached, 2 units on, with its leftmost pen position shifted to the origin;
//! when the position after it would pass 160 units it starts a new row. A
//! row is as tall as the pen's travel over all its streams (upwards from the
//! highest point), rows are 1 unit apart, and the sheet is as wide as the
//! rightmost position reached and at most 240 high.

use crate::{DecodeError, Image};
use alloc::vec::Vec;

/// Right edge of the sheet, in units.
const MAX_WIDTH: i32 = 160;
const MAX_HEIGHT: i32 = 240;
/// Units after the furthest pen position before the next stream starts.
const SPACING: i32 = 2;
/// The pen may leave this box (after which the sheet is too big anyway).
const MAX_TRAVEL: i32 = 1024;
const INK: u32 = 0xee_ee_ee;

/// A shape table: 1024 bytes at $7C00.
pub(super) fn decode_shp(data: &[u8]) -> Result<Image, DecodeError> {
    decode(data, 1024, 0x7c00)
}

/// A font: 3072 bytes at $7000.
pub(super) fn decode_chr(data: &[u8]) -> Result<Image, DecodeError> {
    decode(data, 3072, 0x7000)
}

fn decode(data: &[u8], len: usize, base: usize) -> Result<Image, DecodeError> {
    if data.len() != len {
        return Err(DecodeError::Unrecognized);
    }
    let streams = streams(data, base).ok_or(DecodeError::Unrecognized)?;
    let sheet = layout(&streams).ok_or(DecodeError::Unrecognized)?;
    Ok(sheet.draw())
}

/// The streams the pointer table selects, without their final `08`. The
/// table ends at a zero word or at the first pointer that leads nowhere.
fn streams(data: &[u8], base: usize) -> Option<Vec<&[u8]>> {
    let mut streams = Vec::new();
    for word in data.as_chunks::<2>().0 {
        let pointer = usize::from(u16::from_le_bytes([word[0], word[1]]));
        let stream = pointer
            .checked_sub(base)
            .and_then(|offset| data.get(offset..))
            .and_then(|rest| Some(&rest[..rest.iter().position(|&byte| byte == 0x08)?]));
        match (pointer, stream) {
            (1.., Some(stream)) => streams.push(stream),
            _ => break,
        }
    }
    (!streams.is_empty()).then_some(streams)
}

/// What a stream does, relative to where the pen starts.
#[derive(Clone, Copy, Default)]
struct Extents {
    min_x: i32,
    max_x: i32,
    min_y: i32,
    max_y: i32,
}

/// Runs `stream`, calling `plot` at the pen of every plotting step.
/// `None` if the pen travels absurdly far.
fn walk(stream: &[u8], mut plot: impl FnMut(i32, i32)) -> Option<Extents> {
    let (mut x, mut y) = (0i32, 0i32);
    let mut extents = Extents::default();
    for &command in stream {
        let steps = i32::from(command >> 4) + 1;
        let (dx, dy) = match command & 3 {
            0 => (1, 0),
            1 => (-1, 0),
            2 => (0, -1),
            _ => (0, 1),
        };
        for _ in 0..steps {
            if command & 4 == 0 {
                plot(x, y);
            }
            x += dx;
            y += dy;
            extents.min_x = extents.min_x.min(x);
            extents.max_x = extents.max_x.max(x);
            extents.min_y = extents.min_y.min(y);
            extents.max_y = extents.max_y.max(y);
        }
        if extents.max_x - extents.min_x > MAX_TRAVEL || extents.max_y - extents.min_y > MAX_TRAVEL
        {
            return None;
        }
    }
    Some(extents)
}

struct Placed<'a> {
    stream: &'a [u8],
    /// Where the pen starts, in sheet units.
    x: i32,
}

struct Row<'a> {
    placed: Vec<Placed<'a>>,
    min_y: i32,
    max_y: i32,
}

struct Sheet<'a> {
    rows: Vec<Row<'a>>,
    width: i32,
    height: i32,
}

fn layout<'a>(streams: &[&'a [u8]]) -> Option<Sheet<'a>> {
    let mut rows: Vec<Row> = Vec::new();
    let mut width = 0;
    let mut cursor = 0;
    for &stream in streams {
        let extents = walk(stream, |_, _| {})?;
        let reach = |x: i32| x - extents.min_x + extents.max_x + SPACING;
        let mut start = cursor;
        if rows.is_empty() || reach(start) > MAX_WIDTH {
            start = 0;
            if reach(start) > MAX_WIDTH {
                return None;
            }
            rows.push(Row {
                placed: Vec::new(),
                min_y: 0,
                max_y: 0,
            });
        }
        let row = rows.last_mut()?;
        row.min_y = row.min_y.min(extents.min_y);
        row.max_y = row.max_y.max(extents.max_y);
        row.placed.push(Placed {
            stream,
            x: start - extents.min_x,
        });
        cursor = reach(start);
        width = width.max(cursor);
    }
    let height = rows
        .iter()
        .map(|row| row.max_y - row.min_y + 1)
        .sum::<i32>()
        + rows.len() as i32
        - 1;
    (height <= MAX_HEIGHT).then_some(Sheet {
        rows,
        width,
        height,
    })
}

impl Sheet<'_> {
    fn draw(&self) -> Image {
        let mut image = Image::new(2 * self.width as u32, self.height as u32);
        let mut top = 0;
        for row in &self.rows {
            for placed in &row.placed {
                walk(placed.stream, |x, y| {
                    let (x, y) = (placed.x + x, top + y - row.min_y);
                    for pixel in 0..2 {
                        image.set(2 * x as u32 + pixel, y as u32, INK);
                    }
                });
            }
            top += row.max_y - row.min_y + 2;
        }
        image
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    /// A table at `base` of streams, each ended by `08`, then padding.
    fn file(len: usize, base: u16, streams: &[&[u8]]) -> Vec<u8> {
        let mut data = vec![0u8; 2 * streams.len() + 2];
        for (index, stream) in streams.iter().enumerate() {
            let pointer = base + data.len() as u16;
            data[2 * index..2 * index + 2].copy_from_slice(&pointer.to_le_bytes());
            data.extend_from_slice(stream);
            data.push(0x08);
        }
        data.resize(len, 0);
        data
    }

    #[test]
    fn slash_steps_up_and_right() {
        // (plot, step right), (plot, step up) three times.
        let data = file(1024, 0x7c00, &[&[0x00, 0x02, 0x00, 0x02, 0x00, 0x02]]);
        let image = decode_shp(&data).unwrap();
        // Pen travel: 0..=3 right, up to -3 (the pen ends one above the top row).
        assert_eq!((image.width(), image.height()), (2 * (3 + 2), 4));
        // Bottom row: units 0 and 1 (2 pixels each).
        assert_eq!(image.get(0, 3), INK);
        assert_eq!(image.get(3, 3), INK);
        assert_eq!(image.get(4, 3), 0);
        // Next row up starts one unit to the right.
        assert_eq!(image.get(1, 2), 0);
        assert_eq!(image.get(2, 2), INK);
    }

    #[test]
    fn streams_follow_each_other_and_wrap() {
        // Two 100-unit bars cannot share a 160-unit row.
        let bar = [0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0x30];
        let data = file(1024, 0x7c00, &[&bar, &bar]);
        let image = decode_shp(&data).unwrap();
        assert_eq!(image.width(), 2 * (100 + 2));
        assert_eq!(image.height(), 3);
    }

    #[test]
    fn rejects_wrong_sizes_and_empty_tables() {
        assert!(decode_shp(&[0; 1024]).is_err());
        assert!(decode_chr(&file(1024, 0x7c00, &[&[0]])).is_err());
        let too_wide = [0xf0u8; 11];
        assert!(decode_shp(&file(1024, 0x7c00, &[&too_wide])).is_err());
        // A stream without its 08 is dropped, leaving nothing to show.
        let mut data = file(3072, 0x7000, &[&[0]]);
        data[5] = 1;
        assert!(decode_chr(&data).is_err());
    }
}
