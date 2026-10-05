//! SNK Neo Geo graphics as sheets of tiles: cartridge sprite ROMs (`.c1`
//! with its `.c2`), CD sprite files (`.spr`) and fix layer files (`.fix` on
//! CD, `.s1` on cartridge).
//!
//! Sources:
//! - Neo Geo Development Wiki (<https://wiki.neogeodev.org/>, content
//!   "Public Domain"): "Sprite graphics format", "Fix graphics format" and
//!   "Colors". They give the tile sizes, the block order of a sprite (top
//!   right, bottom right, top left, bottom left), plane 0 and 1 in the odd C
//!   ROM and plane 2 and 3 in the even one, plane order 1, 0, 3, 2 in a CD
//!   file, and for a fix tile four columns of one byte per line, stored right
//!   half first, with the left pixel in the low nibble.
//! - Reverse engineered from freem's "Hello World" tutorial pack
//!   (<https://www.ajworld.net/neogeodev/beginner/media/helloworld_tutorial.zip>:
//!   `hello-c1.c1`, `hello-c2.c2`, `HELLO.SPR`, `hello.fix` and the source
//!   tiles `HELLO.sms`, in Master System format, bit 7 leftmost), in
//!   `corpus/extra/small-consoles/neogeo`:
//!   - The one sprite in `HELLO.SPR` and the C ROMs is the four source tiles
//!     with every byte bit-reversed, so bit 0 is the leftmost pixel of a
//!     sprite row (the wiki says only that rows are stored "backwards").
//!   - Its tiles are the four 32-byte source tiles in the order 2, 3, 0, 1,
//!     that is top right, bottom right, top left, bottom left of a sprite
//!     whose source tiles run down its two columns. The source tiles have
//!     only plane 0 set, so that plane sits at byte 1 of a CD row and at the
//!     first byte of a C ROM word: the order 1, 0, 3, 2 and the split of
//!     planes between the ROMs are confirmed for plane 0 only.
//!   - `hello.fix` draws readable ASCII letters in the half order the wiki
//!     gives (right half first); the other order is garbled.
//!   - Planes 1 to 3 and the quadrant order are checked only against the
//!     wiki, because no sample sets them.
//! - NeoSpriteConv by AJ Kelly/freem (MIT, <https://github.com/freem/NeoSpriteConv>)
//!   confirms the conversion (swap the two 64-byte halves of 128 bytes, bit
//!   reverse every byte); nothing is copied from it.
//!
//! Colors: none of these files holds a palette, so every sheet uses a
//! 16-step gray ramp (`v * 17`) for the 4-bit color numbers. Neo Geo palette
//! words (5 bits per channel plus a dark bit shared as the least significant
//! bit, `D R0 G0 B0 R4..R1 G4..G1 B4..B1`) are therefore not decoded, and the
//! dark bit plays no part. A C ROM shown without its `.c2` has only planes 0
//! and 1, so its four color numbers get four evenly spaced grays.
//!
//! Layout of the output: tiles in order, 32 to a row, at most the first
//! `MAX_SPRITES` sprites or `MAX_FIX_TILES` fix tiles (a CD sprite file can
//! hold 32768 sprites, which would be a picture 16384 pixels tall).
//!
//! Detection: the files are headerless, so each is accepted by extension and
//! size (whole tiles, up to the largest size the hardware addresses), and
//! none has a signature. `.spr` is claimed by several other formats, so this
//! module is registered after all of them. A C ROM's partner is the `.c2`
//! with the same name before the extension; MAME-style pairs (`NNN-c1.c1`
//! and `NNN-c2.c2`) differ in more than the extension, which
//! [`Companions::get`] cannot name, so they decode as a lone `.c1`. Only the
//! first pair is registered: a decoder is not told its own extension, and
//! `.c3` could not be told from `.c1`.

use alloc::vec::Vec;

use crate::image::gray_ramp;
use crate::tiles::TileLayout;
use crate::{BitOrder, Companions, DecodeError, Format, Image};

pub(super) static FORMATS: &[Format] = &[
    Format::with_companions("Neo Geo", "Cartridge sprite ROM", &["c1"], decode_c1),
    Format::new("Neo Geo", "CD sprite file", &["spr"], decode_spr),
    Format::new("Neo Geo", "Fix layer tiles", &["fix", "s1"], decode_fix),
];

/// A sprite: 16 x 16 pixels as four 8 x 8 blocks (top right, bottom right,
/// top left, bottom left). A block row is four bytes, one per bit plane
/// with plane 0 first, and bit 0 is the leftmost pixel.
const SPRITE: TileLayout = TileLayout {
    width: 16,
    height: 16,
    order: BitOrder::LsbFirst,
    ..TileLayout::planar(4, 4)
}
.in_blocks(8, 8, &[(1, 0), (1, 1), (0, 0), (0, 1)]);

/// A fix tile: 8 x 8 pixels as four columns of two pixels, stored right
/// half first, a byte per line with the left pixel in the low nibble.
const FIX: TileLayout =
    TileLayout::packed(4, BitOrder::LsbFirst).in_blocks(2, 8, &[(2, 0), (3, 0), (0, 0), (1, 0)]);

/// Tiles to a row of a sheet.
const PER_ROW: usize = 32;
const MAX_SPRITES: usize = 4096;
const MAX_FIX_TILES: usize = 4096;
/// Largest CD sprite file, and the largest C ROM that a cartridge holds.
const MAX_SPR_LEN: usize = 4 << 20;
const MAX_C_ROM_LEN: usize = 32 << 20;
/// Bytes of a sprite held by one C ROM.
const HALF_SPRITE_LEN: usize = SPRITE.tile_len() / 2;

/// Color numbers of a 4-bit tile.
const COLORS: usize = 16;
/// Color numbers of a sprite that has only planes 0 and 1.
const PLANE_0_1_COLORS: usize = 4;

/// Tiles to a row of the sheet of `tiles`: [`PER_ROW`], or fewer if that is
/// all there are.
fn per_row(layout: &TileLayout, tiles: &[u8]) -> usize {
    (tiles.len() / layout.tile_len()).min(PER_ROW)
}

/// A C ROM, with the planes 2 and 3 of the `.c2` next to it if there is one
/// of the same size.
fn decode_c1(data: &[u8], companions: &dyn Companions) -> Result<Image, DecodeError> {
    if data.is_empty() || !data.len().is_multiple_of(HALF_SPRITE_LEN) || data.len() > MAX_C_ROM_LEN
    {
        return Err(DecodeError::Unrecognized);
    }
    let c2 = companions.get("c2").filter(|c2| c2.len() == data.len());
    let shown = data.len().min(MAX_SPRITES * HALF_SPRITE_LEN);
    // A row of a sprite is a word of each ROM; plane 0 comes first.
    let mut tiles = Vec::with_capacity(shown * 2);
    for (i, word) in data[..shown].as_chunks::<2>().0.iter().enumerate() {
        tiles.extend_from_slice(word);
        tiles.extend_from_slice(c2.as_ref().map_or(&[0, 0], |c2| &c2[i * 2..][..2]));
    }
    let palette = gray_ramp(if c2.is_some() {
        COLORS
    } else {
        PLANE_0_1_COLORS
    });
    SPRITE.sheet(&tiles, per_row(&SPRITE, &tiles), &palette)
}

/// A CD sprite file: the C ROMs' words in one file, plane 1 before plane 0
/// and plane 3 before plane 2.
fn decode_spr(data: &[u8]) -> Result<Image, DecodeError> {
    if data.is_empty() || !data.len().is_multiple_of(SPRITE.tile_len()) || data.len() > MAX_SPR_LEN
    {
        return Err(DecodeError::Unrecognized);
    }
    let shown = data.len().min(MAX_SPRITES * SPRITE.tile_len());
    let tiles: Vec<u8> = data[..shown]
        .as_chunks::<2>()
        .0
        .iter()
        .flat_map(|&[plane_1, plane_0]| [plane_0, plane_1])
        .collect();
    SPRITE.sheet(&tiles, per_row(&SPRITE, &tiles), &gray_ramp(COLORS))
}

/// A fix layer file, `.fix` on CD or `.s1` on cartridge.
fn decode_fix(data: &[u8]) -> Result<Image, DecodeError> {
    if data.is_empty()
        || !data.len().is_multiple_of(FIX.tile_len())
        || data.len() > MAX_FIX_TILES * FIX.tile_len()
    {
        return Err(DecodeError::Unrecognized);
    }
    FIX.sheet(data, per_row(&FIX, data), &gray_ramp(COLORS))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NoCompanions;

    #[test]
    fn spr_is_tried_here_after_every_other_claimant() {
        // Formats recognized by signature are tried after the extension
        // matches, so look at the extension matches only.
        let claimants: Vec<&Format> = crate::candidates("x.spr")
            .filter(|f| f.matches_filename("x.spr"))
            .collect();
        assert!(claimants.len() > 1, "other formats claim .spr too");
        assert_eq!(claimants.last().map(|f| f.platform), Some("Neo Geo"));
    }

    /// The first pixel of row 3 of the sprite in each quadrant is set in the
    /// plane given, so its color number is the plane's bit.
    #[test]
    fn spr_files_swap_plane_pairs_and_c_roms_split_them() {
        // Block 2 (top left): row 3 has pixel 0 (bit 0) set in plane 3.
        let mut spr = [0u8; 128];
        spr[2 * 32 + 3 * 4] = 0x01; // byte 0 of a CD row is plane 1 ...
        spr[2 * 32 + 3 * 4 + 1] = 0x01; // ... byte 1 plane 0 ...
        spr[2 * 32 + 3 * 4 + 2] = 0x01; // ... byte 2 plane 3.
        let image = decode_spr(&spr).unwrap();
        assert_eq!((image.width(), image.height()), (16, 16));
        // Planes 1, 0 and 3 give color number 1 + 2 + 8 = 11, in gray 11 * 17.
        assert_eq!(image.get(0, 3), 11 * 17 * 0x01_0101);
        assert_eq!(image.get(1, 3), 0);
        // The same sprite from a C ROM pair: plane 0, 1 | plane 2, 3.
        let (mut c1, mut c2) = ([0u8; 64], [0u8; 64]);
        c1[2 * 16 + 3 * 2] = 0x01; // plane 0
        c1[2 * 16 + 3 * 2 + 1] = 0x01; // plane 1
        c2[2 * 16 + 3 * 2 + 1] = 0x01; // plane 3
        struct Pair(Vec<u8>);
        impl Companions for Pair {
            fn get(&self, extension: &str) -> Option<Vec<u8>> {
                (extension == "c2").then(|| self.0.clone())
            }
            fn get_named(&self, _: &str) -> Option<Vec<u8>> {
                None
            }
        }
        let pair = decode_c1(&c1, &Pair(c2.to_vec())).unwrap();
        assert_eq!(pair, image);
        // Without the partner only planes 0 and 1 are known, four grays.
        assert_eq!(decode_c1(&c1, &NoCompanions).unwrap().get(0, 3), 0xff_ffff);
    }

    #[test]
    fn sprites_put_their_blocks_in_quadrants() {
        let mut spr = [0u8; 128];
        for block in 0..4 {
            spr[block * 32 + 1] = 0x01; // plane 0, row 0, leftmost pixel
        }
        let image = decode_spr(&spr).unwrap();
        // Top right, bottom right, top left, bottom left.
        for (x, y) in [(8, 0), (8, 8), (0, 0), (0, 8)] {
            assert_eq!(image.get(x, y), 0x11_1111, "block at {x},{y}");
        }
    }

    #[test]
    fn fix_tiles_are_columns_with_the_right_half_first() {
        let mut tile = [0u8; 32];
        tile[0] = 0x21; // right half, first column, line 0: pixels 4 and 5
        tile[8 + 7] = 0x03; // right half, second column, line 7: pixels 6 and 7
        tile[16 + 2] = 0x40; // left half, first column, line 2: pixels 0 and 1
        let image = decode_fix(&tile).unwrap();
        assert_eq!((image.width(), image.height()), (8, 8));
        assert_eq!((image.get(4, 0), image.get(5, 0)), (0x11_1111, 0x22_2222));
        assert_eq!((image.get(6, 7), image.get(7, 7)), (0x33_3333, 0));
        assert_eq!((image.get(0, 2), image.get(1, 2)), (0, 0x44_4444));
    }

    #[test]
    fn sizes_must_be_whole_tiles() {
        assert!(decode_spr(&[0; 127]).is_err());
        assert!(decode_spr(&[]).is_err());
        assert!(decode_fix(&[0; 33]).is_err());
        assert!(decode_c1(&[0; 65], &NoCompanions).is_err());
    }
}
