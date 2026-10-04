//! CharPad `.ctm` versions 6 to 9 (82 is version 8.2): a short header, then
//! blocks that each start with the marker `DA B0+n` (`n` counting the
//! blocks from 0).
//!
//! Sources:
//! - Header fields, block order, colouring methods and screen modes:
//!   c64lib's CharPad reader (`CTM6Processor`, `CTM7Processor`,
//!   `CTM8Processor`, `CTM9Processor` and `BlockBasedCTMProcessor`),
//!   <https://github.com/c64lib/gradle-retro-assembler-plugin/tree/master/processors/charpad>,
//!   MIT licensed; its notice is below.
//! - Which of the two version 8 headers a file has (a prerelease has a
//!   fourth colour byte) is told by the byte after the third colour: the
//!   next block marker starts with `DA`, a colour is below 16.
//! - Checked on the 31 c64lib test projects and 12 Martin Piper projects in
//!   the corpus. The c64lib projects hold the same picture in versions 5 to
//!   8.2; every version renders like the version 5 file, which `recoil2png`
//!   decodes.
//!
//! Only the text modes (hires and multicolour characters) are decoded. The
//! extended background and bitmap modes of versions 7 to 9 are rejected: no
//! sample has them, and the colour byte order of the bitmap modes is not
//! documented. Version 9 is untested against a real file for the same
//! reason.
//!
//! c64lib's notice:
//!
//! ```text
//! MIT License
//!
//! Copyright (c) 2018-2025 c64lib: The Ultimate Commodore 64 Library
//! Copyright (c) 2018-2025 Maciej Małecki
//!
//! Permission is hereby granted, free of charge, to any person obtaining a copy
//! of this software and associated documentation files (the "Software"), to deal
//! in the Software without restriction, including without limitation the rights
//! to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
//! copies of the Software, and to permit persons to whom the Software is
//! furnished to do so, subject to the following conditions:
//!
//! The above copyright notice and this permission notice shall be included in all
//! copies or substantial portions of the Software.
//!
//! THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//! IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//! FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
//! AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//! LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//! OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
//! SOFTWARE.
//! ```

use super::{Cells, ColorTable, Colors, Project, Reader, sizes_in_range};

const GLOBAL: u8 = 0;
const PER_TILE: u8 = 1;
const PER_CHAR: u8 = 2;
const BLOCK_MARKER: u8 = 0xda;
/// Tile names are at most 32 characters.
const MAX_NAME_LEN: usize = 32;

/// The fields every version has, whatever their order in the file.
struct Header {
    multicolor: bool,
    colouring: u8,
    tiled: bool,
    background: u8,
    multi1: u8,
    multi2: u8,
    /// The colour of every character with a global colouring method.
    char_color: u8,
    /// Version 8 and later keep the character materials in a block of
    /// their own and the colours in another; before, one attribute byte
    /// holds both.
    separate_materials: bool,
    /// Bytes per entry of the per-character and per-tile colour blocks, and
    /// the position of the colour in an entry.
    color_stride: usize,
    color_offset: usize,
}

/// Screen mode 0 is hires text, 1 multicolour text; the rest are not decoded.
fn text_mode(mode: u8) -> Option<bool> {
    match mode {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    }
}

fn header6(r: &mut Reader) -> Option<Header> {
    const MULTICOLOR: u8 = 1;
    const TILE_SYSTEM: u8 = 2;
    let [background, multi1, multi2, char_color, colouring, flags] = r.take(6)?.try_into().ok()?;
    (flags & !(MULTICOLOR | TILE_SYSTEM) == 0).then_some(Header {
        multicolor: flags & MULTICOLOR != 0,
        colouring,
        tiled: flags & TILE_SYSTEM != 0,
        background,
        multi1,
        multi2,
        char_color,
        separate_materials: false,
        color_stride: 1,
        color_offset: 0,
    })
}

fn header7(r: &mut Reader) -> Option<Header> {
    let [
        background,
        multi1,
        multi2,
        _,
        char_color,
        colouring,
        mode,
        flags,
    ] = r.take(8)?.try_into().ok()?;
    Some(Header {
        multicolor: text_mode(mode)?,
        colouring,
        tiled: tile_system_flag(flags)?,
        background,
        multi1,
        multi2,
        char_color,
        separate_materials: false,
        color_stride: 1,
        color_offset: 0,
    })
}

/// The only flag of versions 7 to 9 is the tile system.
fn tile_system_flag(flags: u8) -> Option<bool> {
    match flags {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    }
}

/// Version 8 (and 8.2) after the mode, colouring and flags bytes.
fn header8(r: &mut Reader, mode: u8, colouring: u8, flags: u8) -> Option<Header> {
    let [background, multi1, multi2, _, base0, _, _] = r.take(7)?.try_into().ok()?;
    // A prerelease has a fourth colour base, which is the character colour;
    // its colour blocks have four bytes per entry, the last one used.
    let prerelease = r.data.get(r.pos) != Some(&BLOCK_MARKER);
    let char_color = if prerelease { r.byte()? } else { base0 };
    let (color_stride, color_offset) = if prerelease { (4, 3) } else { (1, 0) };
    Some(Header {
        multicolor: text_mode(mode)?,
        colouring,
        tiled: tile_system_flag(flags)?,
        background,
        multi1,
        multi2,
        char_color,
        separate_materials: true,
        color_stride,
        color_offset,
    })
}

/// Version 9: a version 8 header with the flexigrid size and an unused byte
/// before the colours.
fn header9(r: &mut Reader, mode: u8, colouring: u8, flags: u8) -> Option<Header> {
    let [_, _, _, _, _, background, multi1, multi2, _, base0, _, _] =
        r.take(12)?.try_into().ok()?;
    Some(Header {
        multicolor: text_mode(mode)?,
        colouring,
        tiled: tile_system_flag(flags)?,
        background,
        multi1,
        multi2,
        char_color: base0,
        separate_materials: true,
        color_stride: 1,
        color_offset: 0,
    })
}

/// A reader that checks the block markers.
struct Blocks<'a> {
    reader: Reader<'a>,
    next: u8,
}

impl<'a> Blocks<'a> {
    /// Reads the marker of the next block.
    fn marker(&mut self) -> Option<()> {
        let [first, second] = self.reader.take(2)?.try_into().ok()?;
        let ok = first == BLOCK_MARKER && second == 0xb0 + self.next;
        self.next += 1;
        ok.then_some(())
    }

    /// A block of `len` bytes.
    fn block(&mut self, len: usize) -> Option<&'a [u8]> {
        self.marker()?;
        self.reader.take(len)
    }

    /// A block that starts with a 16-bit count minus one.
    fn counted(&mut self) -> Option<usize> {
        self.marker()?;
        Some(self.reader.word()? + 1)
    }
}

pub(super) fn parse(data: &[u8]) -> Option<Project<'_>> {
    let mut r = Reader::new(data, 4);
    let header = match data[3] {
        6 => header6(&mut r)?,
        7 => header7(&mut r)?,
        version @ (8 | 82 | 9) => {
            let [mode, colouring, flags] = r.take(3)?.try_into().ok()?;
            if version == 9 {
                header9(&mut r, mode, colouring, flags)?
            } else {
                header8(&mut r, mode, colouring, flags)?
            }
        }
        _ => return None,
    };
    if header.colouring > PER_CHAR || (header.colouring == PER_TILE && !header.tiled) {
        return None;
    }
    let mut blocks = Blocks { reader: r, next: 0 };

    let table = |data| ColorTable {
        data,
        stride: header.color_stride,
        offset: header.color_offset,
    };

    let chars = blocks.counted()?;
    let char_data = blocks.reader.take(chars * 8)?;
    // Version 6 and 7 keep a colour (low nibble) and material (high nibble)
    // in one attribute byte; later versions have a materials block, then
    // the colours if every character has its own.
    let char_colors = if header.separate_materials {
        blocks.block(chars)?;
        match header.colouring {
            PER_CHAR => Some(table(blocks.block(chars * header.color_stride)?)),
            _ => None,
        }
    } else {
        let attributes = blocks.block(chars)?;
        (header.colouring == PER_CHAR).then(|| table(attributes))
    };

    let (tile_count, tile_width, tile_height, cells, tile_colors) = if header.tiled {
        let tile_count = blocks.counted()?;
        let (tile_width, tile_height) = (
            usize::from(blocks.reader.byte()?),
            usize::from(blocks.reader.byte()?),
        );
        if !sizes_in_range(tile_width, tile_height, 0, 0) {
            return None;
        }
        let cells = blocks
            .reader
            .take(tile_count * tile_width * tile_height * 2)?;
        let tile_colors = match header.colouring {
            PER_TILE => Some(table(blocks.block(tile_count * header.color_stride)?)),
            _ => None,
        };
        blocks.block(tile_count)?; // tags
        blocks.marker()?; // names
        for _ in 0..tile_count {
            let rest = blocks.reader.data.get(blocks.reader.pos..)?;
            let len = rest.iter().position(|&b| b == 0)?;
            if len > MAX_NAME_LEN {
                return None;
            }
            blocks.reader.take(len + 1)?;
        }
        (
            tile_count,
            tile_width,
            tile_height,
            Cells::Words(cells),
            tile_colors,
        )
    } else {
        (chars, 1, 1, Cells::Identity, None)
    };

    blocks.marker()?;
    let (map_width, map_height) = (blocks.reader.word()?, blocks.reader.word()?);
    if !sizes_in_range(1, 1, map_width, map_height) {
        return None;
    }
    let map = blocks.reader.take(map_width * map_height * 2)?;
    if !blocks.reader.at_end() {
        return None;
    }

    let colors = match header.colouring {
        GLOBAL => Colors::Global(header.char_color),
        PER_TILE => Colors::PerTile(tile_colors?),
        _ => Colors::PerChar(char_colors?),
    };
    Some(Project {
        multicolor: header.multicolor,
        background: header.background,
        multi1: header.multi1,
        multi2: header.multi2,
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
