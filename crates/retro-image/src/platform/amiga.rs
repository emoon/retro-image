//! Amiga, including DCTV and HAM-E.
//!
//! Each submodule lists the documents its layouts come from; the platform
//! survey is `docs/research/amiga-apple-misc.md`. This file dispatches IFF
//! pictures:
//! - `FORM` kinds (ILBM, BBM, PBM, ACBM, RGBN/RGB8, DEEP/TVPP, ANIM): EA IFF 85 standard,
//!   <https://wiki.amigaos.net/wiki/EA_IFF_85_Standard_for_Interchange_Format_Files>,
//!   and the IFF FORM and chunk registry,
//!   <https://wiki.amigaos.net/wiki/IFF_FORM_and_Chunk_Registry>.
//! - ANIM shown as its first frame, a complete ILBM `FORM` nested at the
//!   start of the ANIM: ANIM spec,
//!   <https://wiki.amigaos.net/wiki/ANIM_IFF_CEL_Animations>.
//! - `FORM RGFX`, `FORM YAFA` (first frame) and `FORM YUVN`, and deep ILBMs of 12, 32, 48 and 64
//!   planes: Kleinert's IFF-RGFX (<https://aminet.net/dev/misc/IFF-RGFX.zip>) and ILBM64
//!   (<https://aminet.net/docs/misc/ILBM64.readme>) texts, the YAFA document
//!   (<https://aminet.net/docs/misc/YAFA-doc.lha>) and MacroSystem's YUVN text
//!   (<https://wiki.amigaos.net/wiki/YUVN_IFF_YUV_Image_Data>).
//! - A top-level `LIST` or `CAT`, and `FORM ANBM`, shown as their first `FORM` with the `PROP`
//!   chunks of its type behind it: EA IFF 85 and the ANBM page,
//!   <https://wiki.amigaos.net/wiki/ANBM_IFF_Animated_Bitmap>.
//! - PowerPacker (`PP20`), Pack-Ice, Rob Northen (RNC), Imploder and Crunch-Mania wrappers around
//!   an IFF file or an AMOS bank are unpacked first (one layer): PowerPacker file format,
//!   <http://fileformats.archiveteam.org/wiki/PowerPacker>; the depackers are in `codec/`, each
//!   citing Ancient (<https://github.com/temisu/ancient>, BSD-2).
//! - AMOS banks tried as sprite/icon banks, then as a packed picture, then as the picture
//!   packer's files without a bank header: the AMOS file formats page,
//!   <http://alvyn.sourceforge.net/amos_file_formats.html>.
//!
//! Not IFF, each in its own module with its sources: SuperView Graphics (`sgx.rs`, with XPK
//! bodies from `codec/xpk.rs`), CDXL video (`cdxl.rs`), Disney Animation Studio (`cfast.rs`) and
//! bitmap fonts (`bitmap_font.rs`).

mod abk;
mod bitmap_font;
mod cdxl;
mod cfast;
mod chunky;
mod dctv;
mod deep;
mod deep_ilbm;
mod flf;
mod ham_e;
mod icon;
mod iff;
mod iff_group;
mod ilbm;
mod multi_palette;
mod pac_pic;
mod rgbn;
mod rgfx;
mod sgx;
mod vdat;
mod yafa;
mod yuvn;

use crate::codec::{crunch_mania, imploder, pack_ice, powerpacker, rnc};
use crate::{DecodeError, Format, Image};
use alloc::vec::Vec;

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
    // PowerPacker-wrapped IFF; found by content through the IFF entry above.
    Format::new("Amiga", "PowerPacker", &["pp"], decode_iff),
    Format::new("Amiga", "Amiga Continuous Bitmap", &["acbm"], decode_iff),
    Format::new("Amiga", "Hold-And-Modify 6", &["ham", "ham6"], decode_iff),
    Format::new("Amiga", "Hold-And-Modify 8", &["ham8"], decode_iff),
    Format::new(
        "Amiga",
        "Multi-palette",
        &["dhr", "dr", "mp", "beam"],
        decode_iff,
    ),
    Format::new("Amiga", "RGBN", &["rgbn"], decode_iff),
    Format::new("Amiga", "RGB8", &["rgb8"], decode_iff),
    Format::new("Amiga", "AMOS", &["abk"], decode_abk).signature(),
    Format::new("Amiga", "Icon", &["info"], icon::decode).signature(),
    Format::new("Amiga", "IFF-RGFX", &["rgfx", "rgx"], decode_iff),
    Format::new("Amiga", "YAFA animation", &["yafa"], decode_iff),
    // A hunk file with no extension of its own (the files are named by size).
    Format::new("Amiga", "Bitmap font", &[], bitmap_font::decode).signature(),
    Format::new("Amiga", "Disney Animation Studio", &["cft"], cfast::decode).signature(),
    // Headerless: only the extension and a strict header check identify it.
    Format::new("Amiga", "CDXL video", &["cdxl", "xl"], cdxl::decode),
    // SuperView's older `.svg` files are found by their signature; claiming the
    // extension would also catch the vector kind.
    Format::new("Amiga", "SuperView Graphics", &["sgx"], sgx::decode).signature(),
    Format::new("Amiga", "TVPaint", &["deep"], decode_iff),
    Format::new("Amiga", "Sliced HAM", &["sham"], decode_iff),
    Format::new(
        "Amiga",
        "Turbo Rascal Syntax Error",
        &["flf"],
        flf::decode_flf,
    )
    .signature(),
    Format::new("Amiga DCTV", "DCTV", &["dct", "dctv"], decode_dctv),
    Format::new("Amiga HAM-E", "HAM-E", &["iff"], decode_ham_e),
];

/// AMOS sprite, icon or picture bank, possibly packed (one layer), or the
/// picture packer's files without a bank header.
fn decode_abk(data: &[u8]) -> Result<Image, DecodeError> {
    let unpacked = depack(data)?;
    let data = unpacked.as_deref().unwrap_or(data);
    abk::decode(data)
        .or_else(|_| pac_pic::decode(data))
        .or_else(|_| pac_pic::decode_screen(data))
        .or_else(|_| pac_pic::decode_bare(data))
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

/// The contents of a PowerPacker, Pack-Ice, RNC, Imploder or Crunch-Mania file; `None` if `data`
/// is none of them, an error if it is one and damaged.
fn depack(data: &[u8]) -> Result<Option<Vec<u8>>, DecodeError> {
    let unpacked = if powerpacker::is_packed(data) {
        powerpacker::unpack(data)
    } else if pack_ice::is_packed(data) {
        pack_ice::unpack(data)
    } else if rnc::is_packed(data) {
        rnc::unpack(data)
    } else if imploder::is_packed(data) {
        imploder::unpack(data)
    } else if crunch_mania::is_packed(data) {
        crunch_mania::unpack(data)
    } else {
        return Ok(None);
    };
    unpacked.map(Some).ok_or(DecodeError::Unrecognized)
}

/// Any IFF picture FORM we support, plain or packed (one layer).
pub(super) fn decode_iff(data: &[u8]) -> Result<Image, DecodeError> {
    match depack(data)? {
        Some(unpacked) => decode_plain_iff(&unpacked),
        None => decode_plain_iff(data),
    }
}

fn decode_plain_iff(data: &[u8]) -> Result<Image, DecodeError> {
    // NEOchrome Master (Atari ST) pictures: an ILBM plus rasters after the
    // FORM, left to that decoder.
    if super::atari_st::is_neochrome_master(data) {
        return Err(DecodeError::Unrecognized);
    }
    if let Some((kind, contents)) = iff_group::first_form(data) {
        return decode_form(&kind, &contents);
    }
    let (kind, contents) = iff::form(data).ok_or(DecodeError::Unrecognized)?;
    decode_form(&kind, contents)
}

fn decode_form(kind: &[u8; 4], contents: &[u8]) -> Result<Image, DecodeError> {
    match kind {
        // BBM: the form type of PC Deluxe Paint files, laid out like ILBM.
        b"BBM " => ilbm::decode_ilbm(contents),
        b"ILBM" => ilbm::decode_ilbm(contents)
            .or_else(|_| dctv::decode(contents))
            .or_else(|_| ham_e::decode(contents))
            .or_else(|_| deep_ilbm::decode(contents)),
        b"PBM " => ilbm::decode_pbm(contents),
        b"ACBM" => ilbm::decode_acbm(contents),
        b"RGBN" => rgbn::decode(rgbn::Kind::Rgbn, contents),
        b"RGB8" => rgbn::decode(rgbn::Kind::Rgb8, contents),
        b"DEEP" | b"TVPP" => deep::decode(contents),
        b"RGFX" => rgfx::decode(contents),
        b"YAFA" => yafa::decode(contents),
        b"YUVN" => yuvn::decode(contents),
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
    fn bbm_form_decodes_like_ilbm() {
        let mut form = b"FORM\0\0\0\x38ILBM".to_vec();
        form.extend_from_slice(
            b"BMHD\0\0\0\x14\0\x01\0\x01\0\0\0\0\x01\0\0\0\0\0\x01\x01\0\x01\0\x01",
        );
        form.extend_from_slice(b"CMAP\0\0\0\x06\0\0\0\xff\xff\xff");
        form.extend_from_slice(b"BODY\0\0\0\x02\x80\0");
        let ilbm = decode_iff(&form).unwrap();
        form[8..12].copy_from_slice(b"BBM ");
        assert_eq!(decode_iff(&form).unwrap(), ilbm);
    }

    /// 1x1 single-plane ILBM, the same one `bbm_form_decodes_like_ilbm` builds.
    fn tiny_ilbm() -> Vec<u8> {
        let mut form = b"FORM\0\0\0\x38ILBM".to_vec();
        form.extend_from_slice(
            b"BMHD\0\0\0\x14\0\x01\0\x01\0\0\0\0\x01\0\0\0\0\0\x01\x01\0\x01\0\x01",
        );
        form.extend_from_slice(b"CMAP\0\0\0\x06\0\0\0\xff\xff\xff");
        form.extend_from_slice(b"BODY\0\0\0\x02\x80\0");
        form
    }

    #[test]
    fn powerpacker_wrapped_ilbm_decodes_like_the_plain_file() {
        let plain = tiny_ilbm();
        let packed = powerpacker::tests::literal_pp20(&plain, [9, 10, 11, 11]);
        assert_eq!(decode_iff(&packed).unwrap(), decode_iff(&plain).unwrap());
        // Only one layer is unpacked.
        let twice = powerpacker::tests::literal_pp20(&packed, [9, 9, 9, 9]);
        assert!(decode_iff(&twice).is_err());
        // A packed file whose contents are not a picture is rejected.
        let junk = powerpacker::tests::literal_pp20(b"not an IFF file", [9, 9, 9, 9]);
        assert!(decode_iff(&junk).is_err());
    }

    #[test]
    fn a_list_decodes_its_first_form_with_the_properties_of_its_prop() {
        let plain = tiny_ilbm();
        // BMHD and CMAP (28 + 14 bytes) move into a PROP; BODY stays in the FORM.
        let (shared, body) = plain[12..].split_at(28 + 14);
        let group = |id: &[u8; 4], kind: &[u8; 4], payload: &[u8]| {
            let mut out = id.to_vec();
            out.extend_from_slice(&(payload.len() as u32 + 4).to_be_bytes());
            out.extend_from_slice(kind);
            out.extend_from_slice(payload);
            out
        };
        let prop = group(b"PROP", b"ILBM", shared);
        let form = group(b"FORM", b"ILBM", body);
        let list = group(b"LIST", b"ILBM", &[prop.clone(), form.clone()].concat());
        assert_eq!(decode_iff(&list).unwrap(), decode_iff(&plain).unwrap());
        // The same inside an ANBM brush.
        let anbm = group(b"FORM", b"ANBM", &list[..]);
        assert_eq!(decode_iff(&anbm).unwrap(), decode_iff(&plain).unwrap());
        // A FORM without the shared properties is not a picture.
        assert!(decode_iff(&group(b"LIST", b"ILBM", &form)).is_err());
    }

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
