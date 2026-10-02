//! Apple II, IIe, IIGS and Macintosh.
//!
//! Sources are listed per submodule.

mod hires;
mod macpaint;
mod pack_bytes;
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
    Format::new("Apple IIGS", "3201", &["3201"], super_hires::decode_3201),
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
    ),
    Format::new(
        "Apple IIGS",
        "Paintworks",
        &["pnt"],
        super_hires::decode_paintworks,
    ),
    Format::new("Apple IIGS", "320x200", &["sh3", "3200"], decode_3200),
    Format::new("Apple IIGS", "320x200", &["shr"], decode_3200),
    Format::new(
        "Apple Macintosh",
        "MacPaint",
        &["mac", "pnt", "pntg"],
        macpaint::decode,
    ),
];

/// Brooks pictures, also accepting the other 3200-colour and screen-dump
/// layouts found under the same extensions.
fn decode_3200(data: &[u8]) -> Result<Image, DecodeError> {
    super_hires::decode_brooks(data)
        .or_else(|_| super_hires::decode_3201(data))
        .or_else(|_| super_hires::decode_screen(data))
}
