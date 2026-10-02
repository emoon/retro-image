//! Packed Atari 8-bit screens (wave 4): Trzmiel CPR and Kompresor do Animatora
//! KPR.
//!
//! Sources: Just Solve "Trzmiel"
//! (<http://fileformats.archiveteam.org/wiki/Trzmiel>) and "Kompresor do
//! Animatora" (<http://fileformats.archiveteam.org/wiki/Kompresor_do_Animatora>),
//! and the RECOIL formats list (<https://recoil.sourceforge.net/formats.html>)
//! only name the formats (320x192 mono, 4 colours). Both packers are
//! undocumented; the layouts were reverse engineered from the corpus samples
//! (CPR: RMF, WALL, DEMO, WINSTON, TEX, BRUSH, DREAM13; KPR: FONTY) and
//! probed with hand-made files fed to `recoil2png` (black box).
//!
//! CPR: one mode byte, then tokens until 7680 bytes of a 320x192 1-bit screen
//! are produced. A token byte with bit 7 set copies its low 7 bits of
//! literal bytes (0 is invalid); a byte 1-127 repeats the next byte that many
//! times; a zero byte is followed by a big-endian 16-bit count, then the
//! value. Overshoot and trailing bytes are ignored; running out of input
//! early is an error. Mode 2 stores the screen in order; mode 1 stores it by
//! byte columns, each column the 96 even lines then the 96 odd lines. A set
//! bit is black (`00`), a clear bit `0C`.
//!
//! KPR: a binary-load header (`FF FF`, start, end; the end must match the file
//! length), two ignored bytes (`KB`), the number of bands, the cells per
//! band row and the rows, then the map of tile numbers (band by band, each
//! band row by row, left to right), then 8-byte tiles of four 2-bit pixels per
//! line (Graphics 15 shape, drawn 2x1) in grey (`00 04 08 0C`). A tile
//! number must lie inside the tile data.

use super::palette::rgb;
use super::screen::{GREY_COLORS, bitmap, four_color, hires};
use crate::bytes::le16;
use crate::{DecodeError, Image};
use alloc::vec::Vec;

const SCREEN: usize = 7680;
const LINE: usize = 40;

/// Trzmiel: `data[0]` is the mode (1 or 2), tokens follow.
pub(super) fn decode_cpr(data: &[u8]) -> Result<Image, DecodeError> {
    let (&mode, tokens) = data.split_first().ok_or(DecodeError::Unrecognized)?;
    if !matches!(mode, 1 | 2) {
        return Err(DecodeError::Unrecognized);
    }
    let stream = unpack(tokens, SCREEN)?;
    let screen = if mode == 2 {
        stream
    } else {
        // Byte columns of 192 lines: even lines first, then odd lines.
        let mut screen = alloc::vec![0; SCREEN];
        for (i, &byte) in stream.iter().enumerate() {
            let (column, k) = (i / 192, i % 192);
            let line = if k < 96 { 2 * k } else { 2 * (k - 96) + 1 };
            screen[line * LINE + column] = byte;
        }
        screen
    };
    Ok(hires(bitmap(&screen, LINE, 1), rgb(0x0c), rgb(0x00)))
}

/// Runs the CPR token stream until `len` bytes are produced.
fn unpack(mut tokens: &[u8], len: usize) -> Result<Vec<u8>, DecodeError> {
    let mut out = Vec::with_capacity(len);
    while out.len() < len {
        let (&token, rest) = tokens.split_first().ok_or(DecodeError::Unrecognized)?;
        tokens = rest;
        if token & 0x80 != 0 {
            let count = usize::from(token & 0x7f);
            if count == 0 || count > tokens.len() {
                return Err(DecodeError::Unrecognized);
            }
            let (literal, rest) = tokens.split_at(count);
            out.extend_from_slice(literal);
            tokens = rest;
        } else {
            let count = if token == 0 {
                let count = le16(tokens, 0).ok_or(DecodeError::Unrecognized)?;
                tokens = &tokens[2..];
                // Stored big-endian.
                usize::from(count.swap_bytes())
            } else {
                usize::from(token)
            };
            let (&value, rest) = tokens.split_first().ok_or(DecodeError::Unrecognized)?;
            tokens = rest;
            out.resize(out.len() + count.min(len), value);
        }
    }
    out.truncate(len);
    Ok(out)
}

/// Kompresor do Animatora.
pub(super) fn decode_kpr(data: &[u8]) -> Result<Image, DecodeError> {
    let (start, end) = match (data.get(..2), le16(data, 2), le16(data, 4)) {
        (Some(&[0xff, 0xff]), Some(start), Some(end)) if end >= start => (start, end),
        _ => return Err(DecodeError::Unrecognized),
    };
    if data.len() != 6 + usize::from(end - start) + 1 {
        return Err(DecodeError::Unrecognized);
    }
    let &[_, _, bands, per_band, rows, ref rest @ ..] = &data[6..] else {
        return Err(DecodeError::Unrecognized);
    };
    let (bands, per_band, rows) = (usize::from(bands), usize::from(per_band), usize::from(rows));
    let cells = bands * per_band * rows;
    if cells == 0 || rest.len() < cells {
        return Err(DecodeError::Unrecognized);
    }
    let (map, tiles) = rest.split_at(cells);
    let tile_count = tiles.len() / 8;
    if map.iter().any(|&t| usize::from(t) >= tile_count) {
        return Err(DecodeError::Unrecognized);
    }
    let columns = bands * per_band;
    let mut screen = alloc::vec![0; columns * rows * 8];
    for (i, &tile) in map.iter().enumerate() {
        let (band, within) = (i / (per_band * rows), i % (per_band * rows));
        let (row, column) = (within / per_band, band * per_band + within % per_band);
        let tile = &tiles[usize::from(tile) * 8..][..8];
        for (line, &byte) in tile.iter().enumerate() {
            screen[(row * 8 + line) * columns + column] = byte;
        }
    }
    Ok(four_color(bitmap(&screen, columns, 2), 2, 1, GREY_COLORS))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpr_tokens() {
        // 3 literals, a run of 4, an extended run for the rest.
        let mut data = alloc::vec![2, 0x83, 1, 2, 3, 4, 9];
        data.extend_from_slice(&[0, 0x1d, 0xf9, 7]); // 7673 sevens
        let out = unpack(&data[1..], SCREEN).unwrap();
        assert_eq!(&out[..8], &[1, 2, 3, 9, 9, 9, 9, 7]);
        assert_eq!(out.len(), SCREEN);
        assert!(unpack(&[0x80, 1], 4).is_err());
        assert!(unpack(&[3, 1], 8).is_err());
        assert_eq!(unpack(&[0, 0, 4, 5], 4).unwrap(), [5; 4]);
    }

    #[test]
    fn cpr_mode_1_is_column_major() {
        let mut data = alloc::vec![1, 0x81, 0xff];
        data.extend_from_slice(&[0, 0x1d, 0xff, 0]);
        let image = decode_cpr(&data).unwrap();
        assert_eq!(image.get(0, 0), rgb(0x00));
        assert_eq!(image.get(8, 0), rgb(0x0c));
        // Stream byte 96 is line 1, column 0.
        let mut data = alloc::vec![1, 0, 0, 96, 0, 0x81, 0xff];
        data.extend_from_slice(&[0, 0x1d, 0x9f, 0]);
        let image = decode_cpr(&data).unwrap();
        assert_eq!(image.get(0, 1), rgb(0x00));
        assert_eq!(image.get(0, 0), rgb(0x0c));
    }

    #[test]
    fn kpr_checks_load_range_and_tiles() {
        // One band, one cell, one row, one tile.
        let mut data = alloc::vec![0xff, 0xff, 0, 0, 0, 0, b'K', b'B', 1, 1, 1, 0];
        data.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0x1b]);
        let end = (data.len() - 7) as u8;
        data[4] = end;
        assert_eq!(decode_kpr(&data).unwrap().width(), 8);
        data[11] = 1; // tile 1 does not exist
        assert!(decode_kpr(&data).is_err());
        data[11] = 0;
        data[4] += 1;
        assert!(decode_kpr(&data).is_err());
    }
}
