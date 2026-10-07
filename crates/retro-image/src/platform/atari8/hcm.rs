//! Hard Color Map (HCM): 128x192, a 2-bit bitmap over a per-line color map.
//!
//! Sources:
//! - RECOIL's format list (<https://recoil.sourceforge.net/formats.html>):
//!   128x192, 9 colors; nothing else is published about the format.
//! - Everything below was reverse engineered by black-box probing of
//!   `recoil2png` with hand-made files (a tut1.hcm sample with its tables
//!   cleared, one byte set at a time) and checked on both corpus samples.
//!   The file is `HCMA`, `38 01`, a mode byte (0 or 2), the colors
//!   background, A, B, PF0, PF1, PF2 (offsets 7-12; 13-47 are unused), eight
//!   256-byte tables of 192 lines starting at offset 48, then the bitmap at
//!   offset 2064 (192 lines of 32 bytes, 2 bits per pixel, drawn 2 wide).
//!   Each 8-output-pixel column (4 bitmap pixels) of a line can be marked A
//!   and/or B by bits in the tables, which behave like player graphics of
//!   quad width: tables 0 and 1 are the A and B bits of columns 22-29 (bit 7
//!   leftmost), tables 4 and 5 of columns 2-9 and tables 6 and 7 of columns
//!   10-17; table 3 holds four missiles (two bits each, mirrored within a
//!   nibble) over columns 18-21: A in bits 0, 1, 4, 5 and B in bits 2, 3, 6,
//!   7; table 2 is unused. That is the mode 2 layout. Mode 0 orders the
//!   tables differently (tables 4 and 5 are the A bits of columns 2-9 and
//!   10-17, tables 6 and 7 their B bits) and the missile byte holds A in
//!   its low nibble and B in its high nibble, bits 0-3 covering columns 19,
//!   18, 21 and 20. A and B are drawn in the colors of players 0 and
//!   1 (mode 0: A as player 0 but B as player 2, which hides behind PF0 and
//!   PF1) and resolved against the 2-bit pixels (values 1-3 = PF0-2) with the
//!   GTIA priority rules in [`gtia`]: mode 0 is PRIOR 0 and mode 2 behaves
//!   as PRIOR 4 with multicolor players (playfield above the players, which
//!   OR where they overlap).

use super::antic::Bitmap;
use super::gtia::{self, Colors};
use super::palette::register_rgb;
use crate::{DecodeError, Image};

const MAGIC: &[u8] = b"HCMA\x38\x01";
const TABLES: usize = 48;
const BITMAP: usize = 2064;
const LINES: usize = 192;
const LEN: usize = BITMAP + 32 * LINES;
const SLOT: usize = 256;

pub(super) fn decode_hcm(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != LEN || !data.starts_with(MAGIC) {
        return Err(DecodeError::Invalid);
    }
    let (prior, mode2) = match data[6] {
        0 => (0x00, false),
        2 => (0x24, true),
        _ => return Err(DecodeError::Invalid),
    };
    let colors = Colors {
        player: [data[8], data[9], data[9], 0],
        playfield: [data[10], data[11], data[12], 0],
        background: data[7],
    };
    let bitmap = Bitmap {
        data: &data[BITMAP..LEN],
        bytes_per_line: 32,
        lines: LINES,
        bits: 2,
    };
    let mut image = Image::new(256, LINES as u32)?;
    for y in 0..LINES {
        let players = line_players(data, y, mode2);
        for x in 0..128 {
            let value = bitmap.pixel(x, y);
            let playfield = gtia::playfield_bit(usize::from(value));
            let color = gtia::resolve(prior, players[x / 4], playfield, &colors);
            let rgb = register_rgb(color);
            image.set(2 * x as u32, y as u32, rgb);
            image.set(2 * x as u32 + 1, y as u32, rgb);
        }
    }
    Ok(image)
}

/// Which of players 0 (A) and 1 (B) cover each 4-pixel column of line `y`.
fn line_players(data: &[u8], y: usize, mode2: bool) -> [u8; 32] {
    let table = |slot: usize| data[TABLES + slot * SLOT + y];
    let mut columns = [0u8; 32];
    let b_bit = if mode2 { 2 } else { 4 };
    // Player graphics: A and B tables for 8 columns, bit 7 leftmost.
    let groups = if mode2 {
        [(0, 1, 22), (4, 5, 2), (6, 7, 10)]
    } else {
        [(0, 1, 22), (4, 6, 2), (5, 7, 10)]
    };
    for (a, b, first) in groups {
        for bit in 0..8 {
            let shift = 7 - bit;
            columns[first + bit] |= (table(a) >> shift & 1) | ((table(b) >> shift & 1) * b_bit);
        }
    }
    let missiles = table(3);
    if mode2 {
        // Each nibble covers a column pair, mirrored; A and B alternate in pairs.
        for nibble in 0..2 {
            let bits = missiles >> (4 * nibble);
            let first = 18 + 2 * nibble;
            for (column, shift) in [(first + 1, 0), (first, 1)] {
                columns[column] |= (bits >> shift & 1) | ((bits >> (shift + 2) & 1) * b_bit);
            }
        }
    } else {
        for (bit, column) in [19, 18, 21, 20].into_iter().enumerate() {
            columns[column] |= (missiles >> bit & 1) | ((missiles >> (bit + 4) & 1) * b_bit);
        }
    }
    columns
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blank() -> alloc::vec::Vec<u8> {
        let mut data = alloc::vec![0u8; LEN];
        data[..6].copy_from_slice(MAGIC);
        data[7..13].copy_from_slice(&[0x00, 0x04, 0x08, 0x06, 0x0a, 0x02]);
        data
    }

    #[test]
    fn player_bits_pick_the_background_colour() {
        let mut data = blank();
        // Column 22 is A, column 23 is B, column 24 both (OR in mode 2).
        data[TABLES] = 0b1010_0000;
        data[TABLES + SLOT] = 0b0110_0000;
        data[6] = 2;
        let image = decode_hcm(&data).unwrap();
        assert_eq!(image.get(22 * 8, 0), register_rgb(0x04));
        assert_eq!(image.get(23 * 8, 0), register_rgb(0x08));
        assert_eq!(image.get(24 * 8, 0), register_rgb(0x0c));
        assert_eq!(image.get(25 * 8, 0), register_rgb(0x00));
    }

    #[test]
    fn missile_nibbles_are_mirrored() {
        let columns = {
            let mut data = blank();
            data[TABLES + 3 * SLOT] = 0b0000_0001;
            line_players(&data, 0, true)
        };
        assert_eq!((columns[19], columns[18]), (1, 0));
    }

    #[test]
    fn rejects_bad_mode_and_size() {
        let mut data = blank();
        data[6] = 1;
        assert!(decode_hcm(&data).is_err());
        assert!(decode_hcm(&data[1..]).is_err());
    }
}
