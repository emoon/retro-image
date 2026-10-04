//! Amstrad CPC: OCP Art Studio screens and windows, Mode 5 and Perfect Pix
//! pictures, FutureOS wallpapers, SymbOS graphics and snapshots.
//!
//! Each submodule lists the documents its layouts come from; the platform
//! survey is `docs/research/sinclair-cpc-bbc-misc.md`.

mod amsdos;
mod flf;
mod fnt;
mod hardware;
mod hgb;
mod mode5;
mod ocp;
mod overscan;
mod perfect_pix;
mod sgx;
mod sna;

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
    Format::new(
        "Amstrad CPC",
        "Advanced OCP Art Studio font",
        &["fnt"],
        fnt::decode_fnt,
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
    Format::new(
        "Amstrad CPC",
        "Overscan screen with loader",
        &["scr"],
        overscan::decode_overscan,
    )
    .signature(),
    Format::new(
        "Amstrad CPC",
        "Turbo Rascal Syntax Error",
        &["flf"],
        flf::decode_flf,
    )
    .signature(),
    Format::new("Amstrad CPC", "Snapshot", &["sna"], sna::decode_sna).signature(),
];
