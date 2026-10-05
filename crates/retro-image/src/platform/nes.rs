//! Nintendo Entertainment System: pattern tables (`.chr`), nametables (`.nam`)
//! and NES Screen Tool sessions (`.nss`).
//!
//! Sources (details per format in each submodule):
//! - Pattern table encoding (16 bytes per 8x8 tile, two bit planes):
//!   <https://www.nesdev.org/wiki/PPU_pattern_tables> and nametable and
//!   attribute layout <https://www.nesdev.org/wiki/PPU_nametables>,
//!   <https://www.nesdev.org/wiki/PPU_attribute_tables> (nesdev wiki, no
//!   licence shown, used for facts only).
//! - Master palette: one of the many published approximations of the 2C02
//!   NTSC output (the nesdev wiki page
//!   <https://www.nesdev.org/wiki/PPU_palettes> lists several). The 64 values
//!   are the widely circulated "Wikipedia / Nestopia-style" table; the PPU
//!   does not define RGB values, so this is a choice, not a fact of the
//!   hardware. Entries `$0D`-`$0F`, `$1D`-`$1F`, `$2E`-`$2F`, `$3E`-`$3F`
//!   are black.
//! - The platform survey is `docs/research/next-zx-misc.md`.
//!
//! RECOIL does not decode any of these formats, so there is no oracle run;
//! the renders are recorded in `tests/divergences/gameboy-nes.tsv`.

mod chr;
mod nam;
mod nametable;
mod nss;
mod rle;
mod rom;

use crate::Format;
use crate::tiles::TileLayout;

/// The 2C02 colour numbers `$00-$3F` as `0xRRGGBB`.
#[rustfmt::skip]
const MASTER_PALETTE: [u32; 64] = [
    0x666666, 0x002a88, 0x1412a7, 0x3b00a4, 0x5c007e, 0x6e0040, 0x6c0600, 0x561d00,
    0x333500, 0x0b4800, 0x005200, 0x004f08, 0x00404d, 0x000000, 0x000000, 0x000000,
    0xadadad, 0x155fd9, 0x4240ff, 0x7527fe, 0xa01acc, 0xb71e7b, 0xb53120, 0x994e00,
    0x6b6d00, 0x388700, 0x0c9300, 0x008f32, 0x007c8d, 0x000000, 0x000000, 0x000000,
    0xfffeff, 0x64b0ff, 0x9290ff, 0xc676ff, 0xf36aff, 0xfe6ecc, 0xfe8170, 0xea9e22,
    0xbcbe00, 0x88d800, 0x5ce430, 0x45e082, 0x48cdde, 0x4f4f4f, 0x000000, 0x000000,
    0xfffeff, 0xc0dfff, 0xd3d2ff, 0xe8c8ff, 0xfbc2ff, 0xfec4ea, 0xfeccc5, 0xf7d8a5,
    0xe4e594, 0xcfef96, 0xbdf4ab, 0xb3f3cc, 0xb5ebf2, 0xb8b8b8, 0x000000, 0x000000,
];

/// A pattern table tile: the first 8 bytes are the low bit plane, the next 8
/// the high plane, bit 7 is the leftmost pixel.
const PATTERN: TileLayout = TileLayout::planar(2, 1);

/// Bytes of one pattern table: 256 tiles.
const PATTERN_TABLE_LEN: usize = 256 * PATTERN.tile_len();

pub(super) static FORMATS: &[Format] = &[
    Format::new("NES", "Pattern table", &["chr"], chr::decode),
    Format::with_companions("NES", "Nametable", &["nam"], nam::decode),
    Format::with_companions("NES", "Nametable (RLE)", &["rle"], rle::decode),
    Format::new("NES", "NES Screen Tool session", &["nss"], nss::decode).signature(),
    Format::new("NES", "ROM CHR tiles", &["nes", "unf", "unif"], rom::decode).signature(),
];
