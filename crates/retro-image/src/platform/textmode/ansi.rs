//! ANSI art (ANS): text with ANSI X3.64 / ECMA-48 escape sequences as
//! interpreted by DOS ANSI.SYS and BBS terminals.
//!
//! Sources:
//! - ECMA-48, 5th edition (<https://ecma-international.org/publications-and-standards/standards/ecma-48/>):
//!   control sequence syntax (ESC [, parameter bytes, a final byte 40h-7Eh),
//!   CUU/CUD/CUF/CUB, CUP/HVP, ED, EL, SGR.
//! - Deark `modules/ansiart.c` (<https://github.com/jsummers/deark>, MIT
//!   licence): the behaviour art viewers settled on. A line feed also
//!   returns the cursor to column 0; SGR bold brightens the foreground and,
//!   in non-blink mode, blink brightens the background, both applied as the
//!   character is written, before reverse video swaps them; erased cells are
//!   light grey spaces on black; `ESC [ ? 33 h` turns non-blink mode on;
//!   PabloDraw's 24-bit colour sequence `ESC [ 0|1 ; r ; g ; b t`
//!   (<http://picoe.ca/2014/03/07/24-bit-ansi/>) and SGR 38/48 `;2;r;g;b`;
//!   SGR 90-97 and 100-107 select bright colours; the SAUCE width is used
//!   only from 40 to 2048 columns. Like Deark, the picture is as tall as the
//!   lowest row written to.
//! - The SAUCE specification (<https://www.acid.org/info/sauce/sauce.htm>):
//!   the EOF character (1Ah) ends the text.
//! - DOS console behaviour, as in FSC-0037 (<http://ftsc.org/docs/fsc-0037.001>)
//!   for Avatar: BEL makes no mark, backspace moves left, tab moves to the
//!   next multiple of 8. ANSI.SYS keeps the cursor on screen: moves stop at
//!   the edges and a row or column of 0 means 1.

// The behaviour listed above as taken from Deark follows Deark
// `modules/ansiart.c`, under this licence:
//
// Copyright (C) 2016-2026 Jason Summers
// <jason1@pobox.com>
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

use super::sauce::{self, Sauce};
use super::screen::{Cell, PALETTE, Terminal};
use crate::{DecodeError, Image};

/// SAUCE FileTypes of DataType "Character" that use ANSiFlags: plain text,
/// ANSI and ANSiMation.
const ANSI_FILE_TYPES: [u8; 3] = [0, 1, 2];

/// Most parameters read from one control sequence.
const MAX_PARAMS: usize = 16;

/// ANSI colour numbers (SGR 30-37) to PC attribute colours.
const ANSI_TO_PC: [u8; 8] = [0, 4, 2, 6, 1, 5, 3, 7];

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    let (text, sauce) = sauce::split(data);
    let sauce = sauce.filter(|s| ANSI_FILE_TYPES.iter().any(|&t| s.is_character(t)));
    let width = sauce.as_ref().and_then(Sauce::width).unwrap_or(80);
    let ice = sauce.as_ref().is_some_and(Sauce::ice);
    let mut ansi = Ansi::new(width, ice).ok_or(DecodeError::Unrecognized)?;
    ansi.feed(text);
    ansi.terminal.finish(&sauce::style_of(sauce.as_ref()))
}

/// A colour: one of the 16 palette entries or 24-bit RGB.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Color {
    Index(u8),
    Rgb(u32),
}

/// The current graphic rendition (SGR state).
#[derive(Debug, Clone, Copy)]
struct Rendition {
    fg: Color,
    bg: Color,
    bold: bool,
    blink: bool,
    reverse: bool,
    conceal: bool,
}

impl Rendition {
    const DEFAULT: Self = Self {
        fg: Color::Index(7),
        bg: Color::Index(0),
        bold: false,
        blink: false,
        reverse: false,
        conceal: false,
    };
}

/// The ANSI interpreter: a terminal and the rendition state.
struct Ansi {
    terminal: Terminal,
    rendition: Rendition,
    /// Non-blink mode: blink brightens the background.
    ice: bool,
    saved: (usize, usize),
}

impl Ansi {
    fn new(width: usize, ice: bool) -> Option<Self> {
        Some(Self {
            terminal: Terminal::new(width)?,
            rendition: Rendition::DEFAULT,
            ice,
            saved: (0, 0),
        })
    }

    fn feed(&mut self, text: &[u8]) {
        let mut i = 0;
        while let Some(&byte) = text.get(i) {
            i += 1;
            match byte {
                0x1a => return,
                0x1b if text.get(i) == Some(&b'[') => {
                    let params_at = i + 1;
                    let Some(len) = text[params_at..]
                        .iter()
                        .position(|b| (0x40..=0x7e).contains(b))
                    else {
                        return; // unterminated sequence at the end
                    };
                    let end = params_at + len;
                    self.control(&text[params_at..end], text[end]);
                    i = end + 1;
                }
                0x1b | 0x07 => {}
                b'\r' => self.terminal.x = 0,
                b'\n' => {
                    self.terminal.x = 0;
                    self.terminal.y = self.terminal.y.saturating_add(1);
                }
                0x08 => self.terminal.x = self.terminal.x.saturating_sub(1),
                b'\t' => {
                    let x = (self.terminal.x / 8 + 1) * 8;
                    self.terminal.move_to(x, self.terminal.y);
                }
                glyph => self.write(glyph),
            }
        }
    }

    fn write(&mut self, glyph: u8) {
        let r = &self.rendition;
        let fg = match r.fg {
            Color::Index(i) => PALETTE[usize::from(i | u8::from(r.bold) << 3)],
            Color::Rgb(rgb) => rgb,
        };
        let bg = match r.bg {
            Color::Index(i) => PALETTE[usize::from(i | u8::from(r.blink && self.ice) << 3)],
            Color::Rgb(rgb) => rgb,
        };
        let (fg, bg) = if r.reverse { (bg, fg) } else { (fg, bg) };
        let fg = if r.conceal { bg } else { fg };
        self.terminal.put(Cell { glyph, fg, bg });
    }

    /// Executes the control sequence `ESC [ params final`.
    fn control(&mut self, params: &[u8], final_byte: u8) {
        let private = params.first() == Some(&b'?');
        let mut values = [None; MAX_PARAMS];
        let count = parse_params(params, &mut values);
        let values = &values[..count];
        // The first parameter, with 0 or none meaning 1 for cursor moves.
        let n = values.first().copied().flatten().unwrap_or(0) as usize;
        let steps = n.max(1);
        let t = &mut self.terminal;
        match final_byte {
            b'A' => t.y = t.y.saturating_sub(steps),
            b'B' => t.y = t.y.saturating_add(steps),
            b'C' => t.move_to(t.x.saturating_add(steps), t.y),
            b'D' => t.x = t.x.saturating_sub(steps),
            b'H' | b'f' => {
                let at = |i: usize| values.get(i).copied().flatten().unwrap_or(1).max(1) as usize;
                t.move_to(at(1) - 1, at(0) - 1);
            }
            b'J' => self.erase_display(n),
            b'K' => self.erase_line(n),
            b'm' => self.select_rendition(values),
            b's' => self.saved = (t.x, t.y),
            b'u' => t.move_to(self.saved.0, self.saved.1),
            b't' => self.true_color(values),
            b'h' | b'l' if private && values.contains(&Some(33)) => self.ice = final_byte == b'h',
            _ => {}
        }
    }

    /// ED: 0 erases from the cursor to the end of the screen, 1 from the
    /// start to the cursor, 2 everything (and homes the cursor).
    fn erase_display(&mut self, mode: usize) {
        let t = &mut self.terminal;
        if mode == 2 {
            return t.clear();
        }
        if mode == 1 {
            t.erase_to_cursor();
        } else {
            t.erase_from_cursor();
        }
    }

    /// EL: 0 erases from the cursor to the end of the line, 1 from the start
    /// to the cursor, 2 the whole line.
    fn erase_line(&mut self, mode: usize) {
        let t = &mut self.terminal;
        let columns = match mode {
            0 => t.x..t.width(),
            1 => 0..t.x + 1,
            _ => 0..t.width(),
        };
        for column in columns {
            t.erase(column, t.y, Cell::BLANK);
        }
    }

    fn select_rendition(&mut self, values: &[Option<u32>]) {
        let r = &mut self.rendition;
        let mut i = 0;
        while i < values.len() {
            let code = values[i].unwrap_or(0);
            i += 1;
            match code {
                0 => *r = Rendition::DEFAULT,
                1 => r.bold = true,
                5 | 6 => r.blink = true,
                7 => r.reverse = true,
                8 => r.conceal = true,
                22 => r.bold = false,
                25 => r.blink = false,
                27 => r.reverse = false,
                28 => r.conceal = false,
                30..=37 => r.fg = Color::Index(ANSI_TO_PC[code as usize - 30]),
                39 => r.fg = Rendition::DEFAULT.fg,
                40..=47 => r.bg = Color::Index(ANSI_TO_PC[code as usize - 40]),
                49 => r.bg = Rendition::DEFAULT.bg,
                90..=97 => r.fg = Color::Index(ANSI_TO_PC[code as usize - 90] | 8),
                100..=107 => r.bg = Color::Index(ANSI_TO_PC[code as usize - 100] | 8),
                38 | 48 => {
                    // 38;2;r;g;b sets 24-bit colour; 38;5;n (256 colours) is skipped.
                    let mode = values.get(i).copied().flatten();
                    let len = match mode {
                        Some(2) => 4,
                        Some(5) => 2,
                        _ => 1,
                    };
                    if mode == Some(2) {
                        let rgb = rgb(values.get(i + 1..i + 4).unwrap_or(&[]));
                        if code == 38 {
                            r.fg = rgb;
                        } else {
                            r.bg = rgb;
                        }
                    }
                    i += len;
                }
                _ => {}
            }
        }
    }

    /// PabloDraw's `ESC [ 0 ; r ; g ; b t` (background) and `1;...` (foreground).
    fn true_color(&mut self, values: &[Option<u32>]) {
        let value = |i: usize| values[i].unwrap_or(0);
        if values.len() != 4 || value(0) > 1 || (1..4).any(|i| value(i) > 255) {
            return;
        }
        let color = Color::Rgb(value(1) << 16 | value(2) << 8 | value(3));
        if value(0) == 0 {
            self.rendition.bg = color;
        } else {
            self.rendition.fg = color;
        }
    }
}

/// A colour from three parameters (red, green, blue), each modulo 256.
fn rgb(values: &[Option<u32>]) -> Color {
    let channel = |i: usize| values.get(i).copied().flatten().unwrap_or(0) & 0xff;
    Color::Rgb(channel(0) << 16 | channel(1) << 8 | channel(2))
}

/// Parses `;`-separated decimal parameters into `out` (an empty one is
/// `None`), ignoring a leading private-use marker; returns the count.
/// Values saturate; parameters past `out`'s length are dropped.
fn parse_params(params: &[u8], out: &mut [Option<u32>]) -> usize {
    let params = params.strip_prefix(b"?").unwrap_or(params);
    let mut count = 0;
    for (slot, param) in out.iter_mut().zip(params.split(|&b| b == b';')) {
        let digits = param.iter().take_while(|b| b.is_ascii_digit());
        *slot = digits.fold(None, |value: Option<u32>, &d| {
            Some(
                value
                    .unwrap_or(0)
                    .saturating_mul(10)
                    .saturating_add(u32::from(d - b'0')),
            )
        });
        count += 1;
    }
    count
}

#[cfg(test)]
mod tests {
    use super::super::sauce::with_sauce;
    use super::*;

    /// Interprets `text` on a terminal `width` columns wide.
    fn run(width: usize, text: &[u8]) -> Ansi {
        let mut ansi = Ansi::new(width, false).unwrap();
        ansi.feed(text);
        ansi
    }

    fn glyph_at(ansi: &Ansi, x: usize, y: usize) -> u8 {
        ansi.terminal.get(x, y).glyph
    }

    #[test]
    fn parses_parameters() {
        let mut out = [None; 4];
        assert_eq!(parse_params(b"1;;99999999999", &mut out), 3);
        assert_eq!(out[..3], [Some(1), None, Some(u32::MAX)]);
        assert_eq!(parse_params(b"?33", &mut out), 1);
        assert_eq!(out[0], Some(33));
        assert_eq!(parse_params(b"", &mut out), 1);
        assert_eq!(out[0], None);
        assert_eq!(parse_params(b"1;2;3;4;5;6", &mut out), 4, "extra dropped");
    }

    #[test]
    fn moves_the_cursor_within_the_screen() {
        let ansi = run(80, b"\x1b[5;10Hx\x1b[2Ay\x1b[100Cz\x1b[200D\x1b[Bw");
        assert_eq!(glyph_at(&ansi, 9, 4), b'x');
        assert_eq!(glyph_at(&ansi, 10, 2), b'y');
        // CUF stops at the last column; the next character wraps.
        assert_eq!(glyph_at(&ansi, 79, 2), b'z');
        assert_eq!(
            glyph_at(&ansi, 0, 4),
            b'w',
            "z wrapped to row 3, B moved down"
        );
        let ansi = run(80, b"\x1b[0;0Ha\x1b[99Ab\x1b[s\x1b[10;10H\x1b[uc");
        assert_eq!(glyph_at(&ansi, 0, 0), b'a', "0 means 1");
        assert_eq!(glyph_at(&ansi, 1, 0), b'b', "CUU stops at the top");
        assert_eq!(glyph_at(&ansi, 2, 0), b'c', "restored position");
    }

    #[test]
    fn wraps_at_the_width_and_line_feed_returns() {
        let ansi = run(4, b"abcdef\nxy\r\tZ");
        assert_eq!(glyph_at(&ansi, 3, 0), b'd');
        assert_eq!(glyph_at(&ansi, 1, 1), b'f');
        assert_eq!(glyph_at(&ansi, 0, 2), b'x', "LF also returns to column 0");
        assert_eq!(glyph_at(&ansi, 3, 2), b'Z', "tab stops at the last column");
        assert_eq!(ansi.terminal.y, 3);
    }

    #[test]
    fn eof_and_unterminated_sequences_end_the_text() {
        let ansi = run(80, b"ab\x1acd");
        assert_eq!(glyph_at(&ansi, 2, 0), b' ');
        let ansi = run(80, b"ab\x1b[1;3");
        assert_eq!(ansi.terminal.x, 2);
        let ansi = run(80, b"a\x07\x08b\x1bc");
        assert_eq!(glyph_at(&ansi, 0, 0), b'b', "BEL is silent, BS moves back");
        assert_eq!(glyph_at(&ansi, 1, 0), b'c', "a lone ESC is dropped");
    }

    #[test]
    fn selects_colours_bold_blink_and_reverse() {
        let ansi = run(
            80,
            b"\x1b[1;31;44ma\x1b[0;5;47mb\x1b[7mc\x1b[0;8;33md\x1b[mE",
        );
        let cell = |x| ansi.terminal.get(x, 0);
        assert_eq!((cell(0).fg, cell(0).bg), (PALETTE[12], PALETTE[1]));
        assert_eq!(
            cell(1).bg,
            PALETTE[7],
            "blink without iCE keeps the background dark"
        );
        assert_eq!((cell(2).fg, cell(2).bg), (PALETTE[7], PALETTE[7]));
        assert_eq!(cell(3).fg, cell(3).bg, "concealed");
        assert_eq!((cell(4).fg, cell(4).bg), (PALETTE[7], PALETTE[0]));
        let ansi = run(80, b"\x1b[?33h\x1b[5;46ma\x1b[?33l\x1b[92;103mb");
        assert_eq!(
            ansi.terminal.get(0, 0).bg,
            PALETTE[11],
            "iCE: bright background"
        );
        let cell = ansi.terminal.get(1, 0);
        assert_eq!((cell.fg, cell.bg), (PALETTE[10], PALETTE[14]));
    }

    #[test]
    fn selects_24_bit_colours() {
        let ansi = run(
            80,
            b"\x1b[1;255;128;0ta\x1b[0;1;2;3tb\x1b[38;2;9;8;7;48;5;3mc",
        );
        assert_eq!(ansi.terminal.get(0, 0).fg, 0xff8000);
        assert_eq!(ansi.terminal.get(1, 0).bg, 0x010203);
        assert_eq!(ansi.terminal.get(2, 0).fg, 0x090807);
        let ansi = run(80, b"\x1b[2;1;1;1t\x1b[1;256;0;0tx");
        assert_eq!(ansi.terminal.get(0, 0).fg, PALETTE[7], "invalid t ignored");
    }

    #[test]
    fn erases_without_growing_the_picture() {
        let ansi = run(80, b"abc\x1b[1;2H\x1b[K");
        assert_eq!(glyph_at(&ansi, 0, 0), b'a');
        assert_eq!(glyph_at(&ansi, 1, 0), b' ');
        let ansi = run(80, b"abc\ndef\x1b[1;2H\x1b[1J");
        assert_eq!((glyph_at(&ansi, 1, 0), glyph_at(&ansi, 2, 0)), (b' ', b'c'));
        let ansi = run(80, b"abc\ndef\x1b[2Jx\x1b[5;1H\x1b[J");
        assert_eq!(glyph_at(&ansi, 0, 0), b'x');
        assert_eq!(glyph_at(&ansi, 0, 1), b' ');
        let image = ansi.terminal.finish(&sauce::DEFAULT_STYLE).unwrap();
        assert_eq!(image.height(), 32, "two rows were written to");
    }

    #[test]
    fn honours_sauce_width_and_flags() {
        let text = [b'x'; 100];
        let data = with_sauce(&text, 1, 1, 50, 0x15, b"IBM VGA", &[]);
        let image = decode(&data).unwrap();
        assert_eq!((image.width(), image.height()), (50 * 9, 2 * 16));
        let narrow = with_sauce(&text, 1, 1, 20, 0, b"", &[]);
        assert_eq!(
            decode(&narrow).unwrap().width(),
            80 * 8,
            "implausible width"
        );
        let other = with_sauce(&text, 1, 8, 50, 0, b"", &[]);
        assert_eq!(
            decode(&other).unwrap().width(),
            80 * 8,
            "not an ANSI record"
        );
        assert!(decode(b"").is_err());
    }
}
