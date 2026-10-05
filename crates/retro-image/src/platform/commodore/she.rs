//! Super Hires Editor (`.she`): a small hires picture with two layers of
//! hires sprites over it.
//!
//! Sources: no documentation found (see `docs/research/commodore.md`,
//! "Wave 6: PetDraw and Super Hires Editor"). Reverse engineered from one
//! CSDb sample (`doggy.she`) by black-box probing of `recoil2png` with
//! modified copies; the layout is a memory dump (load address ignored):
//!
//! - `2..1058`: the hires bitmap, 12×11 cells of eight bytes, row-major.
//! - `1058..1190`: one screen RAM byte per cell (set pixels: high nibble).
//! - `1190..3238`: 32 sprites of 64 bytes (63 used), four bands of eight:
//!   in each band the first four are the lower layer and the next four the
//!   upper layer, one sprite per 24-pixel column, 21 lines per band. Lines
//!   84-87 have no sprites.
//! - `3238`, `3239`: colors of the lower and the upper layer.
//! - 10 more ignored bytes: the file is exactly 3250 bytes.
//!
//! The upper sprite layer wins over the lower one, which wins over the
//! bitmap. The picture is 96×88. `recoil2png` takes a second, larger
//! layout of 8642 bytes (192×168) for the same extension; no sample exists
//! and it is not decoded.

use super::superhires::{NARROW, ROW, bit, hires, render};
use crate::{DecodeError, Image};

const HEIGHT: usize = 88;
const BITMAP: usize = 2;
const SCREEN: usize = BITMAP + ROW * 11 * 8;
const SPRITES: usize = SCREEN + ROW * 11;
const COLORS: usize = SPRITES + 32 * 64;
const LEN: usize = COLORS + 12;
/// Lines covered by the four bands of 21-line sprites.
const SPRITE_LINES: usize = 84;

/// Super Hires Editor.
pub(super) fn decode_she(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != LEN {
        return Err(DecodeError::Unrecognized);
    }
    Ok(render(NARROW, HEIGHT, |x, y| {
        if y < SPRITE_LINES {
            let sprite_set = |layer: usize| {
                let sprite = y / 21 * 8 + layer * 4 + x / 24;
                let start = SPRITES + sprite * 64 + y % 21 * 3;
                bit(&data[start..start + 3], x % 24)
            };
            // The upper layer's color is stored second.
            if sprite_set(1) {
                return data[COLORS + 1];
            }
            if sprite_set(0) {
                return data[COLORS];
            }
        }
        let cell = y / 8 * ROW + x / 8;
        let byte = data[BITMAP + cell * 8 + y % 8];
        hires(bit(&[byte], x % 8), data[SCREEN + cell])
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upper_layer_wins_over_lower_and_bitmap() {
        let mut data = alloc::vec![0u8; LEN];
        data[SCREEN] = 0x21; // red set pixels on white
        data[COLORS] = 5; // lower layer: green
        data[COLORS + 1] = 6; // upper layer: blue
        data[BITMAP] = 0x80;
        assert_eq!(decode_she(&data).unwrap().get(0, 0), 0x68372b);
        data[SPRITES] = 0x80; // lower layer, first column
        assert_eq!(decode_she(&data).unwrap().get(0, 0), 0x588d43);
        data[SPRITES + 4 * 64] = 0x80; // upper layer
        assert_eq!(decode_she(&data).unwrap().get(0, 0), 0x352879);
        assert!(decode_she(&data[1..]).is_err());
    }
}
