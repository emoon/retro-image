//! CharPad projects (`.ctm` version 5): a character set, tiles built from
//! characters and a map of tiles, rendered as the whole map.
//!
//! Sources:
//! - CTM version 4 description (header fields, section order, character
//!   attributes `MMMMCCCC`),
//!   <https://github.com/martinpiper/C64Public/blob/master/ExternalTools/CharPad/Docs/CharPad%20-%20CTM%20(V4)%20Format.txt>
//!   (documentation only).
//! - Version 5 differences, found by matching section sizes in sample files
//!   and checked against `recoil2png` output: a 20-byte header with a
//!   16-bit tile count, 16-bit tile cells and map entries, colours taken
//!   from the character attributes; bit 2 of the flags byte makes every
//!   character multicolour (`01`/`10` the shared multicolours, `11` the
//!   attribute colour bits 0-2).

use super::vic2::rgb;
use crate::{DecodeError, Image};

fn word(data: &[u8], at: usize) -> usize {
    usize::from(u16::from_le_bytes([data[at], data[at + 1]]))
}

pub(super) fn decode_ctm(data: &[u8]) -> Result<Image, DecodeError> {
    let header = data.get(..20).ok_or(DecodeError::Unrecognized)?;
    // Only the colour mode and flags seen in samples (2; 1 or 5).
    if &header[..4] != b"CTM\x05" || header[8] != 2 || !matches!(header[9], 1 | 5) {
        return Err(DecodeError::Unrecognized);
    }
    let [background, multi1, multi2] = [header[4], header[5], header[6]];
    let multicolor = header[9] & 4 != 0;
    let chars = word(header, 10) + 1;
    let tiles = word(header, 12) + 1;
    let (tile_width, tile_height) = (usize::from(header[14]), usize::from(header[15]));
    let (map_width, map_height) = (word(header, 16), word(header, 18));
    let tile_cells = tile_width * tile_height;
    let char_data = 20;
    let attributes = char_data + chars * 8;
    let tile_data = attributes + chars;
    let map_data = tile_data + tiles * tile_cells * 2;
    let width = map_width * tile_width * 8;
    let height = map_height * tile_height * 8;
    if data.len() != map_data + map_width * map_height * 2
        || tile_cells == 0
        || width == 0
        || height == 0
        || width * height > 1 << 24
    {
        return Err(DecodeError::Unrecognized);
    }
    let mut image = Image::new(width as u32, height as u32);
    for y in 0..height {
        for x in 0..width {
            let (cell_x, cell_y) = (x / 8, y / 8);
            let tile = word(
                data,
                map_data + (cell_y / tile_height * map_width + cell_x / tile_width) * 2,
            );
            let cell = cell_y % tile_height * tile_width + cell_x % tile_width;
            let char = word(data, tile_data + (tile % tiles * tile_cells + cell) * 2) % chars;
            let byte = data[char_data + char * 8 + y % 8];
            let color = data[attributes + char] & 15;
            let index = if multicolor {
                match byte >> (6 - (x & 6)) & 3 {
                    0 => background,
                    1 => multi1,
                    2 => multi2,
                    _ => color & 7,
                }
            } else if byte & (0x80 >> (x % 8)) != 0 {
                color
            } else {
                background
            };
            image.set(x as u32, y as u32, rgb(index));
        }
    }
    Ok(image)
}
