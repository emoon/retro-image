//! Atari 8-bit, including VBXE, and Atari Portfolio.
//!
//! Each submodule lists the sources of its formats. The palette is in
//! [`palette`]; ANTIC/GTIA bitmap rendering in [`antic`].

mod antic;
mod apac;
mod font;
mod hip;
mod interlace;
mod koala;
mod mad_studio;
mod palette;
mod portfolio;
mod screen;
mod vbxe;

use crate::Format;

const ATARI8: &str = "Atari 8-bit";
const VBXE: &str = "Atari 8-bit VBXE";
const PORTFOLIO: &str = "Atari Portfolio";

pub(super) static FORMATS: &[Format] = &[
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
    Format::new(ATARI8, "Graphics 9", &["gr9"], screen::decode_gr9),
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
    Format::new(ATARI8, "Graphics 10", &["g10"], screen::decode_g10),
    Format::new(ATARI8, "Graphics 11", &["g11"], screen::decode_g11),
    Format::new(ATARI8, "Micro Illustrator", &["mic"], screen::decode_mic),
    Format::new(ATARI8, "Sketch-PadDles", &["skp"], screen::decode_skp),
    Format::new(ATARI8, "Hard Interlace Picture", &["hip"], hip::decode_hip),
    Format::new(ATARI8, "VertiZontal Interlacing", &["vzi"], hip::decode_vzi),
    Format::new(ATARI8, "InterPainter", &["inp"], interlace::decode_inp),
    Format::new(ATARI8, "INT95a", &["int"], interlace::decode_int),
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
    ),
];
