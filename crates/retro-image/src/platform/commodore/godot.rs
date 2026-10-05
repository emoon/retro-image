//! GoDot 4Bit pictures (`.4bt`) and clips (`.clp`): 4 bits per pixel in
//! 8×8 tiles of 32 bytes, RLE packed.
//!
//! Sources:
//! - GoDot file formats, <https://godot64.de/german/4bitformate.htm>, and
//!   <https://www.godot64.de/german/4bit.htm> (tile layout, `$AD` RLE).
//! - GoDot's color order (C64 color to 4-bit index): the `dnib` table in
//!   GoDot's MIT-licensed Koala loader,
//!   <https://github.com/godot64/GoDot/blob/master/loaders/l_Koala.a>.
//! - Left pixel in the high nibble: checked against `recoil2png` output.
//!
//! The color order table is taken from GoDot (`loaders/l_Koala.a`), whose
//! notice is:
//!
//! ```text
//! MIT License
//!
//! Copyright (c) 2021 Arndt Dettke & Wolfgang Kling
//!
//! Permission is hereby granted, free of charge, to any person obtaining a copy
//! of this software and associated documentation files (the "Software"), to deal
//! in the Software without restriction, including without limitation the rights
//! to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
//! copies of the Software, and to permit persons to whom the Software is
//! furnished to do so, subject to the following conditions:
//!
//! The above copyright notice and this permission notice shall be included in all
//! copies or substantial portions of the Software.
//!
//! THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//! IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//! FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
//! AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//! LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//! OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
//! SOFTWARE.
//! ```

use super::unpack::{Run, escape_rle};
use super::vic2::rgb;
use crate::tiles::TileLayout;
use crate::{BitOrder, DecodeError, Image};
use alloc::vec::Vec;

/// C64 color of each GoDot 4-bit index (brightness order).
const COLORS: [u8; 16] = [0, 6, 9, 11, 2, 4, 8, 12, 14, 10, 5, 15, 3, 7, 13, 1];

/// A tile: 32 bytes, 4 bytes per row, the left pixel in the high nibble.
const TILE: TileLayout = TileLayout::packed(4, BitOrder::MsbFirst);

/// Unpacks `$AD count value` runs (count 0 = 256) to exactly `len` bytes.
fn unpack(packed: &[u8], len: usize) -> Option<Vec<u8>> {
    escape_rle(packed, 0xad, Run::CountValue, len).filter(|out| out.len() == len)
}

/// Draws the tiles, stored row by row, `columns` to a row.
fn render(tiles: &[u8], columns: usize) -> Result<Image, DecodeError> {
    TILE.sheet(tiles, columns, &COLORS.map(rgb))
}

/// `GOD0`, then 40×25 packed tiles.
pub(super) fn decode_4bt(data: &[u8]) -> Result<Image, DecodeError> {
    let packed = data
        .strip_prefix(b"GOD0")
        .ok_or(DecodeError::Unrecognized)?;
    let tiles = unpack(packed, 40 * 25 * TILE.tile_len()).ok_or(DecodeError::Unrecognized)?;
    render(&tiles, 40)
}

/// `GOD1`, start row and column, width and height in tiles, packed tiles.
pub(super) fn decode_clp(data: &[u8]) -> Result<Image, DecodeError> {
    let rest = data
        .strip_prefix(b"GOD1")
        .ok_or(DecodeError::Unrecognized)?;
    let [_, _, columns, rows, packed @ ..] = rest else {
        return Err(DecodeError::Unrecognized);
    };
    let (columns, rows) = (usize::from(*columns), usize::from(*rows));
    if columns == 0 || rows == 0 {
        return Err(DecodeError::Unrecognized);
    }
    let tiles =
        unpack(packed, columns * rows * TILE.tile_len()).ok_or(DecodeError::Unrecognized)?;
    render(&tiles, columns)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unpacks_runs() {
        assert_eq!(
            unpack(&[1, 0xad, 2, 9, 3], 4),
            Some(alloc::vec![1, 9, 9, 3])
        );
        assert_eq!(unpack(&[0xad, 0, 7], 256).map(|v| v.len()), Some(256));
        assert_eq!(unpack(&[1], 2), None);
    }

    #[test]
    fn clip_tile_uses_high_nibble_first() {
        let mut data = b"GOD1\0\0\x01\x01".to_vec();
        data.push(0xf1);
        data.extend([0xad, 31, 0]);
        let image = decode_clp(&data).unwrap();
        assert_eq!(&image.rgb()[..6], &[0xff, 0xff, 0xff, 0x35, 0x28, 0x79]);
    }
}
