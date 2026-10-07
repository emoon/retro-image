//! Game Boy Tile Designer `.gbr` tile sets and Game Boy Map Builder `.gbm`
//! maps (Harry Mulder's GBTD and GBMB).
//!
//! Sources:
//! - Format specifications by Harry Mulder, (c) 1999: `GBRS9906.ZIP` (GBR,
//!   1 June 1999) and `GBMS9910.ZIP` (GBM, October 1999), from
//!   <http://www.devrs.com/gb/hmgd/supp.html>. They are the author's published
//!   format documents with no license stated; used as a prose specification.
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
//! count, a 4-byte color set and one byte per pixel (0-3, remapped through
//! the color set), tile after tile. A GBR is shown as a sheet of tiles, 128
//! pixels wide where the tiles allow. TileSettings (type 3) selects a color
//! set in byte 11 (the document's field order, with a 1-byte split order as
//! the files have it): 0 Pocket, 1 Game Boy, 2 Game Boy Color, 3 Super Game
//! Boy. Only the Game Boy Color set is drawn in color: Palettes (type 0x0D)
//! holds the palettes as `R, G, B, 0` colors and TilePal (0x0E) the palette
//! number of each tile (confirmed by the Petris and word-blocks samples, whose
//! color set 2 files use several palettes, while the color set 0 and 1 files
//! carry unused default palettes). The SGB palettes are ignored.
//!
//! GBM: `GBO1`, then objects with a 20-byte header (`HPJMTL`, `u16` type,
//! `u16` id, `u16` master id, `u32` CRC, `u32` length). Map (type 2) gives
//! the size and the path of the GBR it uses; Map Tile data (type 3) holds
//! one 3-byte big-endian record per cell: tile number in bits 0-9, the Game
//! Boy Color palette in bits 10-14 (0 for the tile's own palette, else
//! palette number plus one), bit 22 flips horizontally and bit 23 vertically.
//! The Petris intro map sets palettes and the horizontal flip (the mirrored
//! bushes line up); no sample sets the vertical flip, which is from the
//! document only. The tiles come from the GBR named by the map, taken from
//! the companion files, so the GBM alone is rejected.

use alloc::vec::Vec;

use super::SHADES;
use crate::bytes::{le16, le32};
use crate::image::check_size;
use crate::{Companions, DecodeError, Image};

const GBR_MAGIC: &[u8] = b"GBO0";
const GBM_MAGIC: &[u8] = b"GBO1";
const GBM_MARKER: &[u8] = b"HPJMTL";
const TILE_DATA: u16 = 2;
const TILE_SETTINGS: u16 = 3;
const PALETTES: u16 = 0x0d;
const TILE_PALETTES: u16 = 0x0e;
/// Offset of the selected color set in the TileSettings body, and its
/// value for Game Boy Color.
const COLOR_SET_AT: usize = 11;
const COLOR_SET_GBC: u8 = 2;
const MAP: u16 = 2;
const MAP_TILES: u16 = 3;
const SHEET_WIDTH: usize = 128;

/// A tile set: `count` tiles of `width` x `height` pixels, one byte per
/// pixel, and the color set that remaps them to shades.
struct TileSet<'a> {
    width: usize,
    height: usize,
    count: usize,
    colors: [u8; 4],
    pixels: &'a [u8],
    /// Present when the tile set was saved in Game Boy Color mode.
    gbc: Option<GbcColors>,
}

/// The Game Boy Color palettes of a tile set and the palette each tile uses.
struct GbcColors {
    palettes: Vec<[u32; 4]>,
    tile_palette: Vec<u32>,
}

/// What a map cell shows: a tile, possibly mirrored, and the Game Boy Color
/// palette the map picks for it instead of the tile's own.
struct Cell {
    tile: usize,
    palette: Option<usize>,
    flip_x: bool,
    flip_y: bool,
}

impl Cell {
    /// A cell as drawn in a tile sheet: the tile as it is.
    fn plain(tile: usize) -> Self {
        Self {
            tile,
            palette: None,
            flip_x: false,
            flip_y: false,
        }
    }

    /// A 3-byte big-endian GBM map record.
    fn from_record(record: &[u8]) -> Self {
        Self {
            tile: usize::from(record[1] & 3) << 8 | usize::from(record[2]),
            palette: usize::from(record[1] >> 2 & 0x1f).checked_sub(1),
            flip_x: record[0] & 0x40 != 0,
            flip_y: record[0] & 0x80 != 0,
        }
    }
}

impl TileSet<'_> {
    /// The color at (`x`, `y`) of the tile `cell` shows, white for a tile
    /// that isn't there.
    fn color(&self, cell: &Cell, x: usize, y: usize) -> u32 {
        if cell.tile >= self.count {
            return SHADES[0];
        }
        let x = if cell.flip_x { self.width - 1 - x } else { x };
        let y = if cell.flip_y { self.height - 1 - y } else { y };
        let value = usize::from(self.pixels[(cell.tile * self.height + y) * self.width + x] & 3);
        let palette = self.gbc.as_ref().and_then(|gbc| {
            let number = cell
                .palette
                .or_else(|| usize::try_from(*gbc.tile_palette.get(cell.tile)?).ok())?;
            gbc.palettes.get(number)
        });
        match palette {
            Some(palette) => palette[value],
            None => SHADES[usize::from(self.colors[value] & 3)],
        }
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
        let (kind, master, length) = fields.ok_or(DecodeError::Invalid)?;
        let start = at + header;
        let body = data
            .get(
                start
                    ..start
                        .checked_add(length as usize)
                        .ok_or(DecodeError::Invalid)?,
            )
            .ok_or(DecodeError::Invalid)?;
        found.push(Object { kind, master, body });
        at = start + length as usize;
    }
    Ok(found)
}

fn parse_gbr(data: &[u8]) -> Result<TileSet<'_>, DecodeError> {
    if !data.starts_with(GBR_MAGIC) {
        return Err(DecodeError::Invalid);
    }
    let objects = objects(data, 8)?;
    let body = objects
        .iter()
        .find(|o| o.kind == TILE_DATA)
        .ok_or(DecodeError::Invalid)?
        .body;
    let (Some(width), Some(height), Some(count)) = (le16(body, 30), le16(body, 32), le16(body, 34))
    else {
        return Err(DecodeError::Invalid);
    };
    let (width, height, count) = (usize::from(width), usize::from(height), usize::from(count));
    let size = width
        .checked_mul(height)
        .and_then(|tile| tile.checked_mul(count))
        .ok_or(DecodeError::Invalid)?;
    let colors = body.get(36..40).ok_or(DecodeError::Invalid)?;
    let pixels = body.get(40..40 + size).ok_or(DecodeError::Invalid)?;
    if size == 0 {
        return Err(DecodeError::Invalid);
    }
    let gbc = objects
        .iter()
        .find(|o| o.kind == TILE_SETTINGS)
        .filter(|o| o.body.get(COLOR_SET_AT) == Some(&COLOR_SET_GBC))
        .and_then(|_| gbc_colors(&objects));
    Ok(TileSet {
        width,
        height,
        count,
        colors: [colors[0], colors[1], colors[2], colors[3]],
        pixels,
        gbc,
    })
}

/// The Palettes and TilePal objects: `u16` object id, `u16` count, then
/// `count` sets of four `R, G, B, 0` colors, respectively `count` `u32`
/// palette numbers (one per tile). Each is followed by the same for SGB,
/// which is not drawn.
fn gbc_colors(objects: &[Object]) -> Option<GbcColors> {
    let find = |kind| objects.iter().find(|o| o.kind == kind).map(|o| o.body);
    let (palettes, tile_palette) = (find(PALETTES)?, find(TILE_PALETTES)?);
    let palettes = palettes
        .get(4..4 + usize::from(le16(palettes, 2)?) * 16)?
        .as_chunks::<16>()
        .0
        .iter()
        .map(|set| {
            let mut colors = [0; 4];
            for (color, rgb) in colors.iter_mut().zip(set.as_chunks::<4>().0) {
                *color = u32::from(rgb[0]) << 16 | u32::from(rgb[1]) << 8 | u32::from(rgb[2]);
            }
            colors
        })
        .collect();
    let tile_palette = tile_palette
        .get(4..4 + usize::from(le16(tile_palette, 2)?) * 4)?
        .as_chunks::<4>()
        .0
        .iter()
        .map(|n| u32::from_le_bytes(*n))
        .collect();
    Some(GbcColors {
        palettes,
        tile_palette,
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
        set.color(&Cell::plain(tile), x % set.width, y % set.height)
    });
    Image::from_colors(width as u32, height as u32, colors)
}

pub(super) fn decode_gbm(data: &[u8], companions: &dyn Companions) -> Result<Image, DecodeError> {
    if !data.starts_with(GBM_MAGIC) {
        return Err(DecodeError::Invalid);
    }
    let objects = objects(data, 20)?;
    let map = objects
        .iter()
        .find(|o| o.kind == MAP && o.master == 0)
        .ok_or(DecodeError::Invalid)?
        .body;
    let (Some(columns), Some(rows)) = (le32(map, 128), le32(map, 132)) else {
        return Err(DecodeError::Invalid);
    };
    let (columns, rows) = (columns as usize, rows as usize);
    let cells = columns.checked_mul(rows).ok_or(DecodeError::Invalid)?;
    let records = objects
        .iter()
        .find(|o| o.kind == MAP_TILES && o.master != 0)
        .ok_or(DecodeError::Invalid)?
        .body;
    let records = cells
        .checked_mul(3)
        .and_then(|len| records.get(..len))
        .filter(|_| cells > 0)
        .ok_or(DecodeError::Invalid)?;

    let tile_file = map.get(140..396).ok_or(DecodeError::Invalid)?;
    let tile_file = tile_file.split(|&b| b == 0).next().unwrap_or_default();
    let gbr = tile_set_file(tile_file, companions).ok_or(DecodeError::Invalid)?;
    let set = parse_gbr(&gbr)?;

    let width = columns.checked_mul(set.width).ok_or(DecodeError::Invalid)?;
    let height = rows.checked_mul(set.height).ok_or(DecodeError::Invalid)?;
    check_size(width, height)?;
    let colors = (0..width * height).map(|i| {
        let (x, y) = (i % width, i / width);
        let cell = y / set.height * columns + x / set.width;
        let cell = Cell::from_record(&records[cell * 3..cell * 3 + 3]);
        set.color(&cell, x % set.width, y % set.height)
    });
    Image::from_colors(width as u32, height as u32, colors)
}

/// The GBR a map names (its path's last component), else the GBR next to
/// the map.
fn tile_set_file(path: &[u8], companions: &dyn Companions) -> Option<Vec<u8>> {
    let name = path.rsplit(|&b| b == b'/' || b == b'\\').next()?;
    core::str::from_utf8(name)
        .ok()
        .and_then(|name| companions.get_named(name))
        .or_else(|| companions.get("gbr"))
        .map(alloc::borrow::Cow::into_owned)
}
