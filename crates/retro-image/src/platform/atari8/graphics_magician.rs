//! The Graphics Magician Picture Painter, Atari 8-bit version (`.SPC`): a
//! vector picture of line, fill and brush commands drawn on a 160 x 192
//! four-color canvas and shown 320 x 192.
//!
//! Sources:
//! - Command set and fill idea: the Apple II Picture Painter data format and
//!   fill description from Andy McFadden's disassembly
//!   (<https://6502disassembly.com/a2-graphics-magician/>, prose only), Just Solve
//!   "The Graphics Magician Picture Painter"
//!   (<http://fileformats.archiveteam.org/wiki/The_Graphics_Magician_Picture_Painter>).
//!   Neither documents the Atari port, so everything below is reverse
//!   engineered from the samples (`COIN1`, `COIN2`, `TEST`, `ROCKETOR`) and
//!   from synthetic files rendered by `recoil2png`:
//! - Layout: `LE16` length (file size - 3), command stream, `00` as the last
//!   byte. The opcode is the whole first byte (no argument in its low
//!   nibble, unlike the Apple II): `00` end (rest ignored), `20`-`23` line
//!   color, `40`-`47` brush, `60 n` fill pattern 0-70, `70` + 6 bytes
//!   palette band, `80 x y` move, `A0 x y` line to, `C0 x y` stamp the brush,
//!   `E0 x y` fill. Coordinates are 160 x 192 pixels, any byte is accepted
//!   and clipped, except a fill start, which must be on the canvas. Text
//!   commands (`10`, `30`, `50`) are not decoded.
//! - Lines: major axis stepped one pixel at a time, minor axis rounded to
//!   nearest with ties going down when both axes run the same way and up
//!   when they run opposite ways (matches random lines exactly).
//! - Fill: pixels of color 0 only. Scan up from the start until a nonzero
//!   pixel, then per row fill both ways to the nearest nonzero pixel, move to
//!   `(left + right + 1) / 2` (right capped at 158) on the next row and stop
//!   at a nonzero pixel. It is not a true flood fill: concave shapes are left
//!   partly empty (matches random polygons with 25 patterns exactly).
//! - Patterns: 71 tiles of 4 x 2 pixels, anchored to the canvas, read back
//!   by filling the screen with each. Default pattern 5, line color 3,
//!   brush 0.
//! - Brushes: 7 x 14 masks read back by stamping each with a solid pattern.
//!   The top-left corner is `(x, y)`: recoil2png clips by that corner but
//!   paints every stamp at (0, 0).
//! - Palette band: `70 a b c0 c1 c2 c3` recolors rows `2a ..= 2b + 1` with
//!   Atari color bytes for pixel values 0-3 (luminance bit 0 ignored), later
//!   bands over earlier ones, independent of drawing order. It is an error
//!   when `b >= a` and `b > 95`. Default colors `00 14 94 36`.

use super::palette::register_rgb;
use crate::bytes::le16;
use crate::{DecodeError, Image};

const WIDTH: usize = 160;
const HEIGHT: usize = 192;
const DEFAULT_COLORS: [u8; 4] = [0x00, 0x14, 0x94, 0x36];
const DEFAULT_PATTERN: u8 = 5;
const DEFAULT_LINE_COLOR: u8 = 3;
const BRUSH_WIDTH: i32 = 7;

/// Fill patterns: rows 0 and 1 of the 4 x 2 tile, 2 bits a pixel, first
/// pixel in the top bits.
#[rustfmt::skip]
const PATTERNS: [u16; 71] = [
    0x0000, 0x5555, 0xaaaa, 0xffff, 0x1144, 0x2288, 0x33cc, 0x6699, 0x77dd, 0xbbee, 0x1551, 0x2aa2,
    0x3ff3, 0x4004, 0x6aa6, 0x7ff7, 0x8008, 0x9559, 0xbffb, 0xc00c, 0xd55d, 0xeaae, 0x5588, 0x22cc,
    0x3344, 0x4499, 0x44dd, 0x7799, 0x88ee, 0x8866, 0x99ee, 0xcc77, 0xccbb, 0xddbb, 0x1bb1, 0x1559,
    0x155d, 0x955d, 0x4008, 0x400c, 0x800c, 0x6aac, 0x2aae, 0x6aa2, 0x7ffb, 0x3ffb, 0xbff7, 0x5584,
    0x22c4, 0x3348, 0x4491, 0x44d1, 0x7791, 0x88e2, 0x8862, 0x99e1, 0xcc73, 0xccb3, 0xddb3, 0x558c,
    0x22c8, 0x334c, 0x449d, 0x44d9, 0x779d, 0x88e6, 0x886e, 0x99e5, 0xcc7b, 0xccb7, 0xddb7,
];

/// Brush stamps: 14 rows of 7 bits, leftmost pixel in bit 6.
#[rustfmt::skip]
const BRUSHES: [[u8; 14]; 8] = [
    [0, 0, 0, 0, 0, 0, 0b0001000, 0b0001000, 0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0b0011000, 0b0011000, 0b0011000, 0b0011000, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0b0001000, 0b0011100, 0b0011100, 0b0011100, 0b0011100, 0b0001000, 0, 0, 0, 0],
    [
        0, 0, 0b0001000, 0b0011100, 0b0011100, 0b0111110, 0b0111110, 0b0111110, 0b0111110,
        0b0011100, 0b0011100, 0b0001000, 0, 0,
    ],
    [
        0, 0b0001100, 0b0001100, 0b0011110, 0b0011110, 0b0111111, 0b0111111, 0b0111111, 0b0111111,
        0b0011110, 0b0011110, 0b0001100, 0b0001100, 0,
    ],
    [
        0b0001000, 0b0011100, 0b0111110, 0b0111110, 0b0111110, 0b1111111, 0b1111111, 0b1111111,
        0b1111111, 0b0111110, 0b0111110, 0b0111110, 0b0011100, 0b0001000,
    ],
    [
        0, 0, 0b0001000, 0b0010100, 0b0010100, 0b0101000, 0b0011110, 0b0111100, 0b0001010,
        0b0010100, 0b0010100, 0b0001000, 0, 0,
    ],
    [
        0b0001000, 0b0010100, 0b0101010, 0b0010100, 0b0101010, 0b1011101, 0b0111110, 0b0111110,
        0b1011101, 0b0101010, 0b0010100, 0b0101010, 0b0010100, 0b0001000,
    ],
];

/// The canvas and drawing state while the command stream runs.
struct Painter {
    pixels: [u8; WIDTH * HEIGHT],
    row_colors: [[u8; 4]; HEIGHT],
    line_color: u8,
    pattern: usize,
    brush: usize,
    pos: (i32, i32),
}

impl Painter {
    fn new() -> Self {
        Self {
            pixels: [0; WIDTH * HEIGHT],
            row_colors: [DEFAULT_COLORS; HEIGHT],
            line_color: DEFAULT_LINE_COLOR,
            pattern: usize::from(DEFAULT_PATTERN),
            brush: 0,
            pos: (0, 0),
        }
    }

    /// Index into `pixels`, `None` off the canvas.
    fn index(x: i32, y: i32) -> Option<usize> {
        let x = usize::try_from(x).ok().filter(|&x| x < WIDTH)?;
        let y = usize::try_from(y).ok().filter(|&y| y < HEIGHT)?;
        Some(y * WIDTH + x)
    }

    fn plot(&mut self, x: i32, y: i32, color: u8) {
        if let Some(i) = Self::index(x, y) {
            self.pixels[i] = color;
        }
    }

    fn pattern_color(&self, x: usize, y: usize) -> u8 {
        let shift = 14 - 2 * ((y & 1) * 4 + (x & 3));
        (PATTERNS[self.pattern] >> shift & 3) as u8
    }

    fn line_to(&mut self, (x1, y1): (i32, i32)) {
        let (x0, y0) = self.pos;
        let (dx, dy) = (x1 - x0, y1 - y0);
        let steps = dx.abs().max(dy.abs());
        let x_major = dx.abs() == steps;
        let (major, minor) = if x_major { (dx, dy) } else { (dy, dx) };
        let same_direction = major.signum() == minor.signum() || minor == 0;
        for i in 0..=steps {
            let step = major.signum() * i;
            // Nearest minor offset of `minor * i / steps`, ties by direction.
            let twice = 2 * minor * i;
            let offset = if steps == 0 {
                0
            } else if same_direction {
                -(steps - twice).div_euclid(2 * steps)
            } else {
                (twice + steps).div_euclid(2 * steps)
            };
            let (x, y) = if x_major {
                (x0 + step, y0 + offset)
            } else {
                (x0 + offset, y0 + step)
            };
            self.plot(x, y, self.line_color);
        }
        self.pos = (x1, y1);
    }

    fn stamp(&mut self, (x, y): (i32, i32)) {
        for (row, mask) in (0..).zip(BRUSHES[self.brush]) {
            for col in 0..BRUSH_WIDTH {
                if mask >> (BRUSH_WIDTH - 1 - col) & 1 != 0 {
                    let (px, py) = (x + col, y + row);
                    if let Some(i) = Self::index(px, py) {
                        self.pixels[i] = self.pattern_color(i % WIDTH, i / WIDTH);
                    }
                }
            }
        }
    }

    /// The Picture Painter fill: not a flood fill, see the module docs.
    fn fill(&mut self, x: usize, y: usize) -> Option<()> {
        if x >= WIDTH || y >= HEIGHT {
            return None;
        }
        let empty = |p: &Self, x: usize, y: usize| p.pixels[y * WIDTH + x] == 0;
        // The row below the nearest nonzero pixel at or above the start.
        let mut row = (0..=y)
            .rev()
            .find(|&r| !empty(self, x, r))
            .map_or(0, |r| r + 1);
        let mut x = x;
        while row < HEIGHT && empty(self, x, row) {
            let mut left = x;
            while left > 0 && empty(self, left - 1, row) {
                left -= 1;
            }
            let mut right = x;
            while right < WIDTH - 1 && empty(self, right + 1, row) {
                right += 1;
            }
            for px in left..=right {
                self.pixels[row * WIDTH + px] = self.pattern_color(px, row);
            }
            x = (left + right.min(WIDTH - 2)).div_ceil(2);
            row += 1;
        }
        Some(())
    }

    /// `70 a b c0 c1 c2 c3`: colors for rows `2a ..= 2b + 1`.
    fn band(&mut self, first: u8, last: u8, colors: [u8; 4]) -> Option<()> {
        if first > last {
            return Some(());
        }
        let rows = usize::from(first) * 2..=usize::from(last) * 2 + 1;
        for row in self.row_colors.get_mut(rows)? {
            *row = colors;
        }
        Some(())
    }

    fn into_image(self) -> Result<Image, DecodeError> {
        let colors = self
            .pixels
            .iter()
            .enumerate()
            .map(|(i, &pixel)| register_rgb(self.row_colors[i / WIDTH][usize::from(pixel)]));
        Image::from_colors(WIDTH as u32, HEIGHT as u32, colors)?.scaled(2, 1)
    }
}

pub(super) fn decode_spc(data: &[u8]) -> Result<Image, DecodeError> {
    run(data).ok_or(DecodeError::Unrecognized)?.into_image()
}

fn run(data: &[u8]) -> Option<Painter> {
    if usize::from(le16(data, 0)?) + 3 != data.len() || data.last() != Some(&0) {
        return None;
    }
    let mut painter = Painter::new();
    let mut stream = data[2..].iter().copied();
    let mut arg = || stream.next();
    loop {
        match arg()? {
            0x00 => return Some(painter),
            op @ 0x20..=0x23 => painter.line_color = op & 3,
            op @ 0x40..=0x47 => painter.brush = usize::from(op & 7),
            0x60 => {
                painter.pattern = usize::from(arg()?);
                if painter.pattern >= PATTERNS.len() {
                    return None;
                }
            }
            0x70 => {
                let [first, last, c0, c1, c2, c3] = [(); 6].map(|()| arg());
                let (first, last) = (first?, last?);
                painter.band(first, last, [c0?, c1?, c2?, c3?])?;
            }
            op @ (0x80 | 0xa0 | 0xc0 | 0xe0) => {
                let (x, y) = (arg()?, arg()?);
                let to = (i32::from(x), i32::from(y));
                match op {
                    0x80 => painter.pos = to,
                    0xa0 => painter.line_to(to),
                    0xc0 => painter.stamp(to),
                    _ => painter.fill(usize::from(x), usize::from(y))?,
                }
            }
            _ => return None,
        }
    }
}
