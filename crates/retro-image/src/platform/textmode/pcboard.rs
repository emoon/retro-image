//! PCBoard display files (PCB): text with `@X` colour codes.
//!
//! Sources:
//! - libansilove `src/loaders/pcboard.c`
//!   (<https://github.com/ansilove/libansilove>, BSD-2-Clause): `@X` and
//!   two hexadecimal digits (background, then foreground) set the colour,
//!   `@CLS@` clears the screen, a line feed also returns the cursor to
//!   column 0, light grey on black to start, 80 columns.
//! - The SAUCE specification (<https://www.acid.org/info/sauce/sauce.htm>):
//!   DataType 1, FileType 4 is PCBoard, with the width in TInfo1; the EOF
//!   character (1Ah) ends the text.
//! - PCBoard's other `@` codes are macros the BBS replaced with live values
//!   (`@USER@`, `@TIMELEFT@`, seen in `RVCMDPM.PCB` from the dexvert
//!   samples); with no BBS to ask they are shown as written (libansilove
//!   drops the `@`). The attribute's bit 7 blinks, as on a DOS screen.

// The colour codes follow libansilove `src/loaders/pcboard.c`, under this
// licence:
//
// Copyright (c) 2011-2026, Stefan Vogt, Brian Cassidy, and Frederic Cambus
// All rights reserved.
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are met:
//
//   * Redistributions of source code must retain the above copyright
//     notice, this list of conditions and the following disclaimer.
//
//   * Redistributions in binary form must reproduce the above copyright
//     notice, this list of conditions and the following disclaimer in the
//     documentation and/or other materials provided with the distribution.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
// AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
// ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS
// BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
// CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
// SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
// INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
// CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
// ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
// POSSIBILITY OF SUCH DAMAGE.

use super::sauce::{self, DEFAULT_STYLE, Sauce};
use super::screen::{Cell, PALETTE, Terminal};
use crate::{DecodeError, Image};

const PCBOARD_FILE_TYPE: u8 = 4;

/// The attribute of an `@X` code at the start of `text`.
fn color_code(text: &[u8]) -> Option<u8> {
    let [b'@', b'X', high, low, ..] = *text else {
        return None;
    };
    let digit = |d: u8| char::from(d).to_digit(16).map(|v| v as u8);
    Some(digit(high)? << 4 | digit(low)?)
}

/// Decodes text with at least one `@X` code and almost no control
/// characters (at most 1 in 100 besides CR, LF and tab; the samples have
/// none): `.pcb` is also used for circuit board layouts.
pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let (text, sauce) = sauce::split(data);
    if !(0..text.len()).any(|i| color_code(&text[i..]).is_some()) {
        return Err(fail);
    }
    let width = sauce
        .filter(|s| s.is_character(PCBOARD_FILE_TYPE))
        .as_ref()
        .and_then(Sauce::width)
        .unwrap_or(80);
    let mut terminal = Terminal::new(width).ok_or(fail)?;
    let mut attribute = 0x07;
    let (mut drawn, mut controls) = (0usize, 0usize);
    let mut i = 0;
    while let Some(&byte) = text.get(i) {
        i += 1;
        match byte {
            0x1a => break,
            b'\r' => terminal.x = 0,
            b'\n' => {
                terminal.x = 0;
                terminal.y = terminal.y.saturating_add(1);
            }
            b'@' if color_code(&text[i - 1..]).is_some() => {
                attribute = color_code(&text[i - 1..]).unwrap_or(attribute);
                i += 3;
            }
            b'@' if text[i..].starts_with(b"CLS@") => {
                terminal.clear();
                i += 4;
            }
            glyph => {
                drawn += 1;
                controls += usize::from(glyph < 0x20 && glyph != b'\t');
                terminal.put(Cell::from_attribute(glyph, attribute, &PALETTE, false));
            }
        }
    }
    if controls * 100 > drawn {
        return Err(fail);
    }
    terminal.finish(&DEFAULT_STYLE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_colour_codes() {
        assert_eq!(color_code(b"@X1F"), Some(0x1f));
        assert_eq!(color_code(b"@Xe4rest"), Some(0xe4));
        assert_eq!(color_code(b"@XG0"), None);
        assert_eq!(color_code(b"@X1"), None);
    }

    #[test]
    fn colours_text_and_shows_macros() {
        let image = decode(b"@CLS@@X1Fab\r\n@X8Ec@USER@").unwrap();
        assert_eq!((image.width(), image.height()), (640, 32));
        assert_eq!(image.get(0, 0), PALETTE[1], "blue background");
        // Background 8 blinks: dark grey becomes black on a still picture.
        assert_eq!(image.get(0, 16), PALETTE[0]);
        // "@USER@" drawn as text: '@' at column 1.
        let lit = (8..16).any(|x| (16..32).any(|y| image.get(x, y) != PALETTE[0]));
        assert!(lit);
        assert!(decode(b"plain text").is_err(), "no colour code");
        assert!(
            decode(b"@X07\x01\x02\x03binary").is_err(),
            "control characters"
        );
    }
}
