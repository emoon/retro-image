//! ZX Spectrum family: Spectrum, Profi, ULAplus, ZX Evolution, Next, ZX81 and Timex.
//!
//! Sources (details per format in each submodule):
//! - Screen memory layout (bitmap interleave, attribute byte): ZX Spectrum
//!   hardware documentation, see `docs/formats/sinclair-cpc-bbc-misc.md`.
//! - Palette levels (0x00 / 0xCD / 0xFF) and frame blending: observed from
//!   `recoil2png` output.

mod border;
mod chars;
mod enhanced;
mod multicolor;
mod profi;
mod screen;
mod standard;
mod timex;
mod zx81;
mod zxp;

use crate::{DecodeError, Format, Image};

use super::amstrad_cpc::has_amsdos_header;

/// `.SCR` is shared with the Amstrad CPC: a file with an AMSDOS header is a
/// CPC file even if its size matches a Spectrum screen.
fn scr(data: &[u8], decode: fn(&[u8]) -> Result<Image, DecodeError>) -> Result<Image, DecodeError> {
    if has_amsdos_header(data) {
        return Err(DecodeError::Unrecognized);
    }
    decode(data)
}

pub(super) static FORMATS: &[Format] = &[
    Format::new("ZX Spectrum", "Screen dump", &["scr"], |data| {
        scr(data, standard::decode_scr)
    }),
    Format::new("ZX Spectrum", "Attributes", &["atr"], standard::decode_atr),
    Format::new("ZX Spectrum", "Gigascreen", &["img"], standard::decode_img),
    Format::new(
        "ZX Spectrum",
        "Attributes Gigascreen",
        &["hlr"],
        standard::decode_hlr,
    ),
    Format::new(
        "ZX Spectrum",
        "256x384 interlace",
        &["lce"],
        standard::decode_lce,
    ),
    Format::new("ZX Spectrum", "Stellar", &["stl"], standard::decode_stl),
    Format::new("ZX Spectrum", "Tricolor", &["3"], standard::decode_3),
    Format::new("ZX Spectrum", "Tricolor", &["rgb"], standard::decode_rgb),
    Format::new("ZX Spectrum ULAplus", "ULAplus screen", &["scr"], |data| {
        scr(data, timex::decode_ulaplus)
    }),
    Format::new("Timex 2048", "Hi-color screen", &["scr"], |data| {
        scr(data, timex::decode_hicolor)
    }),
    Format::new("Timex 2048", "Hi-res screen", &["scr"], |data| {
        scr(data, timex::decode_hires)
    }),
    Format::new(
        "Timex 2048",
        "Hi-res gigascreen",
        &["hrg"],
        timex::decode_hrg,
    ),
    Format::new(
        "ZX Spectrum",
        "Multicolor 8x1",
        &["mlt"],
        multicolor::decode_mlt,
    ),
    Format::new(
        "ZX Spectrum",
        "Multicolor 8x1",
        &["mc"],
        multicolor::decode_mc,
    ),
    Format::new(
        "ZX Spectrum",
        "Multicolor 8x2",
        &["ifl"],
        multicolor::decode_ifl,
    ),
    Format::new("ZX Spectrum", "MultiArtist", &["mg1"], |data| {
        multicolor::decode_mgh(data, 1)
    })
    .signature(),
    Format::new("ZX Spectrum", "MultiArtist", &["mg2"], |data| {
        multicolor::decode_mgh(data, 2)
    })
    .signature(),
    Format::new("ZX Spectrum", "MultiArtist", &["mg4"], |data| {
        multicolor::decode_mgh(data, 4)
    })
    .signature(),
    Format::new("ZX Spectrum", "MultiArtist", &["mg8"], |data| {
        multicolor::decode_mgh(data, 8)
    })
    .signature(),
    Format::new("ZX Spectrum", "Border Screen", &["bsc"], border::decode_bsc),
    Format::new(
        "ZX Spectrum",
        "Border Multicolor 8x4",
        &["bmc4"],
        border::decode_bmc4,
    ),
    Format::new(
        "ZX Spectrum",
        "Border Screen by Trefi",
        &["bsp"],
        border::decode_bsp,
    )
    .signature(),
    Format::new(
        "ZX Spectrum",
        "Font",
        &["ch4", "ch6", "ch8"],
        chars::decode_font,
    ),
    Format::new("ZX Spectrum", "CHR$", &["ch$"], chars::decode_chr).signature(),
    Format::new("ZX Spectrum", "Big font", &["chx"], chars::decode_chx).signature(),
    Format::new(
        "ZX Spectrum Next",
        "Layer 2 image",
        &["nxi"],
        enhanced::decode_nxi,
    ),
    Format::new(
        "ZX Evolution",
        "Speccy eXtended Graphics",
        &["sxg"],
        enhanced::decode_sxg,
    )
    .signature(),
    Format::new(
        "ZX Spectrum Profi",
        "Profi screen",
        &["grf"],
        profi::decode_grf,
    )
    .signature(),
    Format::new("ZX81", "Program with screen", &["p"], zx81::decode_p),
    Format::new("ZX Spectrum", "ZX-Paintbrush", &["zxp"], zxp::decode_zxp).signature(),
];
