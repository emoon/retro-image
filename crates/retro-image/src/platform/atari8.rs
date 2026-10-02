//! Atari 8-bit, including VBXE, and Atari Portfolio.
//!
//! This file only registers the formats. The documentation survey with
//! every source per format is `docs/research/atari-8bit.md`; each submodule
//! cites the sources of its own formats. The palette is in [`palette`];
//! ANTIC/GTIA bitmap rendering in [`antic`].

mod antic;
mod apac;
mod blazing_paddles;
mod colorview;
mod cpi;
mod envision;
mod font;
mod fwa;
mod ged;
mod graph2font;
mod gtia;
mod hcm;
mod hip;
mod ice;
mod inflate;
mod interlace;
mod interlace2;
mod koala;
mod leo;
mod mad_studio;
mod mcs;
mod misc_screen;
mod packed;
mod palette;
mod pmg;
mod portfolio;
mod rambrandt;
mod rip;
mod rom_font;
mod screen;
mod sfdn;
mod shapes;
mod spred;
mod technicolor;
mod text;
mod text_art;
mod tip;
mod vbxe;
mod xl_paint;

use crate::Format;

const ATARI8: &str = "Atari 8-bit";
const VBXE: &str = "Atari 8-bit VBXE";
const PORTFOLIO: &str = "Atari Portfolio";

pub(super) static FORMATS: &[Format] = &[
    Format::new(
        ATARI8,
        "Mad Studio Graphics 0",
        &["gr0", "asc", "scr", "sge"],
        text::decode_gr0,
    ),
    Format::new(ATARI8, "Mad Studio ANTIC 2", &["an2"], text::decode_an2),
    Format::new(ATARI8, "Mad Studio Graphics 1", &["gr1"], text::decode_gr1),
    Format::new(ATARI8, "Mad Studio Graphics 2", &["gr2"], text::decode_gr2),
    Format::new(ATARI8, "Mad Studio ANTIC 4", &["an4"], text::decode_an4),
    Format::new(ATARI8, "Mad Studio ANTIC 5", &["an5"], text::decode_an5),
    Format::new(ATARI8, "Dir Logo Maker", &["dlm"], text::decode_dlm),
    Format::new(ATARI8, "Graphics 3", &["gr3"], screen::decode_gr3),
    Format::new(ATARI8, "Standard Graphics 3", &["sg3"], screen::decode_sg3),
    Format::new(ATARI8, "Graphics 7", &["gr7"], screen::decode_gr7),
    Format::new(ATARI8, "DrawIt", &["dit"], screen::decode_dit),
    Format::new(
        ATARI8,
        "Movie Maker background",
        &["bkg"],
        screen::decode_bkg,
    ),
    Format::new(ATARI8, "Magic Painter", &["mgp"], screen::decode_mgp),
    Format::new(ATARI8, "Graphics 8", &["gr8"], screen::decode_gr8),
    Format::new(ATARI8, "AtariCAD", &["drg"], screen::decode_drg),
    Format::new(ATARI8, "Mad Designer", &["mbg"], screen::decode_mbg),
    Format::new(ATARI8, "Print Shop", &["psf"], screen::decode_psf),
    Format::new(ATARI8, "Graphics 9", &["gr9"], screen::decode_gr9),
    Format::new(ATARI8, "Graphics 9 (G09)", &["g09"], screen::decode_g09),
    Format::new(ATARI8, "TXE", &["txe"], screen::decode_txe),
    Format::new(ATARI8, "Zoom 4", &["zm4"], screen::decode_zm4),
    Format::new(ATARI8, "Texture Maker0", &["tx0"], screen::decode_tx0),
    Format::new(
        ATARI8,
        "Blazing Paddles window",
        &["wnd"],
        screen::decode_wnd,
    ),
    Format::new(ATARI8, "Vidig Paint", &["rap"], screen::decode_rap),
    Format::new(ATARI8, "APAC 80x96", &["256", "ap2"], apac::decode_planar),
    Format::new(
        ATARI8,
        "APAC 80x96 interleaved",
        &["apa", "apc", "plm"],
        apac::decode_interleaved,
    ),
    Format::new(
        ATARI8,
        "APAC 80x192 interlaced",
        &["ap3", "apv", "dgi", "dgp", "esc", "ilc", "pzm"],
        apac::decode_interlaced,
    ),
    Format::new(ATARI8, "Champions' Interlace", &["cin"], apac::decode_cin),
    Format::new(
        ATARI8,
        "Graphics 9 (SFDN)",
        &["g9s", "sfd"],
        sfdn::decode_g9s,
    ),
    Format::new(ATARI8, "Plama 256 (SFDN)", &["pls"], sfdn::decode_pls),
    Format::new(
        ATARI8,
        "Any Point, Any Color (SFDN)",
        &["aps"],
        sfdn::decode_aps,
    ),
    Format::new(ATARI8, "Apac3 Linker-Viewer", &["app"], sfdn::decode_app),
    Format::new(ATARI8, "APACVIEW (SFDN)", &["ils"], sfdn::decode_ils),
    Format::new(ATARI8, "InterPainter (SFDN)", &["ins"], sfdn::decode_ins),
    Format::new(
        ATARI8,
        "Hard Interlace Picture (SFDN)",
        &["hps"],
        sfdn::decode_hps,
    ),
    Format::new(ATARI8, "Graphics 10", &["g10"], screen::decode_g10),
    Format::new(ATARI8, "Graphics 11", &["g11"], screen::decode_g11),
    Format::with_companions(ATARI8, "Micro Illustrator", &["mic"], screen::decode_mic),
    Format::new(ATARI8, "Sketch-PadDles", &["skp"], screen::decode_skp),
    Format::with_companions(
        ATARI8,
        "Technicolor Dream",
        &["lum"],
        technicolor::decode_lum,
    ),
    Format::new(ATARI8, "Hard Interlace Picture", &["hip"], hip::decode_hip),
    Format::new(
        ATARI8,
        "Taquart Interlace Picture",
        &["tip"],
        tip::decode_tip,
    )
    .signature(),
    Format::new(ATARI8, "VertiZontal Interlacing", &["vzi"], hip::decode_vzi),
    Format::new(ATARI8, "INT95a", &["int"], interlace::decode_int).signature(),
    Format::new(
        ATARI8,
        "InterPainter",
        &["inp", "int"],
        interlace::decode_inp,
    ),
    Format::new(
        ATARI8,
        "HCI interlace",
        &["hci", "hr2"],
        interlace::decode_hci,
    ),
    Format::new(
        ATARI8,
        "Atari Interlace Studio",
        &["ist"],
        interlace::decode_ist,
    ),
    Format::new(
        ATARI8,
        "SAMAR Hires Interlace",
        &["shc"],
        interlace::decode_shc,
    ),
    Format::new(ATARI8, "McPainter", &["mcp"], interlace::decode_mcp),
    Format::new(ATARI8, "Paradox", &["mcpp"], interlace::decode_mcpp),
    Format::new(
        ATARI8,
        "AtariTools-800 graphic",
        &["agp"],
        screen::decode_agp,
    ),
    Format::new(
        ATARI8,
        "Koala MicroIllustrator",
        &["pic"],
        koala::decode_pic,
    ),
    Format::new(ATARI8, "Visualizer", &["pic"], screen::decode_visualizer),
    Format::new(ATARI8, "Magic Painter", &["pic"], screen::decode_mgp_pic),
    Format::new(ATARI8, "8x8 font", &["fnt"], font::decode_fnt),
    Format::new(
        ATARI8,
        "Atari FontMaker dual font",
        &["fn2"],
        font::decode_fn2,
    ),
    Format::new(
        ATARI8,
        "Mad Studio player",
        &["spr"],
        mad_studio::decode_spr,
    ),
    Format::new(
        ATARI8,
        "Mad Studio multi-color player",
        &["mpl"],
        mad_studio::decode_mpl,
    ),
    Format::new(
        ATARI8,
        "Mad Studio missile",
        &["msl"],
        mad_studio::decode_msl,
    ),
    Format::new(
        ATARI8,
        "Mad Studio ANTIC 4 tile",
        &["tl4"],
        mad_studio::decode_tl4,
    ),
    Format::new(ATARI8, "Super-IRG font", &["sif"], font::decode_sif),
    Format::new(ATARI8, "SXS font", &["sxs"], font::decode_sxs),
    Format::new(ATARI8, "OD Font Editor", &["odf"], font::decode_odf),
    Format::new(ATARI8, "The Last Word font", &["f80"], font::decode_f80),
    Format::new(
        ATARI8,
        "AtariTools-800 player",
        &["pla"],
        mad_studio::decode_pla,
    ),
    Format::new(
        ATARI8,
        "AtariTools-800 missile",
        &["mis"],
        mad_studio::decode_mis,
    ),
    Format::new(ATARI8, "Daisy-Dot NLQ font", &["nlq"], font::decode_nlq).signature(),
    Format::new(ATARI8, "AtariTools-800 font", &["acs"], font::decode_acs),
    Format::new(ATARI8, "Jet Graphics Planner", &["jgp"], font::decode_jgp),
    Format::new(ATARI8, "Graph2Font", &["mch"], graph2font::decode_mch),
    Format::new(ATARI8, "Graph2Font", &["g2f"], graph2font::decode_g2f).signature(),
    Format::new(VBXE, "SlideShow for VBXE", &["dap"], vbxe::decode_dap),
    Format::new(
        PORTFOLIO,
        "Portfolio Graphics",
        &["pgf"],
        portfolio::decode_pgf,
    ),
    Format::new(
        PORTFOLIO,
        "Portfolio Graphics Compressed",
        &["pgc"],
        portfolio::decode_pgc,
    )
    .signature(),
    // Wave 4: charset interlace
    Format::new(ATARI8, "ICE MIN", &["imn"], ice::decode_imn),
    Format::new(ATARI8, "ICE CIN", &["icn"], ice::decode_icn),
    Format::new(ATARI8, "ICE PCIN", &["ipc"], ice::decode_ipc),
    Format::new(ATARI8, "ICE PCIN+", &["ip2"], ice::decode_ip2),
    Format::new(ATARI8, "Super IRG", &["irg"], ice::decode_irg),
    Format::new(ATARI8, "Super IRG 2", &["ir2"], ice::decode_ir2),
    Format::new(ATARI8, "DIN", &["din"], ice::decode_din),
    Format::new(
        ATARI8,
        "Interlace Character Editor font",
        &["ice"],
        ice::decode_ice,
    ),
    // Wave 4: small screens
    Format::new(ATARI8, "TXS", &["txs"], misc_screen::decode_txs),
    Format::new(ATARI8, "Floor Designer", &["fge"], misc_screen::decode_fge),
    Format::new(ATARI8, "KFX", &["kfx"], misc_screen::decode_kfx),
    Format::new(ATARI8, "Cut Creator", &["cut"], misc_screen::decode_cut),
    Format::new(ATARI8, "Graphics 9+", &["gr9p"], misc_screen::decode_gr9p),
    Format::new(ATARI8, "Mamut", &["rys"], misc_screen::decode_rys),
    Format::new(ATARI8, "KSS-Paint", &["kss"], misc_screen::decode_kss),
    Format::new(
        ATARI8,
        "Gephard Hires Graphics",
        &["ghg"],
        misc_screen::decode_ghg,
    ),
    Format::new(ATARI8, "PI8", &["pi8"], misc_screen::decode_pi8),
    Format::new(ATARI8, "PI9", &["pi9"], misc_screen::decode_pi9),
    Format::new(ATARI8, "Trzmiel (compressed)", &["cpr"], packed::decode_cpr),
    Format::new(
        ATARI8,
        "Kompresor do Animatora",
        &["kpr"],
        packed::decode_kpr,
    ),
    Format::new(ATARI8, "Graph", &["all"], text_art::decode_all),
    Format::new(
        ATARI8,
        "Atari Graphics Studio",
        &["ags"],
        misc_screen::decode_ags,
    ),
    // ART is also an Atari ST and Commodore extension; the sizes and
    // headers below keep those files out.
    Format::new(
        ATARI8,
        "Monochrome ART",
        &["art"],
        misc_screen::decode_mono_art,
    ),
    Format::new(
        ATARI8,
        "Artist by David Eaton",
        &["art"],
        misc_screen::decode_artist_art,
    ),
    Format::new(
        ATARI8,
        "Ascii-Art Editor",
        &["art"],
        text_art::decode_ascii_art,
    ),
    // Wave 4: interlace and multi-frame bitmaps
    Format::new(
        ATARI8,
        "Interlace Graphics Editor",
        &["ige"],
        interlace2::decode_ige,
    ),
    Format::new(
        ATARI8,
        "Interlace Logo Designer",
        &["ild"],
        interlace2::decode_ild,
    ),
    Format::new(ATARI8, "ING 15", &["ing"], interlace2::decode_ing),
    Format::new(ATARI8, "Atari HR", &["hr"], interlace2::decode_hr),
    Format::new(ATARI8, "MegaColor 80x96", &["mga"], interlace2::decode_mga),
    Format::new(
        ATARI8,
        "Bugbiter APAC239i",
        &["bgp"],
        interlace2::decode_bgp,
    ),
    Format::new(
        ATARI8,
        "Champions' Interlace (packed)",
        &["cci"],
        interlace2::decode_cci,
    ),
    Format::new(ATARI8, "ColorViewSquash", &["rgb"], colorview::decode_rgb),
    Format::new(ATARI8, "Rocky Interlace Picture", &["rip"], rip::decode_rip),
    Format::new(ATARI8, "Rambrandt GTIA 10", &["rm2"], rambrandt::decode_rm2),
    Format::new(
        ATARI8,
        "Rambrandt Graphics 15",
        &["rm4"],
        rambrandt::decode_rm4,
    ),
    // Wave 4: headered bitmaps with per-line colours
    Format::new(ATARI8, "Hard Color Map", &["hcm"], hcm::decode_hcm).signature(),
    Format::new(ATARI8, "XL-Paint MAX raw", &["raw"], xl_paint::decode_raw),
    Format::new(ATARI8, "XL-Paint MAX", &["max"], xl_paint::decode_max),
    Format::new(ATARI8, "XL-Paint", &["xlp"], xl_paint::decode_xlp),
    Format::new(ATARI8, "Marco Pixel Editor", &["cpi"], cpi::decode_cpi),
    Format::new(ATARI8, "Fun with Art", &["fwa"], fwa::decode_fwa),
    Format::new(ATARI8, "MCS", &["mcs"], mcs::decode_mcs),
    Format::new(ATARI8, "GED", &["ged"], ged::decode_ged),
    // Wave 4: player/missile graphics, shapes, fonts and maps
    Format::new(
        ATARI8,
        "AtariTools-800 4 missiles",
        &["4mi"],
        pmg::decode_4mi,
    ),
    Format::new(
        ATARI8,
        "AtariTools-800 4 players",
        &["4pl"],
        pmg::decode_4pl,
    ),
    Format::new(
        ATARI8,
        "AtariTools-800 4 players and missiles",
        &["4pm"],
        pmg::decode_4pm,
    ),
    Format::new(ATARI8, "Atari Player Editor", &["apl"], pmg::decode_apl),
    Format::new(ATARI8, "Ludek Maker", &["ldm"], pmg::decode_ldm).signature(),
    Format::new(ATARI8, "Larka Edytor Obiektow", &["leo"], leo::decode_leo),
    Format::new(ATARI8, "PMG Designer", &["pmd"], pmg::decode_pmd).signature(),
    Format::new(ATARI8, "Envision", &["map"], envision::decode_map),
    Format::new(ATARI8, "EnvisionPC", &["map"], envision::decode_map_pc),
    Format::new(
        ATARI8,
        "Movie Maker shapes",
        &["shp"],
        shapes::decode_movie_maker,
    ),
    Format::new(
        VBXE,
        "Graph2Font VBXE",
        &["g2f"],
        graph2font::decode_g2f_vbxe,
    )
    .signature(),
    Format::new(
        ATARI8,
        "Blazing Paddles shape table",
        &["shp"],
        blazing_paddles::decode_shp,
    ),
    Format::new(
        ATARI8,
        "Blazing Paddles font",
        &["chr"],
        blazing_paddles::decode_chr,
    ),
    // Wave 5: Amiga and misc
    Format::new(ATARI8, "SprEd", &["spr"], spred::decode).signature(),
];
