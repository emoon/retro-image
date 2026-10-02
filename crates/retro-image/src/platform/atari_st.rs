//! Atari ST/STE, TT and Falcon.
//!
//! Each submodule lists the documents its layouts come from; the platform
//! survey is `docs/formats/atari-st-tt-falcon.md`.

mod common;
mod crackart;
mod degas;
mod gem_img;
mod simple;
mod spectrum;
mod tiny;

use crate::Format;

const ST: &str = "Atari ST";

pub(super) static FORMATS: &[Format] = &[
    Format::new(ST, "DEGAS", &["pi1", "pi2", "pi3", "suh"], degas::decode_pi),
    Format::new(
        ST,
        "DEGAS Elite compressed",
        &["pc1", "pc2", "pc3"],
        degas::decode_pc,
    ),
    Format::new(ST, "NEOchrome", &["neo"], simple::decode_neo),
    Format::new(ST, "Doodle", &["doo"], simple::decode_doo),
    Format::new(
        ST,
        "Tiny Stuff",
        &["tny", "tn1", "tn2", "tn3", "tn4", "tn5", "tn6"],
        tiny::decode_tny,
    ),
    Format::new(ST, "CrackArt", &["ca1", "ca2", "ca3"], crackart::decode_ca),
    Format::new(ST, "Spectrum 512", &["spu"], spectrum::decode_spu),
    Format::new(
        ST,
        "Spectrum 512 compressed",
        &["spc"],
        spectrum::decode_spc,
    ),
    Format::new(ST, "Spectrum 512 smooshed", &["sps"], spectrum::decode_sps),
    Format::new(ST, "GEM Bit Image", &["img", "ximg"], gem_img::decode_img),
];
