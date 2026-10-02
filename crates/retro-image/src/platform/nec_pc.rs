//! NEC PC-80, PC-88, PC-88 VA and PC-98.
//!
//! Also home of the Japanese cross-platform formats (Maki-chan, Pi), which the MSX
//! and Sharp X68000 modules reuse. Sources are listed per submodule; the
//! platform survey is `docs/research/msx-japanese.md`.

pub(super) mod maki;
pub(super) mod pi;
mod precision;

use crate::Format;

/// Computer a Japanese cross-platform picture (MAG, MKI, Pi, PIC) was saved
/// on; each platform module registers the formats for its own machines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Machine {
    Msx,
    Pc80,
    Pc88,
    Pc88Va,
    Pc98,
    X68000,
    FmTowns,
}

pub(super) static FORMATS: &[Format] = &[
    Format::new("NEC PC-80", "Maki-chan Graphics", &["mag"], |d| {
        maki::decode_mag(d, Machine::Pc80)
    })
    .signature(),
    Format::new("NEC PC-88", "Maki-chan Graphics", &["mag"], |d| {
        maki::decode_mag(d, Machine::Pc88)
    })
    .signature(),
    Format::new("NEC PC-88 VA", "Maki-chan Graphics", &["mag"], |d| {
        maki::decode_mag(d, Machine::Pc88Va)
    })
    .signature(),
    Format::new("NEC PC-98", "Maki-chan Graphics", &["mag"], |d| {
        maki::decode_mag(d, Machine::Pc98)
    })
    .signature(),
    Format::new("NEC PC-98", "Maki-chan Graphics (MAKI01)", &["mki"], |d| {
        maki::decode_mki(d, Machine::Pc98)
    })
    .signature(),
    Format::new("NEC PC-88", "Pi", &["pi"], |d| {
        pi::decode_pi(d, Machine::Pc88)
    })
    .signature(),
    Format::new("NEC PC-88 VA", "Pi", &["pi"], |d| {
        pi::decode_pi(d, Machine::Pc88Va)
    })
    .signature(),
    Format::new("NEC PC-98", "Pi", &["pi"], |d| {
        pi::decode_pi(d, Machine::Pc98)
    })
    .signature(),
    Format::new("NEC PC-88 VA", "PIC", &["pic"], |d| {
        super::sharp_x68000::pic::decode_pic(d, Machine::Pc88Va)
    })
    .signature(),
];
