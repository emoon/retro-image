//! Envision character maps (MAP): a map of character codes, drawn in one of
//! six ANTIC text modes with up to 255 character sets, one per map row.
//!
//! Sources:
//! - EnvisionPC manual, "MAP file format" (ANTIC mode, map width and height,
//!   five colours, map data, font data)
//!   (<http://ftp.pigwa.net/stuff/collections/holmes%20cd/Holmes%202/PC%20Atari%20Programming%20Utils/EnvisionPC%20V0.5/envision.txt>).
//!   Only that prose manual was read; the C source in the same zip has no
//!   stated licence and was not opened. Just Solve "Envision"
//!   (<http://fileformats.archiveteam.org/wiki/Envision>).
//! - ANTIC text modes 2 to 7 (glyph size, 2-bit and 1-bit glyph pixels, the
//!   inverse bit, mode 3 descenders, colours by the top code bits in modes 6
//!   and 7): De Re Atari ch. 2 (<https://www.atariarchives.org/dere/chapt02.php>).
//! - The layout is reverse engineered from the corpus samples PRYLL.MAP,
//!   ISLAND.MAP (mode 7) and NCC1701.MAP (mode 2, six fonts) and from
//!   `recoil2png` probing of synthetic files: the mode byte (bit 7 is
//!   ignored), width-1, height-1, five colour registers (playfield 0-3,
//!   background), the map, a 256-byte table RECOIL ignores, a 208-byte table
//!   (entry `y` is the font number of map row `y`; entry 206 the number of
//!   fonts, entry 207 must be 1), an 8-byte name and 1024-byte font, then per
//!   further font its number, an 8-byte name and 1024 bytes. The map height is
//!   at most 204. The glyph and colour details of every mode were checked
//!   against `recoil2png` with random synthetic files.

use super::antic::fill;
use super::palette::register_rgb;
use crate::{DecodeError, Image};

const HEADER: usize = 8;
const FONT: usize = 1024;
const IGNORED_TABLE: usize = 256;
const ROW_TABLE: usize = 208;
const FONT_COUNT_ENTRY: usize = 206;
const VERSION_ENTRY: usize = 207;
const MAX_HEIGHT: usize = 204;
/// Largest EnvisionPC picture we draw.
const MAX_PIXELS: usize = 1 << 25;
/// An extra font: number, name, glyphs.
const EXTRA_FONT: usize = 1 + 8 + FONT;

/// ANTIC text mode 2-7.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// 8x8 and 8x10 monochrome (the latter with descenders).
    Hires { tall: bool },
    /// 4 colours from 2-bit pixels, 8x8 or 8x16.
    Multi { tall: bool },
    /// 5 colours chosen by the code, 16x8 or 16x16 of 1-bit pixels.
    Colored { tall: bool },
}

impl Mode {
    fn from_byte(mode: u8) -> Option<Self> {
        Some(match mode & 0x7f {
            2 => Self::Hires { tall: false },
            3 => Self::Hires { tall: true },
            4 => Self::Multi { tall: false },
            5 => Self::Multi { tall: true },
            6 => Self::Colored { tall: false },
            7 => Self::Colored { tall: true },
            _ => return None,
        })
    }

    /// Cell size and the output lines per glyph line.
    fn cell(self) -> (u32, u32, u32) {
        match self {
            Self::Hires { tall } => (8, if tall { 10 } else { 8 }, 1),
            Self::Multi { tall } => (8, if tall { 16 } else { 8 }, if tall { 2 } else { 1 }),
            Self::Colored { tall } => (16, if tall { 16 } else { 8 }, if tall { 2 } else { 1 }),
        }
    }
}

/// Envision (Atari): see the module docs.
pub(super) fn decode_map(data: &[u8]) -> Result<Image, DecodeError> {
    let [mode, w, h, colors @ ..] = data else {
        return Err(DecodeError::Unrecognized);
    };
    let (Some(mode), Some(&colors)) = (Mode::from_byte(*mode), colors.first_chunk::<5>()) else {
        return Err(DecodeError::Unrecognized);
    };
    let (width, height) = (usize::from(*w) + 1, usize::from(*h) + 1);
    let map_end = HEADER + width * height;
    let table = map_end + IGNORED_TABLE;
    let fonts = table + ROW_TABLE;
    let (Some(map), Some(font_of_row)) = (data.get(HEADER..map_end), data.get(table..fonts)) else {
        return Err(DecodeError::Unrecognized);
    };
    let font_count = usize::from(font_of_row[FONT_COUNT_ENTRY]);
    // The first font has no number: just a name and the glyphs.
    let first = fonts + 8;
    if height > MAX_HEIGHT
        || font_count == 0
        || font_of_row[VERSION_ENTRY] != 1
        || data.len() != first + FONT + (font_count - 1) * EXTRA_FONT
    {
        return Err(DecodeError::Unrecognized);
    }
    let font = |number: usize| -> Option<&[u8]> {
        let start = match number {
            1 => first,
            2.. => {
                let entry = first + FONT + (number - 2) * EXTRA_FONT;
                (usize::from(*data.get(entry)?) == number).then_some(entry + 1 + 8)?
            }
            0 => return None,
        };
        data.get(start..start + FONT)
    };
    render(mode, width, map, colors, |y| {
        font(usize::from(font_of_row[y]))
    })
}

/// EnvisionPC: the mode, width and height (16 bits each), the colour
/// registers background and playfield 0-3, the map, one font, then
/// optionally zero padding (any other byte after the font is rejected).
/// There is no sample: this was probed with synthetic files only, where
/// RECOIL takes widths up to 32767 and any height.
pub(super) fn decode_map_pc(data: &[u8]) -> Result<Image, DecodeError> {
    let [
        mode,
        w_low,
        w_high,
        h_low,
        h_high,
        background,
        pf0,
        pf1,
        pf2,
        pf3,
        ref rest @ ..,
    ] = *data
    else {
        return Err(DecodeError::Unrecognized);
    };
    // Bit 7 of the mode byte is not accepted here.
    let mode = Mode::from_byte(mode)
        .filter(|_| mode & 0x80 == 0)
        .ok_or(DecodeError::Unrecognized)?;
    let width = usize::from(u16::from_le_bytes([w_low, w_high]));
    let height = usize::from(u16::from_le_bytes([h_low, h_high]));
    let (cell_width, cell_height, _) = mode.cell();
    let pixels = width * height * (cell_width * cell_height) as usize;
    if width == 0 || height == 0 || width > 32767 || pixels > MAX_PIXELS {
        return Err(DecodeError::Unrecognized);
    }
    let (map, rest) = rest
        .split_at_checked(width * height)
        .ok_or(DecodeError::Unrecognized)?;
    let (font, padding) = rest
        .split_at_checked(FONT)
        .ok_or(DecodeError::Unrecognized)?;
    if padding.iter().any(|&byte| byte != 0) {
        return Err(DecodeError::Unrecognized);
    }
    render(mode, width, map, [pf0, pf1, pf2, pf3, background], |_| {
        Some(font)
    })
}

/// Draws a `width`-column map, taking each row's font from `font_of_row`.
fn render<'a>(
    mode: Mode,
    width: usize,
    map: &[u8],
    colors: [u8; 5],
    font_of_row: impl Fn(usize) -> Option<&'a [u8]>,
) -> Result<Image, DecodeError> {
    let (cell_width, cell_height, _) = mode.cell();
    let height = map.len() / width;
    let mut image = Image::new(width as u32 * cell_width, height as u32 * cell_height);
    for (y, codes) in map.chunks_exact(width).enumerate() {
        let font = font_of_row(y).ok_or(DecodeError::Unrecognized)?;
        for (x, &code) in codes.iter().enumerate() {
            let origin = (x as u32 * cell_width, y as u32 * cell_height);
            draw_cell(&mut image, origin, mode, code, font, colors);
        }
    }
    Ok(image)
}

/// The glyph line shown on line `line` of a mode 2 or 3 cell: mode 3 puts
/// two blank lines on top of codes 0x60-0x7F and their first two glyph
/// lines at the bottom, the rest of the glyph in place; other characters
/// have two blank lines below.
fn hires_line(tall: bool, code: u8, line: usize) -> Option<usize> {
    let descender = tall && code & 0x60 == 0x60;
    match line {
        0 | 1 if descender => None,
        0..=7 => Some(line),
        8 | 9 if descender => Some(line - 8),
        _ => None,
    }
}

/// Draws character `code` of `font` with its top-left corner at `origin`.
fn draw_cell(
    image: &mut Image,
    (x0, y0): (u32, u32),
    mode: Mode,
    code: u8,
    font: &[u8],
    colors: [u8; 5],
) {
    let [pf0, pf1, pf2, pf3, background] = colors.map(register_rgb);
    let (cell_width, cell_height, line_height) = mode.cell();
    let inverse = code & 0x80 != 0;
    let glyph_at = |index: u8| &font[usize::from(index) * 8..][..8];
    for out_line in 0..cell_height / line_height {
        let line = out_line as usize;
        // Per pixel column: the colour for each glyph pixel value.
        let (bits, palette, pixel_bits): (u8, [u32; 4], u32) = match mode {
            Mode::Hires { tall } => {
                let glyph = glyph_at(code & 0x7f);
                let bits = hires_line(tall, code, line).map_or(0, |source| glyph[source]);
                let ink = register_rgb((colors[2] & 0xf0) | (colors[1] & 0x0e));
                let (off, on) = if inverse { (ink, pf2) } else { (pf2, ink) };
                (bits, [off, on, 0, 0], 1)
            }
            Mode::Multi { .. } => {
                let third = if inverse { pf3 } else { pf2 };
                (
                    glyph_at(code & 0x7f)[line],
                    [background, pf0, pf1, third],
                    2,
                )
            }
            Mode::Colored { .. } => {
                let ink = [pf0, pf1, pf2, pf3][usize::from(code >> 6)];
                (glyph_at(code & 0x3f)[line], [background, ink, 0, 0], 1)
            }
        };
        let pixel_width = match mode {
            Mode::Hires { .. } => 1,
            _ => 2,
        };
        for column in 0..cell_width / pixel_width {
            let shift = 8 - pixel_bits * (column + 1);
            let value = (bits >> shift) & ((1 << pixel_bits) - 1);
            fill(
                image,
                x0 + column * pixel_width,
                y0 + out_line * line_height,
                pixel_width,
                line_height,
                palette[usize::from(value)],
            );
        }
    }
}
