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
//! Not decoded: planar bitmaps with 2 to 8 planes (the spec does not say
//! whether the planes follow one another or interleave) and the pixel
//! layouts it only recommends. Mono bitmaps and the HAM/EHB reading of
//! 8-bit chunky data follow the spec but have no sample file.

use alloc::borrow::Cow;

use super::chunky::{Pixels, Rows, render};
use crate::bytes::{be16, be32};
use crate::codec::inflate;
use crate::{DecodeError, Image};

const SIGNATURES: [&[u8]; 2] = [b"SGX Graphics File\0", b"SVG Graphics File\0"];
/// Offset of the version word, right after the 18-byte signature.
const VERSION_AT: usize = 18;
/// The only version the spec defines.
const VERSION: u16 = 1;
const COLORS_AT: usize = 54;
const COLORS_LEN: usize = 256 * 3;
const HAM_KEY: u32 = 0x800;
const EXTRA_HALF_BRITE_KEY: u32 = 0x80;

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
    let body = bitmap_bytes(&data[data_offset..], rows.len()?)?;
    render(pixels, rows, &body, &palette)
}

/// The pixel layout for `PixelBits`, `PixelPlanes`, `ColorDepth` and
/// `ViewMode32`, if it is one we decode.
fn pixel_layout(bits: u8, planes: u8, depth: u32, view_mode: u32) -> Option<Pixels> {
    match (bits, planes) {
        (1, 1) => Some(Pixels::Mono),
        (8, 1) if view_mode & HAM_KEY != 0 && depth == 6 => Some(Pixels::Ham6),
        (8, 1) if view_mode & HAM_KEY != 0 && depth == 8 => Some(Pixels::Ham8),
        (8, 1) if view_mode & EXTRA_HALF_BRITE_KEY != 0 && depth == 6 => {
            Some(Pixels::ExtraHalfBrite)
        }
        (8, 1) => Some(Pixels::Indexed8),
        (24, 1) => Some(Pixels::Rgb24),
        (48, 1) => Some(Pixels::Rgb48),
        _ => None,
    }
}

/// The `len` bytes of bitmap data in `payload`: stored as they are, or as
/// `LZ77`, the uncompressed size followed by a zlib stream.
fn bitmap_bytes(payload: &[u8], len: usize) -> Result<Cow<'_, [u8]>, DecodeError> {
    let fail = DecodeError::Unrecognized;
    if let Some(packed) = payload.strip_prefix(b"LZ77") {
        let size = be32(packed, 0).ok_or(fail)? as usize;
        if size != len {
            return Err(fail);
        }
        let inflated = inflate::zlib(&packed[4..], len).ok_or(fail)?;
        return Ok(Cow::Owned(inflated));
    }
    payload.get(..len).map(Cow::Borrowed).ok_or(fail)
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
    fn rejects_a_lying_length_and_missing_data() {
        let mut lying = sgx(b"LZ77\0\0\0\x05");
        lying.extend_from_slice(&stored_zlib(&[1, 2, 3, 4]));
        assert!(decode(&lying).is_err());
        assert!(decode(&sgx(&[1, 2, 3])).is_err());
        assert!(decode(&sgx(&[1, 2, 3, 4])[..100]).is_err());
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
