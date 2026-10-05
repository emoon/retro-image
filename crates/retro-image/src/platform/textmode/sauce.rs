//! SAUCE, the 128-byte metadata record appended to text-mode art: data
//! and file type, width in columns, the ANSiFlags (non-blink, letter
//! spacing, aspect ratio) and the font name.
//!
//! Sources:
//! - The SAUCE specification, revision 00.5
//!   (<https://www.acid.org/info/sauce/sauce.htm>): record layout, the
//!   optional "COMNT" block before it, the EOF character (1Ah) before both,
//!   FileSize, the per-type TInfo meaning, ANSiFlags and the TInfoS font
//!   names ("IBM VGA50" and "IBM EGA43" are 8x8 fonts).
//! - Deark `modules/sauce.c` (<https://github.com/jsummers/deark>, MIT
//!   license): a FileSize of 0 or past the record is wrong in some files,
//!   and the data then ends where the comments or the record begin.

// The FileSize fallback follows Deark `modules/sauce.c`, under this license:
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

use super::font::{VGA_8X8, VGA_8X16};
use super::screen::{MAX_COLUMNS, Style};
use crate::bytes::{le16, le32};

const RECORD_LEN: usize = 128;
const COMMENT_LEN: usize = 64;

/// SAUCE DataType values.
pub(super) const CHARACTER: u8 = 1;
pub(super) const BINARY_TEXT: u8 = 5;

/// The fields of a SAUCE record that rendering needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Sauce<'a> {
    pub(super) data_type: u8,
    pub(super) file_type: u8,
    pub(super) tinfo1: u16,
    flags: u8,
    font_name: &'a [u8],
}

/// Splits `data` into the picture data and its SAUCE record, if any. The
/// data excludes the EOF character, comment block and record.
pub(super) fn split(data: &[u8]) -> (&[u8], Option<Sauce<'_>>) {
    let Some(record_at) = data.len().checked_sub(RECORD_LEN) else {
        return (data, None);
    };
    let record = &data[record_at..];
    if &record[..7] != b"SAUCE00" {
        return (data, None);
    }
    let comments = usize::from(record[104]) * COMMENT_LEN + 5;
    let end = match record_at.checked_sub(comments) {
        Some(at) if record[104] > 0 && data[at..].starts_with(b"COMNT") => at,
        _ => record_at,
    };
    let size = le32(record, 90).map_or(0, |s| s as usize);
    let content = match size {
        1.. if size <= end => &data[..size],
        _ => data[..end].strip_suffix(&[0x1a]).unwrap_or(&data[..end]),
    };
    let font_name = &record[106..];
    let font_name = match font_name.iter().rposition(|&b| b != 0 && b != b' ') {
        Some(last) => &font_name[..=last],
        None => &[],
    };
    let sauce = Sauce {
        data_type: record[94],
        file_type: record[95],
        tinfo1: le16(record, 96).unwrap_or(0),
        flags: record[105],
        font_name,
    };
    (content, Some(sauce))
}

impl Sauce<'_> {
    /// Whether the record describes a character stream of `file_type`
    /// (1 ANSI, 4 PCBoard, 5 Avatar, 8 TundraDraw, ...).
    pub(super) fn is_character(&self, file_type: u8) -> bool {
        self.data_type == CHARACTER && self.file_type == file_type
    }

    /// The width in columns (TInfo1) if plausible: 40 to [`MAX_COLUMNS`].
    pub(super) fn width(&self) -> Option<usize> {
        let width = usize::from(self.tinfo1);
        (40..=MAX_COLUMNS).contains(&width).then_some(width)
    }

    /// Whether the ANSiFlags and font name apply: plain text, ANSI and
    /// ANSiMation, and BinaryText.
    fn has_ansi_flags(&self) -> bool {
        matches!(
            (self.data_type, self.file_type),
            (CHARACTER, 0..=2) | (BINARY_TEXT, _)
        )
    }

    /// Non-blink mode (iCE color): the attribute's bit 7 selects a bright
    /// background instead of blinking.
    pub(super) fn ice(&self) -> bool {
        self.has_ansi_flags() && self.flags & 1 != 0
    }

    /// How the ANSiFlags and font name say to draw the picture. Fonts other
    /// than the VGA/EGA 8x8 ones are drawn with the VGA 8x16 font, the only
    /// other one built in. The aspect-ratio bits are not honoured: pictures
    /// keep square pixels.
    pub(super) fn style(&self) -> Style<'static> {
        if !self.has_ansi_flags() {
            return DEFAULT_STYLE;
        }
        let eight_high = [&b"IBM VGA50"[..], b"IBM EGA43"]
            .iter()
            .any(|name| self.font_name.starts_with(name));
        Style {
            font: if eight_high { VGA_8X8 } else { VGA_8X16 },
            nine_pixels: self.flags >> 1 & 3 == 2,
        }
    }
}

/// The VGA 8x16 font with 8-pixel letter spacing.
pub(super) const DEFAULT_STYLE: Style<'static> = Style {
    font: VGA_8X16,
    nine_pixels: false,
};

/// The style of `sauce`, or the default without a record.
pub(super) fn style_of(sauce: Option<&Sauce>) -> Style<'static> {
    sauce.map_or(DEFAULT_STYLE, Sauce::style)
}

/// `body`, EOF, optional comment lines, then a record (tests only).
#[cfg(test)]
pub(super) fn with_sauce(
    body: &[u8],
    data_type: u8,
    file_type: u8,
    tinfo1: u16,
    flags: u8,
    font: &[u8],
    comments: &[&[u8]],
) -> alloc::vec::Vec<u8> {
    let mut data = body.to_vec();
    data.push(0x1a);
    if !comments.is_empty() {
        data.extend_from_slice(b"COMNT");
        for line in comments {
            let mut padded = [b' '; 64];
            padded[..line.len()].copy_from_slice(line);
            data.extend_from_slice(&padded);
        }
    }
    let mut record = [0u8; 128];
    record[..7].copy_from_slice(b"SAUCE00");
    record[90..94].copy_from_slice(&(body.len() as u32).to_le_bytes());
    record[94] = data_type;
    record[95] = file_type;
    record[96..98].copy_from_slice(&tinfo1.to_le_bytes());
    record[104] = comments.len() as u8;
    record[105] = flags;
    record[106..106 + font.len()].copy_from_slice(font);
    data.extend_from_slice(&record);
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_data_from_eof_comments_and_record() {
        let data = with_sauce(
            b"hello",
            1,
            1,
            132,
            0x15,
            b"IBM VGA",
            &[b"first", b"second"],
        );
        let (content, sauce) = split(&data);
        let sauce = sauce.unwrap();
        assert_eq!(content, b"hello");
        assert!(sauce.is_character(1));
        assert_eq!(sauce.tinfo1, 132);
        assert_eq!(sauce.font_name, b"IBM VGA");
        assert!(sauce.ice());
        let style = sauce.style();
        assert!(style.nine_pixels);
        assert_eq!(style.font.height(), 16);
    }

    #[test]
    fn bad_file_size_falls_back_to_the_record_start() {
        let mut data = with_sauce(b"abc", 1, 1, 80, 0, b"", &[b"note"]);
        let size_at = data.len() - 128 + 90;
        data[size_at..size_at + 4].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(split(&data).0, b"abc", "EOF and comments are dropped");
        data[size_at..size_at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(split(&data).0, b"abc");
        // A comment count with no "COMNT" block: the data runs to the record.
        let mut data = with_sauce(b"abc", 1, 1, 80, 0, b"", &[]);
        let record = data.len() - 128;
        data[record + 104] = 3;
        data[record + 90..record + 94].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(split(&data).0, b"abc");
    }

    #[test]
    fn no_record_keeps_everything() {
        assert_eq!(split(b"short"), (&b"short"[..], None));
        let long = [b'x'; 300];
        assert_eq!(split(&long), (&long[..], None));
    }

    #[test]
    fn flags_apply_to_ansi_and_binary_text_only() {
        let font = b"IBM VGA50 437";
        let ansi = with_sauce(b"", 1, 1, 80, 0x03, font, &[]);
        let style = split(&ansi).1.unwrap().style();
        assert_eq!(style.font.height(), 8);
        assert!(!style.nine_pixels, "LS bits 10 are 9 pixels, 01 are 8");
        let bin = with_sauce(b"", 5, 80, 0, 0x01, b"", &[]);
        assert!(split(&bin).1.unwrap().ice());
        let tundra = with_sauce(b"", 1, 8, 80, 0x15, font, &[]);
        let tundra = split(&tundra).1.unwrap();
        assert!(!tundra.ice());
        assert_eq!(tundra.style().font.height(), 16);
        assert!(!tundra.style().nine_pixels);
    }
}
