//! Amiga, including DCTV and HAM-E.
//!
//! Sources are listed per submodule.

pub(crate) use byte_run1::unpack as unpack_byte_run1;

mod abk;
mod byte_run1;
mod deep;
mod icon;
mod iff;
mod ilbm;
mod multi_palette;
mod pac_pic;
mod vdat;

use crate::{DecodeError, Format, Image};

pub(super) static FORMATS: &[Format] = &[
    Format::new("Amiga", "Interleaved Bitmap", &["lbm", "ilbm"], decode_iff),
    Format::new(
        "Amiga",
        "Interchange File Format",
        &["iff", "256"],
        decode_iff,
    ),
    Format::new("Amiga", "Amiga Continuous Bitmap", &["acbm"], decode_iff),
    Format::new("Amiga", "Hold-And-Modify 6", &["ham", "ham6"], decode_iff),
    Format::new("Amiga", "Hold-And-Modify 8", &["ham8"], decode_iff),
    Format::new(
        "Amiga",
        "Multi-palette",
        &["dhr", "dr", "mp", "beam"],
        decode_iff,
    ),
    Format::new("Amiga", "AMOS", &["abk"], decode_abk),
    Format::new("Amiga", "Icon", &["info"], icon::decode),
    Format::new("Amiga", "TVPaint", &["deep"], decode_iff),
    Format::new("Amiga", "Sliced HAM", &["sham"], decode_iff),
];

/// AMOS sprite, icon or picture bank.
fn decode_abk(data: &[u8]) -> Result<Image, DecodeError> {
    abk::decode(data).or_else(|_| pac_pic::decode(data))
}

/// Any IFF picture FORM we support.
fn decode_iff(data: &[u8]) -> Result<Image, DecodeError> {
    let (kind, contents) = iff::form(data).ok_or(DecodeError::Unrecognized)?;
    decode_form(&kind, contents)
}

fn decode_form(kind: &[u8; 4], contents: &[u8]) -> Result<Image, DecodeError> {
    match kind {
        b"ILBM" => ilbm::decode_ilbm(contents),
        b"PBM " => ilbm::decode_pbm(contents),
        b"ACBM" => ilbm::decode_acbm(contents),
        b"DEEP" | b"TVPP" => deep::decode(contents),
        // ANIM: the first frame is a complete ILBM.
        b"ANIM" => match iff::chunks(contents).next() {
            Some((id, body)) if &id == b"FORM" && body.len() >= 4 => {
                let (kind, contents) = body.split_at(4);
                match kind {
                    b"ILBM" => ilbm::decode_ilbm(contents),
                    _ => Err(DecodeError::Unrecognized),
                }
            }
            _ => Err(DecodeError::Unrecognized),
        },
        _ => Err(DecodeError::Unrecognized),
    }
}
