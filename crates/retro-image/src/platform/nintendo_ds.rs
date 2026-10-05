//! Nintendo DS and DSi: the icon inside a ROM (`.nds`, `.dsi`, `.srl`).
//!
//! Sources (details per format in each submodule): the platform survey is
//! `docs/research/gaps-nintendo.md`.
//!
//! RECOIL does not decode any of these formats, so there is no oracle run;
//! the renders are recorded in `tests/divergences/nintendo-rom-icons.tsv`.

mod banner;

use crate::Format;

pub(super) static FORMATS: &[Format] = &[Format::new(
    "Nintendo DS",
    "ROM banner icon",
    &["nds", "dsi", "srl"],
    banner::decode,
)
.signature()];
