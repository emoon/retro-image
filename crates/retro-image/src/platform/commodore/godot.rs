//! GoDot 4Bit pictures (`.4bt`) and clips (`.clp`): 4 bits per pixel in
//! 8×8 tiles of 32 bytes, RLE packed.
//!
//! Sources:
//! - GoDot file formats, <https://godot64.de/german/4bitformate.htm>, and
//!   <https://www.godot64.de/german/4bit.htm> (tile layout, `$AD` RLE).
//! - GoDot's colour order (C64 colour to 4-bit index): the `dnib` table in
//!   GoDot's MIT-licensed Koala loader,
//!   <https://github.com/godot64/GoDot/blob/master/loaders/l_Koala.a>.
//! - Left pixel in the high nibble: checked against `recoil2png` output.

use super::vic2::rgb;
use crate::{DecodeError, Image};
use alloc::vec::Vec;

/// C64 colour of each GoDot 4-bit index (brightness order).
const COLORS: [u8; 16] = [0, 6, 9, 11, 2, 4, 8, 12, 14, 10, 5, 15, 3, 7, 13, 1];

/// Unpacks `$AD count value` runs (count 0 = 256) until `len` bytes.
fn unpack(packed: &[u8], len: usize) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(len);
    let mut bytes = packed.iter().copied();
    while out.len() < len {
        match bytes.next()? {
            0xad => {
                let count = bytes.next()?;
                let value = bytes.next()?;
                let count = if count == 0 { 256 } else { usize::from(count) };
                out.extend(core::iter::repeat_n(value, count));
            }
            byte => out.push(byte),
        }
    }
    out.truncate(len);
    Some(out)
}

/// Renders `columns`×`rows` tiles stored row by row.
fn render(tiles: &[u8], columns: usize, rows: usize) -> Image {
    let mut image = Image::new((columns * 8) as u32, (rows * 8) as u32);
    for (i, tile) in tiles.chunks_exact(32).enumerate() {
        let (tx, ty) = (i % columns * 8, i / columns * 8);
        for (j, &byte) in tile.iter().enumerate() {
            let (x, y) = (tx + j % 4 * 2, ty + j / 4);
            image.set(x as u32, y as u32, rgb(COLORS[usize::from(byte >> 4)]));
            image.set(x as u32 + 1, y as u32, rgb(COLORS[usize::from(byte & 15)]));
        }
    }
    image
}

/// `GOD0`, then 40×25 packed tiles.
pub(super) fn decode_4bt(data: &[u8]) -> Result<Image, DecodeError> {
    let packed = data
        .strip_prefix(b"GOD0")
        .ok_or(DecodeError::Unrecognized)?;
    let tiles = unpack(packed, 40 * 25 * 32).ok_or(DecodeError::Unrecognized)?;
    Ok(render(&tiles, 40, 25))
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
    let tiles = unpack(packed, columns * rows * 32).ok_or(DecodeError::Unrecognized)?;
    Ok(render(&tiles, columns, rows))
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
