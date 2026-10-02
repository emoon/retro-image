//! MUFLI Editor (`.mup`) and MUIFLI Editor (`.mui`).
//!
//! Sources: reverse engineered from the MUP and MUI samples by black-box
//! probing of `recoil2png`. (placeholder, expanded below)

use super::nufli::{screen, sprite_rows};
use super::unpack::backward_rle;
use super::vic2;
use crate::{DecodeError, Image};
use alloc::vec::Vec;

/// Start of the memory image of one frame.
const LOAD: usize = 0x2100;
/// Bytes of one frame (`$2100-$75FF`).
const LEN: usize = 0x5500;
const HEIGHT: usize = 200;
const BUG: usize = 24;
/// Colour tables of the six underlay sprites (as in NUFLI).
const COLOR_TABLES: [usize; 6] = [0x2400, 0x2480, 0x2800, 0x2880, 0x2c00, 0x2c80];

struct Frame<'a> {
    mem: &'a [u8],
    rows: [u8; HEIGHT],
}

impl Frame<'_> {
    fn byte(&self, addr: usize) -> Option<u8> {
        self.mem.get(addr.checked_sub(LOAD)?).copied()
    }

    fn bitmap_set(&self, x: usize, y: usize) -> Option<bool> {
        let offset = y / 8 * 320 + x / 8 * 8 + y % 8;
        let addr = if offset < 0x1400 {
            0x6000 + offset
        } else {
            0x3400 + offset - 0x1400
        };
        Some(self.byte(addr)? & (0x80 >> (x % 8)) != 0)
    }

    /// Colour of underlay sprite `s` for each line: the six tables work as
    /// in NUFLI (entry 0 is the initial colour, a nonzero high nibble keeps
    /// the previous colour, entry `k` colours lines `2k-1` and `2k`).
    fn underlay_colors(&self, s: usize) -> Option<[u8; HEIGHT]> {
        let table = COLOR_TABLES[s];
        let mut by_entry = [0u8; 101];
        let mut color = self.byte(table)? & 15;
        for (k, slot) in by_entry.iter_mut().enumerate() {
            let entry = self.byte(table + k)?;
            if k == 0 || entry >> 4 == 0 {
                color = entry & 15;
            }
            *slot = color;
        }
        Some(core::array::from_fn(|y| by_entry[y.div_ceil(2)]))
    }

    /// Colour of underlay sprite `s` on line `y`: white when the line's
    /// entry in the mask table at `$3300` has bit `s + 1` set.
    fn sprite_color(&self, colors: &[[u8; HEIGHT]; 6], s: usize, y: usize) -> Option<u8> {
        let mask = self.byte(0x3300 + y.div_ceil(2))?;
        Some(if mask >> (s + 1) & 1 != 0 {
            1
        } else {
            colors[s][y]
        })
    }

    fn pixel(&self, colors: &[[u8; HEIGHT]; 6], x: usize, y: usize) -> Option<u8> {
        let screen = screen(y / 2);
        let cell = self.byte(screen + x / 8)?;
        if self.bitmap_set(x, y)? {
            return Some(cell >> 4);
        }
        let s = x.checked_sub(BUG).map(|x| x / 48).filter(|&s| s < 6);
        if let Some(s) = s {
            let bank = screen & !0x3ff;
            let pointer = usize::from(self.byte(bank + 0x3f9 + s)?);
            let column = (x - BUG) % 48 / 2;
            let row = usize::from(self.rows[y] + (column / 8) as u8) & 0x3f;
            let addr = (bank & 0xc000) + pointer * 64 + row;
            if self.byte(addr)? & (0x80 >> (column % 8)) != 0 {
                return self.sprite_color(colors, s, y);
            }
        }
        Some(cell & 15)
    }

    fn pixels(&self) -> Option<Vec<u8>> {
        let mut colors = [[0; HEIGHT]; 6];
        for (s, slot) in colors.iter_mut().enumerate() {
            *slot = self.underlay_colors(s)?;
        }
        (0..HEIGHT)
            .flat_map(|y| (0..vic2::WIDTH).map(move |x| (x, y)))
            .map(|(x, y)| self.pixel(&colors, x, y))
            .collect()
    }
}

pub(super) fn decode_mup(data: &[u8]) -> Result<Image, DecodeError> {
    let [_, _, escape, packed @ ..] = data else {
        return Err(DecodeError::Unrecognized);
    };
    let mem = backward_rle(packed, *escape, LEN).ok_or(DecodeError::Unrecognized)?;
    let pixels = Frame {
        mem: &mem,
        rows: sprite_rows(),
    }
    .pixels().ok_or(DecodeError::Unrecognized)?;
    Ok(vic2::Frame::from_fn(HEIGHT, |x, y| pixels[y * vic2::WIDTH + x]).to_image(BUG))
}
