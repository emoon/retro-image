//! HPM pictures of Grass' Slideshow (a 1996 Atari 8-bit slideshow): 160x192,
//! four colors.
//!
//! Sources: reverse engineered from the original program, with the
//! maintainer's permission to disassemble it. `SLIDESHO.W` from
//! `Grass-Slideshow.7z` (<http://atari.fox-1.nl/wp-content/uploads/Grass-Slideshow.7z>,
//! `Grass Slideshow.atr`) was run through a 6502 emulator to get past its
//! cruncher and then read; no code was copied. Findings:
//!
//! - The loader reads the file byte by byte until the end. A byte `00` starts
//!   a run (`value`, `count`; count 0 is 256), any other byte `n` is followed
//!   by `n` literal bytes. The output goes to screen memory, 40 bytes for
//!   each of 192 lines in ANTIC mode E (2 bits per pixel; pixel value 0 is
//!   the background register, 1-3 are playfield registers 0-2). Every one
//!   of the 15 pictures on the disk unpacks to 7681 bytes, the last being a
//!   copy of color register 0 (`COLOR0`): the program never reads it, but
//!   it is the first byte of the picture's entry in the program's palette
//!   table.
//! - The other colors are not in the file. The program sets `COLOR0`,
//!   `COLOR1`, `COLOR2` and the background from a table of four bytes per
//!   picture (at `$23DC`, in reverse order of the file list). The table
//!   entry belongs to the picture, not to the `COLOR0` value, but the values
//!   agree for all pictures except one: six of the seven pictures with
//!   `COLOR0` `$34` use `$C8`, `$7C` and a black background, `JORDAN.HPM`
//!   uses `$38`, `$3C`. A stand-alone file can't say which, so the entry is
//!   picked by `COLOR0`, and `JORDAN.HPM` by its file length (3494 bytes),
//!   as `recoil2png` does (observed: a repacked copy of the same picture
//!   shows the common colors).
//! - A `COLOR0` that is not in the table, or a missing trailer, gives the
//!   gray ramp `$00 $04 $08 $0C` (observed from `recoil2png`).
//! - Observed from `recoil2png`: at least 7680 bytes must unpack, whatever
//!   follows the 7681st byte is ignored, and the stream may end in the
//!   middle of a run.
//! - A run with count 0 would be repeated 256 times by the program, and
//!   `recoil2png` takes it as empty. None of the 15 pictures has one, so
//!   the decoder refuses such a stream: it is how the 19203-byte
//!   `DRAGON.HPM` of another program (which `recoil2png` shows as noise)
//!   stays out.

use super::screen::{GREY_COLORS, bitmap, four_color};
use crate::{DecodeError, Image};
use alloc::vec::Vec;

const SCREEN: usize = 40 * 192;

/// The table entries of the program, as `[background, COLOR0, COLOR1, COLOR2]`
/// (the register order of a four-color screen), by `COLOR0`.
const PALETTES: [[u8; 4]; 8] = [
    [0x00, 0x34, 0xc8, 0x7c],
    [0x00, 0xe4, 0xc8, 0xbe],
    [0x00, 0x05, 0x08, 0x0c],
    [0x06, 0x04, 0x00, 0x0a],
    [0x00, 0x74, 0x58, 0x7e],
    [0xa4, 0x51, 0xb9, 0x7c],
    [0x00, 0x35, 0xc8, 0x7c],
    [0x0e, 0x30, 0xc7, 0x7b],
];
/// `JORDAN.HPM`, the one picture whose colors differ from its `COLOR0`'s.
const JORDAN_LEN: usize = 3494;
const JORDAN: [u8; 4] = [0x00, 0x34, 0x38, 0x3c];

/// Unpacks up to `SCREEN + 1` bytes; a stream cut off in a run or literal
/// block just ends.
fn unpack(data: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(SCREEN + 1);
    let mut input = data.iter().copied();
    while out.len() <= SCREEN {
        let Some(control) = input.next() else { break };
        if control == 0 {
            let (Some(value), Some(count)) = (input.next(), input.next()) else {
                break;
            };
            if count == 0 {
                return None;
            }
            out.extend(core::iter::repeat_n(value, usize::from(count)));
        } else {
            out.extend(input.by_ref().take(usize::from(control)));
        }
    }
    Some(out)
}

/// Grass' Slideshow picture.
pub(super) fn decode_hpm(data: &[u8]) -> Result<Image, DecodeError> {
    let unpacked = unpack(data).ok_or(DecodeError::Unrecognized)?;
    let screen = unpacked.get(..SCREEN).ok_or(DecodeError::Unrecognized)?;
    let colors = match unpacked.get(SCREEN) {
        Some(0x34) if data.len() == JORDAN_LEN => JORDAN,
        Some(&color0) => PALETTES
            .iter()
            .find(|palette| palette[1] == color0)
            .copied()
            .unwrap_or(GREY_COLORS),
        None => GREY_COLORS,
    };
    four_color(bitmap(screen, 40, 2), 2, 1, colors)
}
