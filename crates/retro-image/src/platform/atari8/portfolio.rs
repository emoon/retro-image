//! Atari Portfolio: PGF and PGC (240x64 monochrome LCD).
//!
//! Sources:
//! - PGC spec by Don Messerli (1991), <http://www.textfiles.com/programming/FORMATS/pgcspec.txt>:
//!   magic `PG\x01`, then runs: index byte with bit 7 set repeats the next
//!   byte (low 7 bits) times, otherwise copies (low 7 bits) literal bytes.
//! - PGF: Just Solve "PGF (Portfolio Graphics)"
//!   (<http://fileformats.archiveteam.org/wiki/PGF_(Portfolio_Graphics)>):
//!   1920 bytes, 30 bytes per row.
//! - Colors (set bit = black on white): observed from `recoil2png` output.

use super::antic::Bitmap;
use crate::{DecodeError, Image};

const SCREEN_LEN: usize = 1920;

/// Raw 1920-byte screen.
pub(super) fn decode_pgf(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != SCREEN_LEN {
        return Err(DecodeError::Unrecognized);
    }
    render(data)
}

/// Run-length compressed screen.
pub(super) fn decode_pgc(data: &[u8]) -> Result<Image, DecodeError> {
    let packed = data
        .strip_prefix(b"PG\x01")
        .ok_or(DecodeError::Unrecognized)?;
    let screen = unpack(packed).ok_or(DecodeError::Unrecognized)?;
    render(&screen)
}

fn unpack(mut packed: &[u8]) -> Option<[u8; SCREEN_LEN]> {
    let mut screen = [0; SCREEN_LEN];
    let mut len = 0;
    while len < SCREEN_LEN {
        let (&index, rest) = packed.split_first()?;
        let count = usize::from(index & 0x7f);
        let end = len.checked_add(count).filter(|&end| end <= SCREEN_LEN)?;
        if index & 0x80 != 0 {
            let (&value, rest) = rest.split_first()?;
            screen[len..end].fill(value);
            packed = rest;
        } else {
            let literal = rest.get(..count)?;
            screen[len..end].copy_from_slice(literal);
            packed = &rest[count..];
        }
        len = end;
    }
    Some(screen)
}

fn render(screen: &[u8]) -> Result<Image, DecodeError> {
    let bitmap = Bitmap {
        data: screen,
        bytes_per_line: 30,
        lines: 64,
        bits: 1,
    };
    bitmap.render(1, 1, |_, value| if value == 0 { 0xffffff } else { 0 })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unpacks_runs_and_literals() {
        let mut data = alloc::vec![b'P', b'G', 1, 0x82, 0xaa, 0x01, 0x80];
        for _ in 0..(SCREEN_LEN - 3) / 127 {
            data.extend_from_slice(&[0xff, 0]);
        }
        data.extend_from_slice(&[0x80 | ((SCREEN_LEN - 3) % 127) as u8, 0]);
        let image = decode_pgc(&data).unwrap();
        assert_eq!(&image.rgb()[..3], &[0, 0, 0]);
        assert_eq!(&image.rgb()[3..6], &[0xff, 0xff, 0xff]);
    }

    #[test]
    fn rejects_truncated_and_overlong_runs() {
        assert!(decode_pgc(b"PG\x01\x85\x00").is_err());
        assert!(decode_pgc(b"PG\x01\x05\x00").is_err());
        assert!(decode_pgf(&[0; 1919]).is_err());
    }
}
