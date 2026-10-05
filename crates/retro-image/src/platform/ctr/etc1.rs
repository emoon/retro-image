//! ETC1 and ETC1A4 blocks, as the 3DS GPU stores them.
//!
//! Sources: GBATEK, "3DS GPU Texture Formats" (ETC1 and ETC1A4,
//! <https://problemkaputt.de/gbatek.htm>, no license, facts only), and the
//! Khronos extension `OES_compressed_ETC1_RGB8_texture`
//! (<https://registry.khronos.org/OpenGL/extensions/OES/OES_compressed_ETC1_RGB8_texture.txt>),
//! the format's specification, for the modifier tables and the meaning of each
//! field. Checked against `texture2ddecoder` (MIT), run as a black box on
//! random blocks: see `tests/divergences/nintendo-resources.tsv`.
//!
//! A block is 4 x 4 pixels in 64 bits, stored little-endian on the 3DS (the
//! specification writes it big-endian, so the bits below are those of the
//! little-endian value):
//! - bits 0-15: low bit of each pixel's index; 16-31: high bit; pixel
//!   `4 * x + y`, so the pixels run down the columns;
//! - bit 32: flip (0 splits the block into a left and a right half, 1 into
//!   an upper and a lower half); bit 33: differential mode;
//! - bits 34-36 and 37-39: the modifier table of the second and the first half;
//! - bits 40-63: the two base colors. Individually (bit 33 clear) each is 4
//!   bits a channel: red 60-63 and 56-59, green 52-55 and 48-51, blue 44-47 and
//!   40-43, first and second color; each widens by repeating the nibble.
//!   Differentially the first is 5 bits a channel at bits 59-63, 51-55 and
//!   43-47 and the second adds a signed 3-bit change (bits 56-58, 48-50 and
//!   40-42) to it, before both widen to 8 bits by repeating the top bits.
//!   A second color outside 0 to 31 is undefined; it is clamped here.
//!
//! A pixel is its half's base color plus a modifier from its table, in each
//! channel, clamped to 0..=255. The index (high bit, low bit) picks one of
//! the table's four values: 00 the small positive, 01 the large positive, 10
//! the small negative and 11 the large negative one. ETC1A4 puts 64 bits of
//! 4-bit alpha (pixel 0 in bits 0-3) before the ETC1 block.

use crate::image::widen_channel;

/// The (small, large) modifier of each table.
const MODIFIERS: [(i16, i16); 8] = [
    (2, 8),
    (5, 17),
    (9, 29),
    (13, 42),
    (18, 60),
    (24, 80),
    (33, 106),
    (47, 183),
];

/// The 16 pixels of an ETC1 block `block` as red, green, blue and alpha, pixel
/// `4 * x + y` at `4 * x + y`. `alpha` is the 64-bit alpha of an ETC1A4
/// block, and a block without it is opaque.
pub(super) fn decode_block(block: u64, alpha: Option<u64>) -> [[u8; 4]; 16] {
    let field = |shift: u32, bits: u32| (block >> shift & ((1 << bits) - 1)) as i16;
    let flip = field(32, 1) != 0;
    let differential = field(33, 1) != 0;
    let (first, second) = if differential {
        let signed = |v: i16| if v >= 4 { v - 8 } else { v };
        let widen = |v: i16| widen_channel(v.clamp(0, 31) as u32, 5) as u8;
        let channel = |shift: u32| {
            let base = field(shift + 3, 5);
            [widen(base), widen(base + signed(field(shift, 3)))]
        };
        let (red, green, blue) = (channel(56), channel(48), channel(40));
        ([red[0], green[0], blue[0]], [red[1], green[1], blue[1]])
    } else {
        let widen = |v: i16| widen_channel(v as u32, 4) as u8;
        let channel = |shift: u32| [widen(field(shift + 4, 4)), widen(field(shift, 4))];
        let (red, green, blue) = (channel(56), channel(48), channel(40));
        ([red[0], green[0], blue[0]], [red[1], green[1], blue[1]])
    };
    let tables = [field(37, 3) as usize, field(34, 3) as usize];
    let bases = [first, second];
    let mut pixels = [[0; 4]; 16];
    for (k, pixel) in pixels.iter_mut().enumerate() {
        let (x, y) = (k / 4, k % 4);
        let half = usize::from(if flip { y >= 2 } else { x >= 2 });
        let low = block >> k & 1;
        let high = block >> (16 + k) & 1;
        let (small, large) = MODIFIERS[tables[half]];
        let modifier = match (high, low) {
            (0, 0) => small,
            (0, _) => large,
            (_, 0) => -small,
            _ => -large,
        };
        let [r, g, b] = bases[half].map(|c| (i16::from(c) + modifier).clamp(0, 255) as u8);
        let a = alpha.map_or(255, |alpha| {
            widen_channel((alpha >> (4 * k) & 15) as u32, 4) as u8
        });
        *pixel = [r, g, b, a];
    }
    pixels
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An individual-mode block from the 4-bit base colors and the table
    /// numbers of its two halves, and the 32 pixel index bits.
    fn individual(r: [u64; 2], g: [u64; 2], b: [u64; 2], tables: [u64; 2], indices: u32) -> u64 {
        r[0] << 60
            | r[1] << 56
            | g[0] << 52
            | g[1] << 48
            | b[0] << 44
            | b[1] << 40
            | tables[0] << 37
            | tables[1] << 34
            | u64::from(indices)
    }

    #[test]
    fn the_individual_mode_widens_nibbles_and_adds_the_modifier() {
        // R1 = 14, G1 = 3, B1 = 8 widen to (238, 51, 136), as in the
        // specification; table 2 is (9, 29). Pixel 0 has index 00 (+9),
        // pixel 1 index 01 (+29), pixel 2 index 10 (-9) and pixel 3
        // index 11 (-29): low bits in 0-15, high bits in 16-31.
        let indices = 0b1010 | 0b1100 << 16;
        let block = individual([14, 0], [3, 0], [8, 0], [2, 0], indices);
        let pixels = decode_block(block, None);
        assert_eq!(pixels[0], [247, 60, 145, 255]);
        assert_eq!(pixels[1], [255, 80, 165, 255]);
        assert_eq!(pixels[2], [229, 42, 127, 255]);
        assert_eq!(pixels[3], [209, 22, 107, 255]);
        // The right half has base color 0 and table 0 (2, 8): pixel 8 (x = 2).
        assert_eq!(pixels[8], [2, 2, 2, 255]);
    }

    #[test]
    fn the_differential_mode_adds_a_signed_change_before_widening() {
        // The specification's example: R1' = 28, G1' = 4, B1' = 3 give
        // (231, 33, 24); the changes -4, +2 and 0 give (198, 49, 24).
        let block = 28u64 << 59 | 0b100 << 56 | 4 << 51 | 2 << 48 | 3 << 43 | 1 << 33;
        let pixels = decode_block(block, None);
        // Index 00 is +2 with table 0, in the left half (first color) and the
        // right half (second color).
        assert_eq!(pixels[0], [233, 35, 26, 255]);
        assert_eq!(pixels[8], [200, 51, 26, 255]);
        // A change that leaves 0..=31 is clamped: 31 + 3 stays 31.
        let over = 31u64 << 59 | 3 << 56 | 1 << 33;
        assert_eq!(decode_block(over, None)[8][0], 255);
    }

    #[test]
    fn flip_splits_the_block_across_instead_of_down() {
        // The first color is white, the second black (+2 from its table).
        let block = individual([15, 0], [15, 0], [15, 0], [0, 0], 0);
        let side = decode_block(block, None);
        // Pixel k is (x = k / 4, y = k % 4): the left two columns are the
        // first color.
        assert_eq!((side[7][0], side[8][0]), (255, 2));
        let stacked = decode_block(block | 1 << 32, None);
        assert_eq!((stacked[1][0], stacked[2][0]), (255, 2));
        assert_eq!((stacked[13][0], stacked[14][0]), (255, 2));
    }

    #[test]
    fn etc1a4_alpha_runs_over_the_pixels_a_nibble_each() {
        let alpha = 1 | 8 << 4 | 0xf << 60;
        let pixels = decode_block(0, Some(alpha));
        assert_eq!(pixels[0][3], 17);
        assert_eq!(pixels[1][3], 0x88);
        assert_eq!(pixels[15][3], 255);
        assert_eq!(pixels[2][3], 0);
        assert_eq!(decode_block(0, None)[2][3], 255);
    }
}
