//! MSX, MSX2, MSX2+ and V9990.
//!
//! Sources are listed per submodule; see also `docs/formats/msx-japanese.md`.

mod bitbuster;
mod dynamic_publisher;
mod g9b;
mod screen;
mod vdp;

use screen::{Bitmap, Tiled};
pub(super) use vdp::{level5, yjk_group};

use super::nec_pc::pi;
use super::nec_pc::{Machine, maki};
use crate::Format;

pub(super) static FORMATS: &[Format] = &[
    Format::new("MSX", "Screen 2", &["sc2", "grp"], |d| {
        screen::decode_tiled_dump(Tiled::Graphic2, d)
    }),
    Format::new("MSX", "Screen 3", &["sc3"], |d| {
        screen::decode_tiled_dump(Tiled::Multicolour, d)
    }),
    Format::new("MSX2", "Screen 4", &["sc4"], |d| {
        screen::decode_tiled_dump(Tiled::Graphic3, d)
    }),
    Format::new("MSX2", "Screen 5", &["sc5", "ge5"], |d| {
        screen::decode_bitmap_dump(Bitmap::Graphic4, d)
    }),
    Format::new("MSX2", "Screen 6", &["sc6"], |d| {
        screen::decode_bitmap_dump(Bitmap::Graphic5, d)
    }),
    Format::new("MSX2", "Screen 7", &["sc7", "ge7"], |d| {
        screen::decode_bitmap_dump(Bitmap::Graphic6, d)
    }),
    Format::new("MSX2", "Screen 8", &["sc8", "ge8", "pic"], |d| {
        screen::decode_bitmap_dump(Bitmap::Graphic7, d)
    }),
    Format::new("MSX2+", "Screen 10/11", &["sca", "scb"], |d| {
        screen::decode_bitmap_dump(Bitmap::Yae, d)
    }),
    Format::new("MSX2+", "Screen 12", &["scc", "yjk"], |d| {
        screen::decode_bitmap_dump(Bitmap::Yjk, d)
    }),
    Format::new("MSX2", "Graph Saurus Screen 5", &["sr5"], |d| {
        screen::decode_graph_saurus(Bitmap::Graphic4, d)
    }),
    Format::new("MSX2", "Graph Saurus Screen 6", &["sr6"], |d| {
        screen::decode_graph_saurus(Bitmap::Graphic5, d)
    }),
    Format::new("MSX2", "Graph Saurus Screen 7", &["sr7"], |d| {
        screen::decode_graph_saurus(Bitmap::Graphic6, d)
    }),
    Format::new("MSX2", "Graph Saurus Screen 8", &["sr8"], |d| {
        screen::decode_graph_saurus(Bitmap::Graphic7, d)
    }),
    Format::new("MSX2+", "Graph Saurus Screen 12", &["srs"], |d| {
        screen::decode_graph_saurus(Bitmap::Yjk, d)
    }),
    Format::new("MSX2", "BASIC COPY Screen 5", &["gl5"], |d| {
        screen::decode_copy(Bitmap::Graphic4, d)
    }),
    Format::new("MSX2", "BASIC COPY Screen 6", &["gl6"], |d| {
        screen::decode_copy(Bitmap::Graphic5, d)
    }),
    Format::new("MSX2", "BASIC COPY Screen 7", &["gl7"], |d| {
        screen::decode_copy(Bitmap::Graphic6, d)
    }),
    Format::new("MSX2", "BASIC COPY Screen 8", &["gl8"], |d| {
        screen::decode_copy(Bitmap::Graphic7, d)
    }),
    Format::new("MSX2+", "BASIC COPY Screen 12", &["glc", "gls"], |d| {
        screen::decode_copy(Bitmap::Yjk, d)
    }),
    Format::new("MSX2", "Maki-chan Graphics", &["mag", "max"], |d| {
        maki::decode_mag(d, Machine::Msx)
    }),
    Format::new("MSX2", "Maki-chan Graphics (MAKI01)", &["mki"], |d| {
        maki::decode_mki(d, Machine::Msx)
    }),
    Format::new("MSX2", "Pi", &["pi"], |d| pi::decode_pi(d, Machine::Msx)),
    Format::new("MSX2", "PIC", &["pic"], |d| {
        super::sharp_x68000::pic::decode_pic(d, Machine::Msx)
    }),
    Format::new("MSX V9990 VDP", "GFX9k library G9B", &["g9b"], g9b::decode),
    Format::new(
        "MSX2",
        "Dynamic Publisher screen",
        &["pct"],
        dynamic_publisher::decode_pct,
    ),
    Format::new(
        "MSX2",
        "Dynamic Publisher font",
        &["fnt"],
        dynamic_publisher::decode_fnt,
    ),
    Format::new(
        "MSX2",
        "Dynamic Publisher stamp",
        &["stp"],
        dynamic_publisher::decode_stp,
    ),
];
