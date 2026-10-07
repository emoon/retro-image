//! Monochrome tile-based pictures: Printfox and Pagefox (Scanntronik) `.bs`
//! screens (320×200), `.gb` large pictures (640×400) and `.pg` Pagefox
//! pages, and Star Painter `.gr`/`.cs`.
//!
//! Sources:
//! - Star Painter: GoDot StarPntr loader page,
//!   <https://www.godot64.de/german/l_starp.htm> (width and height in tiles,
//!   then the bitmap in tiles); tile order and colors checked against
//!   `recoil2png` output.
//! - GoDot PFoxSelect loader page, <https://www.godot64.de/german/l_pfoxs.htm>:
//!   type byte (`B`, `G`, `P`), sizes in 8×8 tiles, `$9B count value` RLE
//!   with a word count for `B`/`G` and a byte count for `P`, Pagefox
//!   header (height, width, `K` contour data ended by `$00`), black on white.
//! - <http://fileformats.archiveteam.org/wiki/Printfox_bitmap>
//! - Tile order (row by row) and the low-byte-first word count checked
//!   against `recoil2png` output.

use crate::{DecodeError, Image};
use alloc::vec::Vec;

/// Unpacks `$9B count value` runs (count 0 = 256) until `len` bytes.
fn unpack(packed: &[u8], word_count: bool, len: usize) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(len);
    let mut bytes = packed.iter().copied();
    while out.len() < len {
        match bytes.next()? {
            0x9b => {
                let mut count = usize::from(bytes.next()?);
                if word_count {
                    count |= usize::from(bytes.next()?) << 8;
                }
                let value = bytes.next()?;
                let count = if count == 0 { 256 } else { count };
                out.extend(core::iter::repeat_n(value, count));
            }
            byte => out.push(byte),
        }
    }
    out.truncate(len);
    Some(out)
}

/// Renders `columns`×`rows` tiles of 8 bytes, row by row; set bits are black.
pub(super) fn render(tiles: &[u8], columns: usize, rows: usize) -> Result<Image, DecodeError> {
    let mut image = Image::new((columns * 8) as u32, (rows * 8) as u32)?;
    for (i, &byte) in tiles.iter().enumerate().take(columns * rows * 8) {
        let tile = i / 8;
        let (x, y) = (tile % columns * 8, tile / columns * 8 + i % 8);
        for bit in 0..8 {
            let color = if byte & (0x80 >> bit) != 0 {
                0
            } else {
                0xffffff
            };
            image.set((x + bit) as u32, y as u32, color);
        }
    }
    Ok(image)
}

fn decode(data: &[u8], kind: u8, columns: usize, rows: usize) -> Result<Image, DecodeError> {
    let packed = match data {
        [first, packed @ ..] if *first == kind => packed,
        _ => return Err(DecodeError::Invalid),
    };
    let tiles = unpack(packed, true, columns * rows * 8).ok_or(DecodeError::Invalid)?;
    render(&tiles, columns, rows)
}

/// Printfox screen: `B`, 40×25 tiles.
pub(super) fn decode_bs(data: &[u8]) -> Result<Image, DecodeError> {
    decode(data, b'B', 40, 25)
}

/// Printfox large picture: `G`, 80×50 tiles.
pub(super) fn decode_gb(data: &[u8]) -> Result<Image, DecodeError> {
    decode(data, b'G', 80, 50)
}

/// Star Painter picture (`.gr`) or construction set (`.cs`): width and
/// height in tiles, then the tiles, unpacked.
pub(super) fn decode_star_painter(data: &[u8]) -> Result<Image, DecodeError> {
    let [columns, rows, tiles @ ..] = data else {
        return Err(DecodeError::Invalid);
    };
    let (columns, rows) = (usize::from(*columns), usize::from(*rows));
    if columns == 0 || rows == 0 || tiles.len() != columns * rows * 8 {
        return Err(DecodeError::Invalid);
    }
    render(tiles, columns, rows)
}

/// Pagefox page: `P`, height and width in tiles, `K` contour data up to
/// `$00`, then the tiles.
pub(super) fn decode_pg(data: &[u8]) -> Result<Image, DecodeError> {
    let [b'P', rows, columns, b'K', rest @ ..] = data else {
        return Err(DecodeError::Invalid);
    };
    let (columns, rows) = (usize::from(*columns), usize::from(*rows));
    let end = rest
        .iter()
        .position(|&b| b == 0)
        .ok_or(DecodeError::Invalid)?;
    if columns == 0 || rows == 0 {
        return Err(DecodeError::Invalid);
    }
    let tiles = unpack(&rest[end + 1..], false, columns * rows * 8).ok_or(DecodeError::Invalid)?;
    render(&tiles, columns, rows)
}
