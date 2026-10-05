//! SuperView Graphics, SGX (and its older twin SVG): Andreas Kleinert's
//! picture format of the SView5 library.
//!
//! Sources:
//! - Andreas R. Kleinert, "The SGX Graphics File Format" v4.2, 2009-04-06
//!   (<https://aminet.net/docs/misc/SGX-Specs.lha>, file `FormatSpecs`):
//!   the header fields, the pixel layouts, the `LZ77` wrapper (a length
//!   followed by a zlib stream), and the note that HAM and EHB pictures
//!   are told apart by `ViewMode32`.
//! - The offsets printed in that spec are two bytes too high after the
//!   version word (they would put the data offset at 22). The offsets
//!   below were measured on `testimg.sgx`, whose header ends at 822 =
//!   54 + 768 where the data starts, and cross-checked on the other
//!   samples (Sembiance's `image/sgx` folder).
//! - Checked against samples: `testimg.sgx` (raw) and `testimg-lz77.sgx` give
//!   identical pixels, and the zlib stream of the second inflates to exactly
//!   the raw data of the first.
//!
//! - SVG files (the older name, "SVG Graphics File") may hold an XPK or
//!   PowerPacker stream where SGX has `LZ77`; the XPK ones were seen with
//!   RLEN and NUKE packers (`codec/xpk.rs`). The PowerPacker form is read as
//!   the spec describes it and has no sample.
//!
//! - The 32-bit layout is red, green, blue, alpha, read off `abydos.rlen.svg`
//!   (800x600, `ColorDepth` 32): bytes 0-2 are the rainbow and byte 3 is 0 or
//!   255 with a few partial values at the edges. Transparent parts show
//!   `TRANSPARENT_FILL`.
//!
//! Not decoded: planar bitmaps with 2 to 8 planes (the spec does not say
//! whether the planes follow one another or interleave) and the pixel
//! layouts it only recommends. Mono bitmaps, 48- and 64-bit pixels (the
//! latter in the same channel order as 32-bit) and the HAM/EHB reading of
//! 8-bit chunky data follow the spec but have no sample file.

use super::chunky::{Alpha, Direct, Fourth, Packing, Pixels, Rows, bitmap_bytes, render};
use crate::bytes::{be16, be32};
use crate::codec::{powerpacker, xpk};
use crate::{DecodeError, Image};

const SIGNATURES: [&[u8]; 2] = [b"SGX Graphics File\0", b"SVG Graphics File\0"];
/// Offset of the version word, right after the 18-byte signature.
const VERSION_AT: usize = 18;
/// The only version the spec defines.
const VERSION: u16 = 1;
const COLORS_AT: usize = 54;
const COLORS_LEN: usize = 256 * 3;

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    if !SIGNATURES.iter().any(|s| data.starts_with(s)) || be16(data, VERSION_AT) != Some(VERSION) {
        return Err(fail);
    }
    let data_offset = be32(data, 20).ok_or(fail)? as usize;
    let header = data.get(..data_offset).ok_or(fail)?;
    let rows = Rows {
        width: be32(header, 32).ok_or(fail)? as usize,
        height: be32(header, 36).ok_or(fail)? as usize,
        bytes_per_line: be32(header, 50).ok_or(fail)? as usize,
    };
    let depth = be32(header, 40).ok_or(fail)?;
    let view_mode = be32(header, 44).ok_or(fail)?;
    let (&bits, &planes) = (header.get(48).ok_or(fail)?, header.get(49).ok_or(fail)?);
    let pixels = pixel_layout(bits, planes, depth, view_mode).ok_or(fail)?;

    let mut palette = [0u32; 256];
    if matches!(
        pixels,
        Pixels::Mono | Pixels::Indexed8 | Pixels::ExtraHalfBrite | Pixels::Ham6 | Pixels::Ham8
    ) {
        let colors = header.get(COLORS_AT..COLORS_AT + COLORS_LEN).ok_or(fail)?;
        for (entry, rgb) in palette.iter_mut().zip(colors.as_chunks::<3>().0) {
            *entry = u32::from_be_bytes([0, rgb[0], rgb[1], rgb[2]]);
        }
    }
    let (packing, payload) = packing(&data[data_offset..]);
    let body = bitmap_bytes(packing, payload, rows.len(pixels)?)?;
    render(pixels, rows, &body, &palette)
}

/// The pixel layout for `PixelBits`, `PixelPlanes`, `ColorDepth` and
/// `ViewMode32`, if it is one we decode.
fn pixel_layout(bits: u8, planes: u8, depth: u32, view_mode: u32) -> Option<Pixels> {
    match (bits, planes) {
        (1, 1) => Some(Pixels::Mono),
        (8, 1) => Some(Pixels::indexed(depth, view_mode)),
        (24, 1) => Some(direct(false, None)),
        (48, 1) => Some(direct(true, None)),
        // The fourth channel is alpha only if `ColorDepth` counts it.
        (32, 1) => Some(direct(false, Some(depth == 32))),
        (64, 1) => Some(direct(true, Some(depth == 64))),
        _ => None,
    }
}

/// Red, green, blue, and for `Some(has_alpha)` a fourth channel after them.
fn direct(wide: bool, fourth: Option<bool>) -> Pixels {
    Pixels::Direct(Direct {
        wide,
        fourth: fourth.map(|has_alpha| Fourth {
            first: false,
            alpha: has_alpha.then_some(Alpha::Straight),
        }),
    })
}

/// How the bitmap data at the end of the header is packed, told by its
/// first bytes, and the bytes to unpack: `LZ77` files hold the uncompressed
/// size and a zlib stream after the tag; the older SVG files may hold an XPK
/// or PowerPacker stream.
fn packing(payload: &[u8]) -> (Packing, &[u8]) {
    if let Some(zlib) = payload.strip_prefix(b"LZ77") {
        (Packing::Zlib, zlib)
    } else if xpk::is_packed(payload) {
        (Packing::Xpk, payload)
    } else if powerpacker::is_packed(payload) {
        (Packing::PowerPacker, payload)
    } else {
        (Packing::Stored, payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    /// An SGX file of 2x2 8-bit pixels with a gray palette; `body` follows the header.
    fn sgx(body: &[u8]) -> Vec<u8> {
        let mut file = alloc::vec![0u8; COLORS_AT + COLORS_LEN];
        file[..18].copy_from_slice(b"SGX Graphics File\0");
        file[18..20].copy_from_slice(&1u16.to_be_bytes());
        let header_len = file.len() as u32;
        file[20..24].copy_from_slice(&header_len.to_be_bytes());
        file[32..36].copy_from_slice(&2u32.to_be_bytes());
        file[36..40].copy_from_slice(&2u32.to_be_bytes());
        file[40..44].copy_from_slice(&8u32.to_be_bytes());
        file[48] = 8;
        file[49] = 1;
        file[50..54].copy_from_slice(&2u32.to_be_bytes());
        for i in 0..256 {
            file[COLORS_AT + i * 3..][..3].fill(i as u8);
        }
        file.extend_from_slice(body);
        file
    }

    /// A zlib stream of stored blocks holding `raw`.
    fn stored_zlib(raw: &[u8]) -> Vec<u8> {
        let mut out = alloc::vec![0x78, 0x01, 0x01];
        out.extend_from_slice(&(raw.len() as u16).to_le_bytes());
        out.extend_from_slice(&(!(raw.len() as u16)).to_le_bytes());
        out.extend_from_slice(raw);
        let (mut a, mut b) = (1u32, 0u32);
        for &byte in raw {
            a = (a + u32::from(byte)) % 65521;
            b = (b + a) % 65521;
        }
        out.extend_from_slice(&(b << 16 | a).to_be_bytes());
        out
    }

    #[test]
    fn raw_and_lz77_bodies_give_the_same_picture() {
        let raw = sgx(&[1, 2, 3, 4]);
        let mut packed = sgx(b"LZ77\0\0\0\x04");
        packed.extend_from_slice(&stored_zlib(&[1, 2, 3, 4]));
        let image = decode(&raw).unwrap();
        assert_eq!(image.rgb(), &[1, 1, 1, 2, 2, 2, 3, 3, 3, 4, 4, 4]);
        assert_eq!(decode(&packed).unwrap(), image);
    }

    #[test]
    fn svg_bodies_may_be_packed_by_xpk_or_powerpacker() {
        let plain = decode(&sgx(&[1, 2, 3, 4])).unwrap();
        // RLEN: four literals, one chunk of the stream.
        let xpk = xpk::tests::stream(b"RLEN", 4, &[(1, &[4, 1, 2, 3, 4], 4)]);
        let mut svg = sgx(&xpk);
        svg[..18].copy_from_slice(b"SVG Graphics File\0");
        assert_eq!(decode(&svg).unwrap(), plain);
        let packed = powerpacker::tests::literal_pp20(&[1, 2, 3, 4], [9, 9, 9, 9]);
        assert_eq!(decode(&sgx(&packed)).unwrap(), plain);
        // A stream that unpacks to fewer bytes than the picture needs.
        let short = xpk::tests::stream(b"RLEN", 3, &[(1, &[2, 1, 2, 255, 3], 3)]);
        assert!(decode(&sgx(&short)).is_err());
    }

    #[test]
    fn rejects_a_lying_length_and_missing_data() {
        let mut lying = sgx(b"LZ77\0\0\0\x05");
        lying.extend_from_slice(&stored_zlib(&[1, 2, 3, 4]));
        assert!(decode(&lying).is_err());
        assert!(decode(&sgx(&[1, 2, 3])).is_err());
        assert!(decode(&sgx(&[1, 2, 3, 4])[..100]).is_err());
    }

    #[test]
    fn svg_files_are_found_by_content_not_by_extension() {
        let mut svg = sgx(&[1, 2, 3, 4]);
        svg[..18].copy_from_slice(b"SVG Graphics File\0");
        assert!(crate::decode("picture.svg", &svg).is_ok());
        // A vector drawing is not ours, and nobody claims its extension.
        let drawing = b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>";
        assert_eq!(
            crate::decode("drawing.svg", drawing),
            Err(crate::DecodeError::UnknownFormat)
        );
    }

    #[test]
    fn a_huge_row_length_is_rejected_before_unpacking() {
        // A 1x1 picture with 1 GiB rows and an XPK RLEN body that claims as
        // much: this used to allocate the gigabyte.
        let mut file = sgx(&[]);
        file[32..36].copy_from_slice(&1u32.to_be_bytes());
        file[36..40].copy_from_slice(&1u32.to_be_bytes());
        file[50..54].copy_from_slice(&0x4000_0000u32.to_be_bytes());
        let mut xpk = b"XPKF".to_vec();
        xpk.extend_from_slice(&[0; 4]);
        xpk.extend_from_slice(b"RLEN");
        xpk.extend_from_slice(&0x4000_0000u32.to_be_bytes());
        xpk.resize(32, 0);
        xpk.extend_from_slice(&[1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 2]);
        xpk.extend_from_slice(&0x4000_0000u32.to_be_bytes());
        xpk.extend_from_slice(&[0, 0, 0, 0]);
        let packed_len = (xpk.len() - 8) as u32;
        xpk[4..8].copy_from_slice(&packed_len.to_be_bytes());
        file.extend_from_slice(&xpk);
        assert!(decode(&file).is_err());
    }

    #[test]
    fn needs_the_signature_and_the_version() {
        let mut file = sgx(&[1, 2, 3, 4]);
        file[19] = 2;
        assert!(decode(&file).is_err());
        file[19] = 1;
        file[0] = b'X';
        assert!(decode(&file).is_err());
    }
}
