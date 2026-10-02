//! Oric HIRES screens and character sets, saved as Oric tape files.
//!
//! Sources:
//! - HIRES screen (40 bytes x 200 rows at 0xA000, serial attributes, bit 7
//!   inverse, ink/paper reset to white on black every row): defence-force
//!   Oric coding part 7, <https://www.defence-force.org/computing/oric/coding/part_7/index.htm>,
//!   and OSDK "Oric graphics in detail", <https://osdk.org/index.php?page=articles&ref=ART9>.
//! - Character set RAM (characters 32-127 at 0xB500, 8 bytes each, 6 pixels
//!   used): the same sources.
//! - Tape header layout (0x16 sync bytes, 0x24, 9 header bytes with big-endian
//!   end and start addresses, zero-terminated name): reverse engineered from
//!   samples.
//! - Character sheet layout (32 per row, 8-pixel cells, white on black):
//!   observed from `recoil2png` output.

use crate::{DecodeError, Format, Image};

pub(super) static FORMATS: &[Format] = &[
    Format::new("Oric", "HIRES screen", &["hir", "hrs"], decode_hires),
    Format::new("Oric", "Character set", &["chs"], decode_charset),
];

const HIRES_START: u16 = 0xa000;
const HIRES_LEN: usize = 8000;
const CHARSET_START: u16 = 0xb500;
const CHARSET_LEN: usize = 768;

/// HIRES screen: 200 rows of 40 bytes, 6 pixels per byte.
fn decode_hires(data: &[u8]) -> Result<Image, DecodeError> {
    let screen = tape_body(data, HIRES_START, HIRES_LEN)?;
    let mut image = Image::new(240, 200);
    for (y, row) in screen.chunks_exact(40).enumerate() {
        let (mut ink, mut paper) = (7, 0);
        for (column, &byte) in row.iter().enumerate() {
            let pixels = if byte & 0x60 == 0 {
                match byte & 0x18 {
                    0x00 => ink = byte & 7,
                    0x10 => paper = byte & 7,
                    _ => {}
                }
                0
            } else {
                byte & 0x3f
            };
            let invert = if byte & 0x80 != 0 { 7 } else { 0 };
            for bit in 0..6 {
                let index = if pixels & (0x20 >> bit) != 0 {
                    ink
                } else {
                    paper
                };
                image.set((column * 6 + bit) as u32, y as u32, color(index ^ invert));
            }
        }
    }
    Ok(image)
}

/// Character set: 96 characters drawn as a 32x3 sheet of 8x8 cells.
fn decode_charset(data: &[u8]) -> Result<Image, DecodeError> {
    let charset = tape_body(data, CHARSET_START, CHARSET_LEN + 1)?;
    let mut image = Image::new(256, 24);
    for (index, glyph) in charset[..CHARSET_LEN].chunks_exact(8).enumerate() {
        let (left, top) = (index % 32 * 8, index / 32 * 8);
        for (y, &byte) in glyph.iter().enumerate() {
            for bit in 0..8 {
                let set = byte & (0x80 >> bit) != 0;
                let color = if set { 0xffffff } else { 0 };
                image.set((left + bit) as u32, (top + y) as u32, color);
            }
        }
    }
    Ok(image)
}

/// Colour bits: 0 red, 1 green, 2 blue.
fn color(index: u8) -> u32 {
    let channel = |bit: u8| if index & bit != 0 { 0xff } else { 0 };
    channel(1) << 16 | channel(2) << 8 | channel(4)
}

/// Body of an Oric tape file that loads `len` bytes at `start`.
fn tape_body(data: &[u8], start: u16, len: usize) -> Result<&[u8], DecodeError> {
    let syncs = data.iter().take_while(|&&b| b == 0x16).count();
    let header = data
        .get(syncs..syncs + 10)
        .ok_or(DecodeError::Unrecognized)?;
    if syncs == 0 || header[0] != 0x24 {
        return Err(DecodeError::Unrecognized);
    }
    let end = u16::from_be_bytes([header[5], header[6]]);
    let load = u16::from_be_bytes([header[7], header[8]]);
    if load != start || usize::from(end.wrapping_sub(load)) + 1 != len {
        return Err(DecodeError::Unrecognized);
    }
    let name = &data[syncs + 10..];
    let name_len = name
        .iter()
        .position(|&b| b == 0)
        .ok_or(DecodeError::Unrecognized)?;
    let body = &name[name_len + 1..];
    if body.len() != len {
        return Err(DecodeError::Unrecognized);
    }
    Ok(body)
}
