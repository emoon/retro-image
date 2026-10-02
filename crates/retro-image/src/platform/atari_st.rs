//! Atari ST/STE, TT and Falcon.
//!
//! Each submodule lists the documents its layouts come from; the platform
//! survey is `docs/formats/atari-st-tt-falcon.md`.

mod canvas;
mod common;
mod computereyes;
mod crackart;
mod dali;
mod degas;
mod duo;
mod gem_img;
mod mono;
mod mpp;
mod paintshop;
mod paintworks;
mod simple;
mod spectrum;
mod tiny;

use crate::{DecodeError, Format, Image};

type Decoder = fn(&[u8]) -> Result<Image, DecodeError>;

const fn st(name: &'static str, extensions: &'static [&'static str], decoder: Decoder) -> Format {
    Format::new("Atari ST", name, extensions, decoder)
}

pub(super) static FORMATS: &[Format] = &[
    st("DEGAS", &["pi1", "pi2", "pi3", "suh"], degas::decode_pi),
    st(
        "DEGAS Elite compressed",
        &["pc1", "pc2", "pc3"],
        degas::decode_pc,
    ),
    st("EZ-Art Professional", &["eza"], degas::decode_eza),
    st("NEOchrome", &["neo"], simple::decode_neo),
    st("Doodle", &["doo"], simple::decode_doo),
    st(
        "Art Director / GFA Artist / Palette Master",
        &["art"],
        simple::decode_art,
    ),
    st("ColorSTar", &["bil"], simple::decode_bil),
    st("PaintPro / PlusPaint", &["pic"], simple::decode_pic),
    st("Dali (low resolution)", &["sd0"], simple::decode_sd0),
    st("Dali (medium resolution)", &["sd1"], simple::decode_sd1),
    st("Dali (high resolution)", &["sd2"], simple::decode_sd2),
    st(
        "Dali compressed (low resolution)",
        &["lpk"],
        dali::decode_lpk,
    ),
    st(
        "Dali compressed (medium resolution)",
        &["mpk"],
        dali::decode_mpk,
    ),
    st(
        "Dali compressed (high resolution)",
        &["hpk"],
        dali::decode_hpk,
    ),
    st("ZZ_ROUGH", &["rgh"], dali::decode_rgh),
    st("Synthetic Arts", &["srt"], simple::decode_srt),
    st("Sinbad Slideshow", &["ssb"], simple::decode_ssb),
    st("Cyber Paint Cell", &["cel"], simple::decode_cel),
    st("DeskPic", &["gfb"], simple::decode_gfb),
    st("PaintShop", &["da4"], simple::decode_da4),
    st("PaintShop compressed", &["psc"], paintshop::decode_psc),
    st("RGB Intermediate", &["rgb"], simple::decode_rgb),
    st(
        "Tiny Stuff",
        &["tny", "tn1", "tn2", "tn3", "tn4", "tn5", "tn6"],
        tiny::decode_tny,
    ),
    st("CrackArt", &["ca1", "ca2", "ca3"], crackart::decode_ca),
    st("Spectrum 512", &["spu"], spectrum::decode_spu),
    st("Spectrum 512 compressed", &["spc"], spectrum::decode_spc),
    st("Spectrum 512 smooshed", &["sps"], spectrum::decode_sps),
    st("GEM Bit Image", &["img", "ximg"], gem_img::decode_img),
    st("Multi Palette Picture", &["mpp"], mpp::decode_mpp),
    st(
        "ComputerEyes",
        &["ce1", "ce2", "ce3"],
        computereyes::decode_ce,
    ),
    st("Public Painter", &["cmp"], mono::decode_cmp),
    st("Calamus Raster Graphic", &["crg"], mono::decode_crg),
    st("Canvas compressed", &["cpt"], canvas::decode_cpt),
    st("DUO", &["du1", "duo"], duo::decode_duo),
    st("DUO (medium resolution)", &["du2"], duo::decode_du2),
    st("STAD", &["pac"], mono::decode_pac),
    st("MegaPaint", &["bld"], mono::decode_bld),
    st(
        "Paintworks",
        &[
            "sc0", "sc1", "sc2", "cl0", "cl1", "cl2", "pg0", "pg1", "pg2",
        ],
        paintworks::decode_paintworks,
    ),
];
