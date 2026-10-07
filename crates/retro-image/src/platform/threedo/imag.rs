//! `IMAG` pictures: a header chunk and one `PDAT` chunk of 16-bit pixels.
//!
//! Sources:
//! - "3DO File Format" in the 3DO Portfolio 2.5 documentation
//!   (`ppgfldr/smmfldr/cdmfldr/08CDM001.html`, mirrored at
//!   <https://3dodev.com/documentation/file_formats/media/container/3do> and in
//!   <https://github.com/trapexit/3do-devkit> `docs/3dosdk/`; prose only, its
//!   license is not stated). The `IMAG` chunk (`ImageCC`) holds width, height
//!   and bytes per row (32 bits each), then bits per pixel, components,
//!   planes, color space, compression, `hv` format, pixel order and version
//!   (one byte each); the pixels are in the `PDAT` chunk after it.
//! - The `VDL ` and pixel order findings below: looking at the samples of
//!   `trapexit/3do-devkit` (data only, none of its source was read).
//!
//! Pictures that come with a `VDL ` chunk are refused: the SDK's VDL pictures
//! decode to noise as plain 16-bit pixels.
//!
//! Pixel order 0 stores rows one after another. Orders 1 and 2 ("LRform")
//! store each pair of rows interleaved pixel by pixel, order 1 starting with
//! the upper row and order 2 with the lower one. Order 1 is the one in the
//! samples (all 175 16-bit pictures of the SDK); order 2 follows the
//! documentation only.

use super::chunks;
use crate::bytes::{be16, be32};
use crate::image::{check_size, xrgb1555};
use crate::{DecodeError, Image};

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    const FAIL: DecodeError = DecodeError::Invalid;
    // A `VDL ` chunk (a color table per scanline) changes what the pixels
    // mean, and the documentation does not say how, so those pictures are not
    // read: the pixels alone show noise.
    if chunks(data).any(|chunk| &chunk.tag == b"VDL ") {
        return Err(FAIL);
    }
    let mut all = chunks(data);
    let header = all.find(|chunk| &chunk.tag == b"IMAG").ok_or(FAIL)?.body;
    let pixels = all.find(|chunk| &chunk.tag == b"PDAT").ok_or(FAIL)?.body;

    let width = be32(header, 0).ok_or(FAIL)? as usize;
    let height = be32(header, 4).ok_or(FAIL)? as usize;
    let row_len = be32(header, 8).ok_or(FAIL)? as usize;
    // 16 bits per pixel, 3 components in 1 plane, RGB, uncompressed.
    let [16, 3, 1, 0, 0, _, order] = header.get(12..19).ok_or(FAIL)? else {
        return Err(FAIL);
    };
    check_size(width, height)?;
    if *order > 2 || row_len < width * 2 || (*order != 0 && !height.is_multiple_of(2)) {
        return Err(FAIL);
    }
    if row_len.checked_mul(height).ok_or(FAIL)? > pixels.len() {
        return Err(FAIL);
    }
    let colors = (0..height).flat_map(|y| {
        (0..width).map(move |x| {
            let at = match order {
                0 => y * row_len + x * 2,
                // Order 2 has the lower row of a pair first.
                _ => {
                    let lower = (y % 2) ^ usize::from(*order == 2);
                    y / 2 * 2 * row_len + (x * 2 + lower) * 2
                }
            };
            xrgb1555(be16(pixels, at).unwrap_or(0))
        })
    });
    Image::from_colors(width as u32, height as u32, colors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    /// An `IMAG` chunk and a `PDAT` chunk of `pixels` (16-bit, big-endian).
    fn file(width: u32, height: u32, order: u8, pixels: &[u16]) -> Vec<u8> {
        let mut data = b"IMAG".to_vec();
        data.extend_from_slice(&28u32.to_be_bytes());
        for value in [width, height, width * 2] {
            data.extend_from_slice(&value.to_be_bytes());
        }
        data.extend_from_slice(&[16, 3, 1, 0, 0, 0, order, 0]);
        data.extend_from_slice(b"PDAT");
        data.extend_from_slice(&(8 + pixels.len() as u32 * 2).to_be_bytes());
        for pixel in pixels {
            data.extend_from_slice(&pixel.to_be_bytes());
        }
        data
    }

    #[test]
    fn pixel_order_1_interleaves_each_pair_of_rows() {
        // 2x2: red, green, blue, white in file order. Order 1 reads
        // (0,0) (0,1) (1,0) (1,1) as column, row.
        let pixels = [0x7c00, 0x03e0, 0x001f, 0x7fff];
        let image = decode(&file(2, 2, 1, &pixels)).unwrap();
        assert_eq!((image.get(0, 0), image.get(0, 1)), (0xff0000, 0x00ff00));
        assert_eq!((image.get(1, 0), image.get(1, 1)), (0x0000ff, 0xffffff));
        let plain = decode(&file(2, 2, 0, &pixels)).unwrap();
        assert_eq!((plain.get(1, 0), plain.get(0, 1)), (0x00ff00, 0x0000ff));
        let lower_first = decode(&file(2, 2, 2, &pixels)).unwrap();
        assert_eq!(
            (lower_first.get(0, 0), lower_first.get(0, 1)),
            (0x00ff00, 0xff0000)
        );
    }

    #[test]
    fn pictures_with_a_vdl_chunk_are_refused() {
        let mut data = file(2, 2, 0, &[0; 4]);
        data.extend_from_slice(b"VDL \0\0\0\x0c\0\0\0\0");
        assert!(decode(&data).is_err());
    }

    #[test]
    fn rejects_other_depths_odd_interleaved_heights_and_short_data() {
        let mut other_depth = file(2, 2, 0, &[0; 4]);
        other_depth[8 + 12] = 24;
        assert!(decode(&other_depth).is_err());
        assert!(decode(&file(2, 3, 1, &[0; 6])).is_err());
        assert!(decode(&file(2, 2, 0, &[0; 3])).is_err());
        assert!(decode(&file(2, 2, 3, &[0; 4])).is_err());
    }
}
