//! `KT4` and `KTY` "Kitty" pictures (NEC PC-88 and PC-88 VA, 640x400 digital
//! 8 colors; `KTY` is the 640x200 variant).
//!
//! Sources:
//! - The ftz article on note.com (<https://note.com/ftz/n/n84d9dd98c1e2>),
//!   prose saying the format paints rectangles of one repeated 4x2 pixel tile.
//! - `spec/KTY_FORMAT.md` of <https://github.com/rururutan/ifkty> (MIT-declared;
//!   its provenance is unclear, so it was used only as a cross-check of the
//!   layout and of the mode 0 rules). No source file of that repository was
//!   opened.
//! - Reverse engineered from `20220101.kt4`, `anguishglassesjk.kt4` and
//!   `decreaseage.kt4` and by black-box probing of `recoil2png` with
//!   hand-made files (see `docs/research/msx-japanese.md`, "Wave 5b:
//!   Japanese", and `docs/research/next-blocked.md`, "KTY"). `recoil2png`
//!   decodes `.kty` exactly as `.kt4`; no genuine `.kty` file was found.
//!
//! The picture is a grid of 160 x 100 cells, each 4 pixels wide and 4 tall: a
//! top and a bottom tile of 4 x 2 pixels. A tile is three bytes (blue, red,
//! green plane), one bit per pixel, row-major, most significant bit first.
//! The file is a run of segments, each introduced by a mode byte `n`:
//! - `0xFF` starts the raw tail (below).
//! - Otherwise a tile follows: three bytes for modes 0 and 1, six bytes (top,
//!   then bottom) for any other mode. Mode 1 uses the tile for both halves of
//!   the cell. Mode 0 is the top half of a cell only: a file uses it in every
//!   segment or in none (`recoil2png` accepts some mixes, by a rule not worked
//!   out; they are refused here).
//! - Then a list of area entries ended by `0xFF`. The first byte's top two
//!   bits give the kind and the rest, with the second byte, a cell number
//!   `row * 160 + column`. Kind 1 (`0x40`): one more byte, the last column of a
//!   horizontal run. Kind 2 (`0x80`): one more byte, the last row of a vertical
//!   run. Kind 0: two more bytes, the last column and the last row of a
//!   rectangle. Kind 3 is refused.
//! - Then a list of single cells ended by `0xFF`: column, row.
//!
//! Every cell in the listed areas gets the segment's tile; a later segment
//! overwrites an earlier one. After `0xFF` come the tiles of every cell no
//! segment painted, in raster order, and the file ends exactly there: six
//! bytes (top and bottom tile) each, or three if every segment is mode 0 (or
//! there are none). Such a file is a 640x200 picture, drawn with every line
//! doubled.

use alloc::vec;
use alloc::vec::Vec;

use super::pc88_planes::palette;
use crate::{DecodeError, Image};

const COLUMNS: usize = 160;
const ROWS: usize = 100;
const WIDTH: usize = COLUMNS * 4;
const RAW_MARKER: u8 = 0xff;
/// How many times the cell count plus the input length the area fills may
/// write before the file is refused.
const WORK_FACTOR: usize = 4;

/// Top tile then bottom tile, three plane bytes each.
type Cell = [u8; 6];

/// A cell whose bottom half is blank.
fn top_only(tile: [u8; 3]) -> Cell {
    let [a, b, c] = tile;
    [a, b, c, 0, 0, 0]
}

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
    const BAD: DecodeError = DecodeError::Invalid;
    let mut reader = Reader { data, pos: 0 };
    let mut cells: Vec<Option<Cell>> = vec![None; COLUMNS * ROWS];
    // Whether the segments use mode 0 (all of them or none).
    let mut half_height: Option<bool> = None;
    // Areas may overlap, so bound the cells written by the input size.
    let mut budget = WORK_FACTOR * (COLUMNS * ROWS + data.len());

    loop {
        let count = reader.byte().ok_or(BAD)?;
        if count == RAW_MARKER {
            break;
        }
        if *half_height.get_or_insert(count == 0) != (count == 0) {
            return Err(BAD);
        }
        let cell: Cell = match count {
            0 => top_only(reader.take().ok_or(BAD)?),
            1 => {
                let [a, b, c]: [u8; 3] = reader.take().ok_or(BAD)?;
                [a, b, c, a, b, c]
            }
            _ => reader.take().ok_or(BAD)?,
        };
        loop {
            let first = reader.byte().ok_or(BAD)?;
            if first == RAW_MARKER {
                break;
            }
            let (columns, rows) = area(&mut reader, first).ok_or(BAD)?;
            budget = budget
                .checked_sub(columns.clone().count() * rows.clone().count())
                .ok_or(BAD)?;
            for row in rows {
                for column in columns.clone() {
                    cells[row * COLUMNS + column] = Some(cell);
                }
            }
        }
        loop {
            let column = reader.byte().ok_or(BAD)?;
            if column == RAW_MARKER {
                break;
            }
            let row = reader.byte().ok_or(BAD)?;
            let slot = cells
                .get_mut(usize::from(row) * COLUMNS + usize::from(column))
                .filter(|_| usize::from(row) < ROWS && usize::from(column) < COLUMNS)
                .ok_or(BAD)?;
            *slot = Some(cell);
        }
    }
    // No segments: 3-byte tiles, as `recoil2png` does.
    let full_height = half_height == Some(false);
    for slot in cells.iter_mut().filter(|slot| slot.is_none()) {
        *slot = Some(if full_height {
            reader.take().ok_or(BAD)?
        } else {
            top_only(reader.take().ok_or(BAD)?)
        });
    }
    if reader.pos != data.len() {
        return Err(BAD);
    }

    // A cell is two tiles of 2 rows, or one in a half-height picture.
    let halves = if full_height { 2 } else { 1 };
    let height = ROWS * halves * 2;
    let mut indices = vec![0u8; WIDTH * height];
    for (n, cell) in cells.iter().enumerate() {
        let cell = cell.as_ref().ok_or(BAD)?;
        let (x, y) = (n % COLUMNS * 4, n / COLUMNS * halves * 2);
        for (half, tile) in cell.chunks(3).take(halves).enumerate() {
            for bit in 0..8 {
                // Plane bytes are blue, red, green: color bits 0, 1, 2.
                let colour = tile
                    .iter()
                    .enumerate()
                    .fold(0u8, |c, (plane, byte)| c | (byte >> (7 - bit) & 1) << plane);
                let (px, py) = (x + bit % 4, y + half * 2 + bit / 4);
                indices[py * WIDTH + px] = colour;
            }
        }
    }
    let image = Image::from_indexed(WIDTH as u32, height as u32, &indices, &palette())?;
    // The 200-line picture is shown with every line doubled.
    if full_height {
        Ok(image)
    } else {
        image.scaled(1, 2)
    }
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
    fn repeated_full_screen_fills_are_bounded() {
        // One segment with 100000 whole-screen fills: 1.6e9 writes unbounded.
        let mut file = vec![2, 1, 2, 3, 4, 5, 6];
        for _ in 0..100_000 {
            file.extend_from_slice(&[0x00, 0, 0, 159, 99]);
        }
        file.extend_from_slice(&[0xff, 0xff, 0xff]);
        assert!(decode_kt4(&file).is_err());
    }

    #[test]
    fn mode_zero_is_a_half_height_picture() {
        // One mode 0 segment painting cell 0 with a blue 1010/0101 tile, then
        // 3-byte tiles for the other cells.
        let mut file = vec![0, 0xa5, 0, 0, 0xff, 0, 0, 0xff, 0xff];
        file.resize(file.len() + 3 * (COLUMNS * ROWS - 1), 0);
        let image = decode_kt4(&file).unwrap();
        assert_eq!((image.width(), image.height()), (640, 400));
        let blue = |x, y| image.get(x, y) == image.get(0, 0);
        // Rows 0 and 1 repeat the tile's first row, rows 2 and 3 the second.
        assert!(blue(0, 0) && blue(0, 1) && !blue(1, 0) && !blue(1, 1));
        assert!(!blue(0, 2) && !blue(0, 3) && blue(1, 2) && blue(1, 3));
        // Three-byte raw tiles only suit a file whose segments are all mode 0.
        file[0] = 2;
        assert!(decode_kt4(&file).is_err());
    }

    #[test]
    fn refuses_empty_and_mixed_modes() {
        assert!(decode_kt4(&[]).is_err());
        assert!(decode_kt4(&[0]).is_err());
        let mixed = [
            0, 0, 0, 0, 0xff, 0xff, 2, 0, 0, 0, 0, 0, 0, 0xff, 0xff, 0xff,
        ];
        assert!(decode_kt4(&mixed).is_err());
    }
}
