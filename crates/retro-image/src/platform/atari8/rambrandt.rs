//! Rambrandt (RM0 to RM4): pictures whose color registers change on selected
//! scanlines. The extension digit is the Rambrandt graphics mode.
//!
//! Sources:
//! - Just Solve "RAMbrandt" (<http://fileformats.archiveteam.org/wiki/RAMbrandt>)
//!   and gury.atari8.info software pages
//!   (<http://gury.atari8.info/software/476.php>,
//!   <http://gury.atari8.info/software/962.php>) for the program and its modes.
//! - The program's documentation disk (`Rambrandt_docs.ATR`, files DOC.003 and
//!   DOC.004, copyright 1985 Antic Publishing and Bard Ermentrout), read as
//!   prose for the modes (0: 160x96, 1: GTIA 9, 2: GTIA 10, 3: GTIA 11, 4:
//!   160x192 four colors) and the picture layout (screen, registers 704-712
//!   and three 128-byte tables); see `docs/research/next-blocked.md`.
//! - Everything else was reverse engineered from TITLE.RM2 and ADVANCED.RM4 and
//!   by black-box probing of `recoil2png`: one-byte changes to find which bytes
//!   are read, hand-made tables with one change at a time, random tables,
//!   and truncation and padding scans. `recoil2png` takes the pixel mode from
//!   the extension and the container from the content, so TITLE.RM2 and
//!   ADVANCED.RM4 renamed to .rm1 and .rm3 are the RM1 and RM3 oracle samples;
//!   RM0 was checked with synthetic files. No genuine RM0, RM1 or RM3 file was
//!   found.
//!
//! RM2: exactly 8192 bytes: a Graphics 10 screen of 192 lines x 40 bytes
//! (4 bits per pixel, drawn 4 wide), the 9 registers 704-712 (the GTIA mode 10
//! colors), 119 unread bytes, and the three change tables below.
//!
//! RM4: a Koala file (see `koala.rs`) of a Graphics 15 screen, then the 9
//! registers 464 bytes before the end of the file (the colors in the Koala
//! header are not read) and the three change tables as the last 384 bytes.
//! Appending bytes to the file moves both, so a file with extra data is
//! misread, as it is by `recoil2png`.
//!
//! RM1 and RM3 take either container and read the same 40-byte lines as
//! 4-bit pixels, drawn 4 wide, with only the background register (712):
//! RM1 (GTIA 9) shows the pixel value as luminance on the register's hue, RM3
//! (GTIA 11) the value as hue on its luminance. In RM1 the luminance of the
//! register's starting value is not used, only that of a change-table entry.
//!
//! RM0 is the RM2 container with a 160x96 screen of 2-bit pixels in the first
//! 3840 bytes, drawn 2x2 and colored like RM4 (value 0 is register 8, 1 to 3
//! are registers 4 to 6).
//!
//! In every mode a line code in the first table must be 0, 3, 6 to 99 or 102
//! to 197 (RM0: 0, 3 or 6 to 100), or the file is refused.
//!
//! The change tables are three lists of 128 bytes: line codes, register
//! numbers and colors. Entry p belongs to the line pair 2 (p - 15) and
//! 2 (p - 15) + 1. A register number of 8 or less selects a register
//! (0-8, the numbering of 704-712) and 0x80 means no change. The line code
//! is the line plus 3 (plus 5 from line 97 on, since two scanlines of the
//! 4 KB boundary can't take a change); the entry applies from that line if
//! it is one of the pair, and the register keeps the color from there on.
//! A line code of 0 or one outside the pair leaves the entry unused.
//!
//! RM0 has one entry per line of the 96: entry n + 15 belongs to line n (line 0
//! has none) and its register change applies when the table lists, in any
//! entry, the line's code: 3 for line 1 and n + 4 for the others.

use super::antic::Bitmap;
use super::koala;
use super::palette::{register_rgb, rgb};
use super::screen::gtia10_register;
use crate::{DecodeError, Image};
use alloc::vec::Vec;

const LINES: usize = 192;
const TABLE: usize = 128;
const TABLES: usize = 3 * TABLE;

/// How the change-table entries map to scanlines.
#[derive(Clone, Copy)]
enum Layout {
    /// 192 lines (RM1-RM4): an entry covers a pair of lines.
    Pairs,
    /// 96 lines (RM0): an entry covers one line.
    Single,
}

impl Layout {
    fn lines(self) -> usize {
        match self {
            Self::Pairs => 192,
            Self::Single => 96,
        }
    }

    /// Whether `code` can appear in a line-code table: 0 (unused), 3, and
    /// the codes of lines in the mode.
    fn accepts(self, code: u8) -> bool {
        match self {
            Self::Pairs => matches!(code, 0 | 3 | 6..=99 | 102..=197),
            Self::Single => matches!(code, 0 | 3 | 6..=100),
        }
    }

    /// The line the register change in `entry` starts on, given the line
    /// codes of the whole table.
    fn line(self, entry: usize, codes: &[u8]) -> Option<usize> {
        let first = entry.checked_sub(15)?;
        match self {
            Self::Pairs => {
                let line = match codes[entry] {
                    0 => return None,
                    code @ ..=99 => usize::from(code).checked_sub(3)?,
                    code => usize::from(code).checked_sub(5)?,
                };
                (2 * first..2 * first + 2).contains(&line).then_some(line)
            }
            // The entry's registers are used when the code of its line
            // (line 1: 3, line n: n + 4) is listed anywhere in the table.
            // Line 0 has no code.
            Self::Single => {
                let code = if first == 1 { 3 } else { first + 4 };
                (1..96)
                    .contains(&first)
                    .then_some(first)
                    .filter(|_| codes.iter().any(|&c| usize::from(c) == code))
            }
        }
    }
}

/// The nine color registers on every line.
struct Registers(Vec<[u8; 9]>);

impl Registers {
    /// The registers `start` with the changes of `tables` (line codes,
    /// register numbers and colors) applied.
    fn new(start: [u8; 9], tables: &[u8], layout: Layout) -> Self {
        let (codes, numbers, colors) = (
            &tables[..TABLE],
            &tables[TABLE..2 * TABLE],
            &tables[2 * TABLE..],
        );
        let mut changes: Vec<Vec<(usize, u8)>> = alloc::vec![Vec::new(); layout.lines()];
        for entry in 0..TABLE {
            let number = usize::from(numbers[entry]);
            if let Some(line) = layout
                .line(entry, codes)
                .filter(|&line| number < 9 && line < layout.lines())
            {
                changes[line].push((number, colors[entry]));
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

/// The screen and per-line registers of either container.
struct Picture {
    screen: Vec<u8>,
    start: [u8; 9],
    tables: Vec<u8>,
    layout: Layout,
}

impl Picture {
    /// Refuses tables with a line code the mode can't use.
    fn new(
        screen: Vec<u8>,
        start: [u8; 9],
        tables: &[u8],
        layout: Layout,
    ) -> Result<Self, DecodeError> {
        if !tables[..TABLE].iter().all(|&code| layout.accepts(code)) {
            return Err(DecodeError::Unrecognized);
        }
        Ok(Self {
            screen,
            start,
            tables: tables.to_vec(),
            layout,
        })
    }

    /// The raw 8192-byte form (RM2).
    fn raw(data: &[u8], layout: Layout) -> Result<Self, DecodeError> {
        if data.len() != 8192 {
            return Err(DecodeError::Unrecognized);
        }
        let start = data[7680..7689]
            .try_into()
            .map_err(|_| DecodeError::Unrecognized)?;
        Self::new(data[..7680].to_vec(), start, &data[8192 - TABLES..], layout)
    }

    /// The Koala-compacted form (RM4); `mode` is the ANTIC mode its header
    /// must declare.
    fn koala(data: &[u8], mode: u8) -> Result<Self, DecodeError> {
        let pic = koala::parse(data)?;
        if pic.mode != mode {
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
        Self::new(
            pic.screen.to_vec(),
            start,
            &data[data.len() - TABLES..],
            Layout::Pairs,
        )
    }

    /// The registers on every line, from `start` and the change tables.
    fn registers(&self, start: [u8; 9]) -> Registers {
        Registers::new(start, &self.tables, self.layout)
    }

    /// The screen as 4-bit pixels, drawn 4 wide.
    fn nibbles(&self) -> Bitmap<'_> {
        Bitmap {
            data: &self.screen,
            bytes_per_line: 40,
            lines: LINES,
            bits: 4,
        }
    }
}

pub(super) fn decode_rm2(data: &[u8]) -> Result<Image, DecodeError> {
    let pic = Picture::raw(data, Layout::Pairs)?;
    let registers = pic.registers(pic.start);
    pic.nibbles().render(4, 1, |y, value| {
        register_rgb(registers.0[y][gtia10_register(value)])
    })
}

pub(super) fn decode_rm4(data: &[u8]) -> Result<Image, DecodeError> {
    let pic = Picture::koala(data, koala::ANTIC_E)?;
    let registers = pic.registers(pic.start);
    let bitmap = Bitmap {
        bits: 2,
        ..pic.nibbles()
    };
    bitmap.render(2, 1, |y, value| {
        let registers = &registers.0[y];
        let color = [registers[8], registers[4], registers[5], registers[6]];
        register_rgb(color[usize::from(value)])
    })
}

/// GTIA mode 11 pixels: the data is ORed into the background's hue and a
/// non-zero value takes the background's luminance (bit 0 dropped); value 0
/// is the background's hue at luminance 0.
fn gtia11_color(background: u8, value: u8) -> u32 {
    let luminance = if value == 0 { 0 } else { background & 0x0e };
    rgb((background & 0xf0 | value << 4) | luminance)
}

/// RM0: a 160x96 Graphics 7+ screen (first 3840 bytes, 2 bits per pixel,
/// drawn 2x2) in the RM2 container. The line-code table must hold only codes
/// the 96-line mode can use.
pub(super) fn decode_rm0(data: &[u8]) -> Result<Image, DecodeError> {
    let pic = Picture::raw(data, Layout::Single)?;
    let registers = pic.registers(pic.start);
    let bitmap = Bitmap {
        data: &pic.screen[..3840],
        bytes_per_line: 40,
        lines: 96,
        bits: 2,
    };
    bitmap.render(2, 2, |y, value| {
        let registers = &registers.0[y];
        register_rgb([registers[8], registers[4], registers[5], registers[6]][usize::from(value)])
    })
}

pub(super) fn decode_rm1(data: &[u8]) -> Result<Image, DecodeError> {
    let pic =
        Picture::raw(data, Layout::Pairs).or_else(|_| Picture::koala(data, koala::ANTIC_E))?;
    // Only a change-table entry brings in the background luminance; the
    // starting register contributes its hue alone.
    let mut start = pic.start;
    start[8] &= 0xf0;
    let registers = pic.registers(start);
    // Pixel value = luminance, ORed into the background's (bit 0 dropped).
    pic.nibbles()
        .render(4, 1, |y, value| rgb(registers.0[y][8] & 0xfe | value))
}

pub(super) fn decode_rm3(data: &[u8]) -> Result<Image, DecodeError> {
    let pic =
        Picture::raw(data, Layout::Pairs).or_else(|_| Picture::koala(data, koala::ANTIC_E))?;
    let registers = pic.registers(pic.start);
    pic.nibbles()
        .render(4, 1, |y, value| gtia11_color(registers.0[y][8], value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn changes_apply_from_their_line() {
        let mut tables = vec![0u8; TABLES];
        tables[TABLE..2 * TABLE].fill(0x80);
        // Entry 28 is the pair of lines 26 and 27: register 3 gets color 0x44
        // on line 27 (line code 27 + 3).
        tables[28] = 30;
        tables[TABLE + 28] = 3;
        tables[2 * TABLE + 28] = 0x44;
        let registers = Registers::new([0x10; 9], &tables, Layout::Pairs);
        assert_eq!(registers.0[26][3], 0x10);
        assert_eq!(registers.0[27][3], 0x44);
        assert_eq!(registers.0[191][3], 0x44);
        assert_eq!(registers.0[191][2], 0x10);
        // The line code of a line in the other pair does nothing.
        tables[28] = 40;
        assert_eq!(
            Registers::new([0x10; 9], &tables, Layout::Pairs).0[191][3],
            0x10
        );
    }

    #[test]
    fn gtia_modes_use_the_background_register() {
        // Hue 9 and luminance 6 in the starting register of a raw file; the
        // first pixel is value 2.
        let mut file = vec![0u8; 8192];
        file[0] = 0x20;
        file[7680 + 8] = 0x96;
        // GTIA 9 takes luminance 2 from the pixel, the register gives the hue.
        assert_eq!(decode_rm1(&file).unwrap().get(0, 0), rgb(0x92));
        // GTIA 11 shows hue 2 | 9 at the register's luminance 6.
        assert_eq!(decode_rm3(&file).unwrap().get(0, 0), rgb(0xb6));
    }

    #[test]
    fn rm2_has_a_fixed_size() {
        assert!(decode_rm2(&[0; 8191]).is_err());
        assert_eq!(decode_rm2(&[0; 8192]).unwrap().width(), 320);
    }

    #[test]
    fn rm0_lines_and_codes() {
        let mut tables = vec![0u8; TABLES];
        tables[TABLE..2 * TABLE].fill(0x80);
        // Entry 16 is line 1 (code 3); entry 20 is line 5 (code 9), listed
        // in an unrelated entry.
        tables[16] = 3;
        tables[TABLE + 16] = 8;
        tables[2 * TABLE + 16] = 0x22;
        tables[100] = 9;
        tables[TABLE + 20] = 8;
        tables[2 * TABLE + 20] = 0x44;
        let registers = Registers::new([0x10; 9], &tables, Layout::Single);
        assert_eq!(registers.0.len(), 96);
        assert_eq!((registers.0[0][8], registers.0[1][8]), (0x10, 0x22));
        assert_eq!((registers.0[4][8], registers.0[5][8]), (0x22, 0x44));
        // Codes 1, 2, 4, 5 and anything past 100 are not RM0 files.
        let mut file = vec![0u8; 8192];
        assert!(decode_rm0(&file).is_ok());
        for bad in [1, 4, 101] {
            file[8192 - TABLES + 40] = bad;
            assert!(decode_rm0(&file).is_err());
        }
    }
}
