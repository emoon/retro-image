//! QuantumPaint pictures (`PBX`).
//!
//! Source: <https://temlib.org/AtariForumWiki/index.php/QuantumPaint_file_format>
//! (header, palette records, Tiny-style column order of compressed screens
//! and Hans Wessels' public-domain palette index function).
//! Observed from `recoil2png` output: per-line palettes are 9-bit ST
//! colors (STE bits ignored); in 4096-color mode the two palette sets
//! are averaged per component.

use alloc::vec::Vec;

use super::common::{
    Resolution, SCREEN_LEN, decode_screen_by_line, interleaved_index, palette_words, st_palette,
    st_rgb, words,
};
use crate::bytes::be16;
use crate::codec::packbits;
use crate::{DecodeError, Image};

const HEADER_LEN: usize = 128;
const RECORDS_LEN: usize = 8 * 48;
const LINE_PALETTES_LEN: usize = 200 * 32 * 2;

pub(super) fn decode_pbx(data: &[u8]) -> Result<Image, DecodeError> {
    decode(data).ok_or(DecodeError::Unrecognized)
}

fn decode(data: &[u8]) -> Option<Image> {
    if data.get(..3)? != [0, 0, 0] {
        return None;
    }
    let mode = *data.get(3)?;
    let compressed = be16(data, 4)? == 0x8001;
    let palette_len = match mode {
        0 | 1 => RECORDS_LEN,
        0x80 => LINE_PALETTES_LEN,
        0x81 => 2 * LINE_PALETTES_LEN,
        _ => return None,
    };
    let body = &data[HEADER_LEN.min(data.len())..];
    let unpacked;
    let (palettes, screen) = if compressed {
        if mode == 1 {
            // The column order of compressed medium-resolution screens is
            // not documented precisely enough.
            return None;
        }
        unpacked = packbits::unpack(body, palette_len + SCREEN_LEN)?.0;
        let (palettes, columns) = unpacked.split_at(palette_len);
        (palettes, super::tiny::from_columns(columns)?)
    } else {
        if data.len() != HEADER_LEN + palette_len + SCREEN_LEN {
            return None;
        }
        let (palettes, screen) = body.split_at(palette_len);
        (palettes, screen.to_vec())
    };
    match mode {
        0 | 1 => records(palettes, &screen, mode == 1),
        0x80 => line_palettes(palettes, None, &screen),
        _ => {
            let (first, second) = palettes.split_at(LINE_PALETTES_LEN);
            line_palettes(first, Some(second), &screen)
        }
    }
}

/// Up to eight 16-color palettes, each active from a given line down.
fn records(data: &[u8], screen: &[u8], medium: bool) -> Option<Image> {
    let resolution = if medium {
        Resolution::Medium
    } else {
        Resolution::Low
    };
    let mut starts: Vec<(usize, Vec<u32>)> = Vec::new();
    for record in data.as_chunks::<48>().0 {
        let first_line = usize::from(be16(record, 32)?);
        let active = be16(record, 34)? != 0;
        if active || starts.is_empty() {
            let words = palette_words(record, 0, resolution.colors())?;
            starts.push((first_line, st_palette(&words)));
        }
    }
    starts.sort_by_key(|(line, _)| *line);
    decode_screen_by_line(resolution, screen, |y| {
        let found = starts.iter().rev().find(|(line, _)| *line <= y);
        let (_, palette) = found.or(starts.first())?;
        Some(palette.clone())
    })
}

/// 32 ST colors per line chosen by `find_pbx_index`, optionally
/// averaged with a second palette set.
fn line_palettes(first: &[u8], second: Option<&[u8]>, screen: &[u8]) -> Option<Image> {
    let first = line_palette_image(first, screen)?;
    Some(match second {
        None => first,
        Some(second) => Image::blend(&[&first, &line_palette_image(second, screen)?]),
    })
}

fn line_palette_image(palettes: &[u8], screen: &[u8]) -> Option<Image> {
    let palettes = words(palettes);
    let mut image = Image::new(320, 200);
    for y in 0..200 {
        let line = &screen[y * 160..(y + 1) * 160];
        for x in 0..320 {
            let c = interleaved_index(line, x as u32, 4);
            let color = st_rgb(*palettes.get(y * 32 + palette_index(x, c))?, false);
            image.set(x as u32, y as u32, color);
        }
    }
    Some(image)
}

/// Hans Wessels' `find_pbx_index`.
fn palette_index(x: usize, c: usize) -> usize {
    let mut x1 = 10 * c;
    if c & 1 != 0 {
        x1 -= 5;
    } else {
        x1 += 1;
    }
    if c > 7 {
        x1 += 12;
    }
    if x >= x1 + 75 { c + 16 } else { c }
}
