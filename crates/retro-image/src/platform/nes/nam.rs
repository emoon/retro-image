//! NES nametable (`.nam`) drawn with the pattern table of its `.chr`
//! companion.
//!
//! Sources: nametable and attribute layout from the nesdev wiki,
//! <https://www.nesdev.org/wiki/PPU_nametables> and
//! <https://www.nesdev.org/wiki/PPU_attribute_tables>; file sizes (960 bytes
//! of tile numbers, or 1024 with the 64 attribute bytes appended) observed
//! from the samples in `corpus/extra/nes-nam` (flappy-paratroopa-nes,
//! nes-pong, openNES-Snake, DwarvesManagerNES; see its `MANIFEST.tsv`).
//!
//! The file holds no graphics and no palette, so it needs the `.chr` with the
//! same name and is rejected without one. The palette is not stored, so colour
//! numbers 0-3 are drawn as the same black to white ramp in every
//! sub-palette, which also makes the attribute bytes irrelevant.
//!
//! Which half of an 8 KiB `.chr` holds the background is a PPU register
//! setting the file doesn't record. Of the samples, nes-pong and
//! flappy-paratroopa-nes use the second half and the others the first, so
//! `background_table` guesses (see there). A 4 KiB `.chr` is used as is.

use super::nametable::Nametable;
use super::{PATTERN, PATTERN_TABLE_LEN};
use crate::{Companions, DecodeError, Image};

const WIDTH: usize = 32;
const HEIGHT: usize = 30;
const NAMES_LEN: usize = WIDTH * HEIGHT;
const WITH_ATTRIBUTES_LEN: usize = NAMES_LEN + 64;
/// Colour numbers: black, dark grey, light grey, white.
const RAMP: [u8; 4] = [0x0f, 0x00, 0x10, 0x30];

pub(super) fn decode(data: &[u8], companions: &dyn Companions) -> Result<Image, DecodeError> {
    if data.len() != NAMES_LEN && data.len() != WITH_ATTRIBUTES_LEN {
        return Err(DecodeError::Unrecognized);
    }
    let pattern = companions.get("chr").ok_or(DecodeError::Unrecognized)?;
    if pattern.len() != PATTERN_TABLE_LEN && pattern.len() != 2 * PATTERN_TABLE_LEN {
        return Err(DecodeError::Unrecognized);
    }
    let mut palette = [0; 16];
    for sub in palette.as_chunks_mut::<4>().0 {
        *sub = RAMP;
    }
    let pattern = background_table(&pattern, &data[..NAMES_LEN]);
    Nametable {
        width: WIDTH,
        height: HEIGHT,
        pattern,
        names: &data[..NAMES_LEN],
        attributes: &[0; 64],
        palette: &palette,
    }
    .draw()
}

/// The pattern table the background is probably drawn from. A background is
/// mostly empty backdrop, so a table whose most used tile is blank is
/// preferred; among those, the one that draws more of the picture. A tie
/// keeps the first table. Fitted to the four sample projects, not a rule of
/// the format.
fn background_table<'a>(pattern: &'a [u8], names: &[u8]) -> &'a [u8] {
    let (first, second) = pattern.split_at(PATTERN_TABLE_LEN);
    let mut uses = [0usize; 256];
    for &tile in names {
        uses[usize::from(tile)] += 1;
    }
    let commonest = (0..256).max_by_key(|&tile| uses[tile]).unwrap_or(0);
    let score = |table: &[u8]| {
        let blank = |tile: usize| {
            let len = PATTERN.tile_len();
            table[tile * len..][..len].iter().all(|&b| b == 0)
        };
        let drawn = (0..256).filter(|&tile| !blank(tile)).map(|tile| uses[tile]);
        (blank(commonest), drawn.sum::<usize>())
    };
    if !second.is_empty() && score(second) > score(first) {
        second
    } else {
        first
    }
}
