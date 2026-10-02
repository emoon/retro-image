//! UFLI-editor (`.ufl`, Crest): hires FLI with a new screen RAM every
//! second line and a layer of six X-expanded hires sprites under the
//! bitmap's clear pixels.
//!
//! Sources:
//! - Parts and memory map (sprites from `$4000`, sprite colour `$4FF0`,
//!   four screen RAMs, bitmap `$6000`): Codebase64 "C64 Graphics File Format
//!   Specs" (UFLI),
//!   <http://codebase.c64.org/doku.php?id=base:c64_grafix_files_specs_list_v0.03>,
//!   and Codebase64 "UFLI", <http://codebase.c64.org/doku.php?id=base:ufli>.
//! - Everything else was reverse engineered from the two sample files by
//!   probing `recoil2png` with modified copies: a 16194-byte file is the
//!   unpacked memory `$4000-$7F3F` after a load address; any other size is
//!   two ignored bytes, an escape byte and `escape count value` runs
//!   (count 0 = 256) unpacking forwards to at least that much. Screen RAM
//!   `$5000 + 1024 * (y / 2 % 4)`; the picture shows bitmap columns 3-38.
//!   Line `y` shows sprite row `ceil(y / 2) % 21` of block
//!   `$4000 + $300 * (y / 40) + $180 * (y / 2 % 2) + $40 * column`, one
//!   block per 48-pixel column; the sprite pointers in the file are not used.

use super::unpack::{Run, escape_rle};
use super::vic2;
use crate::{DecodeError, Image};
use alloc::vec::Vec;

const WIDTH: usize = 288;
const HEIGHT: usize = 200;
/// Bytes of the memory `$4000-$7F3F`.
const LEN: usize = 0x3f40;

/// One pixel of the memory image `mem` (`$4000` onwards).
fn pixel(mem: &[u8], x: usize, y: usize) -> u8 {
    let cell = y / 8 * 40 + 3 + x / 8;
    let set = mem[0x2000 + cell * 8 + y % 8] & (0x80 >> (x % 8)) != 0;
    let color = mem[0x1000 + (y / 2 % 4) * 0x400 + cell];
    if set {
        return color >> 4;
    }
    let block = y / 40 * 0x300 + y / 2 % 2 * 0x180 + x / 48 * 0x40;
    let row = y.div_ceil(2) % 21;
    let bit = x % 48 / 2;
    if mem[block + row * 3 + bit / 8] & (0x80 >> (bit % 8)) != 0 {
        return mem[0xff0] & 15;
    }
    color & 15
}

pub(super) fn decode_ufli(data: &[u8]) -> Result<Image, DecodeError> {
    let mem: Vec<u8> = if data.len() == 2 + LEN {
        data[2..].to_vec()
    } else {
        let [_, _, escape, packed @ ..] = data else {
            return Err(DecodeError::Unrecognized);
        };
        escape_rle(packed, *escape, Run::CountValue, LEN)
            .filter(|mem| mem.len() == LEN)
            .ok_or(DecodeError::Unrecognized)?
    };
    let pixels = (0..HEIGHT)
        .flat_map(|y| (0..WIDTH).map(move |x| (x, y)))
        .map(|(x, y)| pixel(&mem, x, y))
        .collect();
    Ok(vic2::image(WIDTH, HEIGHT, pixels))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprites_alternate_every_two_lines() {
        let mut mem = alloc::vec![0u8; LEN];
        mem[0xff0] = 7;
        mem[0x1000 + 3] = 0x12; // screen 0, column 3: paper 2
        mem[0x1400 + 3] = 0x12;
        mem[0] = 0x80; // block 0 (lines 0-1), row 0: pixels 0-1
        mem[0x180 + 3] = 0x80; // block 6 (lines 2-3), row 1
        assert_eq!(pixel(&mem, 0, 0), 7);
        assert_eq!(pixel(&mem, 1, 0), 7);
        assert_eq!(pixel(&mem, 2, 0), 2);
        assert_eq!(pixel(&mem, 0, 1), 2);
        assert_eq!(pixel(&mem, 0, 2), 7);
    }

    #[test]
    fn packed_data_must_fill_the_memory() {
        let mut data = alloc::vec![0, 0x80, 0xee];
        for _ in 0..(LEN / 256) {
            data.extend_from_slice(&[0xee, 0, 0]);
        }
        assert!(decode_ufli(&data).is_err());
        data.extend_from_slice(&[0xee, (LEN % 256) as u8, 0]);
        assert!(decode_ufli(&data).is_ok());
    }
}
