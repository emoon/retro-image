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
mod dreamcast;
mod electronika;
mod fm_towns;
mod game_boy;
mod gamecube;
mod hp48;
mod msx;
mod nec_pc;
mod nes;
mod oric;
mod pc;
mod playstation;
mod playstation2;
mod psion;
mod psp;
mod risc_os;
mod sam_coupe;
mod sharp_x68000;
mod sinclair_ql;
mod tandy1000;
mod textmode;
mod thomson;
mod threedo;
mod trs80;
mod vector06c;
mod zx_spectrum;

use crate::Format;

pub(crate) static ALL: &[&[Format]] = &[
    // Tag-checked formats claiming extensions that headerless formats share
    // (`.cel`, `.img`) come first, so that those are only tried afterwards.
    threedo::FORMATS,
    amiga::FORMATS,
    amstrad_cpc::FORMATS,
    apple::FORMATS,
    atari8::FORMATS,
    atari_st::FORMATS,
    bbc_micro::FORMATS,
    commodore::FORMATS,
    dreamcast::FORMATS,
    electronika::FORMATS,
    fm_towns::FORMATS,
    game_boy::FORMATS,
    gamecube::FORMATS,
    hp48::FORMATS,
    msx::FORMATS,
    nes::FORMATS,
    nec_pc::FORMATS,
    oric::FORMATS,
    pc::FORMATS,
    playstation::FORMATS,
    playstation2::FORMATS,
    psion::FORMATS,
    psp::FORMATS,
    risc_os::FORMATS,
    sam_coupe::FORMATS,
    sharp_x68000::FORMATS,
    sinclair_ql::FORMATS,
    tandy1000::FORMATS,
    textmode::FORMATS,
    thomson::FORMATS,
    trs80::FORMATS,
    vector06c::FORMATS,
    zx_spectrum::FORMATS,
];
