//! C64 character-mode logos: Logo Painter 3 (`.lp3`).
//!
//! Sources:
//! - Multicolor character mode (bit pairs `00` = `$D021`, `01` = `$D022`,
//!   `10` = `$D023`, `11` = color RAM): <https://www.cebix.net/VIC-Article.txt>.
//! - Layout reverse engineered from 2 samples and checked against
//!   `recoil2png` output: load `$1800`, 40×50 screen codes, the character
//!   set at `$2000`. A file with the viewer appended (4174 bytes) keeps
//!   `$D021`, the color RAM color, `$D022` and `$D023` at `$1FFB-$1FFE`
//!   (its viewer copies them there); 4098-byte files have no colors and
//!   `recoil2png` shows them on black with light red, red and white. The
//!   viewer turns on multicolor mode; every picture is drawn multicolor
//!   with the low 3 bits of the color RAM byte (bit 3 is set in the
//!   sample, and `recoil2png` ignores it).

use super::vic2;
use crate::{DecodeError, Image};
use alloc::vec::Vec;

const COLUMNS: usize = 40;
const ROWS: usize = 50;
const CHARSET: usize = 2 + 0x800;
const COLORS: usize = 2 + 0x7fb;
const PLAIN_LEN: usize = CHARSET + 0x800;
const WITH_VIEWER_LEN: usize = PLAIN_LEN + 76;

/// Logo Painter 3: screen codes and a multicolor character set.
pub(super) fn decode_logo_painter(data: &[u8]) -> Result<Image, DecodeError> {
    if data.get(..2) != Some(&[0x00, 0x18]) {
        return Err(DecodeError::Invalid);
    }
    let [background, color_ram, multi1, multi2] = match data.len() {
        PLAIN_LEN => [0, 1, 10, 2],
        WITH_VIEWER_LEN => [
            data[COLORS],
            data[COLORS + 1],
            data[COLORS + 2],
            data[COLORS + 3],
        ],
        _ => return Err(DecodeError::Invalid),
    };
    let screen = &data[2..2 + COLUMNS * ROWS];
    let charset = &data[CHARSET..PLAIN_LEN];
    let (width, height) = (COLUMNS * 8, ROWS * 8);
    let mut pixels = Vec::with_capacity(width * height);
    for y in 0..height {
        for x in 0..width {
            let glyph = usize::from(screen[y / 8 * COLUMNS + x / 8]);
            let byte = charset[glyph * 8 + y % 8];
            pixels.push(match byte >> (6 - (x & 6)) & 3 {
                0 => background,
                1 => multi1,
                2 => multi2,
                _ => color_ram & 7,
            });
        }
    }
    Ok(vic2::image(width, height, pixels))
}
