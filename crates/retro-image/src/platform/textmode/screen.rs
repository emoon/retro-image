//! Character cells and their rendering: the grid shared by the text-mode
//! formats, the cursor that stream formats (ANSI, PCBoard, Avatar,
//! TundraDraw) write through, and the IBM PC attribute byte.
//!
//! Sources:
//! - The SAUCE specification, "ANSiFlags"
//!   (<https://www.acid.org/info/sauce/sauce.htm>): the attribute byte's
//!   blink bit (blinking foreground, or with non-blink "iCE colour" the
//!   high-intensity background), and the 9-pixel letter spacing in which the
//!   VGA repeats the 8th glyph column for characters C0h-DFh only.
//! - The 16 colours of the IBM CGA/EGA/VGA text palette: as in `pc.rs`
//!   (observed from `recoil2png` output), and the default palette of the
//!   XBin specification
//!   (<https://web.archive.org/web/20120204063040/http://www.acid.org/info/xbin/x_spec.htm>).

use alloc::vec;
use alloc::vec::Vec;

use super::font::Font;
use crate::{DecodeError, Image};

/// The 16 colours of the IBM CGA/EGA/VGA text palette, by attribute value.
pub(super) const PALETTE: [u32; 16] = [
    0x000000, 0x0000aa, 0x00aa00, 0x00aaaa, 0xaa0000, 0xaa00aa, 0xaa5500, 0xaaaaaa, 0x555555,
    0x5555ff, 0x55ff55, 0x55ffff, 0xff5555, 0xff55ff, 0xffff55, 0xffffff,
];

/// Most cells a picture may have: 80 columns by 3276 rows. Keeps hostile
/// sizes (huge widths, cursor jumps, long runs) from allocating gigabytes.
pub(super) const MAX_CELLS: usize = 1 << 18;

/// Most columns a picture may have.
pub(super) const MAX_COLUMNS: usize = 2048;

/// One character cell: a glyph and its colours as `0xRRGGBB`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Cell {
    pub(super) glyph: u8,
    pub(super) fg: u32,
    pub(super) bg: u32,
}

impl Cell {
    /// A space in light grey on black, as on a cleared DOS screen.
    pub(super) const BLANK: Self = Self {
        glyph: b' ',
        fg: PALETTE[7],
        bg: PALETTE[0],
    };

    /// A cell from a character and an attribute byte (background in the
    /// high nibble). Without `ice`, bit 7 makes the character blink; a
    /// still picture shows it visible, on the low-intensity background.
    pub(super) fn from_attribute(glyph: u8, attribute: u8, palette: &[u32; 16], ice: bool) -> Self {
        let background = if ice {
            attribute >> 4
        } else {
            attribute >> 4 & 7
        };
        Self {
            glyph,
            fg: palette[usize::from(attribute & 15)],
            bg: palette[usize::from(background)],
        }
    }
}

/// How cells are drawn.
#[derive(Clone, Copy)]
pub(super) struct Style<'a> {
    pub(super) font: Font<'a>,
    /// 9-pixel letter spacing (VGA), instead of 8.
    pub(super) nine_pixels: bool,
}

/// Draws `rows` rows of `width` cells; missing cells are black.
pub(super) fn render(
    cells: &[Cell],
    width: usize,
    rows: usize,
    style: &Style,
) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    if width == 0 || rows == 0 || width > MAX_COLUMNS || width * rows > MAX_CELLS {
        return Err(fail);
    }
    let cell_width = if style.nine_pixels { 9 } else { 8 };
    let cell_height = style.font.height();
    let mut image = Image::new((width * cell_width) as u32, (rows * cell_height) as u32);
    for (i, cell) in cells.iter().take(width * rows).enumerate() {
        let (x0, y0) = ((i % width) * cell_width, (i / width) * cell_height);
        // Line-drawing characters join up across the 9th column.
        let repeat_8th = (0xc0..=0xdf).contains(&cell.glyph);
        for y in 0..cell_height {
            let bits = style.font.row(cell.glyph, y);
            for x in 0..cell_width {
                let set = match x {
                    0..8 => bits & (0x80 >> x) != 0,
                    _ => repeat_8th && bits & 1 != 0,
                };
                let color = if set { cell.fg } else { cell.bg };
                image.set((x0 + x) as u32, (y0 + y) as u32, color);
            }
        }
    }
    Ok(image)
}

/// A screen that grows downwards as characters are written, with a cursor.
///
/// The cursor's column stays below the width: writing in the last column
/// wraps to the start of the next row, as on a DOS screen. Rows past the
/// cell limit are dropped. The picture's height is that of the lowest row
/// a character was written to.
pub(super) struct Terminal {
    width: usize,
    max_rows: usize,
    cells: Vec<Cell>,
    rows: usize,
    pub(super) x: usize,
    pub(super) y: usize,
}

impl Terminal {
    /// A terminal `width` columns wide; `None` for widths outside 1 to
    /// [`MAX_COLUMNS`].
    pub(super) fn new(width: usize) -> Option<Self> {
        (1..=MAX_COLUMNS).contains(&width).then(|| Self {
            width,
            max_rows: MAX_CELLS / width,
            cells: Vec::new(),
            rows: 0,
            x: 0,
            y: 0,
        })
    }

    pub(super) fn width(&self) -> usize {
        self.width
    }

    /// Writes `cell` at the cursor and advances it, wrapping at the right edge.
    pub(super) fn put(&mut self, cell: Cell) {
        self.set(self.x, self.y, cell);
        self.x += 1;
        if self.x == self.width {
            self.x = 0;
            self.y = self.y.saturating_add(1);
        }
    }

    /// Writes `cell` at (`x`, `y`) without moving the cursor. Off-screen
    /// positions are ignored.
    pub(super) fn set(&mut self, x: usize, y: usize, cell: Cell) {
        if x >= self.width || y >= self.max_rows {
            return;
        }
        self.allocate(y);
        self.cells[y * self.width + x] = cell;
        self.rows = self.rows.max(y + 1);
    }

    /// The cell at (`x`, `y`): blank if nothing was written there.
    #[cfg(test)]
    pub(super) fn get(&self, x: usize, y: usize) -> Cell {
        match x < self.width {
            true => self.cells.get(y * self.width + x).copied(),
            false => None,
        }
        .unwrap_or(Cell::BLANK)
    }

    /// Sets the cell at (`x`, `y`) to `cell` without making the picture
    /// taller: erasing below the written rows changes nothing visible.
    pub(super) fn erase(&mut self, x: usize, y: usize, cell: Cell) {
        if x < self.width && y < self.max_rows {
            self.allocate(y);
            self.cells[y * self.width + x] = cell;
        }
    }

    /// Erases everything to blanks and homes the cursor.
    pub(super) fn clear(&mut self) {
        self.cells.fill(Cell::BLANK);
        self.move_to(0, 0);
    }

    /// Rows that have been written or erased.
    pub(super) fn allocated_rows(&self) -> usize {
        self.cells.len() / self.width
    }

    fn allocate(&mut self, y: usize) {
        let needed = (y + 1) * self.width;
        if self.cells.len() < needed {
            self.cells.resize(needed, Cell::BLANK);
        }
    }

    /// Moves the cursor to (`x`, `y`), keeping the column on screen.
    pub(super) fn move_to(&mut self, x: usize, y: usize) {
        self.x = x.min(self.width - 1);
        self.y = y;
    }

    /// The written rows, or an error if nothing was written.
    pub(super) fn finish(mut self, style: &Style) -> Result<Image, DecodeError> {
        self.cells.truncate(self.rows * self.width);
        render(&self.cells, self.width, self.rows, style)
    }
}

/// Cells of `pairs` (character, attribute) bytes, `width` per row; a short
/// last row is padded with black cells.
pub(super) fn attribute_cells(
    pairs: &[u8],
    width: usize,
    palette: &[u32; 16],
    ice: bool,
) -> Result<(Vec<Cell>, usize), DecodeError> {
    let count = pairs.len() / 2;
    let rows = count.div_ceil(width.max(1));
    if width == 0 || width > MAX_COLUMNS || rows == 0 || rows * width > MAX_CELLS {
        return Err(DecodeError::Unrecognized);
    }
    let mut cells = vec![Cell::from_attribute(0, 0, palette, ice); rows * width];
    for (cell, pair) in cells.iter_mut().zip(pairs.chunks_exact(2)) {
        *cell = Cell::from_attribute(pair[0], pair[1], palette, ice);
    }
    Ok((cells, rows))
}

/// A palette of 16 VGA DAC entries, 3 bytes (red, green, blue) of 0-63
/// each, scaled to 8 bits; `None` if a value exceeds 63.
pub(super) fn vga_palette(rgb: &[u8]) -> Option<[u32; 16]> {
    let scale = |v: u8| u32::from(v << 2 | v >> 4);
    let mut palette = [0; 16];
    for (color, c) in palette.iter_mut().zip(rgb.chunks_exact(3)) {
        if c.iter().any(|&v| v > 63) {
            return None;
        }
        *color = scale(c[0]) << 16 | scale(c[1]) << 8 | scale(c[2]);
    }
    (rgb.len() >= 48).then_some(palette)
}

#[cfg(test)]
mod tests {
    use super::super::font::VGA_8X16;
    use super::*;

    const STYLE: Style = Style {
        font: VGA_8X16,
        nine_pixels: false,
    };

    #[test]
    fn attribute_blink_bit_is_bright_background_only_with_ice() {
        let blink = Cell::from_attribute(b'A', 0x9e, &PALETTE, false);
        assert_eq!((blink.fg, blink.bg), (PALETTE[14], PALETTE[1]));
        let ice = Cell::from_attribute(b'A', 0x9e, &PALETTE, true);
        assert_eq!(ice.bg, PALETTE[9]);
    }

    #[test]
    fn ninth_column_repeats_only_for_line_drawing_characters() {
        let style = Style {
            nine_pixels: true,
            ..STYLE
        };
        let white = |glyph| Cell {
            glyph,
            fg: 0xffffff,
            bg: 0,
        };
        // C4h (horizontal line) joins up; DBh (full block) too; B0h-B2h
        // (shades) and 'A' don't.
        let cells = [white(0xc4), white(0xdb), white(0xb2), white(b'A')];
        let image = render(&cells, 4, 1, &style).unwrap();
        assert_eq!((image.width(), image.height()), (36, 16));
        assert_eq!(image.get(8, 7), 0xffffff, "C4h row 7 is the line");
        assert_eq!(image.get(17, 0), 0xffffff, "DBh is solid");
        assert_eq!(image.get(26, 1), 0, "B2h gets a background column");
        assert_eq!(image.get(35, 7), 0);
    }

    #[test]
    fn terminal_wraps_and_grows_with_writes_only() {
        let mut terminal = Terminal::new(2).unwrap();
        let a = Cell {
            glyph: b'a',
            ..Cell::BLANK
        };
        terminal.put(a);
        terminal.put(a);
        assert_eq!((terminal.x, terminal.y), (0, 1));
        terminal.erase(0, 5, a);
        terminal.move_to(9, 3);
        assert_eq!(terminal.x, 1);
        let image = terminal.finish(&STYLE).unwrap();
        assert_eq!(image.height(), 16, "only the written row counts");
    }

    #[test]
    fn terminal_drops_rows_past_the_cell_limit() {
        let mut terminal = Terminal::new(80).unwrap();
        terminal.move_to(0, usize::MAX);
        terminal.put(Cell::BLANK);
        assert_eq!(terminal.y, usize::MAX, "cursor saturates");
        assert!(terminal.finish(&STYLE).is_err(), "nothing was kept");
    }

    #[test]
    fn attribute_cells_pad_the_last_row() {
        let (cells, rows) =
            attribute_cells(&[b'x', 0x1f, b'y', 0x07, b'z'], 3, &PALETTE, false).unwrap();
        assert_eq!((cells.len(), rows), (3, 1));
        assert_eq!(cells[1].glyph, b'y');
        assert_eq!(cells[2].glyph, 0);
        assert!(attribute_cells(&[], 80, &PALETTE, false).is_err());
    }

    #[test]
    fn vga_palette_scales_six_bit_values() {
        let mut rgb = [0u8; 48];
        rgb[45..].copy_from_slice(&[63, 32, 1]);
        let palette = vga_palette(&rgb).unwrap();
        assert_eq!(palette[15], 0xff8204);
        rgb[0] = 64;
        assert!(vga_palette(&rgb).is_none());
        assert!(vga_palette(&rgb[..45]).is_none());
    }
}
