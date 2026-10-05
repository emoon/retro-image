//! Decoders, one module per platform family. Each module exports its
//! formats as `FORMATS`; a module may grow into a directory of submodules.
//!
//! No external format knowledge: the list of platform modules. Each module
//! cites its own sources; the platform surveys are in `docs/research/`.

mod amateur_radio;
mod amiga;
mod amstrad_cpc;
mod amstrad_pcw;
mod apple;
mod atari8;
mod atari_st;
mod bbc_micro;
mod cdi;
mod commodore;
mod dec_vt340;
mod electronika;
mod fm_towns;
mod game_boy;
mod hp48;
mod kiss;
mod mega_drive;
mod msx;
mod nec_pc;
mod neo_geo;
mod nes;
mod oric;
mod palm_os;
mod pc;
mod pico8;
mod playstation;
mod ps1_memory_card;
mod psion;
mod risc_os;
mod sam_coupe;
mod sharp_x68000;
mod sinclair_ql;
mod tandy1000;
mod textmode;
mod thomson;
mod tic80;
mod trs80;
mod unix;
mod vector06c;
mod vmu;
mod zx_spectrum;

use crate::Format;

pub(crate) static ALL: &[&[Format]] = &[
    amateur_radio::FORMATS,
    amiga::FORMATS,
    amstrad_cpc::FORMATS,
    apple::FORMATS,
    atari8::FORMATS,
    atari_st::FORMATS,
    bbc_micro::FORMATS,
    cdi::FORMATS,
    commodore::FORMATS,
    dec_vt340::FORMATS,
    electronika::FORMATS,
    fm_towns::FORMATS,
    game_boy::FORMATS,
    hp48::FORMATS,
    kiss::FORMATS,
    mega_drive::FORMATS,
    msx::FORMATS,
    nes::FORMATS,
    nec_pc::FORMATS,
    oric::FORMATS,
    palm_os::FORMATS,
    pc::FORMATS,
    pico8::FORMATS,
    playstation::FORMATS,
    ps1_memory_card::FORMATS,
    psion::FORMATS,
    risc_os::FORMATS,
    sam_coupe::FORMATS,
    sharp_x68000::FORMATS,
    sinclair_ql::FORMATS,
    tandy1000::FORMATS,
    textmode::FORMATS,
    thomson::FORMATS,
    tic80::FORMATS,
    trs80::FORMATS,
    unix::FORMATS,
    vector06c::FORMATS,
    vmu::FORMATS,
    zx_spectrum::FORMATS,
    // Last: formats recognized by size alone or by extension alone (PCW
    // `.cut`, `.grf` and `.spc`, the KiSS conventional `.cel`, Neo Geo
    // `.spr`), so every other claimant of those extensions goes first.
    amstrad_pcw::FORMATS,
    kiss::BY_SIZE,
    neo_geo::FORMATS,
];
