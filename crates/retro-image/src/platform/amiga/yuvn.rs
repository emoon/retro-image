//! YUVN (`FORM YUVN`): MacroSystem's IFF form for CCIR-601 Y:U:V pictures.
//!
//! Sources:
//! - Henning Friedl (MacroSystem), `yuvn.doc` and `yuvn.i`, 1992-04-18,
//!   reproduced on the AmigaOS wiki
//!   (<https://wiki.amigaos.net/wiki/YUVN_IFF_YUV_Image_Data>): the `YCHD`
//!   header (24 bytes: sizes, aspect, compression, flags, mode, norm), the
//!   `DATY`, `DATU` and `DATV` chunks of one byte per sample in row order
//!   with no padding, the modes 400/411/422/444 and their lores twins 200,
//!   211 and 222, the ranges (Y 16-235, U and V 16-240 around 128).
//! - The YUV to RGB step is the ITU-R BT.601 conversion for those ranges,
//!   in integer form (`298 * (Y - 16)` and the chroma terms `409`, `100`,
//!   `208` and `516`, all over 256); the document only says "CCIR-601".
//!   Shared U and V samples are repeated across the pixels they cover.
//!   Lores modes are shown with the same pixels as the hires ones and the
//!   pixel aspect is not applied.
//!
//! No sample file was found, so this is checked only by unit tests built
//! from the document: unverified. Only uncompressed files (the document
//! defines no other kind) are decoded.

use super::iff::find;
use crate::bytes::be16;
use crate::image::check_size;
use crate::{DecodeError, Image};

const HEADER_LEN: usize = 24;
const COMPRESS_NONE: u8 = 0;

/// How many pixels share one U and V sample, horizontally; `None` for
/// pictures without color.
fn chroma_span(mode: u8) -> Option<Option<usize>> {
    match mode {
        0 | 8 => Some(None),
        1 => Some(Some(4)),
        2 | 9 => Some(Some(2)),
        3 | 10 => Some(Some(1)),
        _ => None,
    }
}

pub(super) fn decode(contents: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let header = find(contents, b"YCHD")
        .filter(|h| h.len() >= HEADER_LEN)
        .ok_or(fail)?;
    let width = usize::from(be16(header, 0).ok_or(fail)?);
    let height = usize::from(be16(header, 2).ok_or(fail)?);
    let (compress, mode) = (header[14], header[16]);
    let span = chroma_span(mode).ok_or(fail)?;
    if compress != COMPRESS_NONE || span.is_some_and(|s| width % s != 0) {
        return Err(fail);
    }
    check_size(width, height)?;

    let luma = find(contents, b"DATY")
        .and_then(|y| y.get(..width * height))
        .ok_or(fail)?;
    let chroma = match span {
        None => None,
        Some(span) => {
            let len = width / span * height;
            let plane = |id: &[u8; 4]| find(contents, id).and_then(|c| c.get(..len));
            Some((
                span,
                plane(b"DATU").ok_or(fail)?,
                plane(b"DATV").ok_or(fail)?,
            ))
        }
    };
    let colors = (0..width * height).map(|i| {
        let (u, v) = match chroma {
            Some((span, u, v)) => {
                let at = i / width * (width / span) + i % width / span;
                (i32::from(u[at]) - 128, i32::from(v[at]) - 128)
            }
            None => (0, 0),
        };
        yuv_to_rgb(i32::from(luma[i]) - 16, u, v)
    });
    Ok(Image::from_colors(width as u32, height as u32, colors))
}

/// BT.601 with `y` already less 16 and `u`, `v` less 128, as `0xRRGGBB`.
fn yuv_to_rgb(y: i32, u: i32, v: i32) -> u32 {
    let scaled = 298 * y;
    let channel = |value: i32| ((value + 128) >> 8).clamp(0, 255) as u32;
    channel(scaled + 409 * v) << 16
        | channel(scaled - 100 * u - 208 * v) << 8
        | channel(scaled + 516 * u)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    fn chunk(id: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut out = id.to_vec();
        out.extend_from_slice(&(body.len() as u32).to_be_bytes());
        out.extend_from_slice(body);
        out.resize(out.len() + body.len() % 2, 0);
        out
    }

    fn ychd(width: u16, height: u16, mode: u8) -> Vec<u8> {
        let mut body = alloc::vec![0u8; HEADER_LEN];
        body[..2].copy_from_slice(&width.to_be_bytes());
        body[2..4].copy_from_slice(&height.to_be_bytes());
        body[16] = mode;
        chunk(b"YCHD", &body)
    }

    #[test]
    fn luminance_only_pictures_are_gray() {
        let contents = [ychd(3, 1, 0), chunk(b"DATY", &[16, 126, 235])].concat();
        let image = decode(&contents).unwrap();
        assert_eq!(image.rgb(), &[0, 0, 0, 128, 128, 128, 255, 255, 255]);
    }

    #[test]
    fn chroma_samples_cover_two_or_four_pixels() {
        // 4x1 in mode 411: one U and one V sample for all four pixels. The
        // chroma of pure BT.601 red (Y 81, U 90, V 240) gives red.
        let contents = [
            ychd(4, 1, 1),
            chunk(b"DATY", &[81, 81, 81, 81]),
            chunk(b"DATU", &[90]),
            chunk(b"DATV", &[240]),
        ]
        .concat();
        let image = decode(&contents).unwrap();
        for pixel in image.rgb().chunks(3) {
            assert!(
                pixel[0] >= 253 && pixel[1] <= 2 && pixel[2] <= 2,
                "{pixel:?}"
            );
        }
        // 4x1 in mode 422: two pixels per sample, so two colors.
        let contents = [
            ychd(4, 1, 2),
            chunk(b"DATY", &[81, 81, 41, 41]),
            chunk(b"DATU", &[90, 240]),
            chunk(b"DATV", &[240, 110]),
        ]
        .concat();
        let image = decode(&contents).unwrap();
        assert_eq!(image.rgb()[..3], image.rgb()[3..6]);
        assert_eq!(image.rgb()[6..9], image.rgb()[9..12]);
        assert_ne!(image.rgb()[..3], image.rgb()[6..9]);
    }

    #[test]
    fn widths_and_data_must_fit_the_mode() {
        // 411 needs a width that is a multiple of four.
        let contents = [
            ychd(6, 1, 1),
            chunk(b"DATY", &[16; 6]),
            chunk(b"DATU", &[128; 2]),
            chunk(b"DATV", &[128; 2]),
        ]
        .concat();
        assert!(decode(&contents).is_err());
        // Missing DATV, short DATY, compression and unknown modes.
        let no_v = [
            ychd(2, 1, 2),
            chunk(b"DATY", &[16, 16]),
            chunk(b"DATU", &[128]),
        ]
        .concat();
        assert!(decode(&no_v).is_err());
        let short = [ychd(3, 1, 0), chunk(b"DATY", &[16, 16])].concat();
        assert!(decode(&short).is_err());
        assert!(decode(&[ychd(1, 1, 5), chunk(b"DATY", &[16])].concat()).is_err());
        let mut packed = [ychd(1, 1, 0), chunk(b"DATY", &[16])].concat();
        packed[8 + 14] = 1;
        assert!(decode(&packed).is_err());
    }
}
