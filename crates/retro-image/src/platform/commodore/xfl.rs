//! X-FLI Editor (`.xfl`): a 192×167 hires FLI picture with eight multicolor
//! sprites behind it.
//!
//! Sources: no documentation found (see `docs/research/commodore.md`, "Wave 6:
//! VHI and X-FLI"). Reverse engineered from the four corpus samples
//! (`q17`, `rol47`, `soph07`, `ns09`) and the X-FLI workstages on the Ray of
//! Light "ROL Work" and Homoerotic "nswork" disks, by black-box probing of
//! `recoil2png` with modified and random copies.
//!
//! The file is `00 40`, an escape byte, then the memory `$4000-$7F3F` packed
//! backwards (`value count escape`, count 0 = 256), which must fill it
//! exactly. Offsets below are from `$4000`:
//!
//! - Eight screen RAMs at `1024 * n` and a hires bitmap at `$2000`. Picture
//!   line `y` is bitmap line `y + 1` and uses screen `(y + 7) % 8`; only cell
//!   columns 16..39 are shown. Set bits take the screen's high nibble.
//! - Where a bitmap bit is clear, eight multicolor sprites (24 pixels each,
//!   side by side) show through; their pair `00` is the screen's low nibble.
//!   Line `y` takes the sprites of set `y % 8` (see [`block`]), row
//!   `(y - FIRST_LINE[set]) / 2` mod 21.
//! - Color registers: index 1 and 2 are the pair `01` and `11` colors, 3..10
//!   the pair `10` colors of sprites 0..7. They start at screen 7's offsets
//!   `1005 + index`. Screens 0..2 each hold 28 changes, the new color at
//!   `960 + k` and the register index (low nibble) at `988 + k`; they take
//!   effect from line `3 + 56 * screen + 2 * k`. Indexes 0 and 11..15 do
//!   nothing.

use super::unpack::backward_rle_filled;
use super::vic2::image;
use crate::{DecodeError, Image};
use alloc::vec::Vec;

const WIDTH: usize = 192;
const HEIGHT: usize = 167;
const MEM_LEN: usize = 0x3f40;
const SCREEN_STRIDE: usize = 1024;
const BITMAP: usize = 0x2000;
/// First shown cell column.
const FIRST_COLUMN: usize = 16;
/// Color registers: unused, pair 01, pair 11, then sprites 0..7.
const REGISTERS: usize = 11;
/// Screen 7 offset of the initial register values.
const INITIAL: usize = 7 * SCREEN_STRIDE + 1005;
/// Screens holding color changes, and changes per screen.
const CHANGE_SCREENS: usize = 3;
const CHANGES: usize = 28;
/// Per sprite set (`y % 8`): a line where its row 0 shows.
const FIRST_LINE: [usize; 8] = [0, 1, 42, 43, 84, 85, 126, 127];

/// X-FLI Editor.
pub(super) fn decode_xfl(data: &[u8]) -> Result<Image, DecodeError> {
    let [0x00, 0x40, escape, packed @ ..] = data else {
        return Err(DecodeError::Invalid);
    };
    match backward_rle_filled(packed, *escape, MEM_LEN) {
        Some((mem, true)) => Ok(render(&mem)),
        _ => Err(DecodeError::Invalid),
    }
}

/// Renders the unpacked memory (exactly [`MEM_LEN`] bytes).
fn render(mem: &[u8]) -> Image {
    let mut registers: [u8; REGISTERS] = core::array::from_fn(|i| mem[INITIAL + i]);
    let mut colors = Vec::with_capacity(WIDTH * HEIGHT);
    for y in 0..HEIGHT {
        apply_changes(mem, y, &mut registers);
        let line = y + 1;
        let screen = &mem[(y + 7) % 8 * SCREEN_STRIDE..];
        let set = y % 8;
        let row = (y + 168 - FIRST_LINE[set]) / 2 % 21;
        for x in 0..WIDTH {
            let cell = line / 8 * 40 + FIRST_COLUMN + x / 8;
            let byte = mem[BITMAP + cell * 8 + line % 8];
            let color = screen[cell];
            if byte << (x % 8) & 0x80 != 0 {
                colors.push(color >> 4);
                continue;
            }
            let sprite = x / 24;
            let start = block(set, sprite) + row * 3 + x % 24 / 8;
            let pair = mem[start] >> (6 - x % 8 / 2 * 2) & 3;
            colors.push(match pair {
                0 => color,
                1 => registers[1],
                3 => registers[2],
                _ => registers[3 + sprite],
            });
        }
    }
    image(WIDTH, HEIGHT, colors)
}

/// Applies the color changes that take effect on line `y`.
fn apply_changes(mem: &[u8], y: usize, registers: &mut [u8; REGISTERS]) {
    let Some(rel) = y.checked_sub(3).filter(|rel| rel % 2 == 0) else {
        return;
    };
    let (screen, k) = (rel / 2 / CHANGES, rel / 2 % CHANGES);
    if screen >= CHANGE_SCREENS {
        return;
    }
    let base = screen * SCREEN_STRIDE;
    let index = usize::from(mem[base + 960 + CHANGES + k] & 15);
    if (1..REGISTERS).contains(&index) {
        registers[index] = mem[base + 960 + k];
    }
}

/// Offset of the 63-byte block shown by `sprite` (0..8) of `set` (0..8).
fn block(set: usize, sprite: usize) -> usize {
    const SET0: [usize; 8] = [
        0x3900, 0x3940, 0x3a40, 0x3a80, 0x3ac0, 0x3b00, 0x3b40, 0x3b80,
    ];
    const SET1: [usize; 8] = [
        0x3bc0, 0x3c00, 0x3c40, 0x3c80, 0x3cc0, 0x3d80, 0x3dc0, 0x3e00,
    ];
    match set {
        0 => SET0[sprite],
        1 => SET1[sprite],
        2 => sprite * SCREEN_STRIDE + 0x380,
        // Two blocks in the free cells (columns 0..15) of each bitmap row.
        _ => BITMAP + (set - 3) * 0x500 + sprite / 2 * 320 + sprite % 2 * 64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Packs `mem` as literals; escape `$73` is written as a run of one.
    fn pack(mem: &[u8]) -> Vec<u8> {
        let mut out = alloc::vec![0x00, 0x40, 0x73];
        for &b in mem {
            if b == 0x73 {
                out.extend([b, 1, 0x73]);
            } else {
                out.push(b);
            }
        }
        out
    }

    #[test]
    fn sprite_colour_changes_from_their_line() {
        let mut mem = alloc::vec![0u8; MEM_LEN];
        // Sprite 0 pair 10 in every block, every row.
        for set in 0..8 {
            for row in 0..21 {
                mem[block(set, 0) + row * 3] = 0x80;
            }
        }
        mem[INITIAL + 3] = 2;
        // Screen 1, change 4: sprite 0 becomes 5 from line 3 + 56 + 8.
        mem[SCREEN_STRIDE + 960 + 4] = 5;
        mem[SCREEN_STRIDE + 988 + 4] = 0x13;
        let picture = decode_xfl(&pack(&mem)).unwrap();
        assert_eq!(picture.get(0, 66), 0x68372b);
        assert_eq!(picture.get(0, 67), 0x588d43);
        assert_eq!(picture.get(8, 67), 0x000000);
    }

    #[test]
    fn rejects_inexact_streams() {
        let data = pack(&[1; MEM_LEN]);
        assert!(decode_xfl(&data).is_ok());
        assert!(decode_xfl(&data[..data.len() - 1]).is_err());
        let mut longer = data.clone();
        longer.insert(3, 1);
        assert!(decode_xfl(&longer).is_err());
        let mut moved = data;
        moved[1] = 0x41;
        assert!(decode_xfl(&moved).is_err());
    }
}
