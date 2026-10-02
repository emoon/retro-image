//! Sharp X68000.
//!
//! Maki-chan pictures saved on the X68000 are decoded by the shared module in
//! `nec_pc::maki`, which lists its sources.

use super::nec_pc::maki::{self, Machine};
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
];
