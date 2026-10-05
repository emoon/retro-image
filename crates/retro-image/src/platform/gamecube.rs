//! Nintendo GameCube and Wii texture files.
//!
//! Sources: each submodule lists its own. The pixel codecs are in
//! `codec::gx`. The platform surveys are `docs/research/gaps-consoles.md`
//! section 3.1 and `docs/research/gaps-nintendo.md` sections A3 and C3.
//!
//! RECOIL decodes none of these formats, so there is no oracle run.

mod banner;
mod gvr;
mod tpl;

use crate::Format;

pub(super) static FORMATS: &[Format] = &[
    Format::with_companions("GameCube", "GVR texture", &["gvr"], gvr::decode_gvr).signature(),
    Format::with_companions("GameCube", "GVM texture archive", &["gvm"], gvr::decode_gvm)
        .signature(),
    Format::new("Wii", "TPL texture library", &["tpl"], tpl::decode_tpl).signature(),
    Format::new("GameCube", "Disc banner", &["bnr"], banner::decode_bnr).signature(),
    Format::new("GameCube", "Memory card save", &["gci"], banner::decode_gci).signature(),
];
