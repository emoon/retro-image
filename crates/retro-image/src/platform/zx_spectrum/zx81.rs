//! ZX81 BASIC programs (`.P`) that draw a picture with `PRINT`, and
//! ZXpaintyONE character screens (`.ZP1`, `.RAW`).
//!
//! Sources:
//! - A `.P` file is RAM from 0x4009 as `SAVE` writes it: system variables,
//!   then the BASIC program from 0x407D up to the display file (`D_FILE`, at
//!   0x400C). Program lines are a big-endian line number, a little-endian
//!   length and the tokenised text ending with NEWLINE (0x76); numbers are
//!   digits followed by 0x7E and a 5-byte float. Character codes 0x00-0x3F
//!   with bit 7 for inverse video; `""` inside a string is token 0xC0:
//!   <https://problemkaputt.de/zxdocs.htm>, kio's "O80 and P81 Format" note,
//!   <https://k1.spdns.de/Develop/Projects/zasm/Info/O80%20and%20P81%20Format.txt>,
//!   and the ZX81 character set,
//!   <https://en.wikipedia.org/wiki/ZX81_character_set>.
//! - Character glyphs: ZX81 ROM at 0x1E00, 64 characters of 8 bytes. Taken
//!   from the ROM dump "zx81 version 2 'improved' rom (Sinclair).rom" (md5
//!   db398d4e4e93a6d4dee3bfe146918219) in the archive.org item `ts1000-roms`,
//!   <https://archive.org/details/ts1000-roms>. Amstrad allows the Sinclair
//!   ROMs to be redistributed.
//! - Which programs are pictures, found by black-box tests of `recoil2png`:
//!   optional `FAST`/`CLS`/`CLEAR`/`SLOW` lines, then `PRINT` lines (string
//!   literals, `AT row,column`, `;`), ended by `STOP`, `PAUSE` or `GOTO`.
//!   Programs from ZXpaintyONE also set `A$` to 64 characters that a POKE
//!   loop copies to the bottom two screen lines (`D_FILE` + 727); a `LET`
//!   after that ends the picture. Anything else is a program, not a picture.
//!   The display file saved with the program is blank in every sample, so
//!   it is not used. Black on white, 256x192: observed from `recoil2png`.
//! - ZXpaintyONE `.RAW` (792 bytes: 24 lines of 32 codes, each ended by
//!   NEWLINE) and `.ZP1` (768 codes as hex digits, trailing data ignored),
//!   and that codes 0x40-0x7F show their low 6 bits' glyph: reverse
//!   engineered from samples and `recoil2png` probes.

use crate::{DecodeError, Image};

/// File offset of `address`; files start at 0x4009.
const fn offset(address: u16) -> usize {
    address as usize - 0x4009
}

const D_FILE: usize = offset(0x400c);
const PROGRAM: usize = offset(0x407d);
const COLUMNS: usize = 32;
const ROWS: usize = 24;

const NEWLINE: u8 = 0x76;
const QUOTE: u8 = 0x0b;
const DOLLAR: u8 = 0x0d;
const EQUALS: u8 = 0x14;
const SEMICOLON: u8 = 0x19;
const COMMA: u8 = 0x1a;
const LETTER_A: u8 = 0x26;
const NUMBER: u8 = 0x7e;
const DOUBLE_QUOTE: u8 = 0xc0;
const AT: u8 = 0xc1;
const STOP: u8 = 0xe3;
const SLOW: u8 = 0xe4;
const FAST: u8 = 0xe5;
const GOTO: u8 = 0xec;
const LET: u8 = 0xf1;
const PAUSE: u8 = 0xf2;
const PRINT: u8 = 0xf5;
const CLS: u8 = 0xfb;
const CLEAR: u8 = 0xfd;

pub(super) fn decode_p(data: &[u8]) -> Result<Image, DecodeError> {
    let d_file = data
        .get(D_FILE..D_FILE + 2)
        .map(|b| usize::from(u16::from_le_bytes([b[0], b[1]])))
        .ok_or(DecodeError::Unrecognized)?;
    let program = d_file
        .checked_sub(0x4009)
        .and_then(|end| data.get(PROGRAM..end))
        .ok_or(DecodeError::Unrecognized)?;
    let screen = run(program).ok_or(DecodeError::Unrecognized)?;
    Ok(render(screen.as_flattened()))
}

/// ZXpaintyONE v2.0 `.RAW`: the display file without its leading HALT, 24
/// lines of 32 character codes each ended by NEWLINE (0x76).
pub(super) fn decode_raw(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != ROWS * (COLUMNS + 1) {
        return Err(DecodeError::Unrecognized);
    }
    let mut codes = [0; ROWS * COLUMNS];
    for (line, cells) in data
        .chunks_exact(COLUMNS + 1)
        .zip(codes.chunks_exact_mut(COLUMNS))
    {
        let text = line
            .strip_suffix(&[NEWLINE])
            .ok_or(DecodeError::Unrecognized)?;
        cells.copy_from_slice(text);
    }
    Ok(render(&codes))
}

/// ZXpaintyONE `.ZP1`: the 768 character codes as two hex digits each
/// (either case); anything after them is ignored.
pub(super) fn decode_zp1(data: &[u8]) -> Result<Image, DecodeError> {
    let digits = data
        .get(..2 * ROWS * COLUMNS)
        .ok_or(DecodeError::Unrecognized)?;
    let mut codes = [0; ROWS * COLUMNS];
    for (code, pair) in codes.iter_mut().zip(digits.chunks_exact(2)) {
        let hex = |c: u8| char::from(c).to_digit(16).ok_or(DecodeError::Unrecognized);
        *code = (hex(pair[0])? << 4 | hex(pair[1])?) as u8;
    }
    Ok(render(&codes))
}

/// A 32x24 character screen, row by row; codes 0x40-0x7F and 0xC0-0xFF
/// show the glyph of their low 6 bits like the others.
fn render(codes: &[u8]) -> Image {
    let mut image = Image::new((COLUMNS * 8) as u32, (ROWS * 8) as u32);
    for (i, &code) in codes.iter().enumerate() {
        draw_char(&mut image, i % COLUMNS, i / COLUMNS, code);
    }
    image
}

/// The screen a picture program prints, or `None` if it isn't one.
fn run(mut program: &[u8]) -> Option<[[u8; COLUMNS]; ROWS]> {
    let mut screen = Screen::default();
    let mut printed = false;
    let mut bottom_lines: Option<&[u8]> = None;
    while let Some(header) = program.get(..4) {
        let len = usize::from(u16::from_le_bytes([header[2], header[3]]));
        let line = program.get(4..4 + len)?;
        program = &program[4 + len..];
        let statement = line.strip_suffix(&[NEWLINE])?;
        let (&keyword, rest) = statement.split_first()?;
        match keyword {
            FAST | CLS | CLEAR | SLOW if !printed => {}
            PRINT => {
                screen.print(rest)?;
                printed = true;
            }
            LET if bottom_lines.is_some() => break,
            LET => {
                let text = rest.strip_prefix(&[LETTER_A, DOLLAR, EQUALS, QUOTE])?;
                let text = text.strip_suffix(&[QUOTE])?;
                if text.len() != 2 * COLUMNS {
                    return None;
                }
                bottom_lines = Some(text);
                printed = true;
            }
            STOP | PAUSE | GOTO => break,
            _ => return None,
        }
    }
    if let Some(text) = bottom_lines {
        for (row, chunk) in screen.codes[ROWS - 2..]
            .iter_mut()
            .zip(text.chunks(COLUMNS))
        {
            for (cell, &code) in row.iter_mut().zip(chunk) {
                *cell = printable(code)?;
            }
        }
    }
    Some(screen.codes)
}

/// The character code shown for `code` in a string literal.
fn printable(code: u8) -> Option<u8> {
    match code {
        DOUBLE_QUOTE => Some(QUOTE),
        0x00..=0x3f | 0x80..=0xbf => Some(code),
        _ => None,
    }
}

#[derive(Default)]
struct Screen {
    codes: [[u8; COLUMNS]; ROWS],
    row: usize,
    column: usize,
}

impl Screen {
    /// Runs the items of one `PRINT` statement: string literals, `AT row,
    /// column` and `;`. Returns `None` for anything else.
    fn print(&mut self, mut items: &[u8]) -> Option<()> {
        let mut newline = true;
        while let Some((&item, rest)) = items.split_first() {
            items = rest;
            match item {
                // A space between items changes nothing.
                0 => continue,
                SEMICOLON => {
                    newline = false;
                    continue;
                }
                QUOTE => {
                    let end = items.iter().position(|&c| c == QUOTE)?;
                    for &code in &items[..end] {
                        self.put(printable(code)?);
                    }
                    items = &items[end + 1..];
                }
                AT => {
                    let (row, rest) = number(items)?;
                    let (column, rest) = number(rest.strip_prefix(&[COMMA])?)?;
                    if row >= ROWS || column >= COLUMNS {
                        return None;
                    }
                    (self.row, self.column) = (row, column);
                    items = rest;
                }
                _ => return None,
            }
            newline = true;
        }
        if newline {
            self.row += 1;
            self.column = 0;
        }
        Some(())
    }

    /// Prints one character, wrapping at the end of a line; characters
    /// below the screen are dropped.
    fn put(&mut self, code: u8) {
        if self.column == COLUMNS {
            self.row += 1;
            self.column = 0;
        }
        if let Some(row) = self.codes.get_mut(self.row) {
            row[self.column] = code;
        }
        self.column += 1;
    }
}

/// A number literal: its digits, then 0x7E and the value as a 5-byte float
/// (exponent + 128, then a 32-bit mantissa whose top bit is the sign).
/// Returns the non-negative integer part and the bytes after it.
fn number(items: &[u8]) -> Option<(usize, &[u8])> {
    let marker = items.iter().position(|&c| c == NUMBER)?;
    // Only digits and '.' may come before the value (not a variable).
    if marker == 0 || !items[..marker].iter().all(|c| (0x1b..=0x25).contains(c)) {
        return None;
    }
    let float = items.get(marker + 1..marker + 6)?;
    let rest = &items[marker + 6..];
    let exponent = i32::from(float[0]) - 128;
    if float[0] == 0 || exponent <= 0 {
        return Some((0, rest));
    }
    if float[1] & 0x80 != 0 || exponent > 16 {
        return None;
    }
    let mantissa = u32::from_be_bytes([float[1] | 0x80, float[2], float[3], float[4]]);
    Some(((mantissa >> (32 - exponent)) as usize, rest))
}

/// Draws character `code` (bits 5-0; bit 7 inverse), black on white.
fn draw_char(image: &mut Image, column: usize, row: usize, code: u8) {
    let glyph = &ZX81_FONT[usize::from(code & 0x3f)];
    let inverse = code & 0x80 != 0;
    for (y, &bits) in glyph.iter().enumerate() {
        for x in 0..8 {
            let ink = (bits & (0x80 >> x) != 0) != inverse;
            let color = if ink { 0x000000 } else { 0xffffff };
            image.set((column * 8 + x) as u32, (row * 8 + y) as u32, color);
        }
    }
}

/// ZX81 ROM 0x1E00-0x1FFF (see the module documentation).
const ZX81_FONT: [[u8; 8]; 64] = [
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0xf0, 0xf0, 0xf0, 0xf0, 0x00, 0x00, 0x00, 0x00],
    [0x0f, 0x0f, 0x0f, 0x0f, 0x00, 0x00, 0x00, 0x00],
    [0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0xf0, 0xf0, 0xf0, 0xf0],
    [0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0, 0xf0],
    [0x0f, 0x0f, 0x0f, 0x0f, 0xf0, 0xf0, 0xf0, 0xf0],
    [0xff, 0xff, 0xff, 0xff, 0xf0, 0xf0, 0xf0, 0xf0],
    [0xaa, 0x55, 0xaa, 0x55, 0xaa, 0x55, 0xaa, 0x55],
    [0x00, 0x00, 0x00, 0x00, 0xaa, 0x55, 0xaa, 0x55],
    [0xaa, 0x55, 0xaa, 0x55, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x24, 0x24, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x1c, 0x22, 0x78, 0x20, 0x20, 0x7e, 0x00],
    [0x00, 0x08, 0x3e, 0x28, 0x3e, 0x0a, 0x3e, 0x08],
    [0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x10, 0x00],
    [0x00, 0x3c, 0x42, 0x04, 0x08, 0x00, 0x08, 0x00],
    [0x00, 0x04, 0x08, 0x08, 0x08, 0x08, 0x04, 0x00],
    [0x00, 0x20, 0x10, 0x10, 0x10, 0x10, 0x20, 0x00],
    [0x00, 0x00, 0x10, 0x08, 0x04, 0x08, 0x10, 0x00],
    [0x00, 0x00, 0x04, 0x08, 0x10, 0x08, 0x04, 0x00],
    [0x00, 0x00, 0x00, 0x3e, 0x00, 0x3e, 0x00, 0x00],
    [0x00, 0x00, 0x08, 0x08, 0x3e, 0x08, 0x08, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x3e, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x14, 0x08, 0x3e, 0x08, 0x14, 0x00],
    [0x00, 0x00, 0x02, 0x04, 0x08, 0x10, 0x20, 0x00],
    [0x00, 0x00, 0x10, 0x00, 0x00, 0x10, 0x10, 0x20],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x08, 0x10],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x00],
    [0x00, 0x3c, 0x46, 0x4a, 0x52, 0x62, 0x3c, 0x00],
    [0x00, 0x18, 0x28, 0x08, 0x08, 0x08, 0x3e, 0x00],
    [0x00, 0x3c, 0x42, 0x02, 0x3c, 0x40, 0x7e, 0x00],
    [0x00, 0x3c, 0x42, 0x0c, 0x02, 0x42, 0x3c, 0x00],
    [0x00, 0x08, 0x18, 0x28, 0x48, 0x7e, 0x08, 0x00],
    [0x00, 0x7e, 0x40, 0x7c, 0x02, 0x42, 0x3c, 0x00],
    [0x00, 0x3c, 0x40, 0x7c, 0x42, 0x42, 0x3c, 0x00],
    [0x00, 0x7e, 0x02, 0x04, 0x08, 0x10, 0x10, 0x00],
    [0x00, 0x3c, 0x42, 0x3c, 0x42, 0x42, 0x3c, 0x00],
    [0x00, 0x3c, 0x42, 0x42, 0x3e, 0x02, 0x3c, 0x00],
    [0x00, 0x3c, 0x42, 0x42, 0x7e, 0x42, 0x42, 0x00],
    [0x00, 0x7c, 0x42, 0x7c, 0x42, 0x42, 0x7c, 0x00],
    [0x00, 0x3c, 0x42, 0x40, 0x40, 0x42, 0x3c, 0x00],
    [0x00, 0x78, 0x44, 0x42, 0x42, 0x44, 0x78, 0x00],
    [0x00, 0x7e, 0x40, 0x7c, 0x40, 0x40, 0x7e, 0x00],
    [0x00, 0x7e, 0x40, 0x7c, 0x40, 0x40, 0x40, 0x00],
    [0x00, 0x3c, 0x42, 0x40, 0x4e, 0x42, 0x3c, 0x00],
    [0x00, 0x42, 0x42, 0x7e, 0x42, 0x42, 0x42, 0x00],
    [0x00, 0x3e, 0x08, 0x08, 0x08, 0x08, 0x3e, 0x00],
    [0x00, 0x02, 0x02, 0x02, 0x42, 0x42, 0x3c, 0x00],
    [0x00, 0x44, 0x48, 0x70, 0x48, 0x44, 0x42, 0x00],
    [0x00, 0x40, 0x40, 0x40, 0x40, 0x40, 0x7e, 0x00],
    [0x00, 0x42, 0x66, 0x5a, 0x42, 0x42, 0x42, 0x00],
    [0x00, 0x42, 0x62, 0x52, 0x4a, 0x46, 0x42, 0x00],
    [0x00, 0x3c, 0x42, 0x42, 0x42, 0x42, 0x3c, 0x00],
    [0x00, 0x7c, 0x42, 0x42, 0x7c, 0x40, 0x40, 0x00],
    [0x00, 0x3c, 0x42, 0x42, 0x52, 0x4a, 0x3c, 0x00],
    [0x00, 0x7c, 0x42, 0x42, 0x7c, 0x44, 0x42, 0x00],
    [0x00, 0x3c, 0x40, 0x3c, 0x02, 0x42, 0x3c, 0x00],
    [0x00, 0xfe, 0x10, 0x10, 0x10, 0x10, 0x10, 0x00],
    [0x00, 0x42, 0x42, 0x42, 0x42, 0x42, 0x3c, 0x00],
    [0x00, 0x42, 0x42, 0x42, 0x42, 0x24, 0x18, 0x00],
    [0x00, 0x42, 0x42, 0x42, 0x42, 0x5a, 0x24, 0x00],
    [0x00, 0x42, 0x24, 0x18, 0x18, 0x24, 0x42, 0x00],
    [0x00, 0x82, 0x44, 0x28, 0x10, 0x10, 0x10, 0x00],
    [0x00, 0x7e, 0x04, 0x08, 0x10, 0x20, 0x7e, 0x00],
];

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use alloc::vec::Vec;

    /// A `.P` file holding `lines` (statement text without NEWLINE).
    fn p_file(lines: &[&[u8]]) -> Vec<u8> {
        let mut data = vec![0; PROGRAM];
        for (number, text) in (10u16..).step_by(10).zip(lines) {
            data.extend_from_slice(&number.to_be_bytes());
            data.extend_from_slice(&(text.len() as u16 + 1).to_le_bytes());
            data.extend_from_slice(text);
            data.push(NEWLINE);
        }
        let d_file = (0x4009 + data.len()) as u16;
        data[D_FILE..D_FILE + 2].copy_from_slice(&d_file.to_le_bytes());
        data.push(NEWLINE);
        data
    }

    /// `AT 2,3;` with the numbers as ZX81 literals.
    const AT_2_3: [u8; 17] = [
        AT, 0x1e, NUMBER, 0x82, 0, 0, 0, 0, COMMA, 0x1f, NUMBER, 0x82, 0x40, 0, 0, 0, SEMICOLON,
    ];

    #[test]
    fn prints_strings_at_positions() {
        let mut print = vec![PRINT];
        print.extend_from_slice(&AT_2_3);
        print.extend_from_slice(&[QUOTE, 0x80, DOUBLE_QUOTE, QUOTE]);
        let image = decode_p(&p_file(&[&[CLS], &print, &[STOP]])).unwrap();
        // Inverse space at row 2, column 3: a black cell.
        assert_eq!(image.get(3 * 8, 2 * 8), 0x000000);
        assert_eq!(image.get(2 * 8, 2 * 8), 0xffffff);
        // The quote glyph has ink in its second line.
        assert_eq!(image.get(4 * 8 + 2, 2 * 8 + 1), 0x000000);
    }

    #[test]
    fn rejects_programs_that_are_not_pictures() {
        let print: &[u8] = &[PRINT, QUOTE, 0x26, QUOTE];
        assert!(decode_p(&p_file(&[print, &[STOP]])).is_ok());
        assert!(decode_p(&p_file(&[&[0xea], print])).is_err()); // REM
        assert!(decode_p(&p_file(&[print, &[CLS]])).is_err());
    }

    #[test]
    fn zxpaintyone_bottom_lines() {
        let mut let_a = vec![LET, LETTER_A, DOLLAR, EQUALS, QUOTE];
        let_a.extend_from_slice(&[0x80; 64]);
        let_a.push(QUOTE);
        let image = decode_p(&p_file(&[&[PRINT], &let_a, &[LET, 0x38]])).unwrap();
        assert_eq!(image.get(0, 22 * 8), 0x000000);
        assert_eq!(image.get(0, 21 * 8), 0xffffff);
    }

    #[test]
    fn raw_needs_a_newline_after_every_line() {
        let mut raw = [0u8; ROWS * (COLUMNS + 1)];
        for line in raw.chunks_exact_mut(COLUMNS + 1) {
            line[COLUMNS] = NEWLINE;
        }
        raw[COLUMNS + 1] = 0x80; // Inverse space at the start of row 1.
        let image = decode_raw(&raw).unwrap();
        assert_eq!(image.get(0, 8), 0x000000);
        assert_eq!(image.get(0, 0), 0xffffff);
        raw[COLUMNS] = 0;
        assert!(decode_raw(&raw).is_err());
    }

    #[test]
    fn zp1_reads_hex_digits_and_ignores_the_rest() {
        let mut zp1 = b"0a".repeat(ROWS * COLUMNS);
        zp1[2..4].copy_from_slice(b"80");
        zp1.extend_from_slice(b"\r\nanything");
        let image = decode_zp1(&zp1).unwrap();
        assert_eq!(image.get(8, 0), 0x000000); // Inverse space.
        zp1[0] = b'g';
        assert!(decode_zp1(&zp1).is_err());
        assert!(decode_zp1(&zp1[1..2 * ROWS * COLUMNS]).is_err());
    }
}
