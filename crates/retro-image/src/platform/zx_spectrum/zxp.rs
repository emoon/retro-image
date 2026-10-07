//! ZX-Paintbrush text images.
//!
//! Sources:
//! - Layout (header line, rows of `0`/`1`, blank line, rows of hex
//!   attributes whose count gives the cell height): SpectraLab
//!   `ZX_SPECTRUM_GRAPHICS_GUIDE.md` (MIT), section ZXP,
//!   <https://github.com/Bedazzle/SpectraLab/blob/main/ZX_SPECTRUM_GRAPHICS_GUIDE.md>.
//! - The plain `ZX-Paintbrush image` header variant: seen in samples.
//!   Files with the optional ULA+ palette line are not accepted (no sample).

use alloc::vec::Vec;

use super::screen::{Frame, attribute_color};
use crate::{DecodeError, Image};

pub(super) fn decode_zxp(data: &[u8]) -> Result<Image, DecodeError> {
    let mut lines = data
        .split(|&b| b == b'\n')
        .map(|line| line.strip_suffix(b"\r").unwrap_or(line));
    let header = lines.next().ok_or(DecodeError::Invalid)?;
    if !header.starts_with(b"ZX-Paintbrush") || lines.next() != Some(&[][..]) {
        return Err(DecodeError::Invalid);
    }
    let rows: Vec<&[u8]> = lines.by_ref().take_while(|line| !line.is_empty()).collect();
    let width = rows.first().map_or(0, |row| row.len());
    let height = rows.len();
    let bitmap_ok = rows
        .iter()
        .all(|row| row.len() == width && row.iter().all(|&c| c == b'0' || c == b'1'));
    if width == 0 || !width.is_multiple_of(8) || !height.is_multiple_of(8) || !bitmap_ok {
        return Err(DecodeError::Invalid);
    }
    let attributes = lines
        .take_while(|line| !line.is_empty())
        .map(|line| parse_hex_row(line, width / 8))
        .collect::<Result<Vec<_>, _>>()?;
    let cell_height = match attributes.len() {
        0 => return Err(DecodeError::Invalid),
        rows if height.is_multiple_of(rows) => height / rows,
        _ => return Err(DecodeError::Invalid),
    };
    let mut frame = Frame::new(width, height)?;
    for (y, row) in rows.iter().enumerate() {
        for (x, &c) in row.iter().enumerate() {
            let attribute = attributes[y / cell_height][x / 8];
            frame.set(x, y, attribute_color(attribute, c == b'1'));
        }
    }
    frame.into_image()
}

/// A line of `count` space-separated two-digit hex bytes.
fn parse_hex_row(line: &[u8], count: usize) -> Result<Vec<u8>, DecodeError> {
    let values = line
        .split(|&b| b == b' ')
        .filter(|word| !word.is_empty())
        .map(|word| {
            core::str::from_utf8(word)
                .ok()
                .and_then(|s| u8::from_str_radix(s, 16).ok())
                .ok_or(DecodeError::Invalid)
        })
        .collect::<Result<Vec<u8>, _>>()?;
    if values.len() != count {
        return Err(DecodeError::Invalid);
    }
    Ok(values)
}
