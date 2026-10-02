//! Amstrad CPC: FutureOS wallpapers and SymbOS graphics.
//!
//! Sources are listed in each submodule.

mod amsdos;
mod hgb;
mod sgx;

use crate::Format;

pub(super) use amsdos::has_amsdos_header;

pub(super) static FORMATS: &[Format] = &[
    Format::new(
        "Amstrad CPC",
        "FutureOS wallpaper",
        &["hgb"],
        hgb::decode_hgb,
    ),
    Format::new("Amstrad CPC", "SymbOS graphic", &["sgx"], sgx::decode_sgx),
];
