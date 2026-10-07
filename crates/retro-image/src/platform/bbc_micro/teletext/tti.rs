//! MRG TTI teletext page files (VBIT, vbit2, wxTED, QTeletextMaker, edit.tf).
//!
//! Sources:
//! - <https://teletext.wiki.zxnet.co.uk/wiki/MRG_TTI_format> (CC BY-SA 4.0;
//!   only the facts are used): CR/LF-separated text lines, each a two-letter
//!   tag and a comma (`DE`, `DS`, `SP`, `PN`, `SC`, `PS`, `CT`, `FL`, `OL`,
//!   ...); `OL,row,text` holds one packet, with control codes written as ESC
//!   followed by the code plus 0x40.
//! - Observed in the 48 files of `corpus/extra/teletext`: rows 0-24 hold the
//!   page, rows 25-28 hold enhancement packets (not drawn), and a file may
//!   hold several sub-pages, each starting at a `PN,` or `SC,` line.
//!
//! Only the first sub-page is drawn, as level 1 teletext with the English
//! character set. A missing row 0 stays blank.

use super::{COLUMNS, Dialect, Page, ROWS};
use crate::{DecodeError, Image};

const TAGS: [&[u8; 2]; 11] = [
    b"DE", b"DS", b"SP", b"PN", b"SC", b"PS", b"CT", b"FL", b"OL", b"RE", b"PF",
];
const ESCAPE: u8 = 0x1b;

/// The tag of a line, if it starts with a known one and a comma.
fn tag(line: &[u8]) -> Option<&'static [u8; 2]> {
    let name: &[u8; 2] = line.get(..2)?.try_into().ok()?;
    let known = TAGS.iter().find(|&&tag| tag == name)?;
    (line.get(2) == Some(&b',')).then_some(*known)
}

/// The row number and text of an `OL,row,text` line.
fn packet(line: &[u8]) -> Option<(usize, &[u8])> {
    let rest = line.get(3..)?;
    let digits = rest.iter().take_while(|byte| byte.is_ascii_digit()).count();
    let row = core::str::from_utf8(&rest[..digits]).ok()?.parse().ok()?;
    Some((
        row,
        rest.get(digits + 1..)
            .filter(|_| rest.get(digits) == Some(&b','))?,
    ))
}

/// Character codes of a packet's text, resolving ESC pairs.
fn codes(text: &[u8]) -> impl Iterator<Item = u8> + '_ {
    let mut bytes = text.iter().copied();
    core::iter::from_fn(move || {
        let byte = bytes.next()?;
        if byte != ESCAPE {
            return Some(byte);
        }
        let next = bytes.next()?;
        Some(if next >= 0x40 {
            next - 0x40
        } else {
            next & 0x1f
        })
    })
}

pub(in crate::platform) fn decode_tti(data: &[u8]) -> Result<Image, DecodeError> {
    let lines = data
        .split(|&byte| byte == b'\n')
        .map(|line| line.strip_suffix(b"\r").unwrap_or(line));
    let mut page = Page::blank();
    let mut drawn = false;
    if lines.clone().next().and_then(tag).is_none() {
        return Err(DecodeError::Invalid);
    }
    for line in lines {
        match tag(line) {
            Some(b"OL") => {
                let (row, text) = packet(line).ok_or(DecodeError::Invalid)?;
                if row < ROWS {
                    page.set_row(row, codes(text).take(COLUMNS));
                    drawn = true;
                }
            }
            Some(b"PN" | b"SC") if drawn => break,
            _ => {}
        }
    }
    if !drawn {
        return Err(DecodeError::Invalid);
    }
    page.render(Dialect::Broadcast)
}
