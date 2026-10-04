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
//! - The 16 colours of the IBM CGA/EGA/VGA text palette and the 6-bit DAC
//!   scaling: shared with `pc.rs` (observed from `recoil2png` output); the
//!   palette is also the default of the XBin specification
//!   (<https://web.archive.org/web/20120204063040/http://www.acid.org/info/xbin/x_spec.htm>).

use alloc::vec;
use alloc::vec::Vec;

use super::super::pc::vga_rgb;
use super::font::Font;
use crate::{DecodeError, Image};

/// The 16 colours of the IBM CGA/EGA/VGA text palette, by attribute value.
pub(super) const PALETTE: [u32; 16] = super::super::pc::CGA_PALETTE;

/// Most cells a picture may have: 80 columns by 3276 rows. Keeps hostile
/// sizes (huge widths, cursor jumps, long runs) from allocating gigabytes.
pub(super) const MAX_CELLS: usize = 1 << 18;

/// Most pixels a picture may have, as for the other platforms (see
/// `atari_st/common.rs`): with tall fonts and 9-pixel cells the cell limit
/// alone would allow about 75 million. Taller pictures are cropped to the
/// rows that fit.
const MAX_PIXELS: usize = 1 << 24;

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

fn rgb(color: u32) -> [u8; 3] {
    let [_, r, g, b] = color.to_be_bytes();
    [r, g, b]
}

/// The 24 bytes of 8 pixels of `color`, as three words (native byte order,
/// like [`GLYPH_MASKS`]).
fn repeat8(color: [u8; 3]) -> [u64; 3] {
    let mut bytes = [0; 24];
    for pixel in bytes.as_chunks_mut::<3>().0 {
        *pixel = color;
    }
    let [a, b, c] = *bytes.as_chunks::<8>().0 else {
        unreachable!("24 bytes make 3 words")
    };
    [a, b, c].map(u64::from_ne_bytes)
}

/// For each glyph row, 0xff over the 3 bytes of every set pixel (leftmost
/// pixel = most significant bit first), 0 elsewhere, as three words in
/// native byte order. Lets a row of 8 pixels be coloured with a few
/// word-wide AND and XOR instead of a branch per pixel.
const GLYPH_MASKS: [[u64; 3]; 256] = {
    let mut masks = [[0; 3]; 256];
    let mut bits = 0;
    while bits < 256 {
        let mut word = 0;
        while word < 3 {
            let mut bytes = [0; 8];
            let mut i = 0;
            while i < 8 {
                if bits & (0x80 >> ((word * 8 + i) / 3)) != 0 {
                    bytes[i] = 0xff;
                }
                i += 1;
            }
            masks[bits][word] = u64::from_ne_bytes(bytes);
            word += 1;
        }
        bits += 1;
    }
    masks
};

/// A cell's colours as bytes, ready to be blended into pixel lines.
struct Painted {
    glyph: u8,
    line_drawing: bool,
    fg: [u8; 3],
    bg: [u8; 3],
    fg8: [u64; 3],
    bg8: [u64; 3],
}

impl Painted {
    fn new(cell: &Cell) -> Self {
        let (fg, bg) = (rgb(cell.fg), rgb(cell.bg));
        Self {
            glyph: cell.glyph,
            line_drawing: (0xc0..=0xdf).contains(&cell.glyph),
            fg,
            bg,
            fg8: repeat8(fg),
            bg8: repeat8(bg),
        }
    }
}

/// Draws `rows` rows of `width` cells; missing cells are black. Only as
/// many top rows as fit in [`MAX_PIXELS`] are drawn.
pub(super) fn render(
    cells: &[Cell],
    width: usize,
    rows: usize,
    style: &Style,
) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let cell_width = if style.nine_pixels { 9 } else { 8 };
    let cell_height = style.font.height();
    let cells_ok = width > 0 && rows > 0 && width <= MAX_COLUMNS && width * rows <= MAX_CELLS;
    let row_pixels = width * cell_width * cell_height;
    if !cells_ok || row_pixels > MAX_PIXELS {
        return Err(fail);
    }
    let rows = rows.min(MAX_PIXELS / row_pixels);
    let mut image = Image::new((width * cell_width) as u32, (rows * cell_height) as u32);
    // Per cell of the current text row, so its colours are expanded once
    // for all of its pixel lines.
    let mut painted = Vec::with_capacity(width);
    for (cell_row, row_cells) in cells.chunks(width).take(rows).enumerate() {
        painted.clear();
        painted.extend(row_cells.iter().map(Painted::new));
        for y in 0..cell_height {
            let line = image.row_mut((cell_row * cell_height + y) as u32);
            for (cell, out) in painted.iter().zip(line.chunks_exact_mut(cell_width * 3)) {
                let bits = style.font.row(cell.glyph, y);
                let (eight, ninth) = out.split_at_mut(24);
                let mask = &GLYPH_MASKS[usize::from(bits)];
                for (((chunk, &m), &f), &b) in eight
                    .as_chunks_mut::<8>()
                    .0
                    .iter_mut()
                    .zip(mask)
                    .zip(&cell.fg8)
                    .zip(&cell.bg8)
                {
                    *chunk = (b ^ ((b ^ f) & m)).to_ne_bytes();
                }
                // Line-drawing characters join up across the 9th column.
                if let Some(pixel) = ninth.first_chunk_mut::<3>() {
                    *pixel = if cell.line_drawing && bits & 1 != 0 {
                        cell.fg
                    } else {
                        cell.bg
                    };
                }
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
    /// Cells before this index are known to be blank, so repeated erases
    /// cost nothing.
    blank_prefix: usize,
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
            blank_prefix: 0,
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
        self.store(x, y, cell);
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
            self.store(x, y, cell);
        }
    }

    fn store(&mut self, x: usize, y: usize, cell: Cell) {
        let index = y * self.width + x;
        self.allocate(y);
        self.cells[index] = cell;
        self.blank_prefix = self.blank_prefix.min(index);
    }

    /// Blanks the cursor cell and everything after it. Missing cells are
    /// blank, so this only drops storage.
    pub(super) fn erase_from_cursor(&mut self) {
        let index = self.y.saturating_mul(self.width) + self.x;
        self.cells.truncate(index);
        self.blank_prefix = self.blank_prefix.min(index);
    }

    /// Blanks everything up to and including the cursor cell.
    pub(super) fn erase_to_cursor(&mut self) {
        let end = (self.y.saturating_mul(self.width) + self.x + 1).min(self.cells.len());
        if self.blank_prefix < end {
            self.cells[self.blank_prefix..end].fill(Cell::BLANK);
            self.blank_prefix = end;
        }
    }

    /// Erases everything to blanks and homes the cursor.
    pub(super) fn clear(&mut self) {
        self.cells.clear();
        self.blank_prefix = 0;
        self.move_to(0, 0);
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
    for (cell, pair) in cells.iter_mut().zip(pairs.as_chunks::<2>().0) {
        *cell = Cell::from_attribute(pair[0], pair[1], palette, ice);
    }
    Ok((cells, rows))
}

/// A palette of 16 VGA DAC entries, 3 bytes (red, green, blue) of 0-63
/// each, scaled to 8 bits; `None` if a value exceeds 63.
pub(super) fn vga_palette(rgb: &[u8]) -> Option<[u32; 16]> {
    let mut palette = [0; 16];
    for (color, c) in palette.iter_mut().zip(rgb.as_chunks::<3>().0) {
        if c.iter().any(|&v| v > 63) {
            return None;
        }
        *color = vga_rgb([c[0], c[1], c[2]]);
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
    fn render_crops_to_the_pixel_cap() {
        let big = Style {
            font: Font::new(32, &[0; 32 * 256]).unwrap(),
            nine_pixels: true,
        };
        // 80 columns of 9x32 cells: 23040 pixels a row, so 728 rows fit in
        // 2^24; 1000 rows are cropped to those. 8x16 cells fit 1000 rows.
        let image = render(&[], 80, 200, &big).unwrap();
        assert_eq!(image.height(), 200 * 32);
        let image = render(&[], 80, 1000, &big).unwrap();
        assert_eq!((image.width(), image.height()), (720, 728 * 32));
        assert_eq!(render(&[], 80, 1000, &STYLE).unwrap().height(), 16000);
        // A row of the widest allowed picture (2048 columns of 9x32 cells,
        // 589824 pixels) always fits; wider ones are rejected outright.
        assert!(render(&[], MAX_COLUMNS + 1, 1, &big).is_err(), "too wide");
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
