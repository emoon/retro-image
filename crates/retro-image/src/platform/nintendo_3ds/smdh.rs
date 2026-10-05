//! Nintendo 3DS SMDH (`.smdh`): the 48x48 icon the Home Menu shows. The same
//! block sits inside 3DSX and CIA files, which find it and call [`icon`].
//!
//! Sources:
//! - GBATEK, "3DS Files - Video Icons (SMDH)" and the texture swizzling page
//!   (<https://problemkaputt.de/gbatek.htm>, no licence stated, so facts
//!   only), cross-checked with 3dbrew, "SMDH"
//!   (<https://www.3dbrew.org/wiki/SMDH>, facts only): `SMDH` at 0, the block
//!   is 0x36C0 bytes, a small 24x24 icon at 0x2040 and a large 48x48 icon at
//!   0x24C0, both RGB565 in little-endian words; 8x8 tiles in plain raster
//!   order, no padding to a power of two, and inside a tile the pixels in
//!   Z-order (x on the even bits of the index, y on the odd ones).
//! - Reverse engineered from samples: the large icons of Universal-Updater
//!   (`.3dsx` and `.cia`) in `corpus/extra/nintendo-rom-icons` come out as
//!   clean pictures.
//!
//! Only the large icon is drawn. GBATEK says it is unknown whether any
//! transparency exists, and RGB565 has no alpha, so the icon is opaque.

use crate::bytes::le16;
use crate::morton::morton_index;
use crate::{DecodeError, Image};

const MAGIC: &[u8] = b"SMDH";
/// Bytes of an SMDH block.
pub(super) const LEN: usize = 0x36c0;
const LARGE_ICON_AT: usize = 0x24c0;
const SIDE: usize = 48;
const TILE_SIDE: usize = 8;

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    icon(data)
}

/// The large icon of an SMDH block that starts at the beginning of `block`.
pub(super) fn icon(block: &[u8]) -> Result<Image, DecodeError> {
    if !block.starts_with(MAGIC) {
        return Err(DecodeError::Unrecognized);
    }
    let pixels = block
        .get(LARGE_ICON_AT..LEN)
        .ok_or(DecodeError::Unrecognized)?;
    let color = |x: usize, y: usize| {
        let tile = y / TILE_SIDE * (SIDE / TILE_SIDE) + x / TILE_SIDE;
        let within = morton_index((x % TILE_SIDE) as u32, (y % TILE_SIDE) as u32) as usize;
        le16(pixels, (tile * TILE_SIDE * TILE_SIDE + within) * 2).map_or(0, rgb565)
    };
    Ok(Image::from_colors(
        SIDE as u32,
        SIDE as u32,
        (0..SIDE).flat_map(|y| (0..SIDE).map(move |x| color(x, y))),
    ))
}

/// 16-bit color: red in bits 11-15, green 5-10, blue 0-4.
fn rgb565(word: u16) -> u32 {
    let word = u32::from(word);
    let (r, g, b) = (word >> 11, word >> 5 & 63, word & 31);
    (r << 3 | r >> 2) << 16 | (g << 2 | g >> 4) << 8 | (b << 3 | b >> 2)
}

/// An SMDH block whose large icon holds the given 16-bit words at the given
/// word indices (counted from the start of the icon), and zeros elsewhere.
#[cfg(test)]
pub(super) fn test_block(words: &[(usize, u16)]) -> alloc::vec::Vec<u8> {
    let mut block = alloc::vec![0; LEN];
    block[..4].copy_from_slice(MAGIC);
    for &(index, word) in words {
        let at = LARGE_ICON_AT + index * 2;
        block[at..at + 2].copy_from_slice(&word.to_le_bytes());
    }
    block
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb565_expands_each_channel_to_8_bits() {
        assert_eq!(rgb565(0xffff), 0xff_ffff);
        assert_eq!(rgb565(0xf800), 0xff_0000);
        assert_eq!(rgb565(0x07e0), 0x00_ff00);
        assert_eq!(rgb565(0x001f), 0x00_00ff);
        assert_eq!(rgb565(0x0020), 0x00_0400);
    }

    #[test]
    fn pixels_are_z_ordered_inside_tiles_laid_out_in_rows() {
        // Pixel 1 of a tile is one to the right, 2 one down, 4 two to the
        // right. Tile 1 is the second tile of the first tile row; tile 6 the
        // first of the second.
        let image = icon(&test_block(&[
            (1, 0xf800),
            (2, 0x07e0),
            (4, 0x001f),
            (64 + 3, 0xffff),
            (6 * 64, 0xffe0),
        ]))
        .unwrap();
        assert_eq!((image.width(), image.height()), (48, 48));
        assert_eq!(image.get(1, 0), 0xff0000);
        assert_eq!(image.get(0, 1), 0x00ff00);
        assert_eq!(image.get(2, 0), 0x0000ff);
        assert_eq!(image.get(8 + 1, 1), 0xffffff);
        assert_eq!(image.get(0, 8), 0xffff00);
        assert_eq!(image.get(0, 0), 0);
    }

    #[test]
    fn a_short_or_unmarked_block_is_rejected() {
        let good = test_block(&[]);
        assert!(icon(&good).is_ok());
        assert!(icon(&good[..LEN - 1]).is_err());
        let mut unmarked = good;
        unmarked[0] = b'X';
        assert!(icon(&unmarked).is_err());
    }
}
