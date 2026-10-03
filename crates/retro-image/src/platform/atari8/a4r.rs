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
//!   `n + 2` times;
//! - otherwise copy `2 + (v & 1)` bytes from `(256 - (v & 0xfe)) / 2` bytes
//!   back, byte by byte.
//!
//! Anything before the start of the output reads as 0. The first 10240
//! output bytes are the picture, 80x256 Graphics 9 pixels (two per byte,
//! high nibble left). The slideshow's credits text follows in the same
//! stream, which has to decode to the end of the file. Files whose fifth
//! byte is `4D`..`50` also exist; only `4F` (the picture at the start) is
//! understood.

use super::screen;
use crate::{DecodeError, Image};
use alloc::vec::Vec;

const PICTURE: usize = 40 * 256;

/// A4R: 80x256 greys.
pub(super) fn decode_a4r(data: &[u8]) -> Result<Image, DecodeError> {
    let Some((&[b0, b1, 0x00, 0x90, 0x4f], stream)) = data.split_first_chunk() else {
        return Err(DecodeError::Unrecognized);
    };
    if b0 & b1 & 0x80 == 0 {
        return Err(DecodeError::Unrecognized);
    }
    let picture = unpack(stream, b0 & 0x7f, b1 & 0x7f, PICTURE).ok_or(DecodeError::Unrecognized)?;
    Ok(screen::gtia9(screen::bitmap(&picture, 40, 4), 0x00))
}

/// Unpacks the stream to its end and returns the picture bytes; `None` if a
/// token is cut short or the picture is incomplete. Output past the picture
/// is parsed but not kept.
fn unpack(stream: &[u8], first_marker: u8, first_flags: u8, size: usize) -> Option<Vec<u8>> {
    let mut input = stream.iter().copied();
    let mut out = Vec::with_capacity(size);
    let mut marker = first_marker;
    'groups: for group in 0usize.. {
        if group % 8 == 0 && group > 0 {
            let Some(next) = input.next() else { break };
            marker = next;
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
                let count = usize::from(input.next()?) + 2;
                let last = out.last().copied().unwrap_or(0);
                (0..count).for_each(|_| keep(&mut out, size, last));
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
            unpack(&stream, 0x7f, 0x18, 13).as_deref(),
            Some(&expected[..])
        );
        // A token cut short is an error.
        assert_eq!(unpack(&stream[..6], 0x7f, 0x18, 13), None);
    }

    #[test]
    fn rejects_bad_header() {
        assert!(decode_a4r(&[0x7f, 0x98, 0x00, 0x90, 0x4f, 0xff]).is_err());
        assert!(decode_a4r(&[0xff, 0x98, 0x00, 0x90, 0x4d, 0xff]).is_err());
        assert!(decode_a4r(&[]).is_err());
    }
}
