//! Sega Dreamcast texture files.
//!
//! Sources: each submodule lists its own. Platform survey:
//! `docs/research/gaps-consoles.md` section 3.1.
//!
//! RECOIL decodes none of these formats, so there is no oracle run.

mod pvr;

use crate::Format;

pub(super) static FORMATS: &[Format] = &[
    Format::with_companions("Dreamcast", "PVR texture", &["pvr"], pvr::decode_pvr).signature(),
    Format::with_companions(
        "Dreamcast",
        "PVM texture archive",
        &["pvm"],
        pvr::decode_pvm,
    )
    .signature(),
];
