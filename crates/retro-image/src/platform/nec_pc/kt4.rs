//! `KT4` "Kitty" pictures (NEC PC-88 VA, 640x400, digital 8 colours).
//!
//! The only prose read was the ftz article on note.com
//! (<https://note.com/ftz/n/n84d9dd98c1e2>), which says the format paints
//! rectangles of one repeated 4x2 pixel tile. The `ifkty` plug-in source was
//! not read. The layout below was reverse engineered from `20220101.kt4`,
//! `anguishglassesjk.kt4` and `decreaseage.kt4` by black-box probing of
//! `recoil2png` (see `docs/research/msx-japanese.md`, "Wave 5b: Japanese").
//!
//! The picture is a grid of 160 x 100 cells, each 4 pixels wide and 4 tall: a
//! top and a bottom tile of 4 x 2 pixels. A tile is three bytes (blue, red,
//! green plane), one bit per pixel, row-major, most significant bit first.
//! The file is a run of segments, each introduced by a byte `n`:
//! - `0` is refused; `0xFF` starts the raw tail (below).
//! - Otherwise a tile follows: three bytes for `n == 1` (used for both halves
//!   of the cell), six bytes (top, then bottom) for any other `n`.
//! - Then a list of area entries ended by `0xFF`. The first byte's top two
//!   bits give the kind and the rest, with the second byte, a cell number
//!   `row * 160 + column`. Kind 1 (`0x40`): one more byte, the last column of a
//!   horizontal run. Kind 2 (`0x80`): one more byte, the last row of a vertical
//!   run. Kind 0: two more bytes, the last column and the last row of a
//!   rectangle. Kind 3 is refused.
//! - Then a list of single cells ended by `0xFF`: column, row.
//!
//! Every cell in the listed areas gets the segment's tile; a later segment
//! overwrites an earlier one. After `0xFF` come six bytes (top and bottom tile)
//! for every cell no segment painted, in raster order, and the file ends
//! exactly there.

use alloc::vec;
use alloc::vec::Vec;

use super::pc88_planes::palette;
use crate::{DecodeError, Image};

const COLUMNS: usize = 160;
const ROWS: usize = 100;
const WIDTH: usize = COLUMNS * 4;
const HEIGHT: usize = ROWS * 4;
const RAW_MARKER: u8 = 0xff;

/// Top tile then bottom tile, three plane bytes each.
type Cell = [u8; 6];

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn take<const N: usize>(&mut self) -> Option<[u8; N]> {
        let bytes = self.data.get(self.pos..self.pos + N)?;
        self.pos += N;
        bytes.try_into().ok()
    }

    fn byte(&mut self) -> Option<u8> {
        self.take::<1>().map(|[b]| b)
    }
}

/// The cells of one area entry that starts with `first`, as an inclusive
/// range of columns and one of rows.
fn area(
    reader: &mut Reader,
    first: u8,
) -> Option<(
    core::ops::RangeInclusive<usize>,
    core::ops::RangeInclusive<usize>,
)> {
    let [low] = reader.take()?;
    let number = usize::from(first & 0x3f) << 8 | usize::from(low);
    let (row, column) = (number / COLUMNS, number % COLUMNS);
    let (last_column, last_row) = match first >> 6 {
        0 => {
            let [column, row] = reader.take()?;
            (usize::from(column), usize::from(row))
        }
        1 => (usize::from(reader.byte()?), row),
        2 => (column, usize::from(reader.byte()?)),
        _ => return None,
    };
    (row <= last_row && last_row < ROWS && column <= last_column && last_column < COLUMNS)
        .then_some((column..=last_column, row..=last_row))
}

pub(in crate::platform) fn decode_kt4(data: &[u8]) -> Result<Image, DecodeError> {
    let bad = DecodeError::Unrecognized;
    let mut reader = Reader { data, pos: 0 };
    let mut cells: Vec<Option<Cell>> = vec![None; COLUMNS * ROWS];

    loop {
        let count = reader.byte().ok_or(bad)?;
        let cell: Cell = match count {
            0 => return Err(bad),
            RAW_MARKER => break,
            1 => {
                let tile: [u8; 3] = reader.take().ok_or(bad)?;
                [tile, tile].concat().try_into().map_err(|_| bad)?
            }
            _ => reader.take().ok_or(bad)?,
        };
        loop {
            let first = reader.byte().ok_or(bad)?;
            if first == RAW_MARKER {
                break;
            }
            let (columns, rows) = area(&mut reader, first).ok_or(bad)?;
            for row in rows {
                for column in columns.clone() {
                    cells[row * COLUMNS + column] = Some(cell);
                }
            }
        }
        loop {
            let column = reader.byte().ok_or(bad)?;
            if column == RAW_MARKER {
                break;
            }
            let row = reader.byte().ok_or(bad)?;
            let slot = cells
                .get_mut(usize::from(row) * COLUMNS + usize::from(column))
                .filter(|_| usize::from(row) < ROWS && usize::from(column) < COLUMNS)
                .ok_or(bad)?;
            *slot = Some(cell);
        }
    }
    for slot in cells.iter_mut().filter(|slot| slot.is_none()) {
        *slot = Some(reader.take().ok_or(bad)?);
    }
    if reader.pos != data.len() {
        return Err(bad);
    }

    let mut indices = vec![0u8; WIDTH * HEIGHT];
    for (n, cell) in cells.iter().enumerate() {
        let cell = cell.as_ref().ok_or(bad)?;
        let (x, y) = (n % COLUMNS * 4, n / COLUMNS * 4);
        for (half, tile) in cell.chunks(3).enumerate() {
            for bit in 0..8 {
                // Plane bytes are blue, red, green: colour bits 0, 1, 2.
                let colour = tile
                    .iter()
                    .enumerate()
                    .fold(0u8, |c, (plane, byte)| c | (byte >> (7 - bit) & 1) << plane);
                let (px, py) = (x + bit % 4, y + half * 2 + bit / 4);
                indices[py * WIDTH + px] = colour;
            }
        }
    }
    Image::from_indexed(WIDTH as u32, HEIGHT as u32, &indices, &palette())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn areas() {
        // Rectangle: cell 0x0102 = row 1, column 98, to column 99 and row 2.
        let mut r = Reader {
            data: &[0x02, 99, 2],
            pos: 0,
        };
        let (columns, rows) = area(&mut r, 0x01).unwrap();
        assert_eq!((columns, rows), (98..=99, 1..=2));
        // Horizontal run ending before its start, and a kind 3 entry, are refused.
        let mut r = Reader {
            data: &[0x20, 0x10],
            pos: 0,
        };
        assert!(area(&mut r, 0x40).is_none());
        let mut r = Reader {
            data: &[0x01, 0x10],
            pos: 0,
        };
        assert!(area(&mut r, 0xc0).is_none());
    }

    #[test]
    fn refuses_empty_and_zero_count() {
        assert!(decode_kt4(&[]).is_err());
        assert!(decode_kt4(&[0]).is_err());
    }
}
