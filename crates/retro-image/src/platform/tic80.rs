//! TIC-80 cartridges (`.tic`), shown as their cover image or sprite sheet.
//!
//! Sources:
//! - The chunk layout, chunk types and their sizes: the TIC-80 wiki page
//!   ".tic File Format" (<https://github.com/nesbox/TIC-80/wiki/.tic-File-Format>,
//!   MIT) and the palette page (<https://github.com/nesbox/TIC-80/wiki/palette>,
//!   MIT), which lists the SWEETIE-16 colors used below.
//! - Which palette a cartridge gets, and the DB16 colors: `src/cart.c` of
//!   <https://github.com/nesbox/TIC-80> (MIT, copyright notice below).
//! - Reverse engineered from the 38 cartridges of `build/assets/*.tic.dat` in
//!   that repository (zlib-compressed `.tic` files written as C arrays,
//!   unpacked with Python's `zlib`) and the two `.tic` files in its
//!   `templates/nim`, in `corpus/extra/small-consoles/tic80`: the chunk
//!   chains of all 40 end exactly at the end of the file, and the font cartridge
//!   (`font.tic`) is legible only with the low nibble as the left pixel,
//!   which the wiki does not say.
//!
//! A file is a chain of chunks: a header byte (bank in bits 5 to 7, type in
//! bits 0 to 4), a little-endian size and a reserved byte, then the data.
//! Only bank 0 is read. The picture is, in order of preference:
//! - the cover, the screen chunk (type 18, 240 x 136 pixels, two to a byte)
//!   unless it is all zero;
//! - the sprite sheet: the tile chunk (type 1) and the sprite chunk (type
//!   2), 256 8 x 8 sprites each, 16 to a row, each part cut after its last
//!   non-blank row and left out if it is missing or blank.
//!
//! A cartridge with neither, because the chunks are missing or blank, is
//! rejected. The cover is untested: no sample has a screen chunk, so its
//! layout is taken from the wiki ("a 240 x 136 x 4bpp raw buffer") and from
//! the sprite nibble order.
//!
//! Palette: the palette chunk's first 48 bytes (16 RGB colors; the second
//! palette, for the overlay, is ignored); the default chunk selects SWEETIE-16;
//! a cartridge with neither, or an all-zero palette, gets DB16, as TIC-80
//! does for old cartridges.
//!
//! Detection: by the `.tic` extension only. The chunk chain must use known
//! types and end exactly at the end of the file, but there is no magic number,
//! and random bytes pass that test now and then. PNG cartridges (`.tic.png`)
//! are not read.

// The two palettes follow the TIC-80 wiki and its src/cart.c
// (TIC-80, https://github.com/nesbox/TIC-80):
//
// MIT License
//
// Copyright (c) 2017 Vadim Grigoruk
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use alloc::vec::Vec;

use crate::bytes::le16;
use crate::tiles::TileLayout;
use crate::{BitOrder, DecodeError, Format, Image};

pub(super) static FORMATS: &[Format] = &[Format::new("TIC-80", "Cartridge", &["tic"], decode)];

/// A sprite: 8 x 8 pixels, two to a byte, the first in the low nibble.
const SPRITE: TileLayout = TileLayout::packed(4, BitOrder::LsbFirst);
/// The screen: 240 x 136 pixels in the same packing.
const SCREEN: TileLayout = TileLayout {
    width: 240,
    height: 136,
    ..SPRITE
};
const SPRITES_PER_ROW: usize = 16;
/// Bytes of a row of the sprite sheet, and of the 256 sprites of a chunk.
const ROW_LEN: usize = SPRITES_PER_ROW * SPRITE.tile_len();
const CHUNK_LEN: usize = 256 * SPRITE.tile_len();

const TILES: u8 = 1;
const SPRITES: u8 = 2;
const CODE: u8 = 5;
const PALETTE: u8 = 12;
const DEFAULT: u8 = 17;
const SCREEN_CHUNK: u8 = 18;
const BINARY: u8 = 19;
/// Every chunk type of the format; the others are reserved.
const KNOWN: [u8; 17] = [1, 2, 3, 4, 5, 6, 9, 10, 12, 13, 14, 15, 16, 17, 18, 19, 20];

#[rustfmt::skip]
const SWEETIE_16: [u32; 16] = [
    0x1a1c2c, 0x5d275d, 0xb13e53, 0xef7d57, 0xffcd75, 0xa7f070, 0x38b764, 0x257179,
    0x29366f, 0x3b5dc9, 0x41a6f6, 0x73eff7, 0xf4f4f4, 0x94b0c2, 0x566c86, 0x333c57,
];
#[rustfmt::skip]
const DB16: [u32; 16] = [
    0x140c1c, 0x442434, 0x30346d, 0x4e4a4e, 0x854c30, 0x346524, 0xd04648, 0x757161,
    0x597dce, 0xd27d2c, 0x8595a1, 0x6daa2c, 0xd2aa99, 0x6dc2ca, 0xdad45e, 0xdeeed6,
];

struct Chunk<'a> {
    bank: u8,
    kind: u8,
    body: &'a [u8],
}

/// The chunks of a file, or `None` if it is not a chain of chunks of known
/// types that ends exactly at the end of the file.
fn chunks(data: &[u8]) -> Option<Vec<Chunk<'_>>> {
    let mut chunks = Vec::new();
    let mut at = 0;
    while at < data.len() {
        let (bank, kind) = (data[at] >> 5, data[at] & 31);
        let size = usize::from(le16(data, at + 1)?);
        // A size of 0 stands for 65536 in the two types that can be that long.
        let len = if size == 0 && matches!(kind, CODE | BINARY) {
            1 << 16
        } else {
            size
        };
        if !KNOWN.contains(&kind) {
            return None;
        }
        let body = data.get(at + 4..at + 4 + len)?;
        chunks.push(Chunk { bank, kind, body });
        at += 4 + len;
    }
    (!chunks.is_empty()).then_some(chunks)
}

/// The palette of bank 0: the last palette or default chunk wins, an empty
/// palette becomes DB16.
fn palette(chunks: &[Chunk]) -> [u32; 16] {
    let mut colors = [0; 16];
    for chunk in chunks.iter().filter(|c| c.bank == 0) {
        match chunk.kind {
            PALETTE => {
                let mut rgb = [0; 48];
                let len = chunk.body.len().min(rgb.len());
                rgb[..len].copy_from_slice(&chunk.body[..len]);
                for (color, bytes) in colors.iter_mut().zip(rgb.as_chunks::<3>().0) {
                    *color = u32::from_be_bytes([0, bytes[0], bytes[1], bytes[2]]);
                }
            }
            DEFAULT => colors = SWEETIE_16,
            _ => {}
        }
    }
    if colors == [0; 16] { DB16 } else { colors }
}

/// The bank 0 chunk of type `kind`.
fn bank_0<'a>(chunks: &'a [Chunk], kind: u8) -> Option<&'a [u8]> {
    chunks
        .iter()
        .find(|c| c.bank == 0 && c.kind == kind)
        .map(|c| c.body)
}

/// The sprites of a chunk up to the end of the last row that is not blank.
fn used_rows(body: &[u8]) -> Vec<u8> {
    let body = &body[..body.len().min(CHUNK_LEN)];
    let used = body
        .iter()
        .rposition(|&b| b != 0)
        .map_or(0, |last| (last / ROW_LEN + 1) * ROW_LEN);
    let mut rows = body[..used.min(body.len())].to_vec();
    rows.resize(used, 0);
    rows
}

fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    let chunks = chunks(data).ok_or(DecodeError::Unrecognized)?;
    let palette = palette(&chunks);
    if let Some(screen) = bank_0(&chunks, SCREEN_CHUNK).filter(|s| s.iter().any(|&b| b != 0)) {
        let mut pixels = screen[..screen.len().min(SCREEN.tile_len())].to_vec();
        pixels.resize(SCREEN.tile_len(), 0);
        return SCREEN.sheet(&pixels, 1, &palette);
    }
    let parts = [bank_0(&chunks, TILES), bank_0(&chunks, SPRITES)];
    let sprites: Vec<u8> = parts.iter().flatten().flat_map(|p| used_rows(p)).collect();
    // Nothing but blank sprites and no cover is not a picture.
    if sprites.is_empty() {
        return Err(DecodeError::Unrecognized);
    }
    SPRITE.sheet(&sprites, SPRITES_PER_ROW, &palette)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A chunk header and body.
    fn chunk(bank: u8, kind: u8, body: &[u8]) -> Vec<u8> {
        let mut out = alloc::vec![bank << 5 | kind];
        out.extend_from_slice(&(body.len() as u16).to_le_bytes());
        out.push(0);
        out.extend_from_slice(body);
        out
    }

    #[test]
    fn sprites_start_with_the_low_nibble_and_blank_parts_are_left_out() {
        let mut tile = [0u8; 32];
        tile[0] = 0x21; // pixels 0 and 1 of row 0
        let mut cart = chunk(0, DEFAULT, &[]);
        cart.extend(chunk(0, TILES, &[0; 64])); // blank
        cart.extend(chunk(0, SPRITES, &tile));
        let image = decode(&cart).unwrap();
        // One row of 16 sprites: the blank tiles are gone.
        assert_eq!((image.width(), image.height()), (128, 8));
        assert_eq!(
            (image.get(0, 0), image.get(1, 0)),
            (SWEETIE_16[1], SWEETIE_16[2])
        );
        // Only bank 0 counts.
        assert!(decode(&chunk(1, SPRITES, &tile)).is_err());
    }

    #[test]
    fn palettes_default_to_db16_and_a_chunk_overrides_the_default() {
        let mut cart = chunk(0, SPRITES, &[0x10]);
        assert_eq!(decode(&cart).unwrap().get(0, 0), DB16[0]);
        assert_eq!(decode(&cart).unwrap().get(1, 0), DB16[1]);
        let mut rgb = [0u8; 48];
        rgb[..3].copy_from_slice(&[1, 2, 3]);
        cart.extend(chunk(0, DEFAULT, &[]));
        cart.extend(chunk(0, PALETTE, &rgb));
        assert_eq!(decode(&cart).unwrap().get(0, 0), 0x01_0203);
    }

    #[test]
    fn the_cover_wins_over_the_sprites_and_is_240_by_136() {
        let mut screen = alloc::vec![0u8; 16384];
        screen[1] = 0x30; // pixel 3 of row 0
        let mut cart = chunk(0, SPRITES, &[0x10]);
        cart.extend(chunk(0, SCREEN_CHUNK, &screen));
        let image = decode(&cart).unwrap();
        assert_eq!((image.width(), image.height()), (240, 136));
        assert_eq!(image.get(3, 0), DB16[3]);
    }

    #[test]
    fn a_cartridge_is_found_by_its_extension_and_not_by_its_content() {
        let cart = chunk(0, SPRITES, &[0x10; 32]);
        assert!(crate::decode("game.tic", &cart).is_ok());
        assert!(crate::decode("game", &cart).is_err());
    }

    #[test]
    fn chains_must_end_at_the_end_of_the_file() {
        let mut cart = chunk(0, SPRITES, &[0x10; 32]);
        assert!(decode(&cart).is_ok());
        cart.push(0);
        assert!(decode(&cart).is_err());
        // A reserved type, and a size past the end.
        assert!(chunks(&chunk(0, 7, &[1])).is_none());
        assert!(chunks(&[SPRITES, 9, 0, 0, 1]).is_none());
    }

    #[test]
    fn carts_without_a_non_blank_sprite_or_cover_are_rejected() {
        // A bare empty tile chunk, which a few random bytes can look like.
        assert!(decode(&[TILES, 0, 0, 0]).is_err());
        assert!(decode(&chunk(0, SPRITES, &[0; 64])).is_err());
        let mut blank = chunk(0, TILES, &[0; 32]);
        blank.extend(chunk(0, SCREEN_CHUNK, &[0; 16384]));
        assert!(decode(&blank).is_err());
        // One non-blank sprite is enough.
        blank.extend(chunk(0, SPRITES, &[0, 0, 1]));
        assert!(decode(&blank).is_ok());
    }
}
