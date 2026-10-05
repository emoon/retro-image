//! Deep ILBM pictures with 12, 32, 48 or 64 planes (the 24-plane kind is
//! handled with the other ILBM modes).
//!
//! Sources:
//! - 12 planes: the ILBM spec, "Interpreting ILBMs" (CMAP): "if a deep ILBM
//!   (like 12 or 24 planes), there should be no CMAP and instead the BODY
//!   planes are interpreted as the bits of RGB in the order R0...Rn G0...Gn
//!   B0...Bn" (<https://wiki.amigaos.net/wiki/ILBM_IFF_Interleaved_Bitmap>).
//!   Four bits per component, widened by repeating them (`* 17`), which the
//!   spec does not say.
//! - 32, 48 and 64 planes: Andreas R. Kleinert, "IFF-ILBM 32/48/64 Bit
//!   extensions" v1.2 (<https://aminet.net/docs/misc/ILBM64.readme>): RGBA
//!   8:8:8:8, RGB 16:16:16 and RGBA 16:16:16:16, the channels following one
//!   another as in the 24-plane order. 16-bit values are shown by their high
//!   byte, as the readme says for alpha.
//! - The alpha channel is not drawn: the readme says `mskHasAlpha` marks it
//!   in BMHD but gives no value for it (the AmigaOS wiki table and libilbm
//!   list masking values 0 to 3 only), so 32- and 64-plane pictures are
//!   shown opaque, the fourth channel ignored.
//! - The 21-plane NewTek order of the same spec is not decoded: its text
//!   lists 24 bit positions for a "21-bit" format.
//!
//! No sample file of any of these was found among about 280 ILBMs, so this
//! is checked only by the unit tests, which build files from the specs
//! above. Unverified.

use alloc::vec::Vec;

use super::iff::find;
use super::ilbm::scale_factors;
use crate::bytes::{be16, be32};
use crate::codec::packbits;
use crate::image::{check_size, planar_pixels};
use crate::{DecodeError, Image};

const MASK_HAS_MASK: u8 = 1;

/// Bits per channel for a plane count: red, green and blue follow one another,
/// then a fourth channel (alpha) in the 32- and 64-plane kinds.
fn channels(planes: usize) -> Option<usize> {
    match planes {
        12 => Some(4),
        32 => Some(8),
        48 | 64 => Some(16),
        _ => None,
    }
}

/// Scales a `bits`-wide channel value to 8 bits.
fn to_byte(value: u32, bits: usize) -> u32 {
    match bits {
        4 => value * 17,
        8 => value,
        _ => value >> (bits - 8),
    }
}

pub(super) fn decode(contents: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let bmhd = find(contents, b"BMHD")
        .filter(|b| b.len() >= 20)
        .ok_or(fail)?;
    let (width, height) = (
        usize::from(be16(bmhd, 0).ok_or(fail)?),
        usize::from(be16(bmhd, 2).ok_or(fail)?),
    );
    let (planes, masking, compression) = (usize::from(bmhd[8]), bmhd[9], bmhd[10]);
    let bits = channels(planes).ok_or(fail)?;
    check_size(width, height)?;

    let stored = planes + usize::from(masking == MASK_HAS_MASK);
    let row_len = width.div_ceil(16) * 2;
    let len = row_len * stored * height;
    let body = find(contents, b"BODY").ok_or(fail)?;
    // ByteRun1 never expands a byte past 128.
    if len > body.len().saturating_mul(128) {
        return Err(fail);
    }
    let data = match compression {
        0 => body.get(..len).ok_or(fail)?.to_vec(),
        1 => packbits::unpack(body, len).ok_or(fail)?.0,
        _ => return Err(fail),
    };
    let channel = |index: usize| -> Vec<u32> {
        planar_pixels(&data, width, height, row_len, bits, |plane, y| {
            (y * stored + index * bits + plane) * row_len
        })
    };
    let (red, green, blue) = (channel(0), channel(1), channel(2));
    let colors = (0..width * height).map(|i| {
        to_byte(red[i], bits) << 16 | to_byte(green[i], bits) << 8 | to_byte(blue[i], bits)
    });
    let image = Image::from_colors(width as u32, height as u32, colors);
    let camg = find(contents, b"CAMG")
        .and_then(|c| be32(c, 0))
        .unwrap_or(0);
    let (sx, sy) = scale_factors(camg);
    image.scaled(sx, sy)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(id: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut out = id.to_vec();
        out.extend_from_slice(&(body.len() as u32).to_be_bytes());
        out.extend_from_slice(body);
        out.resize(out.len() + body.len() % 2, 0);
        out
    }

    /// A 2x1 picture of `planes` planes, uncompressed; `colors` are the two
    /// pixels' channel values, high to low bit order inside each plane.
    fn picture(planes: u8, masking: u8, colors: [&[u32]; 2]) -> Vec<u8> {
        let mut bmhd = alloc::vec![0u8; 20];
        bmhd[..2].copy_from_slice(&2u16.to_be_bytes());
        bmhd[2..4].copy_from_slice(&1u16.to_be_bytes());
        bmhd[8] = planes;
        bmhd[9] = masking;
        let bits = channels(usize::from(planes)).unwrap();
        // Plane `p` of channel `c` is bit `p` of that channel's value; one
        // row of 2 bytes per plane, the pixels in the top two bits.
        let mut body = Vec::new();
        for (first, second) in colors[0].iter().zip(colors[1]) {
            for p in 0..bits {
                let pixels = [first >> p & 1, second >> p & 1];
                body.extend_from_slice(&[(pixels[0] << 7 | pixels[1] << 6) as u8, 0]);
            }
        }
        let mut contents = b"ILBM".to_vec();
        contents.extend_from_slice(&chunk(b"BMHD", &bmhd));
        contents.extend_from_slice(&chunk(b"BODY", &body));
        contents[4..].to_vec()
    }

    #[test]
    fn twelve_planes_hold_four_bits_per_component() {
        let contents = picture(12, 0, [&[0xf, 0x8, 0x0], &[0x0, 0x1, 0xf]]);
        let image = decode(&contents).unwrap();
        assert_eq!(image.rgb(), &[0xff, 0x88, 0x00, 0x00, 0x11, 0xff]);
    }

    #[test]
    fn sixteen_bit_channels_show_their_high_byte() {
        let contents = picture(
            48,
            0,
            [&[0x1234, 0xabcd, 0x00ff], &[0xffff, 0x8000, 0x0100]],
        );
        let image = decode(&contents).unwrap();
        assert_eq!(image.rgb(), &[0x12, 0xab, 0x00, 0xff, 0x80, 0x01]);
    }

    #[test]
    fn the_fourth_channel_is_not_drawn() {
        let pixels: [&[u32]; 2] = [&[10, 20, 30, 0], &[40, 50, 60, 255]];
        // Whatever BMHD masking says, 32 and 64 planes show opaque colors.
        for masking in [0, 4] {
            let image = decode(&picture(32, masking, pixels)).unwrap();
            assert_eq!(image.rgb(), &[10, 20, 30, 40, 50, 60]);
        }
        let wide: [&[u32]; 2] = [&[0x1200, 0, 0, 0xffff], &[0, 0, 0, 0]];
        let image = decode(&picture(64, 4, wide)).unwrap();
        assert_eq!(image.rgb(), &[0x12, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn other_plane_counts_and_short_bodies_are_rejected() {
        let mut other = picture(12, 0, [&[1, 2, 3], &[4, 5, 6]]);
        // BMHD starts after "BMHD" and its length: planes is 8 bytes in.
        other[8 + 8] = 21;
        assert!(decode(&other).is_err());
        let mut short = picture(12, 0, [&[1, 2, 3], &[4, 5, 6]]);
        short.truncate(short.len() - 2);
        assert!(decode(&short).is_err());
    }
}
