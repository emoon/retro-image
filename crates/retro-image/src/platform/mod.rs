//! Decoders, one module per platform family. Each module exports its
//! formats as `FORMATS`; a module may grow into a directory of submodules.
//!
//! No external format knowledge: the list of platform modules. Each module
//! cites its own sources; the platform surveys are in `docs/research/`.

mod amiga;
mod amstrad_cpc;
mod apple;
mod atari8;
mod atari_st;
mod bbc_micro;
mod commodore;
mod electronika;
mod fm_towns;
mod game_boy;
mod hp48;
mod mega_drive;
mod msx;
mod nec_pc;
mod neo_geo;
mod nes;
mod oric;
mod pc;
mod playstation;
mod psion;
mod risc_os;
mod sam_coupe;
mod sharp_x68000;
mod sinclair_ql;
mod tandy1000;
mod textmode;
mod thomson;
mod trs80;
mod vector06c;
mod vmu;
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
    game_boy::FORMATS,
    hp48::FORMATS,
    mega_drive::FORMATS,
    msx::FORMATS,
    nes::FORMATS,
    nec_pc::FORMATS,
    oric::FORMATS,
    pc::FORMATS,
    playstation::FORMATS,
    psion::FORMATS,
    risc_os::FORMATS,
    sam_coupe::FORMATS,
    sharp_x68000::FORMATS,
    sinclair_ql::FORMATS,
    tandy1000::FORMATS,
    textmode::FORMATS,
    thomson::FORMATS,
    trs80::FORMATS,
    vector06c::FORMATS,
    vmu::FORMATS,
    zx_spectrum::FORMATS,
    // Headerless formats on extensions that others claim too go last, so
    // that those get the first try.
    neo_geo::FORMATS,
];
