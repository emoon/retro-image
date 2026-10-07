//! Graph2Font MCH: cells with their character data inline, then
//! per-scanline registers and player/missile graphics.
//!
//! Sources:
//! - Just Solve "Graph2Font" (<http://fileformats.archiveteam.org/wiki/Graph2Font>)
//!   names the format; there is no published layout.
//! - Layout reverse engineered from the corpus samples and by black-box
//!   probing of `recoil2png` with hand-made files (offsets for 40 columns;
//!   48-column files have 1440 cells, so everything after them moves by
//!   2160):
//!
//!   ```text
//!   0      1200 cells of 9 bytes: a code byte, then the 8 bytes shown
//!          on the cell's 8 scanlines. The first code byte's low 6 bits
//!          are the mode: 01 ANTIC 2, 05 ANTIC 4, 09/19/29 ANTIC 2 with
//!          GTIA mode 9/10/11.
//!   10800  20 tables of 240 per-scanline values: COLBK, COLPF0-3,
//!          COLPM0-3, HPOSP0-3, HPOSM0-3, SIZEP0-3 (two bits each,
//!          player 0 lowest), SIZEM, PRIOR
//!   15600  GRAFM for each scanline
//!   15840  player 0-3 memory, 256 bytes each, scanline y at byte 16 + y
//!   16864  ignored by RECOIL (13856 bytes, then a 113-byte tail)
//!   ```
//!
//!   Split inverse mode is on when any code byte has bit 6 set. RECOIL
//!   accepts exactly 30833 or 32993 bytes.

use super::{LINES, Line, Picture, ROWS, Row};
use crate::platform::atari8::gtia::Pmg;
use crate::{DecodeError, Image};

pub(in crate::platform::atari8) fn decode_mch(data: &[u8]) -> Result<Image, DecodeError> {
    let columns = match data.len() {
        30833 => 40,
        32993 => 48,
        _ => return Err(DecodeError::Invalid),
    };
    let (antic4, gtia) = match data[0] & 0x3f {
        0x01 => (false, 0),
        0x05 => (true, 0),
        0x09 => (false, 1),
        0x19 => (false, 2),
        0x29 => (false, 3),
        _ => return Err(DecodeError::Invalid),
    };
    let cells = &data[..9 * columns * ROWS];
    let tables = &data[cells.len()..];
    let table = |index: usize, y: usize| tables[index * LINES + y];
    let lines = (0..LINES)
        .map(|y| Line {
            colors: core::array::from_fn(|i| table(i, y)),
            prior: table(19, y),
            pmg: Pmg {
                hpos_player: core::array::from_fn(|k| table(9 + k, y)),
                hpos_missile: core::array::from_fn(|k| table(13 + k, y)),
                size_player: table(17, y),
                size_missile: table(18, y),
                graf_missile: table(20, y),
                graf_player: core::array::from_fn(|k| tables[21 * LINES + 256 * k + 16 + y]),
            },
        })
        .collect();
    let codes: alloc::vec::Vec<u8> = cells
        .as_chunks::<9>()
        .0
        .iter()
        .map(|cell| cell[0])
        .collect();
    let picture = Picture {
        columns,
        rows: [Row {
            antic4,
            gtia,
            blank: false,
        }; ROWS],
        split: codes.iter().any(|code| code & 0x40 != 0),
        codes,
        glyphs: cells
            .as_chunks::<9>()
            .0
            .iter()
            .map(|cell| &cell[1..])
            .collect(),
        antic4_inverse: true,
        vbxe: None,
        lines,
    };
    picture.render()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::atari8::palette::rgb;
    use alloc::vec;
    use alloc::vec::Vec;

    fn mch(mode: u8) -> Vec<u8> {
        let mut data = vec![0; 30833];
        data[0] = mode;
        data
    }

    #[test]
    fn antic4_inverse_uses_colpf3() {
        let mut data = mch(0x05);
        data[9] = 0x80; // cell 1 inverse
        data[10] = 0xff; // cell 1, scanline 0: four `11` pixels
        data[1] = 0xff; // cell 0, scanline 0
        for (table, color) in [(3, 0x46), (4, 0x88)] {
            data[10800 + 240 * table] = color;
        }
        let image = decode_mch(&data).unwrap();
        assert_eq!(image.get(8, 0), rgb(0x46));
        assert_eq!(image.get(16, 0), rgb(0x88));
    }

    #[test]
    fn gtia9_ors_into_background() {
        let mut data = mch(0x09);
        data[1] = 0x5a;
        data[10800] = 0x20;
        let image = decode_mch(&data).unwrap();
        assert_eq!(image.get(0, 0), rgb(0x20));
        assert_eq!(image.get(8, 0), rgb(0x25));
        assert_eq!(image.get(12, 0), rgb(0x2a));
    }

    #[test]
    fn rejects_other_modes_and_sizes() {
        assert!(decode_mch(&mch(0x07)).is_err());
        assert!(decode_mch(&[1; 30832]).is_err());
    }
}
