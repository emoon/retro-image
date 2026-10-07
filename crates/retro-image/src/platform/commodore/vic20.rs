//! Commodore VIC-20 MiniPaint (Minigrafik, `.mg`), Best Paint (`.bp`) and
//! Picasso (`.pic0` + `.pic1`) pictures.
//!
//! Sources:
//! - GoDot MiniPaint loader page, <https://www.godot64.de/german/l_minipaint.htm>:
//!   load `$10F1`, 15-byte header ending with `$900E` and `$900F`, 160×192
//!   bitmap in vertical 8-pixel strips, 120 bytes of color RAM with
//!   repetitions removed, display routine.
//! - VIC color semantics (color RAM bit 3 selects multicolor; hires set
//!   pixels use the color RAM color, clear pixels the `$900F` background;
//!   multicolor `00` background, `01` border, `10` character color, `11`
//!   `$900E` auxiliary color): VIC-I documentation,
//!   <http://sleepingelephant.com/denial/wiki/index.php?title=MOS_Technology_VIC>.
//! - Color RAM packing (two cells per byte, low nibble first, row by row
//!   for 20×12 cells of 8×16 pixels) checked against `recoil2png` output.
//! - Best Paint: reverse engineered from 10 samples. Load `$1100`, the
//!   160×192 bitmap in vertical 8-pixel strips (as MiniPaint), 20×12 bytes
//!   of color RAM, then `$900F` (background in the high nibble, varied
//!   across samples). Hires only: `recoil2png` rejects color RAM with the
//!   multicolor bit set, and no sample has it.
//! - Picasso (`.pic0` + `.pic1`): reverse engineered from the sample
//!   pair. The bitmap is 242 characters of 16 bytes in screen order and the
//!   file ends with the VIC registers (`$9003` = `$17`: 11 rows of 8×16
//!   characters); `.pic1` is color RAM. Checked against `recoil2png`.
//! - Palette: observed from `recoil2png` output with synthetic pictures.

use crate::{Companions, DecodeError, Image};

pub(super) const PALETTE: [u32; 16] = [
    0x000000, 0xffffff, 0x6d2327, 0xa0fef8, 0x8e3c97, 0x7eda75, 0x252390, 0xffff86, 0xa4643b,
    0xffc8a1, 0xf2a7ab, 0xdbffff, 0xffb4ff, 0xd7ffce, 0x9d9aff, 0xffffc9,
];

/// The VIC color registers a picture uses.
struct Colors {
    background: u8,
    border: u8,
    auxiliary: u8,
}

impl Colors {
    /// From `$900E` (auxiliary color in the high nibble) and `$900F`
    /// (background in the high nibble, border in the low 3 bits).
    fn from_registers(r900e: u8, r900f: u8) -> Self {
        Self {
            background: r900f >> 4,
            border: r900f & 7,
            auxiliary: r900e >> 4,
        }
    }

    /// Palette index of pixel `x` (0-7) of `byte` in a cell whose color RAM
    /// nibble is `color`: bit 3 selects multicolor.
    fn pixel(&self, byte: u8, x: usize, color: u8) -> usize {
        let color = color & 15;
        let index = if color & 8 == 0 {
            if byte & (0x80 >> x) != 0 {
                color
            } else {
                self.background
            }
        } else {
            match byte >> (6 - (x & 6)) & 3 {
                0 => self.background,
                1 => self.border,
                2 => color & 7,
                _ => self.auxiliary,
            }
        };
        usize::from(index)
    }
}

const WIDTH: usize = 160;
const HEIGHT: usize = 192;
const BITMAP: usize = 17;
const COLORS: usize = BITMAP + WIDTH / 8 * HEIGHT;

/// MiniPaint: load address, header, bitmap, color RAM, display routine.
pub(super) fn decode_minipaint(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != COLORS + 240 || data[..2] != [0xf1, 0x10] {
        return Err(DecodeError::Unrecognized);
    }
    let colors = Colors::from_registers(data[15], data[16]);
    let mut image = Image::new(WIDTH as u32, HEIGHT as u32)?;
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let byte = data[BITMAP + x / 8 * HEIGHT + y];
            let cell = y / 16 * 20 + x / 8;
            let color = data[COLORS + cell / 2] >> (cell % 2 * 4);
            image.set(
                x as u32,
                y as u32,
                PALETTE[colors.pixel(byte, x % 8, color)],
            );
        }
    }
    Ok(image)
}

/// Best Paint: load address, bitmap, color RAM, `$900F`.
pub(super) fn decode_best_paint(data: &[u8]) -> Result<Image, DecodeError> {
    const COLOR_RAM: usize = 2 + WIDTH / 8 * HEIGHT;
    if data.len() != COLOR_RAM + 20 * 12 + 1 || data[..2] != [0x00, 0x11] {
        return Err(DecodeError::Unrecognized);
    }
    let color_ram = &data[COLOR_RAM..COLOR_RAM + 20 * 12];
    if color_ram.iter().any(|color| color & 8 != 0) {
        return Err(DecodeError::Unrecognized);
    }
    let colors = Colors::from_registers(0, data[data.len() - 1]);
    let mut image = Image::new(WIDTH as u32, HEIGHT as u32)?;
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let byte = data[2 + x / 8 * HEIGHT + y];
            let color = color_ram[y / 16 * 20 + x / 8];
            image.set(
                x as u32,
                y as u32,
                PALETTE[colors.pixel(byte, x % 8, color)],
            );
        }
    }
    Ok(image)
}

/// Picasso: `.pic0` holds 22×11 characters of 8×16 pixels in screen order
/// (load `$0D00`), then the VIC registers `$9000-$900F`; `.pic1` holds the
/// color RAM. Without the colors there is no picture to show.
pub(super) fn decode_picasso(
    data: &[u8],
    companions: &dyn Companions,
) -> Result<Image, DecodeError> {
    const COLUMNS: usize = 22;
    const ROWS: usize = 11;
    const BITMAP_LEN: usize = COLUMNS * ROWS * 16;
    if data.len() != 2 + BITMAP_LEN + 16 || data[..2] != [0x00, 0x0d] {
        return Err(DecodeError::Unrecognized);
    }
    let (bitmap, registers) = data[2..].split_at(BITMAP_LEN);
    // 22 columns, 11 rows of 8×16 characters, the screen and character
    // memory the program uses.
    if (registers[2], registers[3], registers[5]) != (0x96, 0x17, 0x8c) {
        return Err(DecodeError::Unrecognized);
    }
    let color_ram = companions
        .get("pic1")
        .filter(|c| c.len() == 2 + COLUMNS * ROWS)
        .ok_or(DecodeError::Unrecognized)?;
    let colors = Colors::from_registers(registers[14], registers[15]);
    let (width, height) = (COLUMNS * 8, ROWS * 16);
    let mut image = Image::new(width as u32, height as u32)?;
    for y in 0..height {
        for x in 0..width {
            let cell = y / 16 * COLUMNS + x / 8;
            let byte = bitmap[cell * 16 + y % 16];
            let color = color_ram[2 + cell];
            image.set(
                x as u32,
                y as u32,
                PALETTE[colors.pixel(byte, x % 8, color)],
            );
        }
    }
    Ok(image)
}
