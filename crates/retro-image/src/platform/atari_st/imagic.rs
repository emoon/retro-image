//! Imagic pictures (`IC1`-`IC3`).
//!
//! Sources:
//! - Header (`IMDC`, resolution word, palette) and the outline of the
//!   escape-byte compression:
//!   <https://temlib.org/AtariForumWiki/index.php?title=Imagic_Film/Picture_file_format>
//! - Derived from sample files and black-box tests with `recoil2png`, which
//!   differ from that page in places: bytes 64 and 65 are `$C8 $02`, the
//!   escape byte is at 66 and data starts at 67; repeat counts are one
//!   more than stored; the unpacked bytes fill the screen in 160 columns
//!   of 200 bytes, top to bottom, whatever the resolution. See [`unpack`].
//!   Delta pictures copy from a base picture that isn't available, so those
//!   bytes stay zero.

use alloc::vec::Vec;

use super::common::{Resolution, SCREEN_LEN, be16, decode_screen, palette_words};
use crate::{DecodeError, Image};

const DATA: usize = 67;
const COLUMNS: usize = 160;
const ROWS: usize = SCREEN_LEN / COLUMNS;

pub(super) fn decode_ic(data: &[u8]) -> Result<Image, DecodeError> {
    decode(data).ok_or(DecodeError::Unrecognized)
}

fn decode(data: &[u8]) -> Option<Image> {
    if data.get(..4)? != b"IMDC" || data.get(64..66)? != [0xc8, 0x02] {
        return None;
    }
    let resolution = Resolution::from_index(be16(data, 4)?)?;
    let words = palette_words(data, 6, 16)?;
    let columns = unpack(data.get(DATA..)?, data[66])?;
    let mut bitmap = alloc::vec![0; SCREEN_LEN];
    for (i, &byte) in columns.iter().enumerate() {
        bitmap[i % ROWS * COLUMNS + i / ROWS] = byte;
    }
    decode_screen(resolution, &bitmap, &words)
}

/// Unpacks up to a screen of bytes. Bytes other than `escape` are
/// literals; after `escape`, with `v` a value and counts one more than
/// stored:
/// - `escape`: a literal escape byte;
/// - `0, n, v`: `v` repeated `n + 1` times;
/// - `1` repeated `o` times, any byte, `n`, `v`: `v` repeated
///   `256 * o + n + 1` times;
/// - `2, 0`: the end; the rest of the screen stays zero;
/// - `2, 1...` (as above, without `v`), or `2, n` with `n >= 3`: that
///   many bytes from the base picture (zeros here);
/// - `2, 2`: bytes up to and including a zero are skipped;
/// - `n >= 3, v`: `v` repeated `n + 1` times.
///
/// The data may stop without an end marker once the screen is full.
fn unpack(data: &[u8], escape: u8) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(SCREEN_LEN);
    let mut bytes = data.iter().copied();
    let mut next = || bytes.next();
    while out.len() < SCREEN_LEN {
        let x = next()?;
        if x != escape {
            out.push(x);
            continue;
        }
        let (count, value) = match next()? {
            n if n == escape => (1, escape),
            0 => (usize::from(next()?) + 1, next()?),
            1 => (long_count(&mut next)?, next()?),
            2 => match next()? {
                0 => break,
                1 => (long_count(&mut next)?, 0),
                2 => {
                    while next()? != 0 {}
                    continue;
                }
                n => (usize::from(n) + 1, 0),
            },
            n => (usize::from(n) + 1, next()?),
        };
        let count = count.min(SCREEN_LEN - out.len());
        out.resize(out.len() + count, value);
    }
    Some(out)
}

/// After a first `1`: further `1`s, a byte ending them, then the low byte.
fn long_count(next: &mut impl FnMut() -> Option<u8>) -> Option<usize> {
    let mut ones = 1;
    while next()? == 1 {
        ones += 1;
    }
    Some(256 * ones + usize::from(next()?) + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_codes() {
        let esc = 0x13;
        let data = [
            0xaa, esc, esc, // literal escape
            esc, 0, 2, 0xbb, // 3 x bb
            esc, 4, 0xcc, // 5 x cc
            esc, 2, 3, // 4 base bytes
            esc, 2, 2, 9, 9, 0, // skipped
            esc, 1, 7, 0, 0xdd, // 257 x dd
            esc, 2, 0, 0xee, // end
        ];
        let out = unpack(&data, esc).unwrap();
        let mut expected = alloc::vec![0xaa, esc, 0xbb, 0xbb, 0xbb];
        expected.extend([0xcc; 5]);
        expected.extend([0; 4]);
        expected.extend([0xdd; 257]);
        assert_eq!(out, expected);
        assert!(unpack(&[0xaa, esc, 2, 2, 1], esc).is_none());
    }

    #[test]
    fn full_screen_needs_no_end_marker() {
        let data: Vec<u8> = [0x13, 0xff, 0xaa].repeat(126);
        let out = unpack(&data, 0x13).unwrap();
        assert_eq!(out.len(), SCREEN_LEN);
    }
}
