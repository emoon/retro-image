//! Anime 4ever slideshow pictures (A4R).
//!
//! Sources:
//! - Just Solve, "Anime 4ever slideshow"
//!   (<http://fileformats.archiveteam.org/wiki/Anime_4ever_slideshow>) and
//!   pouet.net prod 71261 (<https://www.pouet.net/prod.php?which=71261>)
//!   name the format; neither publishes a layout.
//! - The layout is reverse engineered from the corpus samples (CATTY,
//!   DEUNAN, IRIA, MIYU, PLASTIC, PUMA, SHIZUKU) and checked against
//!   `recoil2png` output (black box); notes in
//!   `docs/research/gaps-corpus-atari8.md` section 5.
//!
//! ```text
//! 0  b0, b1   bit 7 of both set; b0 & 0x7f is the first marker byte,
//!             b1 & 0x7f the first group's flag byte
//! 2  00 90 4F constant
//! 5  LZSS stream
//! ```
//!
//! The stream is read in groups of eight items. A marker byte covers eight
//! groups, most significant bit first: a set bit means the group starts
//! with a flag byte, a clear bit that its eight items are all literals (the
//! first group always has its flag byte, from the header). After the eighth
//! group the next stream byte is a new marker. In a flag byte, most
//! significant bit first, 0 is a literal byte and 1 a match token `v`:
//!
//! - `v & 0xfe == 0`: the next byte `n` says to repeat the last output byte
//!   `n + 2` times; but `00 90 PP` followed by one byte `VV` is a segment
//!   start, see below;
//! - otherwise copy `2 + (v & 1)` bytes from `(256 - (v & 0xfe)) / 2` bytes
//!   back, byte by byte.
//!
//! Anything before the start of the output reads as 0.
//!
//! The header's `00 90 PP` is the same segment start as in the stream: the
//! segment is the memory image at page `PP`, and its first byte `VV` takes
//! the first item's slot (the flag byte's bit 7 is that item, set in every
//! file, so the stream's first literal is `VV`). The picture is 10240 bytes at
//! page `4F`, 80x256 Graphics 9 pixels (two per byte, high nibble left). With
//! `PP` = `4F` it is the output's start and the slideshow's credits text
//! follows it in the same stream, which has to decode to the end of the
//! file. With `PP` = `4D` or `4E` (MOTOKO has `4D`) the stream starts with
//! the text segment, and a `00 90 4F VV` in the stream puts the output
//! pointer, zero-filled, at the picture page. Only these three are known.

use super::screen;
use crate::{DecodeError, Image};
use alloc::vec::Vec;

const PICTURE: usize = 40 * 256;
/// Page of the picture; the segment pages seen are `4D`..=`4F`.
const PICTURE_PAGE: u8 = 0x4f;
const FIRST_PAGE: u8 = 0x4d;

/// A4R: 80x256 grays.
pub(super) fn decode_a4r(data: &[u8]) -> Result<Image, DecodeError> {
    let Some((&[b0, b1, 0x00, 0x90, page], stream)) = data.split_first_chunk() else {
        return Err(DecodeError::Invalid);
    };
    if b0 & b1 & 0x80 == 0 || !(FIRST_PAGE..=PICTURE_PAGE).contains(&page) {
        return Err(DecodeError::Invalid);
    }
    let start = usize::from(PICTURE_PAGE - page) * 256;
    let image =
        unpack(stream, b0 & 0x7f, b1 & 0x7f, start + PICTURE, page).ok_or(DecodeError::Invalid)?;
    screen::gtia9(screen::bitmap(&image[start..], 40, 4), 0x00)
}

/// Unpacks the stream to its end and returns the first `size` output bytes;
/// `None` if a token is cut short or the output is short. Output past `size`
/// is parsed but not kept. `page` is the page of the output's first byte.
fn unpack(
    stream: &[u8],
    first_marker: u8,
    first_flags: u8,
    size: usize,
    page: u8,
) -> Option<Vec<u8>> {
    let mut input = stream.iter().copied().peekable();
    let mut out = Vec::with_capacity(size);
    let mut marker = first_marker;
    'groups: for group in 0usize.. {
        if group % 8 == 0 && group > 0 {
            let Some(byte) = input.next() else { break };
            marker = byte;
        }
        let flags = if group == 0 {
            first_flags
        } else if marker >> (7 - group % 8) & 1 == 0 {
            0
        } else {
            let Some(flags) = input.next() else { break };
            flags
        };
        for item in 0..8 {
            let Some(byte) = input.next() else {
                break 'groups;
            };
            if flags >> (7 - item) & 1 == 0 {
                keep(&mut out, size, byte);
            } else if byte & 0xfe == 0 {
                let count = input.next()?;
                let target = segment_start(byte, count, input.peek().copied(), page, out.len());
                if let Some(target) = target {
                    input.next()?;
                    let value = input.next()?;
                    out.resize(target, 0);
                    keep(&mut out, size, value);
                } else {
                    let last = out.last().copied().unwrap_or(0);
                    (0..usize::from(count) + 2).for_each(|_| keep(&mut out, size, last));
                }
            } else {
                let distance = (256 - usize::from(byte & 0xfe)) / 2;
                for _ in 0..2 + usize::from(byte & 1) {
                    let value = out.len().checked_sub(distance).map_or(0, |i| out[i]);
                    keep(&mut out, size, value);
                }
            }
        }
    }
    (out.len() == size).then_some(out)
}

/// The output position a `00 90 PP VV` segment start moves to, if the token
/// `token`, `count` and the byte after them `page_byte` make one: a page
/// ahead of the output up to the picture's.
fn segment_start(
    token: u8,
    count: u8,
    page_byte: Option<u8>,
    first_page: u8,
    len: usize,
) -> Option<usize> {
    let page = page_byte?;
    let target = usize::from(page.checked_sub(first_page)?) * 256;
    (token == 0 && count == 0x90 && page <= PICTURE_PAGE && target >= len).then_some(target)
}

/// Appends to the picture while it has room.
fn keep(out: &mut Vec<u8>, size: usize, byte: u8) {
    if out.len() < size {
        out.push(byte);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literals_runs_and_copies() {
        // Flags 0b0001_1000: three literals, a copy, a run, three literals.
        let stream = [0xff, 0xff, 0xef, 0xfa, 0x01, 0x03, 0xfe, 0xee, 0xfe];
        let expected = [
            0xff, 0xff, 0xef, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xfe, 0xee, 0xfe,
        ];
        assert_eq!(
            unpack(&stream, 0x7f, 0x18, 13, 0x4f).as_deref(),
            Some(&expected[..])
        );
        // A token cut short is an error.
        assert_eq!(unpack(&stream[..6], 0x7f, 0x18, 13, 0x4f), None);
    }

    #[test]
    fn segment_start_moves_the_output() {
        // A literal, then `00 90 4F FF`: output continues at page 4F.
        let stream = [
            0x11, 0x00, 0x90, 0x4f, 0xff, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
        ];
        let out = unpack(&stream, 0x7f, 0x40, 516, 0x4d).unwrap();
        assert_eq!(out[0], 0x11);
        assert!(out[1..512].iter().all(|&b| b == 0));
        assert_eq!(out[512..], [0xff, 0x22, 0x33, 0x44]);
        // A page behind the output is a run of 146 instead.
        let out = unpack(&stream, 0x7f, 0x40, 150, 0x4f).unwrap();
        assert_eq!(out[1..147], [0x11; 146]);
    }

    #[test]
    fn rejects_bad_header() {
        assert!(decode_a4r(&[0x7f, 0x98, 0x00, 0x90, 0x4f, 0xff]).is_err());
        assert!(decode_a4r(&[0xff, 0x98, 0x00, 0x90, 0x4d, 0xff]).is_err());
        assert!(decode_a4r(&[]).is_err());
    }
}
