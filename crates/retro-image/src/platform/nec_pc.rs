//! NEC PC-80, PC-88, PC-88 VA and PC-98.
//!
//! Also home of the Japanese cross-platform formats (Maki-chan, Pi), which the MSX
//! and Sharp X68000 modules reuse. Sources are listed per submodule.

pub(super) mod maki;
pub(super) mod pi;
mod precision;

use maki::Machine;

use crate::Format;

pub(super) static FORMATS: &[Format] = &[
    Format::new("NEC PC-80", "Maki-chan Graphics", &["mag"], |d| {
        maki::decode_mag(d, Machine::Pc80)
    }),
    Format::new("NEC PC-88", "Maki-chan Graphics", &["mag"], |d| {
        maki::decode_mag(d, Machine::Pc88)
    }),
    Format::new("NEC PC-88 VA", "Maki-chan Graphics", &["mag"], |d| {
        maki::decode_mag(d, Machine::Pc88Va)
    }),
    Format::new("NEC PC-98", "Maki-chan Graphics", &["mag"], |d| {
        maki::decode_mag(d, Machine::Pc98)
    }),
    Format::new("NEC PC-98", "Maki-chan Graphics (MAKI01)", &["mki"], |d| {
        maki::decode_mki(d, Machine::Pc98)
    }),
    Format::new("NEC PC-88", "Pi", &["pi"], |d| {
        pi::decode_pi(d, Machine::Pc88)
    }),
    Format::new("NEC PC-88 VA", "Pi", &["pi"], |d| {
        pi::decode_pi(d, Machine::Pc88Va)
    }),
    Format::new("NEC PC-98", "Pi", &["pi"], |d| {
        pi::decode_pi(d, Machine::Pc98)
    }),
];
