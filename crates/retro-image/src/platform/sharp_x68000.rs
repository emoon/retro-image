//! Sharp X68000.
//!
//! Maki-chan and Pi pictures saved on the X68000 are decoded by the shared
//! modules `nec_pc::maki` and `nec_pc::pi`; PIC, whose home is the X68000, is
//! decoded here and shared with the FM Towns, PC-88 VA and MSX modules. Each
//! module lists its sources.

pub(super) mod pic;

use super::nec_pc::pi;
use super::nec_pc::{Machine, maki};
use crate::Format;

pub(super) static FORMATS: &[Format] = &[
    Format::new("Sharp X68000", "Maki-chan Graphics", &["mag"], |d| {
        maki::decode_mag(d, Machine::X68000)
    })
    .signature(),
    Format::new(
        "Sharp X68000",
        "Maki-chan Graphics (MAKI01)",
        &["mki"],
        |d| maki::decode_mki(d, Machine::X68000),
    )
    .signature(),
    Format::new("Sharp X68000", "Pi", &["pi"], |d| {
        pi::decode_pi(d, Machine::X68000)
    })
    .signature(),
    Format::new("Sharp X68000", "PIC", &["pic"], |d| {
        pic::decode_pic(d, Machine::X68000)
    })
    .signature(),
];
