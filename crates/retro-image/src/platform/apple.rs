//! Apple II, IIe, IIGS and Macintosh.
//!
//! Each submodule lists the documents its layouts come from; the platform
//! survey is `docs/research/amiga-apple-misc.md`, which lists the extensions
//! of each layout. `decode_3200` here tries Brooks, then `.3201`, then the
//! 32 KB screen dump for `.SH3`, `.3200` and `.SHR`; the layouts differ in
//! size and header, so the order only matters for rejecting files. Brooks
//! is File Type Note $C1/0002
//! (<https://mirrors.apple2.org.za/ftp.gno.org/doc/apple/filetypes/ftn.c1.0002>),
//! the screen dump File Type Note $C1/0000
//! (<https://mirrors.apple2.org.za/ftp.gno.org/doc/apple/filetypes/ftn.c1.0000>)
//! and `.3201` the CiderPress II Super Hi-Res notes
//! (<https://ciderpress2.com/formatdoc/SuperHiRes-notes.html>); details in
//! `super_hires.rs`.

mod dreamgrafix;
mod hires;
mod macpaint;
mod pack_bytes;
mod printshop_gs;
mod sprites;
mod super_hires;

use crate::{DecodeError, Format, Image};

pub(super) static FORMATS: &[Format] = &[
    Format::new("Apple II", "High Resolution", &["hgr"], hires::decode_hgr),
    Format::new(
        "Apple IIe",
        "Double High-Resolution",
        &["dhgr", "dhr"],
        hires::decode_dhgr,
    ),
    Format::new("Apple IIGS", "3201", &["3201"], super_hires::decode_3201).signature(),
    Format::new(
        "Apple IIGS",
        "Multi-palette",
        &["32k"],
        super_hires::decode_apf,
    ),
    Format::new(
        "Apple IIGS",
        "Apple Preferred Format",
        &["gs", "iigs", "pnt", "shr"],
        super_hires::decode_apf,
    )
    .signature(),
    Format::new(
        "Apple IIGS",
        "Paintworks",
        &["pnt"],
        super_hires::decode_paintworks,
    ),
    Format::new("Apple IIGS", "320x200", &["sh3", "3200"], decode_3200),
    Format::new("Apple IIGS", "320x200", &["shr"], decode_3200),
    // `.SCR` is shared with the CPC and ZX Spectrum, so it is checked strictly.
    Format::new(
        "Apple IIGS",
        "320x200",
        &["scr"],
        super_hires::decode_checked_screen,
    ),
    Format::new(
        "Apple IIGS",
        "Packed Super Hi-Res",
        &["shr"],
        super_hires::decode_packed_screen,
    ),
    Format::new(
        "Apple IIGS",
        "DreamGrafix",
        &["256", "3200"],
        dreamgrafix::decode,
    )
    .signature(),
    Format::new(
        "Apple Macintosh",
        "MacPaint",
        &["mac", "pnt", "pntg"],
        macpaint::decode,
    ),
    // The MacBinary header's file type makes these recognisable by content.
    Format::new(
        "Apple Macintosh",
        "MacPaint in MacBinary",
        &["mac", "pnt", "pntg"],
        macpaint::decode_mac_binary,
    )
    .signature(),
    Format::new("Apple II", "Sprites", &["spr"], sprites::decode),
    // ProDOS type $F8 has no extension on disk; `.psg` is this crate's own.
    Format::new(
        "Apple IIGS",
        "Print Shop GS clip art",
        &["psg"],
        printshop_gs::decode,
    ),
];

/// Brooks pictures, also accepting the other 3200-colour and screen-dump
/// layouts found under the same extensions.
fn decode_3200(data: &[u8]) -> Result<Image, DecodeError> {
    super_hires::decode_brooks(data)
        .or_else(|_| super_hires::decode_3201(data))
        .or_else(|_| super_hires::decode_screen(data))
}
