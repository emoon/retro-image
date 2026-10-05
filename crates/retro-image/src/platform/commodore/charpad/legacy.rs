//! CharPad `.ctm` versions 4 and 5, which have a fixed header followed by
//! unmarked sections.
//!
//! Sources:
//! - Version 4: the CTM V4 description,
//!   <https://github.com/martinpiper/C64Public/blob/master/ExternalTools/CharPad/Docs/CharPad%20-%20CTM%20(V4)%20Format.txt>
//!   (documentation only). Its "20 bytes" header is 24 bytes long: that and
//!   the one-byte map entries were checked by adding up the section sizes of
//!   the three version 4 files in the corpus (an expanded 3x3 project and
//!   two 5x5 and 1x1 ones), which match their lengths exactly.
//! - Version 5: header and section order reverse engineered from the
//!   `.ctm` files in RECOIL's sample set and checked against `recoil2png`;
//!   the flag bits (tile system, expanded character set, multicolor) and
//!   the coloring methods agree with c64lib's CTM 5 reader (MIT, see
//!   `blocks.rs` for its notice).

use super::{Cells, ColorTable, Colors, Mode, Project, Reader, sizes_in_range};

/// Coloring methods of versions 4 and 5.
const GLOBAL: u8 = 0;
const PER_TILE: u8 = 1;
const PER_CHAR_OR_CELL: u8 = 2;

pub(super) fn version4(data: &[u8]) -> Option<Project<'_>> {
    let mut r = Reader::new(data, 4);
    let [background, multi1, multi2, ram_color] = r.take(4)?.try_into().ok()?;
    let colouring = r.byte()?;
    let multicolor = match r.byte()? {
        0 => false,
        1 => true,
        _ => return None,
    };
    let chars = r.word()? + 1;
    let tile_count = usize::from(r.byte()?) + 1;
    let tile_width = usize::from(r.byte()?);
    let tile_height = usize::from(r.byte()?);
    let (map_width, map_height) = (r.word()?, r.word()?);
    let expanded = match r.byte()? {
        0 => false,
        1 => true,
        _ => return None,
    };
    r.take(4)?; // reserved
    if !sizes_in_range(tile_width, tile_height, map_width, map_height) {
        return None;
    }
    let tile_cells = tile_width * tile_height;
    let char_data = r.take(chars * 8)?;
    r.take(chars)?; // character attributes: only used when showing the character set
    let cells = if expanded {
        Cells::Identity
    } else {
        Cells::Words(r.take(tile_count * tile_cells * 2)?)
    };
    // Cell attributes exist in every coloring method.
    let cell_attributes = r.take(tile_count * tile_cells)?;
    let colors = match colouring {
        GLOBAL => Colors::Global(ram_color),
        PER_TILE => Colors::PerTile(ColorTable::bytes(r.take(tile_count)?)),
        PER_CHAR_OR_CELL => Colors::PerCell(ColorTable::bytes(cell_attributes)),
        _ => return None,
    };
    let map = r.take(map_width * map_height)?;
    r.at_end().then_some(Project {
        mode: Mode::text(multicolor),
        background,
        multi1,
        multi2,
        chars: char_data,
        colors,
        tile_width,
        tile_height,
        tile_count,
        cells,
        map_width,
        map_height,
        map,
        wide_map: false,
    })
}

pub(super) fn version5(data: &[u8]) -> Option<Project<'_>> {
    const TILE_SYSTEM: u8 = 1;
    const EXPANDED: u8 = 2;
    const MULTICOLOR: u8 = 4;
    let mut r = Reader::new(data, 4);
    let [background, multi1, multi2, char_color] = r.take(4)?.try_into().ok()?;
    let colouring = r.byte()?;
    let flags = r.byte()?;
    let chars = r.word()? + 1;
    let header_tiles = r.word()? + 1;
    let (header_tile_width, header_tile_height) = (usize::from(r.byte()?), usize::from(r.byte()?));
    let (map_width, map_height) = (r.word()?, r.word()?);
    if flags & !(TILE_SYSTEM | EXPANDED | MULTICOLOR) != 0
        || !sizes_in_range(header_tile_width, header_tile_height, map_width, map_height)
    {
        return None;
    }
    let char_data = r.take(chars * 8)?;
    let attributes = r.take(chars)?;
    // Without the tile system the map names characters: single-cell tiles.
    let tiled = flags & TILE_SYSTEM != 0;
    let (tile_count, tile_width, tile_height) = if tiled {
        (header_tiles, header_tile_width, header_tile_height)
    } else {
        (chars, 1, 1)
    };
    let cells = if !tiled || flags & EXPANDED != 0 {
        Cells::Identity
    } else {
        Cells::Words(r.take(tile_count * tile_width * tile_height * 2)?)
    };
    let colors = match colouring {
        GLOBAL => Colors::Global(char_color),
        PER_TILE if tiled => Colors::PerTile(ColorTable::bytes(r.take(tile_count)?)),
        PER_CHAR_OR_CELL => Colors::PerChar(ColorTable::bytes(attributes)),
        _ => return None,
    };
    let map = r.take(map_width * map_height * 2)?;
    r.at_end().then_some(Project {
        mode: Mode::text(flags & MULTICOLOR != 0),
        background,
        multi1,
        multi2,
        chars: char_data,
        colors,
        tile_width,
        tile_height,
        tile_count,
        cells,
        map_width,
        map_height,
        map,
        wide_map: true,
    })
}
