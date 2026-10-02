//! QuantumPaint pictures (`PBX`).
//!
//! Source: <https://temlib.org/AtariForumWiki/index.php/QuantumPaint_file_format>
//! (header, palette records, Tiny-style column order of compressed screens
//! and Hans Wessels' public-domain palette index function).
//! Observed from `recoil2png` output: per-line palettes are 9-bit ST
//! colours (STE bits ignored); in 4096-colour mode the two palette sets
//! are averaged per component.

use alloc::vec::Vec;

use super::common::{
    Resolution, SCREEN_LEN, be16, interleaved_index, palette_words, planar_image, st_palette,
    st_rgb, unpack_bits,
};
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
        unpacked = unpack_bits(body, palette_len + SCREEN_LEN)?.0;
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

/// Up to eight 16-colour palettes, each active from a given line down.
fn records(data: &[u8], screen: &[u8], medium: bool) -> Option<Image> {
    let resolution = if medium {
        Resolution::Medium
    } else {
        Resolution::Low
    };
    let mut starts: Vec<(usize, Vec<u32>)> = Vec::new();
    for record in data.chunks_exact(48) {
        let first_line = usize::from(be16(record, 32)?);
        let active = be16(record, 34)? != 0;
        if active || starts.is_empty() {
            let words = palette_words(record, 0, resolution.colors())?;
            starts.push((first_line, st_palette(&words)));
        }
    }
    starts.sort_by_key(|(line, _)| *line);
    let mut image = Image::new(resolution.width(), 200 * resolution.y_scale());
    for y in 0..200 {
        let palette = &starts
            .iter()
            .rev()
            .find(|(line, _)| *line <= y)
            .unwrap_or(&starts[0])
            .1;
        let line = &screen[y * 160..(y + 1) * 160];
        let strip = planar_image(
            line,
            resolution.width(),
            1,
            resolution.planes(),
            palette,
            resolution.y_scale(),
        )?;
        for (i, p) in strip.rgb().chunks_exact(3).enumerate() {
            let x = i as u32 % resolution.width();
            let dy = i as u32 / resolution.width();
            let color = u32::from_be_bytes([0, p[0], p[1], p[2]]);
            image.set(x, y as u32 * resolution.y_scale() + dy, color);
        }
    }
    Some(image)
}

/// 32 ST colours per line chosen by `find_pbx_index`, optionally
/// averaged with a second palette set.
fn line_palettes(first: &[u8], second: Option<&[u8]>, screen: &[u8]) -> Option<Image> {
    let words = |data: &[u8]| -> Vec<u16> {
        data.chunks_exact(2)
            .map(|w| u16::from_be_bytes([w[0], w[1]]))
            .collect()
    };
    let first = words(first);
    let second = second.map(words);
    let mut image = Image::new(320, 200);
    for y in 0..200 {
        let line = &screen[y * 160..(y + 1) * 160];
        for x in 0..320 {
            let c = interleaved_index(line, x as u32, 4);
            let i = y * 32 + palette_index(x, c);
            let a = st_rgb(*first.get(i)?, false);
            let color = match &second {
                None => a,
                Some(second) => average(a, st_rgb(*second.get(i)?, false)),
            };
            image.set(x as u32, y as u32, color);
        }
    }
    Some(image)
}

fn average(a: u32, b: u32) -> u32 {
    let channel = |shift: u32| (((a >> shift & 0xff) + (b >> shift & 0xff)) / 2) << shift;
    channel(16) | channel(8) | channel(0)
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
