//! Atari 8-bit, including VBXE, and Atari Portfolio.
//!
//! This file only registers the formats. The documentation survey with
//! every source per format is `docs/research/atari-8bit.md`; each submodule
//! cites the sources of its own formats. The palette is in [`palette`];
//! ANTIC/GTIA bitmap rendering in [`antic`].

mod antic;
mod apac;
mod font;
mod graph2font;
mod gtia;
mod hip;
mod inflate;
mod interlace;
mod koala;
mod mad_studio;
mod palette;
mod portfolio;
mod rom_font;
mod screen;
mod sfdn;
mod technicolor;
mod text;
mod tip;
mod vbxe;

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
];
