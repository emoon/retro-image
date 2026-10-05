//! TundraDraw (TND): a character stream with 24-bit colors.
//!
//! Sources:
//! - libansilove `src/loaders/tundra.c`
//!   (<https://github.com/ansilove/libansilove>, BSD-2-Clause): a 9-byte
//!   header (byte 24, "TUNDRA24"), then characters and four commands:
//!   1 moves the cursor (row, then column, as big-endian 32-bit values);
//!   2, 4 and 6 are followed by a character drawn with a new foreground,
//!   background or both, each color a big-endian 32-bit 0x00RRGGBB.
//!   80 columns, wrapping at the right edge.
//! - The SAUCE specification (<https://www.acid.org/info/sauce/sauce.htm>):
//!   DataType 1, FileType 8 is TundraDraw, with the width in TInfo1 and no
//!   flags. The magic already identifies the format, so the width is taken
//!   from any Character record: `avg-thebatman.tnd` (dexvert samples) is
//!   labelled FileType 7 (source code) but only draws right 160 columns
//!   wide, as its TInfo1 says.
//! - Reverse engineered from samples: files may carry SAUCE, which
//!   libansilove draws as characters (`tcf-shocktronics.tnd`, 200 columns,
//!   from the dexvert samples). Text before the first color command is
//!   drawn like DOS text, light gray on black (libansilove starts with
//!   black on black).

// The command layout follows libansilove `src/loaders/tundra.c`, under this
// license:
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
use super::screen::{Cell, Terminal};
use crate::bytes::be32;
use crate::{DecodeError, Image};

const MAGIC: &[u8] = b"\x18TUNDRA24";

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let (content, sauce) = sauce::split(data);
    let stream = content.strip_prefix(MAGIC).ok_or(fail)?;
    let width = sauce
        .filter(|s| s.data_type == sauce::CHARACTER)
        .as_ref()
        .and_then(Sauce::width)
        .unwrap_or(80);
    let mut terminal = Terminal::new(width).ok_or(fail)?;
    let mut cell = Cell::BLANK;
    let color = |at: usize| be32(stream, at).map(|c| c & 0xff_ffff);
    let mut i = 0;
    while let Some(&byte) = stream.get(i) {
        let (fg, bg, len) = match byte {
            1 => {
                let (Some(row), Some(column)) = (be32(stream, i + 1), be32(stream, i + 5)) else {
                    break;
                };
                terminal.move_to(column as usize, row as usize);
                i += 9;
                continue;
            }
            2 => (color(i + 2), Some(cell.bg), 6),
            4 => (Some(cell.fg), color(i + 2), 6),
            6 => (color(i + 2), color(i + 6), 10),
            _ => {
                terminal.put(Cell {
                    glyph: byte,
                    ..cell
                });
                i += 1;
                continue;
            }
        };
        let (Some(fg), Some(bg), Some(&glyph)) = (fg, bg, stream.get(i + 1)) else {
            break;
        };
        cell = Cell { glyph, fg, bg };
        terminal.put(cell);
        i += len;
    }
    terminal.finish(&DEFAULT_STYLE)
}

#[cfg(test)]
mod tests {
    use super::super::sauce::with_sauce;
    use super::*;
    use alloc::vec::Vec;

    fn tundra(stream: &[u8]) -> Vec<u8> {
        [MAGIC, stream].concat()
    }

    #[test]
    fn draws_characters_with_24_bit_colours() {
        let data = tundra(&[
            b'a', // default colors
            2, b'b', 0, 0x12, 0x34, 0x56, // new foreground
            4, b'c', 0xff, 0, 0, 0x80, // new background, top byte ignored
            6, b'd', 0, 1, 2, 3, 0, 4, 5, 6,    // both
            b'e', // keeps the last colors
            1, 0, 0, 0, 2, 0, 0, 0, 79, b'f', // row 2, last column
        ]);
        let image = decode(&data).unwrap();
        assert_eq!((image.width(), image.height()), (640, 48));
        assert_eq!(image.get(0, 0), 0x000000);
        assert_eq!(image.get(8 * 2, 0), 0x000080, "c's background");
        assert_eq!(image.get(8 * 4, 0), 0x040506, "e's background");
        assert_eq!(image.get(8 * 79, 32), 0x040506);
    }

    #[test]
    fn truncated_commands_end_the_stream() {
        let image = decode(&tundra(&[b'a', 2, b'b', 0, 0])).unwrap();
        assert_eq!(image.width(), 640);
        assert!(decode(&tundra(&[1, 0, 0])).is_err(), "nothing drawn");
        assert!(decode(b"\x18TUNDRA23a").is_err());
    }

    #[test]
    fn width_comes_from_a_tundra_sauce_record() {
        let data = with_sauce(&tundra(b"x"), 1, 8, 160, 0, b"", &[]);
        assert_eq!(decode(&data).unwrap().width(), 160 * 8);
        let data = with_sauce(&tundra(b"x"), 5, 80, 160, 0, b"", &[]);
        assert_eq!(
            decode(&data).unwrap().width(),
            80 * 8,
            "not a Character record"
        );
    }
}
