//! farbfeld (`.ff`): 16-bit RGBA with a fixed header.
//!
//! Sources:
//! - The format description on <https://tools.suckless.org/farbfeld/>: the
//!   magic `farbfeld`, the width and the height as 32-bit big-endian
//!   integers, then four 16-bit big-endian values (red, green, blue, alpha)
//!   per pixel in row-major order; the colors are sRGB and not
//!   alpha-premultiplied.
//!
//! Samples scale to 8 bits by rounding, and alpha is kept. The magic is the
//! signature. Data after the last pixel is ignored.
//!
//! Verification: no RECOIL oracle. Output compared pixel for pixel with
//! Deark's `farbfeld` module (see the divergence file `unix-rasters.tsv`).

use super::to_byte;
use crate::bytes::{be16, be32};
use crate::image::check_size;
use crate::{DecodeError, Image};

const FAIL: DecodeError = DecodeError::Unrecognized;
const MAGIC: &[u8] = b"farbfeld";
const HEADER_LEN: usize = 16;

pub(super) fn decode_farbfeld(data: &[u8]) -> Result<Image, DecodeError> {
    if !data.starts_with(MAGIC) {
        return Err(FAIL);
    }
    let width = be32(data, 8).ok_or(FAIL)? as usize;
    let height = be32(data, 12).ok_or(FAIL)? as usize;
    check_size(width, height)?;
    let pixels = data[HEADER_LEN..].as_chunks::<8>().0;
    if pixels.len() < width * height {
        return Err(FAIL);
    }
    let colors = pixels.iter().map(|pixel| {
        let [r, g, b, a] =
            [0, 2, 4, 6].map(|at| to_byte(be16(pixel, at).unwrap_or(0).into(), 0xffff));
        u32::from_be_bytes([a, r, g, b])
    });
    Image::from_argb(width as u32, height as u32, colors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::CLEAR;

    fn file(width: u32, height: u32, pixels: &[[u16; 4]]) -> alloc::vec::Vec<u8> {
        let mut bytes = MAGIC.to_vec();
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&height.to_be_bytes());
        for value in pixels.iter().flatten() {
            bytes.extend_from_slice(&value.to_be_bytes());
        }
        bytes
    }

    #[test]
    fn samples_scale_and_alpha_is_kept() {
        let bytes = file(2, 1, &[[0xffff, 0x8000, 0, 0xffff], [0xffff, 0, 0, 0]]);
        let image = decode_farbfeld(&bytes).unwrap();
        assert_eq!(
            (image.get_argb(0, 0), image.get_argb(1, 0)),
            (0xffff_8000, CLEAR)
        );
    }

    #[test]
    fn rejects_bad_magic_and_short_data() {
        let good = file(1, 1, &[[0; 4]]);
        assert!(decode_farbfeld(&good).is_ok());
        assert!(decode_farbfeld(&good[..good.len() - 1]).is_err());
        assert!(decode_farbfeld(&[b"x".as_slice(), &good[1..]].concat()).is_err());
        assert!(decode_farbfeld(&file(0, 5, &[])).is_err());
        assert!(decode_farbfeld(&file(60_000, 60_000, &[])).is_err());
    }
}
