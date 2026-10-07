//! Atari ST/STE, TT and Falcon.
//!
//! Each submodule lists the documents its layouts come from; the platform
//! survey is `docs/research/atari-st-tt-falcon.md`. ST IFF files are ILBM
//! FORMs read by the Amiga decoder, see
//! <https://temlib.org/AtariForumWiki/index.php/IFF_file_format>.

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
mod flf;
mod gem_img;
mod grafix;
mod iff;
mod imagic;
mod lz4;
mod mono;
mod mpp;
mod paintshop;
mod paintworks;
mod photochrome;
mod quantumpaint;
mod rasters;
mod seq;
mod signum_imc;
mod simple;
mod spectrum;
mod stos_bank;
mod stos_pp;
mod tiny;
mod tt;
mod uimg;

pub(super) use iff::is_neochrome_master;

use crate::{Companions, DecodeError, Format, Image};

type Decoder = fn(&[u8]) -> Result<Image, DecodeError>;
type CompanionDecoder = fn(&[u8], &dyn Companions) -> Result<Image, DecodeError>;

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

/// An ST format that reads companion files when they are available.
const fn st_with(
    name: &'static str,
    extensions: &'static [&'static str],
    decoder: CompanionDecoder,
) -> Format {
    Format::with_companions("Atari ST", name, extensions, decoder)
}

pub(super) static FORMATS: &[Format] = &[
    st("DEGAS", &["pi1", "pi2", "pi3", "suh"], degas::decode_pi),
    st(
        "DEGAS Elite compressed",
        &["pc1", "pc2", "pc3"],
        degas::decode_pc,
    ),
    st("EZ-Art Professional", &["eza"], degas::decode_eza),
    st_with("NEOchrome", &["neo"], simple::decode_neo),
    // Content detection: an ILBM FORM followed by a `RAST` chunk.
    st("NEOchrome Master", &["neo"], iff::decode_neochrome_master).signature(),
    st("Doodle", &["doo"], simple::decode_doo),
    st(
        "Art Director / GFA Artist / Palette Master",
        &["art"],
        simple::decode_art,
    ),
    st("ColorSTar", &["bil"], simple::decode_bil),
    st(
        "Raw low-resolution screen",
        &["dat"],
        simple::decode_raw_screen,
    ),
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
    st("ZZ_ROUGH", &["rgh"], dali::decode_rgh).signature(),
    st("Synthetic Arts", &["srt"], simple::decode_srt),
    st("Sinbad Slideshow", &["ssb"], simple::decode_ssb),
    st("Cyber Paint Cell", &["cel"], simple::decode_cel),
    st("Cyber Paint Sequence", &["seq"], seq::decode_seq).signature(),
    st("DeskPic", &["gfb"], simple::decode_gfb).signature(),
    st("PaintShop", &["da4"], simple::decode_da4),
    st("PaintShop compressed", &["psc"], paintshop::decode_psc).signature(),
    st("RGB Intermediate", &["rgb"], simple::decode_rgb),
    st(
        "Tiny Stuff",
        &["tny", "tn1", "tn2", "tn3", "tn4", "tn5", "tn6"],
        tiny::decode_tny,
    ),
    st("CrackArt", &["ca1", "ca2", "ca3"], crackart::decode_ca).signature(),
    st("Spectrum 512", &["spu"], spectrum::decode_spu),
    st("Spectrum 512 compressed", &["spc"], spectrum::decode_spc).signature(),
    st("Spectrum 512 smooshed", &["sps"], spectrum::decode_sps).signature(),
    st("Spectrum 512 extended", &["spx"], spectrum::decode_spx).signature(),
    st(
        "GEM Bit Image",
        &["img", "ximg", "timg"],
        gem_img::decode_img,
    )
    .signature(),
    st("Multi Palette Picture", &["mpp"], mpp::decode_mpp).signature(),
    st(
        "ComputerEyes",
        &["ce1", "ce2", "ce3"],
        computereyes::decode_ce,
    )
    .signature(),
    st("Public Painter", &["cmp"], mono::decode_cmp),
    st("Calamus Raster Graphic", &["crg"], mono::decode_crg).signature(),
    st_with("Canvas compressed", &["cpt"], canvas::decode_cpt),
    st("Canvas full", &["ful"], canvas::decode_ful),
    st_with("C.O.L.R. Object Editor", &["mur"], simple::decode_mur),
    st("DUO", &["du1", "duo"], duo::decode_duo),
    st("DUO (medium resolution)", &["du2"], duo::decode_du2),
    st("STAD", &["pac"], mono::decode_pac).signature(),
    st("PhotoChrome", &["pcs"], photochrome::decode_pcs).signature(),
    st("Overscan Interlaced", &["pci"], blend::decode_pci),
    st("HighresMedium", &["hrm"], blend::decode_hrm),
    st("PL4", &["pl4"], blend::decode_pl4).signature(),
    st("QuantumPaint", &["pbx"], quantumpaint::decode_pbx),
    st("MegaPaint", &["bld"], mono::decode_bld),
    st("DEGAS Elite font", &["fnt"], mono::decode_fnt),
    st("GDOS font", &["fnt"], mono::decode_gdos_fnt),
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
    )
    .signature(),
    tt("DEGAS (TT low resolution)", &["pi4"], tt::decode_pi4),
    tt("DEGAS (TT medium resolution)", &["pi5"], tt::decode_pi5),
    falcon("ImageLab", &["b&w", "b_w"], falcon::decode_bw).signature(),
    falcon("DuneGraph", &["dg1"], falcon::decode_dg1).signature(),
    falcon("DuneGraph compressed", &["dc1"], falcon::decode_dc1).signature(),
    falcon(
        "FuckPaint",
        &["pi4", "pi7", "pi9"],
        falcon::decode_fuckpaint,
    ),
    falcon("GodPaint", &["god"], falcon::decode_god),
    falcon("Print-Technik", &["hir"], falcon::decode_hir).signature(),
    falcon("InShape", &["iim"], falcon::decode_iim).signature(),
    falcon("IMG Scan", &["raw", "rwh", "rwl"], falcon::decode_img_scan),
    falcon("Rembrandt", &["tcp"], falcon::decode_tcp).signature(),
    falcon("COKE", &["tg1"], falcon::decode_tg1).signature(),
    falcon("EggPaint / Spooky Sprites", &["trp"], falcon::decode_trp).signature(),
    falcon("IndyPaint", &["tru"], falcon::decode_tru).signature(),
    falcon("Falcon True Color", &["ftc"], falcon::decode_ftc),
    falcon("XGA", &["xga"], falcon::decode_xga),
    falcon("TmS Cranach", &["esm"], falcon_paint::decode_esm).signature(),
    falcon("Funny Paint", &["fun"], falcon_paint::decode_fun).signature(),
    falcon("PixArt", &["pix"], falcon_paint::decode_pix).signature(),
    falcon(
        "Prism Paint / TruePaint",
        &["pnt", "tpi"],
        falcon_paint::decode_pnt,
    )
    .signature(),
    falcon("DelmPaint", &["del"], falcon_paint::decode_del),
    falcon("DelmPaint (640x480)", &["dph"], falcon_paint::decode_dph),
    falcon("RAG-D", &["rag"], falcon_paint::decode_rag).signature(),
    falcon("Music Compile", &["ragc"], falcon_paint::decode_ragc),
    st(
        "UIMG",
        &["bp1", "bp2", "bp4", "c01", "c02", "c04"],
        uimg::decode_uimg,
    )
    .signature(),
    falcon(
        "UIMG",
        &["bp6", "bp8", "c06", "c08", "c16", "c24", "c32"],
        uimg::decode_uimg,
    ),
    st("Pablo Paint", &["pa3", "ppp"], simple::decode_pablo).signature(),
    st(
        "Graphics Processor",
        &["pg1", "pg2", "pg3"],
        simple::decode_graphics_processor,
    ),
    st("Atari Image Manager", &["im"], simple::decode_im),
    st("Picworks", &["cp3"], mono::decode_cp3),
    st("DEGAS Elite icon", &["icn"], mono::decode_icn).signature(),
    tt("DEGAS (TT high resolution)", &["pi6"], tt::decode_pi6),
    falcon("Spooky Sprites RLE", &["tre"], falcon::decode_tre).signature(),
    falcon("ICDRAW icon", &["ibi", "ib3"], falcon::decode_icdraw).signature(),
    st("Fullscreen Construction Kit", &["kid"], duo::decode_kid).signature(),
    st("D-GRAPH", &["p3c"], blend::decode_p3c),
    st(
        "Atari Image Manager (colour)",
        &["col"],
        simple::decode_aim_col,
    ),
    st("ColorSTar object", &["obj"], mono::decode_obj),
    st("Grafix", &["grx"], grafix::decode_grx).signature(),
    st("Imagic", &["ic1", "ic2", "ic3"], imagic::decode_ic).signature(),
    st(
        "Signum! image",
        &["imc", "i01", "i02", "i04", "pac"],
        signum_imc::decode_imc,
    )
    .signature(),
    // STOS Picture Packer: the extension tells the variants apart.
    st("STOS Picture Packer PP1", &["pp1"], stos_pp::decode_pp1),
    st("STOS Picture Packer PP2", &["pp2"], stos_pp::decode_pp2),
    st("STOS Picture Packer PP3", &["pp3"], stos_pp::decode_pp3),
    st("STOS Picture Packer DAJ", &["daj"], stos_pp::decode_daj),
    st("STOS packed screen", &["pac", "sz1"], stos_pp::decode_pac).signature(),
    st("STOS memory bank", &["mbk"], stos_bank::decode).signature(),
    st("Turbo Rascal Syntax Error", &["flf"], flf::decode_flf).signature(),
    // ST IFF files (DeluxePaint ST, Spectrum 512 IFF, ...) are ILBM FORMs,
    // including the VDAT-compressed ones; the Amiga entry owns content
    // detection.
    st("IFF", &["iff"], super::amiga::decode_iff),
];

#[cfg(test)]
mod tests {
    #[test]
    fn signature_formats_are_detected_under_other_extensions() {
        // ImageLab: `B&W256`, width 2, height 1, two gray bytes.
        let picture = b"B&W256\0\x02\0\x01\x00\xff";
        let image = crate::decode("picture.org", picture).unwrap().into_image();
        assert_eq!(image.rgb(), &[0, 0, 0, 0xff, 0xff, 0xff]);
    }

    /// Returns the same bytes for every extension.
    struct Any(alloc::vec::Vec<u8>);

    impl crate::Companions for Any {
        fn get_named(&self, _file_name: &str) -> Option<alloc::borrow::Cow<'_, [u8]>> {
            None
        }
        fn get(&self, _extension: &str) -> Option<alloc::borrow::Cow<'_, [u8]>> {
            Some(self.0.clone().into())
        }
    }

    #[test]
    fn malformed_companion_files_do_not_panic() {
        let neo = alloc::vec![0u8; 128 + 32000];
        // CPT: palette, low resolution, end of runs, then 8000 raw units.
        let mut cpt = alloc::vec![0u8; 34];
        cpt.extend_from_slice(&[0xff, 0xff, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        cpt.resize(cpt.len() + 32000, 0x55);
        let mur = alloc::vec![0u8; 32000];
        let mut seed = 1u32;
        for len in (0..2400).step_by(7) {
            let companion: alloc::vec::Vec<u8> = (0..len)
                .map(|i| {
                    seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12345);
                    if i % 3 == 0 { 0xff } else { (seed >> 16) as u8 }
                })
                .collect();
            let companions = Any(companion);
            for (name, data) in [("x.neo", &neo), ("x.cpt", &cpt), ("x.mur", &mur)] {
                for format in crate::candidates(name).filter(|f| f.uses_companions()) {
                    let _ = format.decode_with(data, &companions);
                }
            }
        }
    }

    #[test]
    fn headerless_formats_are_not_detected_by_content() {
        // A raw 32000-byte Doodle screen is recognised only by its extension.
        let screen = alloc::vec![0u8; 32000];
        assert!(crate::decode("screen.doo", &screen).is_ok());
        assert!(crate::decode("screen.xyz", &screen).is_err());
    }
}
