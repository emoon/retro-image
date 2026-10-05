//! Flipnote Studio movie (`.ppm`, DSi): the 64x48 thumbnail stored in the
//! header, which shows the preview frame.
//!
//! Sources:
//! - GBATEK, "DSi SD/MMC Flipnote Files" (<https://problemkaputt.de/gbatek.htm>,
//!   no license stated, so facts only): `PARA` at 0, the thumbnail at 0xA0,
//!   0x600 bytes of 4-bit pixels in 8x6 tiles of 8x8 (the doc does not give the
//!   nibble order or the tile order beyond that).
//! - The 16-color thumbnail palette is taken from `jaames/flipnote.js`
//!   (<https://github.com/jaames/flipnote.js>, MIT, notice below), file
//!   `src/parsers/PpmParser.ts`, `PPM_THUMB_PALETTE`. Entries that no sample
//!   uses are the pure green that table has for them.
//! - Reverse engineered from samples: the low nibble is the left pixel and the
//!   tiles go in raster order. Checked against the 49 distinct files of
//!   `corpus/extra/nintendo-rom-icons/flipnote`, whose preview frame
//!   flipnote.js renders (run as a black box); each thumbnail matches that frame
//!   scaled down.
//!
//! Only the thumbnail is drawn, not the 256x192 frames. The magic alone is a
//! weak signature, so a file is accepted only if the `u16` at 0x0E is 0x24 (as
//! GBATEK says it always is, and as in all 49 samples) and the header, the
//! animation data and the audio data, whose sizes are the `u32` values at 4
//! and 8, fit in the file.

// Parts of this file follow jaames/flipnote.js, src/parsers/PpmParser.ts:
//
// MIT License
//
// Copyright (c) 2018 James Daniel
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

use crate::bytes::{le16, le32};
use crate::tiles::TileLayout;
use crate::{BitOrder, DecodeError, Image};

const MAGIC: &[u8] = b"PARA";
const THUMBNAIL_AT: usize = 0xa0;
const THUMBNAIL_LEN: usize = 0x600;
const TILES_PER_ROW: usize = 8;
/// The `u16` at 0x0E in every file, and where the animation data starts.
const FIXED_AT: usize = 0x0e;
const FIXED: u16 = 0x24;
const ANIMATION_AT: usize = 0x6a0;

/// 8x8 tiles of 4-bit pixels, the left pixel in the low nibble.
const TILE: TileLayout = TileLayout::packed(4, BitOrder::LsbFirst);

/// The thumbnail colors as `0xRRGGBB`.
#[rustfmt::skip]
const PALETTE: [u32; 16] = [
    0xffffff, 0x525252, 0xffffff, 0x9c9c9c,
    0xff4844, 0xc8514f, 0xffadac, 0x00ff00,
    0x4840ff, 0x514fb8, 0xadabff, 0x00ff00,
    0xb657b7, 0x00ff00, 0x00ff00, 0x00ff00,
];

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    if !data.starts_with(MAGIC) || le16(data, FIXED_AT) != Some(FIXED) {
        return Err(fail);
    }
    let (animation, audio) = (le32(data, 4).ok_or(fail)?, le32(data, 8).ok_or(fail)?);
    let end = ANIMATION_AT
        .checked_add(animation as usize)
        .and_then(|end| end.checked_add(audio as usize));
    if end.is_none_or(|end| end > data.len()) {
        return Err(fail);
    }
    let tiles = data
        .get(THUMBNAIL_AT..THUMBNAIL_AT + THUMBNAIL_LEN)
        .ok_or(fail)?;
    TILE.sheet(tiles, TILES_PER_ROW, &PALETTE)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A header-sized file with the animation and audio sizes given.
    fn flipnote(animation: u32, audio: u32) -> alloc::vec::Vec<u8> {
        let mut file = alloc::vec![0; ANIMATION_AT + (animation + audio) as usize];
        file[..4].copy_from_slice(MAGIC);
        file[4..8].copy_from_slice(&animation.to_le_bytes());
        file[8..12].copy_from_slice(&audio.to_le_bytes());
        file[FIXED_AT..FIXED_AT + 2].copy_from_slice(&FIXED.to_le_bytes());
        file
    }

    #[test]
    fn text_that_starts_with_para_is_not_a_flipnote() {
        let mut text = b"PARAMETERS: the usual list of options and what they do".to_vec();
        text.resize(4096, b' ');
        assert!(decode(&text).is_err());
        let mut file = flipnote(100, 50);
        assert!(decode(&file).is_ok());
        // The fixed word, and sizes that do not fit the file.
        file[FIXED_AT] = 0x25;
        assert!(decode(&file).is_err());
        let mut short = flipnote(100, 50);
        short.pop();
        assert!(decode(&short).is_err());
        let mut huge = flipnote(0, 0);
        huge[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(decode(&huge).is_err());
    }

    #[test]
    fn the_thumbnail_is_6_rows_of_8_tiles_with_the_low_nibble_first() {
        let mut file = flipnote(0, 0);
        file[THUMBNAIL_AT] = 0x41; // tile 0, row 0: colors 1 then 4
        file[THUMBNAIL_AT + 32 * 8] = 0x08; // tile 8 starts the second tile row
        let image = decode(&file).unwrap();
        assert_eq!((image.width(), image.height()), (64, 48));
        assert_eq!(image.get(0, 0), 0x525252);
        assert_eq!(image.get(1, 0), 0xff4844);
        assert_eq!(image.get(0, 8), 0x4840ff);
        assert!(decode(&file[..file.len() - 1]).is_err());
        file[0] = b'Q';
        assert!(decode(&file).is_err());
    }
}
