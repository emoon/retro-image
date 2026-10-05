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
//! same name and is rejected without one. Without a `.pal` the palette is not
//! known, so colour numbers 0-3 are drawn as the same black to white ramp in
//! every sub-palette, which also makes the attribute bytes irrelevant.
//!
//! A `.pal` with the same name is read by its size:
//! - 16 or 32 bytes are NES colour numbers, as NES Screen Tool saves them
//!   (reverse engineered from `nes-screens/croom-ingame` in
//!   `corpus/extra/nintendo-rom-icons`, whose game has a screenshot to compare
//!   with). The first 16 bytes are
//!   the four background palettes, four colours each, the first of every
//!   group being the shared background colour; the attribute bytes of the
//!   `.nam` then pick the group of each 2x2 tiles. A 32-byte file is taken to
//!   add the sprite palettes after those, which is a guess from the `.nss`
//!   session layout; no sample has one.
//! - 192 or 1536 bytes are an RGB master palette (<https://www.nesdev.org/wiki/.pal>,
//!   facts only: 64 entries of red, green, blue, and in the longer file the
//!   same with each emphasis setting after them). It replaces the built-in
//!   colours of the ramp. No sample has one.
//!
//! Any other `.pal` size is ignored.
//!
//! Which half of an 8 KiB `.chr` holds the background is a PPU register
//! setting the file doesn't record. Of the samples, nes-pong and
//! flappy-paratroopa-nes use the second half and the others the first, so
//! `background_table` guesses (see there). A 4 KiB `.chr` is used as is.

use super::nametable::Nametable;
use super::{MASTER_PALETTE, PATTERN, PATTERN_TABLE_LEN};
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
    let mut master = MASTER_PALETTE;
    if let Some(file) = companions.get("pal") {
        match file.len() {
            16 | 32 => palette.copy_from_slice(&file[..16]),
            192 | 1536 => {
                for (color, rgb) in master.iter_mut().zip(file.as_chunks::<3>().0) {
                    *color = u32::from(rgb[0]) << 16 | u32::from(rgb[1]) << 8 | u32::from(rgb[2]);
                }
            }
            _ => {}
        }
    }
    let (names, attributes) = data.split_at(NAMES_LEN);
    let pattern = background_table(&pattern, names);
    Nametable {
        width: WIDTH,
        height: HEIGHT,
        pattern,
        names,
        attributes: if attributes.is_empty() {
            &[0; 64]
        } else {
            attributes
        },
        palette: &palette,
        master: &master,
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

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    /// A `.nam` with a `.chr` and an optional `.pal` beside it.
    struct Beside {
        chr: Vec<u8>,
        pal: Option<Vec<u8>>,
    }

    impl Companions for Beside {
        fn get(&self, extension: &str) -> Option<Vec<u8>> {
            match extension {
                "chr" => Some(self.chr.clone()),
                "pal" => self.pal.clone(),
                _ => None,
            }
        }

        fn get_named(&self, _file_name: &str) -> Option<Vec<u8>> {
            None
        }
    }

    /// Tile 1 is solid color 1, tile 2 solid color 3.
    fn pattern() -> Vec<u8> {
        let mut chr = alloc::vec![0; PATTERN_TABLE_LEN];
        chr[16..24].fill(0xff);
        chr[32..48].fill(0xff);
        chr
    }

    #[test]
    fn a_short_pal_gives_the_background_palettes_and_attributes_pick_one() {
        let mut screen = alloc::vec![0; WITH_ATTRIBUTES_LEN];
        screen[0] = 1; // tile (0, 0), attribute group 0
        screen[2] = 1; // tile (2, 0), attribute group 1
        screen[NAMES_LEN] = 0b0000_0100; // 2x2 tiles at (2, 0): palette 1
        let pal: Vec<u8> = [0x0f, 0x01, 0x02, 0x03, 0x0f, 0x11, 0x12, 0x13]
            .into_iter()
            .chain([0x0f; 8])
            .collect();
        let beside = Beside {
            chr: pattern(),
            pal: Some(pal),
        };
        let image = decode(&screen, &beside).unwrap();
        assert_eq!(image.get(0, 0), MASTER_PALETTE[0x01]);
        assert_eq!(image.get(16, 0), MASTER_PALETTE[0x11]);
        assert_eq!(image.get(8, 0), MASTER_PALETTE[0x0f]);
        // The first 16 bytes of a 32-byte file are the same palettes.
        let mut long = beside.pal.clone().unwrap();
        long.extend([0x30; 16]);
        let beside = Beside {
            pal: Some(long),
            ..beside
        };
        assert_eq!(
            decode(&screen, &beside).unwrap().get(16, 0),
            MASTER_PALETTE[0x11]
        );
    }

    #[test]
    fn a_long_pal_replaces_the_master_palette() {
        let mut master = alloc::vec![0; 192];
        master[3 * 0x30..][..3].copy_from_slice(&[1, 2, 3]);
        let mut screen = alloc::vec![0; NAMES_LEN];
        screen[0] = 2; // solid color 3, which the ramp draws as $30
        let beside = |pal: Option<Vec<u8>>| Beside {
            chr: pattern(),
            pal,
        };
        assert_eq!(
            decode(&screen, &beside(Some(master.clone())))
                .unwrap()
                .get(0, 0),
            0x01_0203
        );
        assert_eq!(
            decode(&screen, &beside(None)).unwrap().get(0, 0),
            MASTER_PALETTE[0x30]
        );
        // A file of another size is ignored.
        master.push(0);
        assert_eq!(
            decode(&screen, &beside(Some(master))).unwrap().get(0, 0),
            MASTER_PALETTE[0x30]
        );
    }
}
