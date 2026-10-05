//! IBM KIPS bitmaps (`.KPS`): 8 bits per pixel, palette kept elsewhere.
//!
//! Sources:
//! - Just Solve the File Format Problem, "IBM KIPS bitmap":
//!   <http://justsolve.archiveteam.org/wiki/IBM_KIPS_bitmap> (8 bits per
//!   pixel, signature `DFIMAG00`; the palette is a separate `.PAL` file, with
//!   no signature, or a `.KPL` file that has the same signature).
//! - Reverse engineered from the 8 sample files (Sembiance's `ibmKIPS`
//!   folder): after the 8-byte signature come a 16-bit height at 8 and a
//!   16-bit width at 10 (little-endian), then `01 00 00 00` and zeros up to
//!   offset 32, where the pixels start, one byte each, row after row. Six
//!   files hold exactly `32 + width * height` bytes (320x200, 640x480 and
//!   1024x768). The two small banner clips disagree with their height field
//!   (51 stored, 50 rows present; 33 stored, 35 rows present) and both look
//!   right with all the rows the data holds, so the height comes from the
//!   file size.
//!
//! No sample comes with a palette file, so the layout of `.PAL` and `.KPL` is
//! unknown and the pixels are shown as grays by index (a guess, as for PCX
//! files without a palette). The pictures show their shapes but not their
//! tones. The format is chosen by extension only: a `.KPL` file shares the
//! signature.
//!
//! Verification: no RECOIL or Deark support; output reviewed by eye.

use alloc::vec::Vec;

use crate::bytes::le16;
use crate::image::check_size;
use crate::{DecodeError, Image};

const FAIL: DecodeError = DecodeError::Unrecognized;
const SIGNATURE: &[u8; 8] = b"DFIMAG00";
const HEADER_LEN: usize = 32;

pub(super) fn decode_kps(data: &[u8]) -> Result<Image, DecodeError> {
    if !data.starts_with(SIGNATURE) {
        return Err(FAIL);
    }
    let width = usize::from(le16(data, 10).ok_or(FAIL)?);
    let pixels = data.get(HEADER_LEN..).ok_or(FAIL)?;
    if width == 0 || le16(data, 8) == Some(0) {
        return Err(FAIL);
    }
    let height = pixels.len() / width;
    check_size(width, height)?;
    let grays: Vec<u32> = (0..256u32).map(|v| v * 0x01_01_01).collect();
    Image::from_indexed(
        width as u32,
        height as u32,
        &pixels[..width * height],
        &grays,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_come_from_the_data_not_the_height_field() {
        let mut file = SIGNATURE.to_vec();
        file.extend_from_slice(&[9, 0, 2, 0]); // height field 9, width 2
        file.resize(HEADER_LEN, 0);
        file.extend_from_slice(&[0, 1, 2, 3, 4, 5, 6]);
        let image = decode_kps(&file).unwrap();
        assert_eq!((image.width(), image.height()), (2, 3));
        assert_eq!(&image.rgb()[3..9], &[1, 1, 1, 2, 2, 2]);
        assert!(decode_kps(&file[..HEADER_LEN + 1]).is_err());
        file[0] = b'X';
        assert!(decode_kps(&file).is_err());
    }
}
