//! Nintendo 3DS textures: CLIM (`.bclim`), CTPK (`.ctpk`) and CTXB
//! (`.ctxb`).
//!
//! Sources: GBATEK, "3DS Files - Video Layout Images (CLIM/FLIM)", "Video
//! Texture Package (CTPK)", "Video Texture Binary (CTXB)", "3DS GPU Texture
//! Formats" and "3DS Video Texture Swizzling"
//! (<https://problemkaputt.de/gbatek.htm>, no license, facts only); see the
//! submodules for the layouts. The 3DS icon formats (SMDH) are not here.
//!
//! No sample of these formats could be found (no free ones are published), so
//! the decoders are unverified: they follow GBATEK and are tested only on
//! files built from its layouts, two of which are guesses (the vertical
//! orientation, and where an odd-size picture lies in its padded texture; see
//! `texture.rs`). FLIM is not decoded, see `clim.rs`.
//!
//! Each file has a strong signature: the `CLIM` footer at the end of the file
//! checked against the size, the version and the offsets, the `CTPK` header
//! with its entries inside the file, and the `ctxb` header whose size matches
//! the file.

mod clim;
mod ctpk;
mod ctxb;
mod etc1;
mod texture;

use crate::Format;

pub(super) static FORMATS: &[Format] = &[
    Format::new(
        "Nintendo 3DS",
        "CLIM texture",
        &["bclim", "clim"],
        clim::decode,
    )
    .signature(),
    Format::new(
        "Nintendo 3DS",
        "CTPK texture package",
        &["ctpk"],
        ctpk::decode,
    )
    .signature(),
    Format::new("Nintendo 3DS", "CTXB texture", &["ctxb"], ctxb::decode).signature(),
];
