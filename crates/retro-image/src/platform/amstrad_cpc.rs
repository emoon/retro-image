//! Amstrad CPC: OCP Art Studio screens and windows, Mode 5 and Perfect Pix
//! pictures, FutureOS wallpapers and SymbOS graphics.
//!
//! Sources are listed in each submodule.

mod amsdos;
mod hardware;
mod hgb;
mod mode5;
mod ocp;
mod perfect_pix;
mod sgx;

use crate::Format;

pub(super) use amsdos::has_amsdos_header;

pub(super) static FORMATS: &[Format] = &[
    Format::with_companions(
        "Amstrad CPC",
        "Advanced OCP Art Studio screen",
        &["scr"],
        ocp::decode_scr,
    ),
    // The same screens recognised by their AMSDOS header, whatever the name.
    Format::with_companions(
        "Amstrad CPC",
        "Advanced OCP Art Studio screen",
        &[],
        ocp::decode_amsdos_scr,
    )
    .signature(),
    Format::with_companions(
        "Amstrad CPC",
        "Advanced OCP Art Studio window",
        &["win"],
        ocp::decode_win,
    ),
    Format::with_companions("Amstrad CPC", "Mode 5", &["cm5"], mode5::decode_cm5),
    Format::with_companions(
        "Amstrad CPC",
        "Perfect Pix",
        &["pph"],
        perfect_pix::decode_pph,
    ),
    Format::new(
        "Amstrad CPC",
        "FutureOS wallpaper",
        &["hgb"],
        hgb::decode_hgb,
    ),
    Format::new("Amstrad CPC", "SymbOS graphic", &["sgx"], sgx::decode_sgx),
];
