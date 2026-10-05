//! IFF-RGFX (`FORM RGFX`): Andreas Kleinert's IFF successor to ILBM for
//! chunky and true-color pictures on RTG systems.
//!
//! Sources:
//! - `rgfx.h`, `RGFX-Chunks.txt`, `RGFX-Remarks.txt` and the readme of
//!   IFF-RGFX v4.1, 2012-08-31 (<https://aminet.net/dev/misc/IFF-RGFX.zip>),
//!   read as specification prose: the `RGHD` header (13 longs), `RSCM`,
//!   `RCOL` (transparent color, 256 RGB entries), `RTRN` (alpha per color),
//!   `RBOD` and the compression codes (0 none, 1 XPK, 2 a size plus zlib
//!   stream).
//! - The 8-bit chunky and 24-bit types are confirmed on 16 samples
//!   (Sembiance's `image/rgfx`): 15 files of 160x128 chunky 8-bit pictures
//!   packed with XPK MASH, and `boundless_WB2.rgfx`, 800x600 RGB24 packed with
//!   XPK NUKE (see `codec/xpk.rs`). Header fields were read off those files.
//!   Their `RGHD` fields match `rgfx.h` (compression 1 at long 9, bitmap
//!   type at long 12, pixel aspect 11:10 or 22:22).
//!
//! Decoded without a sample, from `rgfx.h` alone: the 32- and 64-bit ARGB
//! types (alpha first, shown only if the `RMBT_ALPHA` or `RMBT_ALPHAINV` flag
//! is set), 48-bit RGB, 15- and 16-bit types (read as big-endian words, `A`
//! the top bit, as the "1 + 3x5 bit" note reads; the SGX spec's "5:5:5:1"
//! would put it at the bottom), zlib compression, `RTRN` and the
//! transparent color of `RCOL`. Not decoded: the planar type (the header has
//! no plane count and the spec does not say how the planes are laid out) and
//! the two float types (the byte order of a float is not stated). The pixel
//! aspect and `RSCM`'s mode flags other than HAM and EHB are not used.
//!
//! RECOIL has no RGFX support.

use super::chunky::{
    Alpha, Direct, Fourth, Packing, Pixels, Rows, bitmap_bytes, over_fill, render,
};
use super::iff::find;
use crate::bytes::be32;
use crate::image::TRANSPARENT_FILL;
use crate::{DecodeError, Image};

const HEADER_LEN: usize = 13 * 4;

// Bitmap types, `RMBT_` in `rgfx.h`; the alpha flags are or-ed into them.
const CHUNKY8: u32 = 1 << 0;
const RGB24: u32 = 1 << 1;
const ARGB32: u32 = 1 << 2;
const RGB15: u32 = 1 << 4;
const ARGB16: u32 = 1 << 5;
const RGB48: u32 = 1 << 6;
const ARGB64: u32 = 1 << 7;
const ALPHA: u32 = 1 << 30;
const ALPHA_INVERSE: u32 = 1 << 31;

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
        2 => Packing::Zlib,
        _ => return Err(fail),
    };
    let view_mode = find(contents, b"RSCM")
        .and_then(|m| be32(m, 0))
        .unwrap_or(0);
    let pixels = pixel_layout(bitmap_type, pixel_bits, depth, view_mode).ok_or(fail)?;

    let palette = if matches!(
        pixels,
        Pixels::Indexed8 | Pixels::ExtraHalfBrite | Pixels::Ham6 | Pixels::Ham8
    ) {
        palette(contents).ok_or(fail)?
    } else {
        [0; 256]
    };
    let body = find(contents, b"RBOD").ok_or(fail)?;
    let bytes = bitmap_bytes(packing, body, rows.len(pixels)?)?;
    render(pixels, rows, &bytes, &palette)
}

/// The pixel layout for the header's bitmap type (with its alpha flags),
/// pixel size in bits, depth, and the view mode of `RSCM`.
fn pixel_layout(bitmap_type: u32, pixel_bits: u32, depth: u32, view_mode: u32) -> Option<Pixels> {
    let alpha = if bitmap_type & ALPHA_INVERSE != 0 {
        Some(Alpha::Inverse)
    } else if bitmap_type & ALPHA != 0 {
        Some(Alpha::Straight)
    } else {
        None
    };
    let direct = |wide, fourth: bool| {
        Pixels::Direct(Direct {
            wide,
            fourth: fourth.then_some(Fourth { first: true, alpha }),
        })
    };
    Some(match (bitmap_type & !(ALPHA | ALPHA_INVERSE), pixel_bits) {
        (CHUNKY8, 8) if (1..=8).contains(&depth) => Pixels::indexed(depth, view_mode),
        (RGB24, 24) => direct(false, false),
        (ARGB32, 32) => direct(false, true),
        (RGB48, 48) => direct(true, false),
        (ARGB64, 64) => direct(true, true),
        (RGB15, 15) => Pixels::Rgb555(None),
        (ARGB16, 16) => Pixels::Rgb555(alpha),
        _ => return None,
    })
}

/// The `RCOL` colors, made see-through where `RTRN` or the transparent
/// color say so.
fn palette(contents: &[u8]) -> Option<[u32; 256]> {
    let rcol = find(contents, b"RCOL")?;
    let colors = rcol.get(8..8 + 256 * 3)?;
    let mut palette = [0u32; 256];
    for (entry, rgb) in palette.iter_mut().zip(colors.as_chunks::<3>().0) {
        *entry = u32::from_be_bytes([0, rgb[0], rgb[1], rgb[2]]);
    }
    // Levels per color; only the two flag values the spec defines.
    if let Some(rtrn) = find(contents, b"RTRN") {
        let alpha = match be32(rtrn, 0)? {
            0 => Some(Alpha::Straight),
            1 => Some(Alpha::Inverse),
            _ => None,
        };
        if let (Some(alpha), Some(levels)) = (alpha, rtrn.get(4..4 + 256)) {
            for (entry, &level) in palette.iter_mut().zip(levels) {
                *entry = over_fill(*entry, alpha.opacity(u32::from(level)));
            }
        }
    }
    if be32(rcol, 0)? == 1 {
        let index = be32(rcol, 4)? as usize;
        *palette.get_mut(index)? = TRANSPARENT_FILL;
    }
    Some(palette)
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
    fn the_transparent_color_and_rtrn_show_the_fill() {
        let fill = TRANSPARENT_FILL.to_be_bytes();
        let contents = [
            rghd(CHUNKY8, 8, 2, 0),
            rcol(Some(2)),
            chunk(b"RBOD", &[1, 2]),
        ]
        .concat();
        assert_eq!(
            decode(&contents).unwrap().rgb(),
            &[255, 0, 0, fill[1], fill[2], fill[3]]
        );
        // RTRN with straight levels: color 1 fully transparent, the rest opaque.
        let mut rtrn = alloc::vec![255u8; 4 + 256];
        rtrn[..4].fill(0);
        rtrn[5] = 0;
        let contents = [
            rghd(CHUNKY8, 8, 2, 0),
            rcol(None),
            chunk(b"RTRN", &rtrn),
            chunk(b"RBOD", &[1, 2]),
        ]
        .concat();
        assert_eq!(
            decode(&contents).unwrap().rgb(),
            &[fill[1], fill[2], fill[3], 0, 0, 255]
        );
        // The inverse flag turns the levels around.
        rtrn[3] = 1;
        let contents = [
            rghd(CHUNKY8, 8, 2, 0),
            rcol(None),
            chunk(b"RTRN", &rtrn),
            chunk(b"RBOD", &[1, 2]),
        ]
        .concat();
        assert_eq!(
            decode(&contents).unwrap().rgb(),
            &[255, 0, 0, fill[1], fill[2], fill[3]]
        );
    }

    #[test]
    fn argb_shows_alpha_only_when_flagged() {
        let pixels = [128, 10, 20, 30, 0, 40, 50, 60];
        let body = chunk(b"RBOD", &pixels);
        let plain = [rghd(ARGB32, 32, 8, 0), body.clone()].concat();
        assert_eq!(
            decode(&plain).unwrap().rgb(),
            &[10, 20, 30, 40, 50, 60],
            "alpha without its flag is padding"
        );
        let flagged = [rghd(ARGB32 | ALPHA, 32, 8, 0), body].concat();
        let image = decode(&flagged).unwrap();
        let fill = TRANSPARENT_FILL.to_be_bytes();
        assert_eq!(&image.rgb()[3..], &fill[1..]);
        assert_ne!(&image.rgb()[..3], &[10, 20, 30]);
    }

    #[test]
    fn rejects_unknown_types_and_mismatched_sizes() {
        let body = chunk(b"RBOD", &[0; 6]);
        // Planar (type 0), floats, and a pixel size that disagrees with the type.
        for (bitmap_type, bits) in [(0, 8), (1 << 8, 96), (RGB24, 32)] {
            let contents = [rghd(bitmap_type, bits, 6, 0), body.clone()].concat();
            assert!(decode(&contents).is_err());
        }
        // Rows narrower than the picture, and compression codes we do not know.
        assert!(decode(&[rghd(RGB24, 24, 5, 0), body.clone()].concat()).is_err());
        assert!(decode(&[rghd(RGB24, 24, 6, 3), body.clone()].concat()).is_err());
        assert!(decode(&[rghd(RGB24, 24, 6, 0), body].concat()).is_ok());
    }
}
