//! Avatar (AVT): BBS text with the compact AVT/0 colour and cursor codes.
//!
//! Sources:
//! - FSC-0025, "AVATAR" by George A. Stanislav (<http://ftsc.org/docs/fsc-0025.001>):
//!   the basic commands. ^L clears the screen and sets attribute 3 (cyan on
//!   black, also the starting attribute); ^Y repeats a character; ^V^A sets
//!   the attribute (bit 7 masked off), ^V^B turns blink on, ^V^C to ^V^F move
//!   the cursor up, down, left and right (not past the edges), ^V^G clears
//!   to the end of the line in the current attribute, ^V^H moves to a row and
//!   column (1-based).
//! - FSC-0037 (<http://ftsc.org/docs/fsc-0037.001>), the AVT/0+ extension:
//!   the argument lengths of ^V^I to ^V^N and ^V^Y, which are skipped (not
//!   drawn), and the control characters: carriage return goes to column 0,
//!   line feed moves down only, backspace moves left, tab moves to the next
//!   multiple of 8.
//! - The SAUCE specification (<https://www.acid.org/info/sauce/sauce.htm>):
//!   DataType 1, FileType 5 is Avatar, with the width in TInfo1; the EOF
//!   character (1Ah) ends the text.
//!
//! Other ^V commands belong to AVT/1, which isn't documented (e.g. the
//! `DEMO1.AVT` dexvert sample): such files are rejected.

use super::sauce::{self, DEFAULT_STYLE, Sauce};
use super::screen::{Cell, PALETTE, Terminal};
use crate::{DecodeError, Image};

const AVATAR_FILE_TYPE: u8 = 5;
const DEFAULT_ATTRIBUTE: u8 = 3;

/// Decodes text with at least one Avatar command: the `.avt` extension
/// is not exclusive to Avatar.
pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let (text, sauce) = sauce::split(data);
    if !text.iter().any(|b| matches!(b, 0x0c | 0x16 | 0x19)) {
        return Err(fail);
    }
    let width = sauce
        .filter(|s| s.is_character(AVATAR_FILE_TYPE))
        .as_ref()
        .and_then(Sauce::width)
        .unwrap_or(80);
    let mut avatar = Avatar {
        terminal: Terminal::new(width).ok_or(fail)?,
        attribute: DEFAULT_ATTRIBUTE,
    };
    avatar.feed(text)?;
    avatar.terminal.finish(&DEFAULT_STYLE)
}

struct Avatar {
    terminal: Terminal,
    attribute: u8,
}

impl Avatar {
    fn cell(&self, glyph: u8) -> Cell {
        Cell::from_attribute(glyph, self.attribute, &PALETTE, false)
    }

    /// Interprets `text`; fails on an unknown ^V command. A command cut
    /// short by the end of the text is ignored.
    fn feed(&mut self, text: &[u8]) -> Result<(), DecodeError> {
        let mut i = 0;
        while let Some(&byte) = text.get(i) {
            let arg = |n: usize| text.get(i + n).copied();
            let t = &mut self.terminal;
            i += match byte {
                0x1a => break,
                0x0c => {
                    t.clear();
                    self.attribute = DEFAULT_ATTRIBUTE;
                    1
                }
                0x19 => {
                    let (Some(glyph), Some(count)) = (arg(1), arg(2)) else {
                        break;
                    };
                    let cell = self.cell(glyph);
                    (0..count).for_each(|_| self.terminal.put(cell));
                    3
                }
                0x16 => {
                    let Some(command) = arg(1) else { break };
                    2 + self.command(command, &text[i + 2..])?
                }
                b'\r' => {
                    t.x = 0;
                    1
                }
                b'\n' => {
                    t.y = t.y.saturating_add(1);
                    1
                }
                0x08 => {
                    t.x = t.x.saturating_sub(1);
                    1
                }
                b'\t' => {
                    t.move_to((t.x / 8 + 1) * 8, t.y);
                    1
                }
                glyph => {
                    let cell = self.cell(glyph);
                    self.terminal.put(cell);
                    1
                }
            };
        }
        Ok(())
    }

    /// Executes ^V `command` with the bytes after it; returns how many of
    /// them it used.
    fn command(&mut self, command: u8, args: &[u8]) -> Result<usize, DecodeError> {
        let t = &mut self.terminal;
        let used = match command {
            1 => {
                let Some(&attribute) = args.first() else {
                    return Ok(args.len());
                };
                self.attribute = attribute & 0x7f;
                1
            }
            2 => {
                self.attribute |= 0x80;
                0
            }
            3 => {
                t.y = t.y.saturating_sub(1);
                0
            }
            4 => {
                t.y = t.y.saturating_add(1);
                0
            }
            5 => {
                t.x = t.x.saturating_sub(1);
                0
            }
            6 => {
                t.move_to(t.x + 1, t.y);
                0
            }
            7 => {
                let blank = Cell::from_attribute(b' ', self.attribute, &PALETTE, false);
                for x in t.x..t.width() {
                    t.erase(x, t.y, blank);
                }
                0
            }
            8 => {
                let [row, column, ..] = *args else {
                    return Ok(args.len());
                };
                let at = |v: u8| usize::from(v.max(1) - 1);
                t.move_to(at(column), at(row));
                2
            }
            // AVT/0+: insert mode, scroll up/down, clear and fill area,
            // delete character, repeat pattern.
            9 | 14 => 0,
            10 | 11 => 5,
            12 => 3,
            13 => 4,
            25 => args.first().map_or(0, |&n| usize::from(n) + 2),
            _ => return Err(DecodeError::Unrecognized),
        };
        Ok(used.min(args.len()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(text: &[u8]) -> Avatar {
        let mut avatar = Avatar {
            terminal: Terminal::new(80).unwrap(),
            attribute: DEFAULT_ATTRIBUTE,
        };
        avatar.feed(text).unwrap();
        avatar
    }

    #[test]
    fn sets_attributes_and_repeats() {
        let avatar = run(b"a\x16\x01\x9eb\x16\x02c\x19d\x03");
        let t = &avatar.terminal;
        assert_eq!(t.get(0, 0).fg, PALETTE[3], "starts cyan");
        assert_eq!((t.get(1, 0).fg, t.get(1, 0).bg), (PALETTE[14], PALETTE[1]));
        assert_eq!(
            t.get(2, 0).bg,
            PALETTE[1],
            "blink keeps the background dark"
        );
        assert_eq!((t.get(3, 0).glyph, t.get(5, 0).glyph, t.x), (b'd', b'd', 6));
    }

    #[test]
    fn moves_the_cursor() {
        let avatar =
            run(b"\x16\x08\x03\x05x\x16\x03\x16\x05\x16\x05y\x16\x08\x00\x00z\r\nw\x16\x06\tv");
        let t = &avatar.terminal;
        assert_eq!(t.get(4, 2).glyph, b'x');
        assert_eq!(t.get(3, 1).glyph, b'y');
        assert_eq!(t.get(0, 0).glyph, b'z', "0 means 1");
        assert_eq!(
            t.get(0, 1).glyph,
            b'w',
            "line feed keeps the column after CR"
        );
        assert_eq!(t.get(8, 1).glyph, b'v');
    }

    #[test]
    fn clears_and_skips_avt0_plus_arguments() {
        let avatar = run(b"ab\x0cc\x16\x0c\x11\x02\x03d\x16\x19\x02xy\x05e");
        let t = &avatar.terminal;
        assert_eq!(t.get(0, 0).glyph, b'c', "^L cleared and homed");
        assert_eq!(t.get(1, 0).glyph, b'd');
        assert_eq!(t.get(2, 0).glyph, b'e');
        let mut avatar = run(b"");
        assert!(avatar.feed(b"\x16\x12").is_err(), "AVT/1 command");
        assert!(decode(b"no codes").is_err());
        assert_eq!(decode(b"\x0chi").unwrap().width(), 640);
    }
}
