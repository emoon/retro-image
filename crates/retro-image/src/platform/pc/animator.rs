//! Autodesk Animator `PIC` and `CEL` (the original format, not Animator Pro).
//!
//! Sources:
//! - Just Solve the File Format Problem, Animator PIC/CEL:
//!   <http://fileformats.archiveteam.org/wiki/Animator_PIC/CEL> (CC0).
//! - fileformat.info, "Autodesk CEL":
//!   <https://www.fileformat.info/format/cel/corion.htm>.
//! - Deark `misc2.c`, `de_run_animator_pic` (<https://github.com/jsummers/deark>,
//!   MIT licence): header fields and the 6-bit palette; also the oracle for the
//!   sample files.
//! - Reverse engineered from the 21 samples in the corpus: the signature
//!   `19 91`, width and height, a position, the depth byte (8), a flag byte,
//!   a 32-bit data size, a 256-entry palette of 6-bit RGB at offset 32 and the
//!   raw pixels at offset 800. Every sample is exactly 800 + width * height
//!   bytes. The flag byte is 0 except in two CELs (`CLOWN.CEL`, `MOUSE.CEL`)
//!   that carry 0x91 and a zero data size yet hold raw pixels, so it is not
//!   treated as a compression flag. RLE-compressed originals are unverified (no
//!   sample) and Animator Pro pictures (`0x9500`) are a different format.
//!
//! Verification: no RECOIL oracle for this format; output was compared pixel
//! for pixel with Deark's PNG output on the sample files.

use alloc::vec::Vec;

use super::dac_rounded;
use crate::bytes::le16;
use crate::image::check_size;
use crate::{DecodeError, Image};

const FAIL: DecodeError = DecodeError::Unrecognized;
const MAGIC: u16 = 0x9119;
const PALETTE_AT: usize = 32;
const PIXELS_AT: usize = PALETTE_AT + 256 * 3;
const DEPTH_AT: usize = 10;

pub(super) fn decode_cel(data: &[u8]) -> Result<Image, DecodeError> {
    if le16(data, 0) != Some(MAGIC) || data.get(DEPTH_AT) != Some(&8) {
        return Err(FAIL);
    }
    let width = usize::from(le16(data, 2).ok_or(FAIL)?);
    let height = usize::from(le16(data, 4).ok_or(FAIL)?);
    if width == 0 || height == 0 {
        return Err(FAIL);
    }
    check_size(width, height)?;
    let pixels = data
        .get(PIXELS_AT..PIXELS_AT + width * height)
        .ok_or(FAIL)?;
    let palette: Vec<u32> = data[PALETTE_AT..PIXELS_AT]
        .as_chunks::<3>()
        .0
        .iter()
        .map(|c| {
            let [r, g, b] = c.map(|v| dac_rounded(v & 63));
            r << 16 | g << 8 | b
        })
        .collect();
    Image::from_indexed(width as u32, height as u32, pixels, &palette)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cel(width: u16, height: u16, pixels: &[u8]) -> Vec<u8> {
        let mut file = alloc::vec![0x19, 0x91];
        file.extend_from_slice(&width.to_le_bytes());
        file.extend_from_slice(&height.to_le_bytes());
        file.extend_from_slice(&[0, 0, 0, 0, 8, 0]);
        file.resize(PALETTE_AT, 0);
        for i in 0..=255u8 {
            file.extend_from_slice(&[i & 63, 0, 63]);
        }
        file.extend_from_slice(pixels);
        file
    }

    #[test]
    fn palette_is_six_bit_and_pixels_follow_it() {
        let image = decode_cel(&cel(2, 1, &[1, 63])).unwrap();
        assert_eq!(image.get(0, 0), 0x0400ff);
        assert_eq!(image.get(1, 0), 0xff00ff);
    }

    #[test]
    fn short_or_other_depth_files_are_rejected() {
        let file = cel(2, 2, &[0; 3]);
        assert!(decode_cel(&file).is_err());
        let mut deep = cel(1, 1, &[0]);
        deep[DEPTH_AT] = 4;
        assert!(decode_cel(&deep).is_err());
    }
}
