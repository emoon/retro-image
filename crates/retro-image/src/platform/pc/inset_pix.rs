//! Inset Systems PIX pictures (`.PIX`, version 3): the neutral format of
//! InSet and HiJaak, also used by WordStar, MultiMate and PC-Write clip art.
//!
//! Sources:
//! - Inset Systems, "InSet PIX File Specification" (`PIX.TXT`, 1989 or so, no
//!   license stated; the prose is used as documentation). The page that
//!   hosts it is gone; the Wayback Machine has it at
//!   <https://web.archive.org/web/20251213080443/https://www.fileformat.info/format/inset/spec/3d22a7693c404eb28a1704e212fb1bd5/view.htm>.
//!   It gives the file as two words (the revision, 3, and the item count) and
//!   an index of items (id, length, 32-bit location; id `FFFF` is empty), the
//!   image information item (id 0: type, columns and rows at 18 and 20, bit
//!   planes at 22), the palette (id 1: four bytes per entry, intensity, red,
//!   green and blue), the tile information (id 2: rows and columns of a tile,
//!   tiles down and across) and the tiles (id `8000` plus the number, left to
//!   right and then down). A tile is bit planes of rows of whole bytes, most
//!   significant bit leftmost, a plane after plane. The first row of a plane is
//!   stored as it is; each later row is a bit mask with one bit per byte of
//!   the row (the most significant bit first) and then the bytes whose bit is
//!   set; the others repeat the row above. Tiles are at most 4096 bytes
//!   unpacked, and rows past the bottom of the picture are not stored.
//! - What the spec leaves open comes from Deark's `insetpix` module
//!   (`insetpix.c`; <https://github.com/jsummers/deark>, MIT license, notice
//!   below): the four palette sizes (the fields the spec calls bits hold the
//!   number of levels, 64 in the samples with 6-bit VGA values) and, for 4
//!   levels, that the values 1 and 2 are swapped (an EGA primary and
//!   secondary intensity: 1 is two thirds, 2 is one third). The spec says
//!   nothing of the mask size for rows wider than 8 bytes: it is one bit per
//!   byte, `ceil(bytes / 8)` mask bytes, and every tile in every sample
//!   unpacks to exactly its stored length.
//! - Only bitmap pictures (not the text-screen kind), with an intensity
//!   count of 0 and equal red, green and blue levels, are decoded: that is
//!   all the samples have (Sembiance `insetPix`, 11 files, and the clip art
//!   on the PC-Write disk of the Programmer's Software Library CD,
//!   `psl/psl9309/DOS/PCWRITE/PIX/` on cd.textfiles.com, 29 files, 9 of them
//!   also in the first set). The tiny `flag_b24.pix` and `tru256.pix` have
//!   4 levels, and the color names drawn into the picture (`YEL`, `GRN`,
//!   `RED`, `MAG`) match the colors the swap gives. All 40 files match Deark
//!   pixel for pixel (see `dos-clipart.tsv`); the corpus group
//!   `corpus/extra/dos-clipart/` keeps 19 of them.
//! - Other revisions exist (the survey note `docs/research/gaps-computers-extra.md`
//!   C11 counts two incompatible ones) and are rejected. The aspect ratio
//!   bytes are not applied.

// Parts of this file follow Deark's modules/insetpix.c
// (Deark, https://github.com/jsummers/deark):
//
// Copyright (C) 2016-2024 Jason Summers
// <jason1@pobox.com>
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

use alloc::vec::Vec;

use crate::bytes::{le16, le32};
use crate::image::{check_size, planar_pixels};
use crate::{DecodeError, Image};

const FAIL: DecodeError = DecodeError::Unrecognized;
const REVISION: u16 = 3;
const INDEX_AT: usize = 4;
const ITEM_LEN: usize = 8;
/// Items in a file; Deark takes 500 as the limit.
const MAX_ITEMS: usize = 500;
const ID_IMAGE: u16 = 0;
const ID_PALETTE: u16 = 1;
const ID_TILE_INFO: u16 = 2;
const FIRST_TILE: u16 = 0x8000;
const EMPTY: u16 = 0xffff;
/// Bytes of unpacked pixels in a tile, all planes together.
const MAX_TILE_LEN: usize = 4096;
const IMAGE_INFO_LEN: usize = 32;
const TILE_INFO_LEN: usize = 8;
const PALETTE_ENTRY_LEN: usize = 4;

/// An item of the index and the bytes it covers.
struct Item<'a> {
    id: u16,
    data: &'a [u8],
}

fn items(data: &[u8]) -> Result<Vec<Item<'_>>, DecodeError> {
    if le16(data, 0) != Some(REVISION) {
        return Err(FAIL);
    }
    let count = usize::from(le16(data, 2).ok_or(FAIL)?);
    let index_end = INDEX_AT + count * ITEM_LEN;
    if !(4..=MAX_ITEMS).contains(&count) || index_end > data.len() {
        return Err(FAIL);
    }
    let mut items = Vec::with_capacity(count);
    for entry in data[INDEX_AT..index_end].as_chunks::<ITEM_LEN>().0 {
        let id = le16(entry, 0).ok_or(FAIL)?;
        if id == EMPTY {
            continue;
        }
        let len = usize::from(le16(entry, 2).ok_or(FAIL)?);
        let at = le32(entry, 4)
            .and_then(|at| usize::try_from(at).ok())
            .filter(|&at| at >= index_end)
            .ok_or(FAIL)?;
        let data = data.get(at..at.checked_add(len).ok_or(FAIL)?).ok_or(FAIL)?;
        items.push(Item { id, data });
    }
    Ok(items)
}

/// The 8-bit value of a palette sample that has `levels` levels.
fn level(value: u8, levels: u8) -> u32 {
    // Four levels are an EGA channel: the bit for two thirds comes first.
    if levels == 4 {
        return [0, 0xaa, 0x55, 0xff][usize::from(value.min(3))];
    }
    let top = u32::from(levels - 1);
    (u32::from(value).min(top) * 255 * 2 + top) / (2 * top)
}

/// The palette of `colors` entries. Entries the file lacks are black.
fn palette(item: &[u8], levels: [u8; 3], colors: usize) -> Vec<u32> {
    let mut palette: Vec<u32> = item
        .as_chunks::<PALETTE_ENTRY_LEN>()
        .0
        .iter()
        .take(colors)
        .map(|entry| {
            let [_, red, green, blue] = *entry;
            level(red, levels[0]) << 16 | level(green, levels[1]) << 8 | level(blue, levels[2])
        })
        .collect();
    palette.resize(colors, 0);
    palette
}

/// One tile's planes of `rows` rows of `row_len` bytes, plane after plane.
/// Returns `None` if `coded` ends early; bytes after the tile are ignored.
fn unpack_tile(coded: &[u8], planes: usize, rows: usize, row_len: usize) -> Option<Vec<u8>> {
    let mask_len = row_len.div_ceil(8);
    let mut out = Vec::with_capacity(planes * rows * row_len);
    let mut rest = coded;
    for _ in 0..planes {
        for row in 0..rows {
            if row == 0 {
                let (bytes, tail) = rest.split_at_checked(row_len)?;
                out.extend_from_slice(bytes);
                rest = tail;
                continue;
            }
            let (mask, mut tail) = rest.split_at_checked(mask_len)?;
            let above = out.len() - row_len;
            for i in 0..row_len {
                if mask[i / 8] >> (7 - i % 8) & 1 != 0 {
                    let (&byte, after) = tail.split_first()?;
                    out.push(byte);
                    tail = after;
                } else {
                    out.push(out[above + i]);
                }
            }
            rest = tail;
        }
    }
    Some(out)
}

pub(super) fn decode_pix(data: &[u8]) -> Result<Image, DecodeError> {
    let items = items(data)?;
    let item = |id| items.iter().find(|i| i.id == id).map(|i| i.data);
    let info = item(ID_IMAGE)
        .filter(|i| i.len() >= IMAGE_INFO_LEN)
        .ok_or(FAIL)?;
    let tile_info = item(ID_TILE_INFO)
        .filter(|i| i.len() >= TILE_INFO_LEN)
        .ok_or(FAIL)?;
    let palette_item = item(ID_PALETTE).ok_or(FAIL)?;

    // Bitmap, not text.
    if info[1] & 1 == 0 {
        return Err(FAIL);
    }
    let width = usize::from(le16(info, 18).ok_or(FAIL)?);
    let height = usize::from(le16(info, 20).ok_or(FAIL)?);
    let planes = usize::from(info[22]);
    let [intensity, red, green, blue] = [info[25], info[26], info[27], info[28]];
    if !(1..=8).contains(&planes) || intensity != 0 || red < 2 || red != green || red != blue {
        return Err(FAIL);
    }
    check_size(width, height)?;

    let tile_field = |at| usize::from(le16(tile_info, at).unwrap_or(0));
    let (tile_rows, tile_columns) = (tile_field(0), tile_field(2));
    let (strips_down, strips_across) = (tile_field(4), tile_field(6));
    let row_len = tile_columns / 8;
    if tile_rows == 0
        || row_len == 0
        || tile_columns % 8 != 0
        || planes * tile_rows * row_len > MAX_TILE_LEN
        || strips_across * tile_columns < width
        || strips_down * tile_rows < height
    {
        return Err(FAIL);
    }
    let tiles: Vec<&Item<'_>> = items.iter().filter(|i| i.id >= FIRST_TILE).collect();
    // Every tile is there, which also bounds the picture by the file.
    if tiles.len() != strips_across * strips_down {
        return Err(FAIL);
    }

    let mut indices = alloc::vec![0u8; width * height];
    for tile in tiles {
        let number = usize::from(tile.id - FIRST_TILE);
        if number >= strips_across * strips_down {
            return Err(FAIL);
        }
        let (left, top) = (
            number % strips_across * tile_columns,
            number / strips_across * tile_rows,
        );
        // A tile wholly past the bottom or the right edge shows nothing:
        // the rows below the picture are not stored, and a tile of columns
        // past the right edge is padding.
        if top >= height || left >= width {
            continue;
        }
        let rows = tile_rows.min(height - top);
        let unpacked = unpack_tile(tile.data, planes, rows, row_len).ok_or(FAIL)?;
        let values = planar_pixels(
            &unpacked,
            tile_columns,
            rows,
            row_len,
            planes,
            |plane, y| (plane * rows + y) * row_len,
        );
        let visible = tile_columns.min(width - left);
        for (y, row) in values.chunks_exact(tile_columns).enumerate() {
            let start = (top + y) * width + left;
            for (pixel, &value) in indices[start..start + visible].iter_mut().zip(row) {
                *pixel = value as u8;
            }
        }
    }
    let palette = palette(palette_item, [red, green, blue], 1 << planes);
    Image::from_indexed(width as u32, height as u32, &indices, &palette)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_repeat_the_row_above_unless_their_mask_bit_is_set() {
        // 2 planes of 3 rows of 2 bytes. Plane 0: [1,2], mask 0b01 (second
        // byte changes to 9), mask 0 (no change). Plane 1: [3,4], mask 0b11
        // (5, 6), mask 0b10 (7).
        let coded = [
            1, 2, 0x40, 9, 0x00, //
            3, 4, 0xc0, 5, 6, 0x80, 7,
        ];
        let tile = unpack_tile(&coded, 2, 3, 2).unwrap();
        assert_eq!(tile, [1, 2, 1, 9, 1, 9, 3, 4, 5, 6, 7, 6]);
        // Short data, and rows wider than 8 bytes take 2 mask bytes.
        assert!(unpack_tile(&coded[..11], 2, 3, 2).is_none());
        let wide: Vec<u8> = (1..=9)
            .chain([0, 0])
            .chain([0x80, 0x00])
            .chain([7])
            .collect();
        let tile = unpack_tile(&wide, 1, 3, 9).unwrap();
        assert_eq!(tile[9..18], tile[..9]);
        assert_eq!(tile[18..], [7, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn levels_scale_to_8_bits_and_four_levels_are_ega() {
        assert_eq!(level(63, 64), 255);
        assert_eq!(level(32, 64), 130);
        assert_eq!(level(1, 2), 255);
        assert_eq!(
            [0, 1, 2, 3].map(|v| level(v, 4)),
            [0, 0xaa, 0x55, 0xff],
            "primary before secondary"
        );
        assert_eq!(level(200, 64), 255);
    }

    /// A picture `width` pixels wide and 2 rows tall, 1 plane, with the
    /// given tile information (rows, columns, strips down, strips across)
    /// and one item per tile.
    fn file_of(width: u8, tile_info: [u16; 4], tiles: &[&[u8]], palette: &[u8]) -> Vec<u8> {
        let mut info = alloc::vec![0u8; IMAGE_INFO_LEN];
        info[1] = 1;
        info[18] = width;
        info[20] = 2;
        info[22] = 1;
        info[25..29].copy_from_slice(&[0, 64, 64, 64]);
        let tile_info: Vec<u8> = tile_info.iter().flat_map(|v| v.to_le_bytes()).collect();
        let mut sections: Vec<(u16, &[u8])> = alloc::vec![
            (ID_IMAGE, &info),
            (ID_PALETTE, palette),
            (ID_TILE_INFO, &tile_info),
        ];
        for (number, tile) in tiles.iter().enumerate() {
            sections.push((FIRST_TILE + number as u16, tile));
        }
        let mut data = alloc::vec![3, 0, sections.len() as u8, 0];
        let mut at = INDEX_AT + sections.len() * ITEM_LEN;
        for &(id, bytes) in &sections {
            data.extend_from_slice(&id.to_le_bytes());
            data.extend_from_slice(&(bytes.len() as u16).to_le_bytes());
            data.extend_from_slice(&(at as u32).to_le_bytes());
            at += bytes.len();
        }
        for (_, bytes) in sections {
            data.extend_from_slice(bytes);
        }
        data
    }

    /// A 12 x 2 picture in one tile of 2 rows x 16 columns.
    fn file(coded: &[u8], palette: &[u8]) -> Vec<u8> {
        file_of(12, [2, 16, 1, 1], &[coded], palette)
    }

    #[test]
    fn a_one_tile_picture_is_cropped_and_colored_from_the_palette() {
        // Row 0: 0x80 0xff (pixel 0 and pixels 8 to 15 set, the picture ends
        // at 11); row 1 repeats the first byte and has 0x00 for the second.
        let coded = [0x80, 0xff, 0x40, 0x00];
        let palette = [0, 0, 0, 0, 0, 63, 0, 0];
        let image = decode_pix(&file(&coded, &palette)).unwrap();
        assert_eq!((image.width(), image.height()), (12, 2));
        assert_eq!(image.get(0, 0), 0xff0000);
        assert_eq!(image.get(1, 0), 0);
        assert_eq!(image.get(11, 0), 0xff0000);
        assert_eq!((image.get(0, 1), image.get(8, 1)), (0xff0000, 0));
    }

    #[test]
    fn tiles_side_by_side_fill_the_picture_and_tiles_past_its_edge_are_skipped() {
        let palette = [0, 0, 0, 0, 0, 63, 0, 0];
        // Two tiles of 2 rows x 16 columns across a 20 pixel picture: the
        // first all set, the second set in its first 4 pixels (row 1 repeats
        // row 0, as its mask byte is 0).
        let tiles: [&[u8]; 2] = [&[0xff, 0xff, 0x00], &[0xf0, 0x00, 0x00]];
        let data = file_of(20, [2, 16, 1, 2], &tiles, &palette);
        let image = decode_pix(&data).unwrap();
        assert_eq!((image.width(), image.height()), (20, 2));
        assert_eq!(
            [image.get(15, 0), image.get(16, 1), image.get(19, 0)],
            [0xff0000; 3]
        );
        // The same tiles for a picture 12 pixels wide: the second tile lies
        // wholly past the right edge and is not drawn.
        let data = file_of(12, [2, 16, 1, 2], &tiles, &palette);
        let image = decode_pix(&data).unwrap();
        assert_eq!((image.width(), image.height()), (12, 2));
        assert_eq!(image.get(11, 1), 0xff0000);
        // Nor is a tile row past the bottom.
        let row: &[u8] = &[0xff, 0xff];
        let data = file_of(12, [1, 16, 4, 1], &[row; 4], &palette);
        assert_eq!(decode_pix(&data).unwrap().height(), 2);
    }

    #[test]
    fn other_revisions_and_missing_items_are_rejected() {
        let good = file(&[0x80, 0xff, 0x40, 0x00], &[0; 8]);
        let mut bad = good.clone();
        bad[0] = 2;
        assert!(decode_pix(&bad).is_err());
        let mut bad = good.clone();
        bad[2] = 3; // three items cannot hold all four
        assert!(decode_pix(&bad).is_err());
        let mut bad = good;
        bad[INDEX_AT + ITEM_LEN] = 9; // the palette is now item 9
        assert!(decode_pix(&bad).is_err());
    }
}
