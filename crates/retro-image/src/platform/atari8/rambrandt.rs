//! Rambrandt (RM2, RM4): pictures whose colour registers change on selected
//! scanlines.
//!
//! Sources:
//! - Just Solve "RAMbrandt" (<http://fileformats.archiveteam.org/wiki/RAMbrandt>)
//!   and gury.atari8.info software pages
//!   (<http://gury.atari8.info/software/476.php>,
//!   <http://gury.atari8.info/software/962.php>) for the program and its modes.
//!   The program's documentation disk (an ATR image) was not read.
//! - Everything else was reverse engineered from TITLE.RM2 and ADVANCED.RM4 and
//!   by black-box probing of `recoil2png`: one-byte changes to find which bytes
//!   are read, hand-made tables with one change at a time, and truncation and
//!   padding scans.
//!
//! RM2: exactly 8192 bytes: a Graphics 10 screen of 192 lines x 40 bytes
//! (4 bits per pixel, drawn 4 wide), the 9 registers 704-712 (the GTIA mode 10
//! colours), 119 unread bytes, and the three change tables below.
//!
//! RM4: a Koala file (see `koala.rs`) of a Graphics 15 screen, then the 9
//! registers 464 bytes before the end of the file (the colours in the Koala
//! header are not read) and the three change tables as the last 384 bytes.
//! Appending bytes to the file moves both, so a file with extra data is
//! misread, as it is by `recoil2png`.
//!
//! The change tables are three lists of 128 bytes: line codes, register
//! numbers and colours. Entry p belongs to the line pair 2 (p - 15) and
//! 2 (p - 15) + 1. A register number of 8 or less selects a register
//! (0-8, the numbering of 704-712) and 0x80 means no change. The line code
//! is the line plus 3 (plus 5 from line 97 on, since two scanlines of the
//! 4 KB boundary can't take a change); the entry applies from that line if
//! it is one of the pair, and the register keeps the colour from there on.
//! A line code of 0 or one outside the pair leaves the entry unused.

use super::antic::Bitmap;
use super::koala;
use super::palette::register_rgb;
use super::screen::gtia10_register;
use crate::{DecodeError, Image};
use alloc::vec::Vec;

const LINES: usize = 192;
const TABLE: usize = 128;
const TABLES: usize = 3 * TABLE;

/// The nine colour registers on every line.
struct Registers(Vec<[u8; 9]>);

impl Registers {
    /// The registers `start` with the changes of `tables` (line codes,
    /// register numbers and colours) applied.
    fn new(start: [u8; 9], tables: &[u8]) -> Self {
        let (codes, numbers, colors) = (
            &tables[..TABLE],
            &tables[TABLE..2 * TABLE],
            &tables[2 * TABLE..],
        );
        let mut changes: Vec<Vec<(usize, u8)>> = alloc::vec![Vec::new(); LINES];
        for entry in 15..15 + LINES / 2 {
            let number = usize::from(numbers[entry]);
            let pair = 2 * (entry - 15);
            let line = match codes[entry] {
                0 => continue,
                code @ ..=99 => usize::from(code).checked_sub(3),
                code => usize::from(code).checked_sub(5),
            };
            if number < 9 && line.is_some_and(|line| (pair..pair + 2).contains(&line)) {
                changes[line.unwrap_or(0)].push((number, colors[entry]));
            }
        }
        let mut current = start;
        let lines = changes
            .iter()
            .map(|line| {
                for &(number, color) in line {
                    current[number] = color;
                }
                current
            })
            .collect();
        Self(lines)
    }
}

pub(super) fn decode_rm2(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 8192 {
        return Err(DecodeError::Unrecognized);
    }
    let start = data[7680..7689]
        .try_into()
        .map_err(|_| DecodeError::Unrecognized)?;
    let registers = Registers::new(start, &data[8192 - TABLES..]);
    let bitmap = Bitmap {
        data: &data[..7680],
        bytes_per_line: 40,
        lines: LINES,
        bits: 4,
    };
    Ok(bitmap.render(4, 1, |y, value| {
        register_rgb(registers.0[y][gtia10_register(value)])
    }))
}

pub(super) fn decode_rm4(data: &[u8]) -> Result<Image, DecodeError> {
    let pic = koala::parse(data)?;
    if pic.mode != koala::ANTIC_E {
        return Err(DecodeError::Unrecognized);
    }
    let tail = data
        .len()
        .checked_sub(464)
        .filter(|&tail| tail > 18)
        .ok_or(DecodeError::Unrecognized)?;
    let start = data[tail..tail + 9]
        .try_into()
        .map_err(|_| DecodeError::Unrecognized)?;
    let registers = Registers::new(start, &data[data.len() - TABLES..]);
    let bitmap = Bitmap {
        data: &pic.screen,
        bytes_per_line: 40,
        lines: LINES,
        bits: 2,
    };
    Ok(bitmap.render(2, 1, |y, value| {
        let registers = &registers.0[y];
        let color = [registers[8], registers[4], registers[5], registers[6]];
        register_rgb(color[usize::from(value)])
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn changes_apply_from_their_line() {
        let mut tables = vec![0u8; TABLES];
        tables[TABLE..2 * TABLE].fill(0x80);
        // Entry 28 is the pair of lines 26 and 27: register 3 gets colour 0x44
        // on line 27 (line code 27 + 3).
        tables[28] = 30;
        tables[TABLE + 28] = 3;
        tables[2 * TABLE + 28] = 0x44;
        let registers = Registers::new([0x10; 9], &tables);
        assert_eq!(registers.0[26][3], 0x10);
        assert_eq!(registers.0[27][3], 0x44);
        assert_eq!(registers.0[191][3], 0x44);
        assert_eq!(registers.0[191][2], 0x10);
        // The line code of a line in the other pair does nothing.
        tables[28] = 40;
        assert_eq!(Registers::new([0x10; 9], &tables).0[191][3], 0x10);
    }

    #[test]
    fn rm2_has_a_fixed_size() {
        assert!(decode_rm2(&[0; 8191]).is_err());
        assert_eq!(decode_rm2(&[0; 8192]).unwrap().width(), 320);
    }
}
