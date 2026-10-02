//! IBM PC text-mode art (ANSI/BBS art): ANSI, Binary Text, XBin, ArtWorx,
//! iCE Draw, TundraDraw, PCBoard and Avatar. RECOIL doesn't decode these.
//!
//! This file only registers the formats. The documentation survey with
//! every source per format is `docs/research/textmode.md`; each submodule
//! cites the sources of its own formats. SAUCE metadata is read by
//! [`sauce`], cells are drawn by [`screen`] with the fonts in [`font`].
//!
//! Signatures are only claimed for real magic: XBin ("XBIN" 1Ah), iCE Draw
//! ("\x04" "1.4" plus a valid palette) and TundraDraw ("\x18" "TUNDRA24").
//! `.bin` and `.adf` are generic extensions, so those decoders accept only
//! a vouching SAUCE record or content that checks out (see [`binary`]);
//! PCBoard and Avatar need real command structure and little else in the
//! way of control characters.

mod ansi;
mod avatar;
mod binary;
mod font;
mod pcboard;
mod sauce;
mod screen;
mod tundra;
mod xbin;

use crate::Format;

const PC: &str = "PC";

pub(super) static FORMATS: &[Format] = &[
    Format::new(PC, "ANSI art", &["ans"], ansi::decode),
    Format::new(PC, "Binary Text", &["bin"], binary::decode_bin),
    Format::new(PC, "XBin", &["xb"], xbin::decode).signature(),
    Format::new(PC, "ArtWorx Data Format", &["adf"], binary::decode_adf),
    Format::new(PC, "iCE Draw", &["idf"], binary::decode_idf).signature(),
    Format::new(PC, "TundraDraw", &["tnd"], tundra::decode).signature(),
    Format::new(PC, "PCBoard", &["pcb"], pcboard::decode),
    Format::new(PC, "Avatar", &["avt"], avatar::decode),
];
