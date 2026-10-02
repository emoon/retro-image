//! Atari ST/STE, TT and Falcon.
//!
//! Each submodule lists the documents its layouts come from; the platform
//! survey is `docs/formats/atari-st-tt-falcon.md`.

mod common;
mod crackart;
mod dali;
mod degas;
mod gem_img;
mod mpp;
mod paintshop;
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
        "Art Director / GFA Artist / Palette Master",
        &["art"],
        simple::decode_art,
    ),
    Format::new(ST, "ColorSTar", &["bil"], simple::decode_bil),
    Format::new(ST, "PaintPro / PlusPaint", &["pic"], simple::decode_pic),
    Format::new(ST, "Dali (low resolution)", &["sd0"], simple::decode_sd0),
    Format::new(ST, "Dali (medium resolution)", &["sd1"], simple::decode_sd1),
    Format::new(ST, "Dali (high resolution)", &["sd2"], simple::decode_sd2),
    Format::new(
        ST,
        "Dali compressed (low resolution)",
        &["lpk"],
        dali::decode_lpk,
    ),
    Format::new(
        ST,
        "Dali compressed (medium resolution)",
        &["mpk"],
        dali::decode_mpk,
    ),
    Format::new(
        ST,
        "Dali compressed (high resolution)",
        &["hpk"],
        dali::decode_hpk,
    ),
    Format::new(ST, "ZZ_ROUGH", &["rgh"], dali::decode_rgh),
    Format::new(ST, "Synthetic Arts", &["srt"], simple::decode_srt),
    Format::new(ST, "Sinbad Slideshow", &["ssb"], simple::decode_ssb),
    Format::new(ST, "PaintShop", &["da4"], simple::decode_da4),
    Format::new(ST, "PaintShop compressed", &["psc"], paintshop::decode_psc),
    Format::new(ST, "RGB Intermediate", &["rgb"], simple::decode_rgb),
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
    Format::new(ST, "Multi Palette Picture", &["mpp"], mpp::decode_mpp),
];
