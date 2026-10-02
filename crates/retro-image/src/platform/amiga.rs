//! Amiga, including DCTV and HAM-E.
//!
//! Each submodule lists the documents its layouts come from; the platform
//! survey is `docs/research/amiga-apple-misc.md`. This file dispatches IFF
//! pictures:
//! - `FORM` kinds (ILBM, PBM, ACBM, DEEP/TVPP, ANIM): EA IFF 85 standard,
//!   <https://wiki.amigaos.net/wiki/EA_IFF_85_Standard_for_Interchange_Format_Files>,
//!   and the IFF FORM and chunk registry,
//!   <https://wiki.amigaos.net/wiki/IFF_FORM_and_Chunk_Registry>.
//! - ANIM shown as its first frame, a complete ILBM `FORM` nested at the
//!   start of the ANIM: ANIM spec,
//!   <https://wiki.amigaos.net/wiki/ANIM_IFF_CEL_Animations>.
//! - AMOS banks tried as sprite/icon banks, then as a packed picture: the
//!   AMOS file formats page, <http://alvyn.sourceforge.net/amos_file_formats.html>.

mod abk;
mod dctv;
mod deep;
mod ham_e;
mod icon;
mod iff;
mod ilbm;
mod multi_palette;
mod pac_pic;
mod vdat;

use crate::{DecodeError, Format, Image};

pub(super) static FORMATS: &[Format] = &[
    Format::new("Amiga", "Interleaved Bitmap", &["lbm", "ilbm"], decode_iff),
    // Content detection for every IFF picture: `FORM` + a known kind.
    Format::new(
        "Amiga",
        "Interchange File Format",
        &["iff", "256"],
        decode_iff,
    )
    .signature(),
    Format::new("Amiga", "Amiga Continuous Bitmap", &["acbm"], decode_iff),
    Format::new("Amiga", "Hold-And-Modify 6", &["ham", "ham6"], decode_iff),
    Format::new("Amiga", "Hold-And-Modify 8", &["ham8"], decode_iff),
    Format::new(
        "Amiga",
        "Multi-palette",
        &["dhr", "dr", "mp", "beam"],
        decode_iff,
    ),
    Format::new("Amiga", "AMOS", &["abk"], decode_abk).signature(),
    Format::new("Amiga", "Icon", &["info"], icon::decode).signature(),
    Format::new("Amiga", "TVPaint", &["deep"], decode_iff),
    Format::new("Amiga", "Sliced HAM", &["sham"], decode_iff),
    // Wave 5: Amiga and misc
    Format::new("Amiga DCTV", "DCTV", &["dct", "dctv"], decode_dctv),
    Format::new("Amiga HAM-E", "HAM-E", &["iff"], decode_ham_e),
];

/// AMOS sprite, icon or picture bank.
fn decode_abk(data: &[u8]) -> Result<Image, DecodeError> {
    abk::decode(data).or_else(|_| pac_pic::decode(data))
}

/// DCTV pictures: an ILBM with the DCTV signature in its first row.
fn decode_dctv(data: &[u8]) -> Result<Image, DecodeError> {
    match iff::form(data) {
        Some((kind, contents)) if &kind == b"ILBM" => dctv::decode(contents),
        _ => Err(DecodeError::Unrecognized),
    }
}

/// HAM-E pictures: an ILBM that starts with a HAM-E palette line.
fn decode_ham_e(data: &[u8]) -> Result<Image, DecodeError> {
    match iff::form(data) {
        Some((kind, contents)) if &kind == b"ILBM" => ham_e::decode(contents),
        _ => Err(DecodeError::Unrecognized),
    }
}

/// Any IFF picture FORM we support.
fn decode_iff(data: &[u8]) -> Result<Image, DecodeError> {
    // NEOchrome Master (Atari ST) pictures: an ILBM plus rasters after the
    // FORM, left to that decoder.
    if super::atari_st::is_neochrome_master(data) {
        return Err(DecodeError::Unrecognized);
    }
    let (kind, contents) = iff::form(data).ok_or(DecodeError::Unrecognized)?;
    decode_form(&kind, contents)
}

fn decode_form(kind: &[u8; 4], contents: &[u8]) -> Result<Image, DecodeError> {
    match kind {
        b"ILBM" => ilbm::decode_ilbm(contents)
            .or_else(|_| dctv::decode(contents))
            .or_else(|_| ham_e::decode(contents)),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neochrome_master_ilbm_is_left_to_the_atari_st_decoder() {
        // 1x1, one plane, uncompressed; FORM length 4 + 28 + 14 + 10 = 56.
        let mut form = b"FORM\0\0\0\x38ILBM".to_vec();
        form.extend_from_slice(
            b"BMHD\0\0\0\x14\0\x01\0\x01\0\0\0\0\x01\0\0\0\0\0\x01\x01\0\x01\0\x01",
        );
        form.extend_from_slice(b"CMAP\0\0\0\x06\0\0\0\xff\xff\xff");
        form.extend_from_slice(b"BODY\0\0\0\x02\x80\0");
        assert!(decode_iff(&form).is_ok());
        let neochrome = [&form[..], b"RAST\0\0\0\0"].concat();
        assert_eq!(
            decode_iff(&neochrome).err(),
            Some(DecodeError::Unrecognized)
        );
    }
}
