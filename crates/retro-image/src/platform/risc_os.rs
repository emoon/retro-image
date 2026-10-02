//! Acorn Archimedes / RISC OS bitmap formats.
//!
//! Registry only: each submodule cites its sources; the platform survey is
//! `docs/formats/riscos-ql.md`. Extensions: `ff9` is the RISC OS filetype,
//! kept as an extension by some archives (`name.ff9`); RISC OS-style names
//! (`name,ff9`) have no extension and are found by the sprite signature.

mod sprite;

use crate::Format;

const PLATFORM: &str = "Acorn Archimedes";

pub(super) static FORMATS: &[Format] =
    &[Format::new(PLATFORM, "RISC OS sprite", &["ff9", "spr"], sprite::decode).signature()];
