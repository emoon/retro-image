//! Atari ST/STE, TT and Falcon.
//!
//! Each submodule lists the documents its layouts come from; the platform
//! survey is `docs/formats/atari-st-tt-falcon.md`.

mod blend;
mod canvas;
mod common;
mod computereyes;
mod crackart;
mod dali;
mod degas;
mod duo;
mod falcon;
mod falcon_paint;
mod gem_img;
mod iff;
mod lz4;
mod mono;
mod mpp;
mod pack_ice;
mod paintshop;
mod paintworks;
mod photochrome;
mod quantumpaint;
mod simple;
mod spectrum;
mod tiny;
mod tt;
mod uimg;

use crate::{DecodeError, Format, Image};

type Decoder = fn(&[u8]) -> Result<Image, DecodeError>;

const fn tt(name: &'static str, extensions: &'static [&'static str], decoder: Decoder) -> Format {
    Format::new("Atari TT", name, extensions, decoder)
}

const fn falcon(
    name: &'static str,
    extensions: &'static [&'static str],
    decoder: Decoder,
) -> Format {
    Format::new("Atari Falcon", name, extensions, decoder)
}

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
    st("Spectrum 512 extended", &["spx"], spectrum::decode_spx),
    st(
        "GEM Bit Image",
        &["img", "ximg", "timg"],
        gem_img::decode_img,
    ),
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
    st("PhotoChrome", &["pcs"], photochrome::decode_pcs),
    st("Overscan Interlaced", &["pci"], blend::decode_pci),
    st("HighresMedium", &["hrm"], blend::decode_hrm),
    st("PL4", &["pl4"], blend::decode_pl4),
    st("QuantumPaint", &["pbx"], quantumpaint::decode_pbx),
    st("MegaPaint", &["bld"], mono::decode_bld),
    st("DEGAS Elite font", &["fnt"], mono::decode_fnt),
    st("DEGAS Elite brush", &["bru"], mono::decode_bru),
    st(
        "DEGAS Elite block",
        &["bl1", "bl2", "bl3"],
        iff::decode_block,
    ),
    st(
        "Paintworks",
        &[
            "sc0", "sc1", "sc2", "cl0", "cl1", "cl2", "pg0", "pg1", "pg2",
        ],
        paintworks::decode_paintworks,
    ),
    tt("DEGAS (TT low resolution)", &["pi4"], tt::decode_pi4),
    tt("DEGAS (TT medium resolution)", &["pi5"], tt::decode_pi5),
    falcon("ImageLab", &["b&w", "b_w"], falcon::decode_bw),
    falcon("DuneGraph", &["dg1"], falcon::decode_dg1),
    falcon("DuneGraph compressed", &["dc1"], falcon::decode_dc1),
    falcon(
        "FuckPaint",
        &["pi4", "pi7", "pi9"],
        falcon::decode_fuckpaint,
    ),
    falcon("GodPaint", &["god"], falcon::decode_god),
    falcon("Print-Technik", &["hir"], falcon::decode_hir),
    falcon("InShape", &["iim"], falcon::decode_iim),
    falcon("IMG Scan", &["raw", "rwh", "rwl"], falcon::decode_img_scan),
    falcon("Rembrandt", &["tcp"], falcon::decode_tcp),
    falcon("COKE", &["tg1"], falcon::decode_tg1),
    falcon("EggPaint / Spooky Sprites", &["trp"], falcon::decode_trp),
    falcon("IndyPaint", &["tru"], falcon::decode_tru),
    falcon("Falcon True Color", &["ftc"], falcon::decode_ftc),
    falcon("XGA", &["xga"], falcon::decode_xga),
    falcon("TmS Cranach", &["esm"], falcon_paint::decode_esm),
    falcon("Funny Paint", &["fun"], falcon_paint::decode_fun),
    falcon("PixArt", &["pix"], falcon_paint::decode_pix),
    falcon(
        "Prism Paint / TruePaint",
        &["pnt", "tpi"],
        falcon_paint::decode_pnt,
    ),
    falcon("DelmPaint", &["del"], falcon_paint::decode_del),
    falcon("DelmPaint (640x480)", &["dph"], falcon_paint::decode_dph),
    falcon("RAG-D", &["rag"], falcon_paint::decode_rag),
    falcon("Music Compile", &["ragc"], falcon_paint::decode_ragc),
    st(
        "UIMG",
        &["bp1", "bp2", "bp4", "c01", "c02", "c04"],
        uimg::decode_uimg,
    ),
    falcon(
        "UIMG",
        &["bp6", "bp8", "c06", "c08", "c16", "c24", "c32"],
        uimg::decode_uimg,
    ),
];
