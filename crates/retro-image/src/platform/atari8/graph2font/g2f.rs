//! Graph2Font G2F: the editor's own save format, `G2FZLIB` and a zlib
//! stream of its memory.
//!
//! Sources:
//! - G2F manual (<https://g2f.atari8.info/instrukcja_eng.html>): rows
//!   switch between 1x1, 2x1 and 4x1 pixel modes (ANTIC 2, ANTIC 4,
//!   GTIA), one charset per row ("TAB"), PMG with a fifth player. Just Solve
//!   "Graph2Font" (<http://fileformats.archiveteam.org/wiki/Graph2Font>):
//!   the `G2FZLIB` magic.
//! - Layout reverse engineered from the corpus samples (Susanne's G2F and
//!   MCH render the same, so its MCH tables could be found in the G2F) and
//!   by black-box probing of `recoil2png` with modified and hand-made files,
//!   recompressed. Inflated, with `W` columns and `F` fonts:
//!
//!   ```text
//!   0        W (40 or 48), bit 7 ignored
//!   1        GTIA mode of 4x1 rows: low 3 bits 5 = mode 10, 6 = mode 11,
//!            anything else mode 9
//!   2        F - 1, bit 7 ignored
//!   3        30 rows of W screen codes (bit 7 inverse)
//!            F fonts of 1024 bytes
//!            30 bytes: font of each row
//!            9 x 256: COLBK, COLPF0-3, COLPM0-3 for scanline y at y
//!            8 x 512: P0, M0, P1, M1, P2, M2, P3, M3, two bytes per
//!            scanline: X (HPOS - 0x20) and a size byte: bit 7 off, bits
//!            0-3 size (0, 1 normal, 2 double, 4 quad), bits 4-6 flags
//!            4 x 512: player k scanline y at y; missile k graphics in the
//!            top 2 bits of byte 256 + y
//!   end+139201  bit 6: split inverse; low 2 bits 3: inverse ANTIC 4
//!               cells don't use COLPF3 (1 and 2 do)
//!   end+145216  30 row modes: 1 ANTIC 2, 2 ANTIC 4, 4 GTIA, 0xFF blank
//!   end+146753  1: VBXE colour attributes follow (not supported here)
//!   end+284997  30 rows of W bytes, bit 7: inverse of the bottom half
//!   ```
//!
//!   "end" is the end of the player block. RECOIL reads nothing else in the
//!   rest of the file (up to 157 KB more). A scanline's PRIOR comes from
//!   the size byte flags: player 0's select the priority (0: 4, 1: 2,
//!   2: 1, 3: 8, 4: 0) even when it is off, player 1's bit 4 is the fifth
//!   player and bit 5 multicolour players. Files end right after the row
//!   modes at the shortest; the later fields are read when present.

use super::{LINES, Line, Picture, ROWS, Row};
use crate::platform::atari8::gtia::Pmg;
use crate::platform::atari8::inflate;
use crate::{DecodeError, Image};
use alloc::vec::Vec;

/// Larger than any Graph2Font file (324055 bytes in the corpus).
const MAX_INFLATED: usize = 1 << 20;

/// Offsets after the player block.
const OPTIONS: usize = 139201;
const ROW_MODES: usize = 145216;
const VBXE: usize = 146753;
const LOWER_INVERSE: usize = 284997;

pub(in crate::platform::atari8) fn decode_g2f(data: &[u8]) -> Result<Image, DecodeError> {
    let packed = data
        .strip_prefix(b"G2FZLIB")
        .ok_or(DecodeError::Unrecognized)?;
    let raw = inflate::zlib(packed, MAX_INFLATED).ok_or(DecodeError::Unrecognized)?;
    Ok(parse(&raw).ok_or(DecodeError::Unrecognized)?.render())
}

fn parse(raw: &[u8]) -> Option<Picture<'_>> {
    let columns = usize::from(raw.first()? & 0x7f);
    if !matches!(columns, 40 | 48) {
        return None;
    }
    let row_gtia = match raw.get(1)? & 7 {
        5 => 2,
        6 => 3,
        _ => 1,
    };
    let fonts_count = usize::from(raw.get(2)? & 0x7f) + 1;
    let mut rest = raw.get(3..)?;
    let mut take = |len: usize| -> Option<&[u8]> {
        let (head, tail) = (rest.get(..len)?, rest.get(len..)?);
        rest = tail;
        Some(head)
    };
    let screen = take(ROWS * columns)?;
    let fonts = take(1024 * fonts_count)?;
    let font_of_row = take(ROWS)?;
    let colors = take(9 * 256)?;
    let positions = take(8 * 512)?;
    let players = take(4 * 512)?;
    let tail = rest;

    let options = *tail.get(OPTIONS)?;
    let split = options & 0x40 != 0;
    let lower = tail.get(LOWER_INVERSE..LOWER_INVERSE + ROWS * columns);
    if !matches!(options & 0x3f, 1..=3) || (split && (options & 0x3f != 2 || lower.is_none())) {
        return None;
    }
    // VBXE colour attributes aren't decoded.
    if tail.get(VBXE).is_some_and(|&vbxe| vbxe != 0) {
        return None;
    }
    let mut rows = [Row {
        antic4: false,
        gtia: 0,
        blank: false,
    }; ROWS];
    for (row, &mode) in rows.iter_mut().zip(tail.get(ROW_MODES..ROW_MODES + ROWS)?) {
        *row = match mode {
            1 => Row { ..*row },
            2 => Row {
                antic4: true,
                ..*row
            },
            4 => Row {
                gtia: row_gtia,
                ..*row
            },
            0xff => Row {
                blank: true,
                ..*row
            },
            _ => return None,
        };
    }

    let mut codes = Vec::with_capacity(ROWS * columns);
    let mut glyphs = Vec::with_capacity(ROWS * columns);
    for (cell, &code) in screen.iter().enumerate() {
        let font = usize::from(font_of_row[cell / columns]);
        let start = 1024 * font + 8 * usize::from(code & 0x7f);
        glyphs.push(fonts.get(start..start + 8)?);
        let lower_inverse = lower.map_or(0, |lower| lower[cell] >> 1 & 0x40);
        codes.push(code & 0x80 | if split { lower_inverse } else { 0 });
    }

    let lines = (0..LINES)
        .map(|y| line(colors, positions, players, y))
        .collect::<Option<Vec<_>>>()?;
    Some(Picture {
        columns,
        rows,
        codes,
        glyphs,
        split,
        antic4_inverse: options & 3 != 3,
        lines,
    })
}

/// The registers of scanline `y`.
fn line(colors: &[u8], positions: &[u8], players: &[u8], y: usize) -> Option<Line> {
    let mut hpos = [0u8; 8];
    let mut sizes = [0u8; 8];
    let mut flags = [0u8; 8];
    let mut on = [false; 8];
    for object in 0..8 {
        let entry = &positions[512 * object + 2 * y..];
        let (x, size) = (entry[0], entry[1]);
        flags[object] = size >> 4 & 7;
        on[object] = size & 0x80 == 0;
        if on[object] {
            sizes[object] = match size & 0x0f {
                0 | 1 => 0,
                2 => 1,
                4 => 3,
                _ => return None,
            };
        }
        hpos[object] = x.wrapping_add(0x20);
    }
    let priority = *[4, 2, 1, 8, 0].get(usize::from(flags[0]))?;
    let prior = priority | flags[2] << 4 & 0x30;
    let size = |objects: [usize; 4]| {
        objects
            .iter()
            .enumerate()
            .fold(0, |acc, (k, &o)| acc | sizes[o] << (2 * k))
    };
    let graf = |object: usize, value: u8| if on[object] { value } else { 0 };
    Some(Line {
        colors: core::array::from_fn(|i| colors[256 * i + y]),
        prior,
        pmg: Pmg {
            hpos_player: core::array::from_fn(|k| hpos[2 * k]),
            hpos_missile: core::array::from_fn(|k| hpos[2 * k + 1]),
            size_player: size([0, 2, 4, 6]),
            size_missile: size([1, 3, 5, 7]),
            graf_player: core::array::from_fn(|k| graf(2 * k, players[512 * k + y])),
            graf_missile: (0..4).fold(0, |acc, k| {
                acc | graf(2 * k + 1, players[512 * k + 256 + y] >> 6) << (2 * k)
            }),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::atari8::palette::rgb;
    use alloc::vec;

    const END: usize = 3 + 1200 + 1024 + 30 + 2304 + 4096 + 2048;

    /// An inflated 40-column, one-font G2F, as short as RECOIL accepts.
    fn raw() -> Vec<u8> {
        let mut raw = vec![0; END + ROW_MODES + ROWS];
        raw[..3].copy_from_slice(&[40, 0, 0]);
        // All objects off.
        for entry in raw[END - 6144..END - 2048].chunks_exact_mut(2) {
            entry.copy_from_slice(&[0x80, 0x80]);
        }
        raw[END + OPTIONS] = 2;
        raw[END + ROW_MODES..].fill(2);
        raw
    }

    /// `raw` in a zlib stream of stored blocks, behind the magic.
    fn g2f(raw: &[u8]) -> Vec<u8> {
        let mut data = b"G2FZLIB\x78\x01".to_vec();
        let chunks: Vec<&[u8]> = raw.chunks(65535).collect();
        for (i, chunk) in chunks.iter().enumerate() {
            data.push(u8::from(i + 1 == chunks.len()));
            let len = chunk.len() as u16;
            data.extend_from_slice(&len.to_le_bytes());
            data.extend_from_slice(&(!len).to_le_bytes());
            data.extend_from_slice(chunk);
        }
        let (mut a, mut b) = (1u32, 0u32);
        for &byte in raw {
            a = (a + u32::from(byte)) % 65521;
            b = (b + a) % 65521;
        }
        data.extend_from_slice(&(b << 16 | a).to_be_bytes());
        data
    }

    #[test]
    fn antic4_row_uses_the_row_font() {
        let mut raw = raw();
        raw[3] = 1; // cell 0: character 1
        raw[3 + 1200 + 8] = 0b0001_1011; // character 1, scanline 0
        let colors = 3 + 1200 + 1024 + 30;
        for (i, color) in [0x00, 0x46, 0x88, 0xca].into_iter().enumerate() {
            raw[colors + 256 * i] = color;
        }
        let image = decode_g2f(&g2f(&raw)).unwrap();
        assert_eq!(image.get(8, 0), rgb(0x00));
        assert_eq!(image.get(10, 0), rgb(0x46));
        assert_eq!(image.get(12, 0), rgb(0x88));
        assert_eq!(image.get(14, 0), rgb(0xca));
    }

    #[test]
    fn rejects_bad_fields() {
        assert!(decode_g2f(&g2f(&raw())).is_ok());
        let mut bad_row = raw();
        bad_row[END + ROW_MODES] = 3;
        assert!(decode_g2f(&g2f(&bad_row)).is_err());
        let mut split_without_map = raw();
        split_without_map[END + OPTIONS] = 0x42;
        assert!(decode_g2f(&g2f(&split_without_map)).is_err());
        let short = raw();
        assert!(decode_g2f(&g2f(&short[..short.len() - 1])).is_err());
    }
}
