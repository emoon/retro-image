//! NEC PC-80, PC-88, PC-88 VA and PC-98.
//!
//! Also home of the Japanese cross-platform formats (Maki-chan, Pi), which the MSX
//! and Sharp X68000 modules reuse. Sources are listed per submodule; the
//! platform survey is `docs/research/msx-japanese.md`.

mod artmaster88;
mod arv;
mod davinci;
mod ebd;
mod kt4;
pub(super) mod maki;
mod nl3;
mod pc88_planes;
pub(super) mod pi;
mod precision;
mod zim;

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
    // Wave 5: Japanese
    Format::new("NEC PC-88", "DaVinci", &["img"], davinci::decode_davinci),
    Format::new(
        "NEC PC-88",
        "ArtMaster88",
        &["img"],
        artmaster88::decode_artmaster88,
    )
    .signature(),
    Format::new("NEC PC-98", "EBD", &["ebd"], ebd::decode_ebd),
    Format::new("NEC PC-98", "Z's Staff Kid98", &["zim"], zim::decode_zim).signature(),
    // Wave 5b: Japanese
    Format::new("NEC PC-98", "ARTV", &["arv"], arv::decode_arv).signature(),
    Format::new("NEC PC-88 VA", "Kitty", &["kt4"], kt4::decode_kt4),
    // other
    Format::new(
        "NEC PC-98",
        "Mapletown Network NL3",
        &["nl3"],
        nl3::decode_nl3,
    ),
];
