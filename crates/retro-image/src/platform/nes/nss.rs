//! NES Screen Tool session (`.nss`, text variant): the nametable drawn with
//! the session's pattern table and palette.
//!
//! Sources: reverse engineered from the three famidash sessions in
//! `corpus/extra/gameboy-nes/nes` (MIT, <https://github.com/tfdsoft/famidash>).
//! No NESST source code or documentation was read. Nametable, attribute and
//! pattern layout follow the nesdev wiki,
//! <https://www.nesdev.org/wiki/PPU_nametables>,
//! <https://www.nesdev.org/wiki/PPU_attribute_tables>,
//! <https://www.nesdev.org/wiki/PPU_pattern_tables>.
//!
//! The file starts `NSTssTXT` and continues with `key=value` lines. Binary
//! values are hex bytes, with `xx[n]` meaning byte `xx` repeated `n` times
//! (`n` in hex). Keys used here:
//! - `CHRMain`: the pattern tables (8 or 16 KiB). The background uses the
//!   first 4 KiB: two sessions have the second bank selected in the tile view
//!   (`VarBankActive=4096`) yet only the first bank gives a coherent picture
//!   (checked by rendering both), so the selection is not followed.
//! - `Palette`: 64 bytes; the first 16 are the four background palettes, the
//!   first byte of each being the shared background color.
//! - `VarNameW`, `VarNameH`: nametable size in tiles (32x30, or 64x60 for four
//!   screens), default 32x30. `NameTable` is a raster of that size, one tile
//!   number per byte.
//! - `AttrTable`: one byte per 4x4 tiles, as a raster of `ceil(W/4)` by
//!   `ceil(H/4)` bytes; two bits per 2x2 tiles, top-left lowest.

use alloc::vec::Vec;

use super::nametable::Nametable;
use super::{MASTER_PALETTE, PATTERN_TABLE_LEN};
use crate::{DecodeError, Image};

const MAGIC: &[u8] = b"NSTssTXT";
/// Most bytes a hex field may expand to.
const MAX_FIELD: usize = 1 << 20;
const DEFAULT_SIZE: (usize, usize) = (32, 30);

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    if !data.starts_with(MAGIC) {
        return Err(DecodeError::Invalid);
    }
    let (width, height) = match (number(data, b"VarNameW"), number(data, b"VarNameH")) {
        (Some(w), Some(h)) => (w, h),
        (None, None) => DEFAULT_SIZE,
        _ => return Err(DecodeError::Invalid),
    };
    if width == 0 || height == 0 || width > 256 || height > 256 {
        return Err(DecodeError::Invalid);
    }
    let pattern = hex_field(data, b"CHRMain")?;
    let palette = hex_field(data, b"Palette")?;
    let names = hex_field(data, b"NameTable")?;
    let attributes = hex_field(data, b"AttrTable")?;
    let attribute_columns = width.div_ceil(4);
    if pattern.len() < PATTERN_TABLE_LEN
        || palette.len() < 16
        || names.len() != width * height
        || attributes.len() < attribute_columns * height.div_ceil(4)
    {
        return Err(DecodeError::Invalid);
    }
    Nametable {
        width,
        height,
        pattern: &pattern,
        names: &names,
        attributes: &attributes,
        palette: &palette,
        master: &MASTER_PALETTE,
    }
    .draw()
}

/// The value of the `key=value` line whose key is exactly `key`.
fn field<'a>(data: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    data.split(|&b| b == b'\n').find_map(|line| {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        line.strip_prefix(key)?.strip_prefix(b"=")
    })
}

/// A decimal field.
fn number(data: &[u8], key: &[u8]) -> Option<usize> {
    core::str::from_utf8(field(data, key)?).ok()?.parse().ok()
}

/// A field of hex bytes, `xx[n]` repeating a byte `n` times.
fn hex_field(data: &[u8], key: &[u8]) -> Result<Vec<u8>, DecodeError> {
    let text = field(data, key).ok_or(DecodeError::Invalid)?;
    let mut out = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        let byte = hex_byte(rest).ok_or(DecodeError::Invalid)?;
        rest = &rest[2..];
        let mut repeat = 1;
        if let Some(counted) = rest.strip_prefix(b"[") {
            let end = counted
                .iter()
                .position(|&b| b == b']')
                .ok_or(DecodeError::Invalid)?;
            let digits = core::str::from_utf8(&counted[..end]).map_err(|_| DecodeError::Invalid)?;
            repeat = usize::from_str_radix(digits, 16).map_err(|_| DecodeError::Invalid)?;
            rest = &counted[end + 1..];
        }
        if out.len().saturating_add(repeat) > MAX_FIELD {
            return Err(DecodeError::Invalid);
        }
        out.resize(out.len() + repeat, byte);
    }
    Ok(out)
}

fn hex_byte(text: &[u8]) -> Option<u8> {
    let digits = text.get(..2)?;
    let value = |c: u8| char::from(c).to_digit(16);
    Some((value(digits[0])? << 4 | value(digits[1])?) as u8)
}
