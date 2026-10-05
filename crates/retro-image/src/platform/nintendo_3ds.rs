//! Nintendo 3DS: the icon of an SMDH file (`.smdh`), of a homebrew
//! executable (`.3dsx`) and of an installable archive (`.cia`).
//!
//! Sources (details per format in each submodule): the platform survey is
//! `docs/research/gaps-nintendo.md`.
//!
//! RECOIL does not decode any of these formats, so there is no oracle run;
//! the renders are recorded in `tests/divergences/nintendo-rom-icons.tsv`.

mod cia;
mod smdh;
mod threedsx;

use crate::Format;

pub(super) static FORMATS: &[Format] = &[
    Format::new("Nintendo 3DS", "SMDH icon", &["smdh"], smdh::decode).signature(),
    Format::new("Nintendo 3DS", "3DSX icon", &["3dsx"], threedsx::decode).signature(),
    Format::new("Nintendo 3DS", "CIA icon", &["cia"], cia::decode),
];
