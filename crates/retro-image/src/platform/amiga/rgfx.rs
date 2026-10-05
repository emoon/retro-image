//! IFF-RGFX (`FORM RGFX`): Andreas Kleinert's IFF successor to ILBM for
//! chunky and true-color pictures on RTG systems.
//!
//! Sources:
//! - `rgfx.h`, `RGFX-Chunks.txt`, `RGFX-Remarks.txt` and the readme of
//!   IFF-RGFX v4.1, 2012-08-31 (<https://aminet.net/dev/misc/IFF-RGFX.zip>),
//!   read as specification prose: the `RGHD` header (13 longs), `RSCM`,
//!   `RCOL` (transparent color, 256 RGB entries), `RBOD` and the compression
//!   codes (0 none, 1 XPK).
//! - The 8-bit chunky and 24-bit types are confirmed on 16 samples
//!   (Sembiance's `image/rgfx`): 15 files of 160x128 chunky 8-bit pictures
//!   packed with XPK MASH, and `boundless_WB2.rgfx`, 800x600 RGB24 packed with
//!   XPK NUKE (see `codec/xpk.rs`). Header fields were read off those files.
//!   Their `RGHD` fields match `rgfx.h` (compression 1 at long 9, bitmap
//!   type at long 12, pixel aspect 11:10 or 22:22).
//!
//! Only what those samples exercise is decoded, plus the transparent color of
//! `RCOL` (a few lines straight from `rgfx.h`, with a unit test). Everything
//! else is rejected rather than drawn from the spec alone: the planar type
//! (the header has no plane count and the spec does not say how planes are
//! laid out), the float types, the 15-, 16-, 32-, 48- and 64-bit types,
//! `RTRN`, zlib compression, and chunky data whose `RSCM` view mode says HAM
//! or extra-half-brite. The pixel aspect and the other mode flags are not
//! used.
//!
//! RECOIL has no RGFX support.

use super::chunky::{Packing, Pixels, Rows, bitmap_bytes, render};
use super::iff::find;
use crate::bytes::be32;
use crate::{DecodeError, Image};

const HEADER_LEN: usize = 13 * 4;

// Bitmap types, `RMBT_` in `rgfx.h`.
const CHUNKY8: u32 = 1 << 0;
const RGB24: u32 = 1 << 1;

/// Decodes the chunks of a `FORM RGFX`.
pub(super) fn decode(contents: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let header = find(contents, b"RGHD")
        .filter(|h| h.len() >= HEADER_LEN)
        .ok_or(fail)?;
    let field = |i: usize| be32(header, i * 4).ok_or(fail);
    let rows = Rows {
        width: field(2)? as usize,
        height: field(3)? as usize,
        bytes_per_line: field(8)? as usize,
    };
    let (depth, pixel_bits, bitmap_type) = (field(6)?, field(7)?, field(12)?);
    let packing = match field(9)? {
        0 => Packing::Stored,
        1 => Packing::Xpk,
        _ => return Err(fail),
    };
    let view_mode = find(contents, b"RSCM")
        .and_then(|m| be32(m, 0))
        .unwrap_or(0);
    let pixels = match (bitmap_type, pixel_bits) {
        (CHUNKY8, 8) if (1..=8).contains(&depth) => Pixels::indexed(depth, view_mode),
        (RGB24, 24) => Some(Pixels::Rgb24),
        _ => None,
    }
    .ok_or(fail)?;

    let (palette, clear) = if pixels == Pixels::Indexed8 {
        palette(contents).ok_or(fail)?
    } else {
        ([0; 256], None)
    };
    let body = find(contents, b"RBOD").ok_or(fail)?;
    let bytes = bitmap_bytes(packing, body, rows.len(pixels)?)?;
    render(pixels, rows, &bytes, &palette, clear)
}

/// The `RCOL` colors, and the index of the transparent color if the chunk
/// says there is one.
fn palette(contents: &[u8]) -> Option<([u32; 256], Option<u8>)> {
    let rcol = find(contents, b"RCOL")?;
    let colors = rcol.get(8..8 + 256 * 3)?;
    let mut palette = [0u32; 256];
    for (entry, rgb) in palette.iter_mut().zip(colors.as_chunks::<3>().0) {
        *entry = u32::from_be_bytes([0, rgb[0], rgb[1], rgb[2]]);
    }
    let clear = if be32(rcol, 0)? == 1 {
        Some(u8::try_from(be32(rcol, 4)?).ok()?)
    } else {
        None
    };
    Some((palette, clear))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    fn chunk(id: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut out = id.to_vec();
        out.extend_from_slice(&(body.len() as u32).to_be_bytes());
        out.extend_from_slice(body);
        if body.len() % 2 == 1 {
            out.push(0);
        }
        out
    }

    /// RGHD for a 2x1 picture.
    fn rghd(bitmap_type: u32, pixel_bits: u32, bytes_per_line: u32, compression: u32) -> Vec<u8> {
        let longs = [
            0,
            0,
            2,
            1,
            2,
            1,
            8,
            pixel_bits,
            bytes_per_line,
            compression,
            1,
            1,
            bitmap_type,
        ];
        chunk(
            b"RGHD",
            &longs
                .iter()
                .flat_map(|l| l.to_be_bytes())
                .collect::<Vec<u8>>(),
        )
    }

    /// RCOL with color 1 red and color 2 blue.
    fn rcol(transparent: Option<u32>) -> Vec<u8> {
        let mut body = [0u8; 8 + 768];
        body[8 + 3..8 + 6].copy_from_slice(&[255, 0, 0]);
        body[8 + 6..8 + 9].copy_from_slice(&[0, 0, 255]);
        if let Some(index) = transparent {
            body[..4].copy_from_slice(&1u32.to_be_bytes());
            body[4..8].copy_from_slice(&index.to_be_bytes());
        }
        chunk(b"RCOL", &body)
    }

    #[test]
    fn chunky_pictures_use_the_palette() {
        let contents = [rghd(CHUNKY8, 8, 2, 0), rcol(None), chunk(b"RBOD", &[1, 2])].concat();
        assert_eq!(decode(&contents).unwrap().rgb(), &[255, 0, 0, 0, 0, 255]);
        // Without a palette there is nothing to draw.
        let bare = [rghd(CHUNKY8, 8, 2, 0), chunk(b"RBOD", &[1, 2])].concat();
        assert!(decode(&bare).is_err());
    }

    #[test]
    fn rgb24_pictures_need_no_palette() {
        let body = chunk(b"RBOD", &[1, 2, 3, 4, 5, 6]);
        let contents = [rghd(RGB24, 24, 6, 0), body].concat();
        assert_eq!(decode(&contents).unwrap().rgb(), &[1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn the_transparent_color_is_clear() {
        let contents = [
            rghd(CHUNKY8, 8, 2, 0),
            rcol(Some(2)),
            chunk(b"RBOD", &[1, 2]),
        ]
        .concat();
        let image = decode(&contents).unwrap();
        assert_eq!(
            (image.get_argb(0, 0), image.get_argb(1, 0)),
            (0xffff_0000, crate::image::CLEAR)
        );
    }

    #[test]
    fn rejects_what_no_sample_confirms() {
        let body = chunk(b"RBOD", &[0; 8]);
        // Planar (type 0), floats, 15-, 16-, 32-, 48- and 64-bit types, and a
        // pixel size that disagrees with the type.
        let types = [
            (0, 8),
            (1 << 8, 96),
            (1 << 9, 128),
            (1 << 4, 15),
            (1 << 5, 16),
            (1 << 2, 32),
            (1 << 6, 48),
            (1 << 7, 64),
            (RGB24, 32),
        ];
        for (bitmap_type, bits) in types {
            let contents = [rghd(bitmap_type, bits, 8, 0), body.clone()].concat();
            assert!(decode(&contents).is_err(), "type {bitmap_type:#x}");
        }
        // Compression other than none and XPK, notably zlib.
        for compression in [2, 3] {
            assert!(decode(&[rghd(RGB24, 24, 6, compression), body.clone()].concat()).is_err());
        }
        // Rows narrower than the picture.
        assert!(decode(&[rghd(RGB24, 24, 5, 0), body.clone()].concat()).is_err());
        assert!(decode(&[rghd(RGB24, 24, 6, 0), body.clone()].concat()).is_ok());
        // A HAM view of 8-bit chunky data: plain indices would draw it wrong.
        let rscm = chunk(
            b"RSCM",
            &[0, 0, 8, 0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
        );
        let ham = [
            rghd(CHUNKY8, 8, 2, 0),
            rcol(None),
            rscm,
            chunk(b"RBOD", &[1, 2]),
        ]
        .concat();
        assert!(decode(&ham).is_err());
    }
}
