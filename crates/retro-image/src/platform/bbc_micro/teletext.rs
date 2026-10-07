//! Teletext pages (BBC Micro Mode 7): the display model shared by the TTI,
//! EP1 and raw Mode 7 readers.
//!
//! Sources:
//! - Control-code behavior (set-at / set-after, hold graphics, double
//!   height needing the code on both rows, line-start defaults, mosaic bit
//!   layout, BBC byte translation of hash / underline / pound, character
//!   rounding): <http://mdfs.net/Info/Comp/Teletext/Controls>.
//! - Formal reference for the same: the Teletext Wiki's specification page,
//!   <https://teletext.wiki.zxnet.co.uk/wiki/Teletext_specifications>, and
//!   ETSI EN 300 706.
//! - Character rounding rule (diagonal 2x2 clumps of the 5x9 matrix gain
//!   two sub-pixels, giving 10x18 dots in a 12x20 cell) and the mosaic
//!   geometry (blocks of 3/4/3 lines, separated blocks lose their left and
//!   bottom edge): the comments and `dochar_plotter` of Bedstead's
//!   `bedstead.c`, <https://bjh21.me.uk/bedstead/> (CC0); glyphs in [`font`].
//! - Cell layout (glyph columns 1-5 and rows 1-9 of a 6x10 cell, left
//!   column and top row blank) is read off `bedstead.c`'s `getpix`.
//!
//! Flash and conceal render in their steady, revealed state. Characters use
//! the English SAA5050 set whatever the page's language code says. The
//! output is 480x500 (12x20 per cell) with no 4:3 pixel stretch.

mod ep1;
mod font;
mod tti;

use super::physical_color;
use crate::{DecodeError, Image};

pub(super) use ep1::decode_ep1;
pub(super) use tti::decode_tti;

const COLUMNS: usize = 40;
const ROWS: usize = 25;
const CELL_W: usize = 12;
const CELL_H: usize = 20;
const SPACE: u8 = 0x20;

/// A page of 7-bit character codes, 25 rows of 40.
pub(super) struct Page {
    codes: [[u8; COLUMNS]; ROWS],
}

impl Page {
    pub(super) fn blank() -> Self {
        Page {
            codes: [[SPACE; COLUMNS]; ROWS],
        }
    }

    /// Stores `codes` as row `row`, ignoring any beyond the 40th column and
    /// any row outside the page. Missing columns stay spaces.
    pub(super) fn set_row(&mut self, row: usize, codes: impl Iterator<Item = u8>) {
        if let Some(cells) = self.codes.get_mut(row) {
            *cells = [SPACE; COLUMNS];
            for (cell, code) in cells.iter_mut().zip(codes) {
                *cell = code & 0x7f;
            }
        }
    }

    /// Draws the page as `dialect` displays it.
    pub(super) fn render(&self, dialect: Dialect) -> Result<Image, DecodeError> {
        let width = COLUMNS * CELL_W;
        let mut pixels = alloc::vec![0u8; width * ROWS * CELL_H];
        let mut bottom_half = false;
        for (row, stored) in self.codes.iter().enumerate() {
            // Broadcast teletext draws the lower row of a double-height pair
            // from the upper row's packet; the SAA5050 reads its own row.
            let codes = match (dialect, bottom_half) {
                (Dialect::Broadcast, true) => &self.codes[row - 1],
                _ => stored,
            };
            let mut state = Attributes::line_start();
            for (column, &code) in codes.iter().enumerate() {
                let cell = state.advance(code, dialect);
                let origin = row * CELL_H * width + column * CELL_W;
                cell.draw(&mut pixels[origin..], width, bottom_half);
            }
            // A row holding double-height codes is the top of a pair; the
            // row under it shows their lower halves.
            bottom_half = !bottom_half && codes.contains(&0x0d);
        }
        let mut palette = [0u32; 8];
        for (physical, color) in (0u8..).zip(palette.iter_mut()) {
            *color = physical_color(physical);
        }
        Image::from_indexed(width as u32, (ROWS * CELL_H) as u32, &pixels, &palette)
    }
}

/// How a page's bytes are interpreted.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Dialect {
    /// Teletext standard (TTI, EP1): 0x00 and 0x10 select black, and the
    /// row under a double-height row is not stored.
    Broadcast,
    /// BBC Mode 7 memory as the SAA5050 reads it: 0x00 and 0x10 do nothing,
    /// and the lower row of a double-height pair carries its own copy.
    Bbc,
}

#[derive(Clone, Copy)]
enum Shape {
    Blank,
    Text(u8),
    Mosaic { code: u8, separated: bool },
}

/// What one character cell shows.
struct Cell {
    shape: Shape,
    foreground: u8,
    background: u8,
    double: bool,
}

/// The serial attributes in force at a character position.
#[derive(Clone, Copy)]
struct Attributes {
    foreground: u8,
    background: u8,
    graphics: bool,
    separated: bool,
    double: bool,
    hold: bool,
    held: Option<(u8, bool)>,
}

impl Attributes {
    fn line_start() -> Self {
        Attributes {
            foreground: 7,
            background: 0,
            graphics: false,
            separated: false,
            double: false,
            hold: false,
            held: None,
        }
    }

    /// Shows `code` under the current attributes and moves on to the next
    /// position. Color, double height and release take effect after the
    /// cell they sit in; background, size reset, contiguity and hold take
    /// effect on it.
    fn advance(&mut self, code: u8, dialect: Dialect) -> Cell {
        let mut now = *self;
        let mut next = *self;
        let shape = if code >= 0x20 {
            if now.graphics && code & 0x20 != 0 {
                next.held = Some((code, now.separated));
                Shape::Mosaic {
                    code,
                    separated: now.separated,
                }
            } else {
                Shape::Text(code)
            }
        } else {
            match code {
                0x00 if dialect == Dialect::Broadcast => next.set_colour(0, false),
                0x01..=0x07 => next.set_colour(code, false),
                0x10 if dialect == Dialect::Broadcast => next.set_colour(0, true),
                0x11..=0x17 => next.set_colour(code & 7, true),
                0x0c => {
                    now.double = false;
                    next.double = false;
                }
                0x0d => next.double = true,
                0x19 | 0x1a => {
                    now.separated = code == 0x1a;
                    next.separated = now.separated;
                }
                0x1c => {
                    now.background = 0;
                    next.background = 0;
                }
                0x1d => {
                    now.background = now.foreground;
                    next.background = now.foreground;
                }
                0x1e => {
                    now.hold = true;
                    next.hold = true;
                }
                0x1f => next.hold = false,
                _ => {}
            }
            match now.held {
                Some((held, separated)) if now.hold => Shape::Mosaic {
                    code: held,
                    separated,
                },
                _ => Shape::Blank,
            }
        };
        let cell = Cell {
            shape,
            foreground: now.foreground,
            background: now.background,
            double: now.double,
        };
        *self = next;
        cell
    }

    fn set_colour(&mut self, colour: u8, graphics: bool) {
        if self.graphics && !graphics {
            self.held = None;
        }
        self.foreground = colour;
        self.graphics = graphics;
    }
}

impl Cell {
    /// Draws the cell at the start of `pixels`, whose rows are `stride`
    /// apart. In the lower row of a double-height pair only double-height
    /// cells show anything, and they show their lower half.
    fn draw(&self, pixels: &mut [u8], stride: usize, bottom_half: bool) {
        let visible = self.double || !bottom_half;
        let rows = if visible {
            self.shape.rows()
        } else {
            [0; CELL_H]
        };
        for line in 0..CELL_H {
            let source = match (self.double, bottom_half) {
                (false, _) => line,
                (true, false) => line / 2,
                (true, true) => CELL_H / 2 + line / 2,
            };
            let bits = rows[source];
            let target = &mut pixels[line * stride..line * stride + CELL_W];
            for (x, pixel) in target.iter_mut().enumerate() {
                let lit = bits >> (CELL_W - 1 - x) & 1 != 0;
                *pixel = if lit {
                    self.foreground
                } else {
                    self.background
                };
            }
        }
    }
}

impl Shape {
    /// Lit dots of each of the 20 lines, leftmost dot in bit 11.
    fn rows(self) -> [u16; CELL_H] {
        match self {
            Shape::Blank => [0; CELL_H],
            Shape::Text(code) => text_rows(code),
            Shape::Mosaic { code, separated } => mosaic_rows(code, separated),
        }
    }
}

/// A 5x9 glyph drawn into the 6x10 pixel cell (column 0 and row 0 blank),
/// doubled to 12x20 with the SAA5050 diagonal rounding applied.
fn text_rows(code: u8) -> [u16; CELL_H] {
    let glyph = &font::GLYPHS[usize::from(code.clamp(0x20, 0x7f) - 0x20)];
    let pixel = |x: usize, y: usize| {
        (1..=5).contains(&x) && (1..=9).contains(&y) && glyph[y - 1] >> (5 - x) & 1 != 0
    };
    let mut rows = [0u16; CELL_H];
    let mut set = |line: usize, dot: usize| rows[line] |= 1 << (CELL_W - 1 - dot);
    for y in 0..CELL_H / 2 {
        for x in 0..CELL_W / 2 {
            if pixel(x, y) {
                for (dy, dx) in [(0, 0), (0, 1), (1, 0), (1, 1)] {
                    set(2 * y + dy, 2 * x + dx);
                }
            }
        }
    }
    for y in 0..CELL_H / 2 - 1 {
        for x in 0..CELL_W / 2 - 1 {
            let (a, b) = (pixel(x, y), pixel(x + 1, y));
            let (c, d) = (pixel(x, y + 1), pixel(x + 1, y + 1));
            if a && d && !b && !c {
                set(2 * y + 1, 2 * x + 2);
                set(2 * y + 2, 2 * x + 1);
            } else if b && c && !a && !d {
                set(2 * y + 1, 2 * x + 1);
                set(2 * y + 2, 2 * x + 2);
            }
        }
    }
    rows
}

/// Six-block mosaic: bits 0-4 are top-left, top-right, middle-left,
/// middle-right and bottom-left; bit 6 is bottom-right. Bands are 3, 4 and 3
/// pixel rows of the 6x10 cell. Separated blocks lose their left and
/// bottom edges (one pixel of the 6x10 cell, two dots here).
fn mosaic_rows(code: u8, separated: bool) -> [u16; CELL_H] {
    const BANDS: [(usize, usize); 3] = [(0, 6), (6, 14), (14, 20)];
    const BLOCKS: [(u8, usize, usize); 6] = [
        (0x01, 0, 0),
        (0x02, 0, 1),
        (0x04, 1, 0),
        (0x08, 1, 1),
        (0x10, 2, 0),
        (0x40, 2, 1),
    ];
    let gap = if separated { 2 } else { 0 };
    let mut rows = [0u16; CELL_H];
    for (bit, band, half) in BLOCKS {
        if code & bit == 0 {
            continue;
        }
        let (top, bottom) = BANDS[band];
        let left = half * CELL_W / 2 + gap;
        let right = (half + 1) * CELL_W / 2;
        let mask = (1u16 << (CELL_W - left)) - (1 << (CELL_W - right));
        for row in &mut rows[top..bottom - gap] {
            *row |= mask;
        }
    }
    rows
}

/// A raw Mode 7 screen: 1000 bytes of display memory, optionally padded to
/// 1024. The 7-bit codes are stored as the SAA5050 sees them.
pub(super) fn decode_raw(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != COLUMNS * ROWS && data.len() != 1024 {
        return Err(DecodeError::Invalid);
    }
    let mut page = Page::blank();
    for (row, line) in data.as_chunks::<COLUMNS>().0.iter().take(ROWS).enumerate() {
        page.set_row(row, line.iter().copied());
    }
    page.render(Dialect::Bbc)
}
