//! CPC video hardware: colours, screen modes and pixel packing.
//!
//! Sources:
//! - Hardware colour numbers (32 codes, 27 colours, three levels per
//!   channel): Gate Array, <https://cpctech.cpcwiki.de/docs/garray.html>.
//! - Firmware colour numbers (`9 * green + 3 * red + blue`, levels 0-2) and
//!   the power-on inks of the 16 pens: Amstrad CPC6128 User Instructions,
//!   chapter 8 ("INK" and the default ink table),
//!   <https://archive.org/details/amstrad-cpc-6128-user-manual>.
//! - Pixel packing of modes 0, 1 and 2:
//!   <https://cpctech.cpcwiki.de/docs/graphics.html>.
//! - Screen line addressing: <https://cpctech.cpcwiki.de/docs/screen.html>.
//! - Half level 0x80, mode 0 pixels doubled horizontally and mode 2 rows
//!   doubled: observed from `recoil2png` output.

use alloc::vec::Vec;

use crate::{DecodeError, Image};

const LEVELS: [u32; 3] = [0x00, 0x80, 0xff];

const fn rgb(red: usize, green: usize, blue: usize) -> u32 {
    LEVELS[red] << 16 | LEVELS[green] << 8 | LEVELS[blue]
}

/// Colours of the hardware colour numbers 0-31.
const HARDWARE_COLORS: [u32; 32] = [
    rgb(1, 1, 1),
    rgb(1, 1, 1),
    rgb(0, 2, 1),
    rgb(2, 2, 1),
    rgb(0, 0, 1),
    rgb(2, 0, 1),
    rgb(0, 1, 1),
    rgb(2, 1, 1),
    rgb(2, 0, 1),
    rgb(2, 2, 1),
    rgb(2, 2, 0),
    rgb(2, 2, 2),
    rgb(2, 0, 0),
    rgb(2, 0, 2),
    rgb(2, 1, 0),
    rgb(2, 1, 2),
    rgb(0, 0, 1),
    rgb(0, 2, 1),
    rgb(0, 2, 0),
    rgb(0, 2, 2),
    rgb(0, 0, 0),
    rgb(0, 0, 2),
    rgb(0, 1, 0),
    rgb(0, 1, 2),
    rgb(1, 0, 1),
    rgb(1, 2, 1),
    rgb(1, 2, 0),
    rgb(1, 2, 2),
    rgb(1, 0, 0),
    rgb(1, 0, 2),
    rgb(1, 1, 0),
    rgb(1, 1, 2),
];

/// Colour of a hardware colour number (bits 4-0; files often store it
/// with bit 6 set, as the Gate Array command `0x40 | n`).
pub(super) fn hardware_color(value: u8) -> u32 {
    HARDWARE_COLORS[usize::from(value & 31)]
}

/// Colour of a firmware colour number 0-26.
pub(super) const fn firmware_color(n: usize) -> u32 {
    rgb(n / 3 % 3, n / 9, n % 3)
}

/// Power-on colours of pens 0-15 (pens 14 and 15 flash; their first colour).
pub(super) const DEFAULT_PENS: [u32; 16] = {
    const INKS: [usize; 16] = [1, 24, 20, 6, 26, 0, 2, 8, 10, 12, 14, 16, 18, 22, 1, 16];
    let mut pens = [0; 16];
    let mut i = 0;
    while i < 16 {
        pens[i] = firmware_color(INKS[i]);
        i += 1;
    }
    pens
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Mode {
    /// 160x200, 16 colours.
    Zero,
    /// 320x200, 4 colours.
    One,
    /// 640x200, 2 colours.
    Two,
}

impl Mode {
    pub(super) fn from_number(number: u8) -> Option<Self> {
        match number {
            0 => Some(Mode::Zero),
            1 => Some(Mode::One),
            2 => Some(Mode::Two),
            _ => None,
        }
    }

    pub(super) fn pixels_per_byte(self) -> usize {
        match self {
            Mode::Zero => 2,
            Mode::One => 4,
            Mode::Two => 8,
        }
    }

    /// Pen of pixel `n` (0 = leftmost) of a screen byte.
    pub(super) fn pen(self, byte: u8, n: usize) -> u8 {
        let bit = |position: usize| (byte >> position) & 1;
        match self {
            // Pen bits 0, 1, 2, 3 at byte bits 7, 3, 5, 1 (pixel 0) or 6, 2, 4, 0.
            Mode::Zero => bit(7 - n) | bit(3 - n) << 1 | bit(5 - n) << 2 | bit(1 - n) << 3,
            // Pen bits 0, 1 at byte bits 7 - n and 3 - n.
            Mode::One => bit(7 - n) | bit(3 - n) << 1,
            Mode::Two => bit(7 - n),
        }
    }

    /// Output scale that keeps the 4:3 screen shape: mode 0 pixels are
    /// doubled horizontally, mode 2 rows vertically.
    fn scale(self) -> (u32, u32) {
        match self {
            Mode::Zero => (2, 1),
            Mode::One => (1, 1),
            Mode::Two => (1, 2),
        }
    }
}

/// Offset of line `y` of a standard screen with `line_bytes` bytes per
/// line: 8 lines per character row, each 0x800 bytes after the previous one.
pub(super) fn screen_line_offset(y: usize, line_bytes: usize) -> usize {
    (y & 7) * 0x800 + (y >> 3) * line_bytes
}

/// Renders `height` lines of `width` pixels; `line(y)` gives line `y`'s
/// bytes (at least `width / pixels per byte`, rounded up).
pub(super) fn render<'a>(
    mode: Mode,
    width: usize,
    height: usize,
    line: impl Fn(usize) -> &'a [u8],
    pens: &[u32; 16],
) -> Result<Image, DecodeError> {
    let per_byte = mode.pixels_per_byte();
    let indices: Vec<u8> = (0..height)
        .flat_map(|y| {
            let bytes = line(y);
            (0..width).map(move |x| mode.pen(bytes[x / per_byte], x % per_byte))
        })
        .collect();
    let image = Image::from_indexed(width as u32, height as u32, &indices, pens)
        .expect("pens are 0-15 and there is one per pixel");
    let (sx, sy) = mode.scale();
    image.scaled(sx, sy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hardware_colors_follow_gate_array_table() {
        assert_eq!(hardware_color(0x54), 0x000000);
        assert_eq!(hardware_color(0x4b), 0xffffff);
        assert_eq!(hardware_color(0x40), 0x808080);
        assert_eq!(hardware_color(0x5e), 0x808000);
        assert_eq!(hardware_color(0x57), 0x0080ff);
    }

    #[test]
    fn default_pens_are_blue_yellow_cyan_red() {
        assert_eq!(DEFAULT_PENS[..4], [0x000080, 0xffff00, 0x00ffff, 0xff0000]);
    }

    #[test]
    fn pens_unpack_per_mode() {
        assert_eq!(Mode::Zero.pen(0b1000_0000, 0), 1);
        assert_eq!(Mode::Zero.pen(0b0000_1000, 0), 2);
        assert_eq!(Mode::Zero.pen(0b0010_0000, 0), 4);
        assert_eq!(Mode::Zero.pen(0b0000_0001, 1), 8);
        assert_eq!(Mode::One.pen(0b0001_0001, 3), 3);
        assert_eq!(Mode::Two.pen(0b0000_0001, 7), 1);
    }
}
