//! Decoders, one module per platform family. Each module exports its
//! formats as `FORMATS`; a module may grow into a directory of submodules.
//!
//! No external format knowledge: the list of platform modules. Each module
//! cites its own sources; the platform surveys are in `docs/formats/`.

mod amiga;
mod amstrad_cpc;
mod apple;
mod atari8;
mod atari_st;
mod bbc_micro;
mod commodore;
mod electronika;
mod fm_towns;
mod hp48;
mod msx;
mod nec_pc;
mod oric;
mod pc;
mod playstation;
mod psion;
mod sam_coupe;
mod sharp_x68000;
mod tandy1000;
mod thomson;
mod trs80;
mod vector06c;
mod zx_spectrum;

use crate::Format;

pub(crate) static ALL: &[&[Format]] = &[
    amiga::FORMATS,
    amstrad_cpc::FORMATS,
    apple::FORMATS,
    atari8::FORMATS,
    atari_st::FORMATS,
    bbc_micro::FORMATS,
    commodore::FORMATS,
    electronika::FORMATS,
    fm_towns::FORMATS,
    hp48::FORMATS,
    msx::FORMATS,
    nec_pc::FORMATS,
    oric::FORMATS,
    pc::FORMATS,
    playstation::FORMATS,
    psion::FORMATS,
    sam_coupe::FORMATS,
    sharp_x68000::FORMATS,
    tandy1000::FORMATS,
    thomson::FORMATS,
    trs80::FORMATS,
    vector06c::FORMATS,
    zx_spectrum::FORMATS,
];
