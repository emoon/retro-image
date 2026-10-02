//! Sharp X68000.
//!
//! Maki-chan and Pi pictures saved on the X68000 are decoded by the shared
//! modules `nec_pc::maki` and `nec_pc::pi`, which list their sources.

use super::nec_pc::maki::{self, Machine};
use super::nec_pc::pi;
use crate::Format;

pub(super) static FORMATS: &[Format] = &[
    Format::new("Sharp X68000", "Maki-chan Graphics", &["mag"], |d| {
        maki::decode_mag(d, Machine::X68000)
    }),
    Format::new(
        "Sharp X68000",
        "Maki-chan Graphics (MAKI01)",
        &["mki"],
        |d| maki::decode_mki(d, Machine::X68000),
    ),
    Format::new("Sharp X68000", "Pi", &["pi"], |d| {
        pi::decode_pi(d, Machine::X68000)
    }),
];
