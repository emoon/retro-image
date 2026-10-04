//! Game Boy Tile Designer `.gbr` tile sets and Game Boy Map Builder `.gbm`
//! maps (Harry Mulder's GBTD and GBMB).
//!
//! Sources:
//! - Format specifications by Harry Mulder, (c) 1999: `GBRS9906.ZIP` (GBR,
//!   1 June 1999) and `GBMS9910.ZIP` (GBM, October 1999), from
//!   <http://www.devrs.com/gb/hmgd/supp.html>. They are the author's published
//!   format documents with no licence stated; used as a prose specification.
//! - Reverse engineered from the GBDK-2020 example assets in
//!   `corpus/extra/gameboy-nes/gbtd` (see its `MANIFEST.tsv`):
//!   - the documents call every number "hi-endian" but the files are
//!     little-endian, apart from the 3-byte GBM map records, which really are
//!     big-endian;
//!   - a GBM map object holds `width * height` records followed by more
//!     bytes that are ignored;
//!   - `GBO0` GBM files (GBMB before 1.0, undocumented) share the magic of
//!     GBR but have a different layout, and are rejected.
//!
//! GBR: `GBO0`, then objects `u16 type, u16 id, u32 length, body`. TileData
//! (type 2) is a 30-byte name, `u16` tile width and height in pixels, `u16`
//! count, a 4-byte colour set and one byte per pixel (0-3, remapped through
//! the colour set), tile after tile. Only the shades are drawn: the GBC and
//! SGB palettes (types 0x0D, 0x0E) are ignored. A GBR is shown as a sheet
//! of tiles, 128 pixels wide where the tiles allow.
//!
//! GBM: `GBO1`, then objects with a 20-byte header (`HPJMTL`, `u16` type,
//! `u16` id, `u16` master id, `u32` CRC, `u32` length). Map (type 2) gives
//! the size and the path of the GBR it uses; Map Tile data (type 3) holds
//! one 3-byte big-endian record per cell: tile number in bits 0-9, bit 22
//! flips horizontally and bit 23 vertically (the flip bits come from the
//! document only, no sample sets them). The tiles come from the GBR named by
//! the map, taken from the companion files, so the GBM alone is rejected.

use alloc::vec::Vec;

use super::SHADES;
use crate::bytes::{le16, le32};
use crate::image::check_size;
use crate::{Companions, DecodeError, Image};

const GBR_MAGIC: &[u8] = b"GBO0";
const GBM_MAGIC: &[u8] = b"GBO1";
const GBM_MARKER: &[u8] = b"HPJMTL";
const TILE_DATA: u16 = 2;
const MAP: u16 = 2;
const MAP_TILES: u16 = 3;
const SHEET_WIDTH: usize = 128;

/// A tile set: `count` tiles of `width` x `height` pixels, one byte per
/// pixel, and the colour set that remaps them to shades.
struct TileSet<'a> {
    width: usize,
    height: usize,
    count: usize,
    colors: [u8; 4],
    pixels: &'a [u8],
}

impl TileSet<'_> {
    /// The shade at (`x`, `y`) of `tile`, white for a tile that isn't there.
    fn shade(&self, tile: usize, x: usize, y: usize) -> u32 {
        if tile >= self.count {
            return SHADES[0];
        }
        let value = self.pixels[(tile * self.height + y) * self.width + x];
        SHADES[usize::from(self.colors[usize::from(value & 3)] & 3)]
    }
}

/// One object of a file: its type, its master object (GBM only, else 0) and
/// its body.
struct Object<'a> {
    kind: u16,
    master: u16,
    body: &'a [u8],
}

/// The objects of a GBR (`header` = 8 bytes) or GBM (`header` = 20 bytes).
fn objects(data: &[u8], header: usize) -> Result<Vec<Object<'_>>, DecodeError> {
    let mut found = Vec::new();
    let mut at = 4;
    while at < data.len() {
        let fields = if header == 8 {
            le16(data, at)
                .zip(le32(data, at + 4))
                .map(|(k, l)| (k, 0, l))
        } else if data[at..].starts_with(GBM_MARKER) {
            le16(data, at + 6)
                .zip(le16(data, at + 10))
                .zip(le32(data, at + 16))
                .map(|((k, m), l)| (k, m, l))
        } else {
            None
        };
        let (kind, master, length) = fields.ok_or(DecodeError::Unrecognized)?;
        let start = at + header;
        let body = data
            .get(
                start
                    ..start
                        .checked_add(length as usize)
                        .ok_or(DecodeError::Unrecognized)?,
            )
            .ok_or(DecodeError::Unrecognized)?;
        found.push(Object { kind, master, body });
        at = start + length as usize;
    }
    Ok(found)
}

fn parse_gbr(data: &[u8]) -> Result<TileSet<'_>, DecodeError> {
    if !data.starts_with(GBR_MAGIC) {
        return Err(DecodeError::Unrecognized);
    }
    let objects = objects(data, 8)?;
    let body = objects
        .iter()
        .find(|o| o.kind == TILE_DATA)
        .ok_or(DecodeError::Unrecognized)?
        .body;
    let (Some(width), Some(height), Some(count)) = (le16(body, 30), le16(body, 32), le16(body, 34))
    else {
        return Err(DecodeError::Unrecognized);
    };
    let (width, height, count) = (usize::from(width), usize::from(height), usize::from(count));
    let size = width
        .checked_mul(height)
        .and_then(|tile| tile.checked_mul(count))
        .ok_or(DecodeError::Unrecognized)?;
    let colors = body.get(36..40).ok_or(DecodeError::Unrecognized)?;
    let pixels = body.get(40..40 + size).ok_or(DecodeError::Unrecognized)?;
    if size == 0 {
        return Err(DecodeError::Unrecognized);
    }
    Ok(TileSet {
        width,
        height,
        count,
        colors: [colors[0], colors[1], colors[2], colors[3]],
        pixels,
    })
}

pub(super) fn decode_gbr(data: &[u8]) -> Result<Image, DecodeError> {
    let set = parse_gbr(data)?;
    let columns = (SHEET_WIDTH / set.width).clamp(1, set.count);
    let rows = set.count.div_ceil(columns);
    let (width, height) = (columns * set.width, rows * set.height);
    check_size(width, height)?;
    let colors = (0..width * height).map(|i| {
        let (x, y) = (i % width, i / width);
        let tile = y / set.height * columns + x / set.width;
        set.shade(tile, x % set.width, y % set.height)
    });
    Ok(Image::from_colors(width as u32, height as u32, colors))
}

pub(super) fn decode_gbm(data: &[u8], companions: &dyn Companions) -> Result<Image, DecodeError> {
    if !data.starts_with(GBM_MAGIC) {
        return Err(DecodeError::Unrecognized);
    }
    let objects = objects(data, 20)?;
    let map = objects
        .iter()
        .find(|o| o.kind == MAP && o.master == 0)
        .ok_or(DecodeError::Unrecognized)?
        .body;
    let (Some(columns), Some(rows)) = (le32(map, 128), le32(map, 132)) else {
        return Err(DecodeError::Unrecognized);
    };
    let (columns, rows) = (columns as usize, rows as usize);
    let cells = columns.checked_mul(rows).ok_or(DecodeError::Unrecognized)?;
    let records = objects
        .iter()
        .find(|o| o.kind == MAP_TILES && o.master != 0)
        .ok_or(DecodeError::Unrecognized)?
        .body;
    let records = cells
        .checked_mul(3)
        .and_then(|len| records.get(..len))
        .filter(|_| cells > 0)
        .ok_or(DecodeError::Unrecognized)?;

    let tile_file = map.get(140..396).ok_or(DecodeError::Unrecognized)?;
    let tile_file = tile_file.split(|&b| b == 0).next().unwrap_or_default();
    let gbr = tile_set_file(tile_file, companions).ok_or(DecodeError::Unrecognized)?;
    let set = parse_gbr(&gbr)?;

    let width = columns
        .checked_mul(set.width)
        .ok_or(DecodeError::Unrecognized)?;
    let height = rows
        .checked_mul(set.height)
        .ok_or(DecodeError::Unrecognized)?;
    check_size(width, height)?;
    let colors = (0..width * height).map(|i| {
        let (x, y) = (i % width, i / width);
        let cell = y / set.height * columns + x / set.width;
        let (mut tx, mut ty) = (x % set.width, y % set.height);
        let record = &records[cell * 3..cell * 3 + 3];
        if record[0] & 0x40 != 0 {
            tx = set.width - 1 - tx;
        }
        if record[0] & 0x80 != 0 {
            ty = set.height - 1 - ty;
        }
        let tile = usize::from(record[1] & 3) << 8 | usize::from(record[2]);
        set.shade(tile, tx, ty)
    });
    Ok(Image::from_colors(width as u32, height as u32, colors))
}

/// The GBR a map names (its path's last component), else the GBR next to
/// the map.
fn tile_set_file(path: &[u8], companions: &dyn Companions) -> Option<Vec<u8>> {
    let name = path.rsplit(|&b| b == b'/' || b == b'\\').next()?;
    core::str::from_utf8(name)
        .ok()
        .and_then(|name| companions.get_named(name))
        .or_else(|| companions.get("gbr"))
}
