//! Commodore VIC-20 MiniPaint (Minigrafik) pictures (`.mg`).
//!
//! Sources:
//! - GoDot MiniPaint loader page, <https://www.godot64.de/german/l_minipaint.htm>:
//!   load `$10F1`, 15-byte header ending with `$900E` and `$900F`, 160×192
//!   bitmap in vertical 8-pixel strips, 120 bytes of colour RAM with
//!   repetitions removed, display routine.
//! - VIC colour semantics (colour RAM bit 3 selects multicolour; hires set
//!   pixels use the colour RAM colour, clear pixels the `$900F` background;
//!   multicolour `00` background, `01` border, `10` character colour, `11`
//!   `$900E` auxiliary colour): VIC-I documentation,
//!   <http://sleepingelephant.com/denial/wiki/index.php?title=MOS_Technology_VIC>.
//! - Colour RAM packing (two cells per byte, low nibble first, row by row
//!   for 20×12 cells of 8×16 pixels) checked against `recoil2png` output.
//! - Palette: observed from `recoil2png` output with synthetic pictures.

use crate::{DecodeError, Image};

const PALETTE: [u32; 16] = [
    0x000000, 0xffffff, 0x6d2327, 0xa0fef8, 0x8e3c97, 0x7eda75, 0x252390, 0xffff86, 0xa4643b,
    0xffc8a1, 0xf2a7ab, 0xdbffff, 0xffb4ff, 0xd7ffce, 0x9d9aff, 0xffffc9,
];

const WIDTH: usize = 160;
const HEIGHT: usize = 192;
const BITMAP: usize = 17;
const COLORS: usize = BITMAP + WIDTH / 8 * HEIGHT;

/// MiniPaint: load address, header, bitmap, colour RAM, display routine.
pub(super) fn decode_minipaint(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != COLORS + 240 || data[..2] != [0xf1, 0x10] {
        return Err(DecodeError::Unrecognized);
    }
    let auxiliary = data[15] >> 4;
    let background = data[16] >> 4;
    let border = data[16] & 7;
    let mut image = Image::new(WIDTH as u32, HEIGHT as u32);
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let byte = data[BITMAP + x / 8 * HEIGHT + y];
            let cell = y / 16 * 20 + x / 8;
            let color = data[COLORS + cell / 2] >> (cell % 2 * 4) & 15;
            let index = if color & 8 == 0 {
                if byte & (0x80 >> (x % 8)) != 0 {
                    color
                } else {
                    background
                }
            } else {
                match byte >> (6 - (x & 6)) & 3 {
                    0 => background,
                    1 => border,
                    2 => color & 7,
                    _ => auxiliary,
                }
            };
            image.set(x as u32, y as u32, PALETTE[usize::from(index)]);
        }
    }
    Ok(image)
}
