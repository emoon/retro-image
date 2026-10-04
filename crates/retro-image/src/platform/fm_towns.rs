//! FM Towns.
//!
//! Pi pictures saved with the `TOWN` model are decoded by `nec_pc::pi`, which lists
//! its sources. PIC pictures saved on the FM Towns are decoded by `sharp_x68000::pic`, which
//! lists its sources; ICN and HEL are in the submodules, which list theirs.
//! Platform survey: `docs/research/msx-japanese.md`, gaps in
//! `docs/research/gaps-pc-japan.md`.

mod hel;
mod icn;

use super::nec_pc::{Machine, pi};
use super::sharp_x68000::pic;
use crate::Format;

pub(super) static FORMATS: &[Format] = &[
    Format::new("FM Towns", "PIC", &["pic"], |d| {
        pic::decode_pic(d, Machine::FmTowns)
    })
    .signature(),
    Format::new("FM Towns", "Pi", &["pi"], |d| {
        pi::decode_pi(d, Machine::FmTowns)
    })
    .signature(),
    Format::new("FM Towns", "Icons", &["icn"], icn::decode_icn).signature(),
    Format::new("FM Towns", "Animation", &["hel"], hel::decode_hel).signature(),
];
