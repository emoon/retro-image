//! Game Boy: Game Boy Camera saves, Game Boy Tile Designer / Map Builder
//! files and Game Boy Printer packet captures.
//!
//! Sources (details per format in each submodule):
//! - Hardware tile encoding and the 128x112 Game Boy Camera picture: Pan Docs,
//!   <https://gbdev.io/pandocs/Gameboy_Camera.html> (CC0).
//! - The platform surveys are `docs/research/next-zx-misc.md` and
//!   `docs/research/gaps-nintendo.md`.
//! - Palette: the four shades of a Game Boy colour number, lightest first,
//!   as grey levels. The real LCD tints are greenish and differ per model, so
//!   a neutral ramp is used (our choice, not taken from any program).
//!
//! RECOIL does not decode any of these formats, so there is no oracle run;
//! the renders are recorded in `tests/divergences/gameboy-nes.tsv` and
//! `tests/divergences/nintendo-rom-icons.tsv`.

mod camera;
mod gbtd;
mod printer;

use crate::Format;
use crate::tiles::TileLayout;

/// Colour numbers 0-3 as `0xRRGGBB`, lightest first (the Game Boy's default
/// background palette maps colour 0 to white).
const SHADES: [u32; 4] = [0xff_ffff, 0xaa_aaaa, 0x55_5555, 0x00_0000];

/// A hardware tile: 16 bytes, two bytes per row (low bit plane, then high
/// plane), bit 7 is the leftmost pixel.
const TILE: TileLayout = TileLayout::planar(2, 2);

pub(super) static FORMATS: &[Format] = &[
    Format::new(
        "Game Boy",
        "Game Boy Camera save",
        &["sav", "srm"],
        camera::decode,
    ),
    Format::new(
        "Game Boy",
        "Game Boy Tile Designer",
        &["gbr"],
        gbtd::decode_gbr,
    )
    .signature(),
    Format::with_companions(
        "Game Boy",
        "Game Boy Map Builder",
        &["gbm"],
        gbtd::decode_gbm,
    )
    .signature(),
    Format::new("Game Boy", "Game Boy Printer capture", &[], printer::decode).signature(),
];
