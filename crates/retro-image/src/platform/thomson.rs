//! Thomson MO5, MO6, TO7, TO7/70, TO8 and TO9: MAP pictures saved by BASIC
//! `SAVEP` and drawing programs, Graffiti pictures and PHO photos.
//!
//! Each submodule lists the documents its layouts come from; the platform
//! survey is `docs/formats/thomson.md`.

mod graffiti;
mod map;
mod palette;
mod pho;
mod video;

use crate::Format;

const PLATFORM: &str = "Thomson MO/TO";

pub(super) static FORMATS: &[Format] = &[
    // The binary file records, mode, size, packed banks and trailer are all
    // checked, so MAP pictures are also recognised under other names.
    Format::with_companions(PLATFORM, "MAP picture", &["map"], map::decode_map).signature(),
    Format::with_companions(
        PLATFORM,
        "Graffiti bitmap 16",
        &["m16"],
        graffiti::decode_m16,
    ),
    Format::with_companions(
        PLATFORM,
        "Graffiti bitmap 4",
        &["m04"],
        graffiti::decode_m04,
    ),
    Format::with_companions(
        PLATFORM,
        "Graffiti 80 columns",
        &["m02"],
        graffiti::decode_m02,
    ),
    Format::new(PLATFORM, "PHO photo", &["pho"], pho::decode_pho),
];
