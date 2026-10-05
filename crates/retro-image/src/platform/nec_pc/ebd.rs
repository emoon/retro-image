//! `EBD` pictures (NEC PC-98, 640 pixels wide, 16 colours).
//!
//! Nothing is published about this format (see `docs/research/msx-japanese.md`);
//! the layout was reverse engineered from `BKG008.EBD` by black-box probing of
//! `recoil2png`: 16 palette entries of three bytes (red, green, blue, each
//! 0-15 and scaled by 17; larger values are rejected), then four whole-picture
//! bit planes of `640 * height / 8` bytes each, plane 0 first, pixels MSB
//! first. The height is whatever the file size allows (RECOIL accepts any
//! whole number of 320-byte lines).

use alloc::vec::Vec;

use crate::image::{planar_pixels, widen_channel};
use crate::{DecodeError, Image};

const WIDTH: usize = 640;
const PALETTE_BYTES: usize = 48;
const LINE_BYTES: usize = WIDTH / 2;
const MAX_HEIGHT: usize = 1024;

pub(in crate::platform) fn decode_ebd(data: &[u8]) -> Result<Image, DecodeError> {
    let body = data.get(PALETTE_BYTES..).ok_or(DecodeError::Unrecognized)?;
    let height = body.len() / LINE_BYTES;
    if body.len() % LINE_BYTES != 0 || height == 0 || height > MAX_HEIGHT {
        return Err(DecodeError::Unrecognized);
    }
    let palette = data[..PALETTE_BYTES]
        .as_chunks::<3>()
        .0
        .iter()
        .map(|c| {
            if c.iter().any(|&v| v > 15) {
                return Err(DecodeError::Unrecognized);
            }
            let [r, g, b] = c.map(|v| widen_channel(u32::from(v), 4));
            Ok(r << 16 | g << 8 | b)
        })
        .collect::<Result<Vec<u32>, _>>()?;

    let plane_len = body.len() / 4;
    let indices: Vec<u8> = planar_pixels(body, WIDTH, height, WIDTH / 8, 4, |plane, y| {
        plane * plane_len + y * (WIDTH / 8)
    })
    .into_iter()
    .map(|v| v as u8)
    .collect();
    Image::from_indexed(WIDTH as u32, height as u32, &indices, &palette)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn planes_and_palette_range() {
        let mut data = vec![0u8; PALETTE_BYTES];
        data[3..6].copy_from_slice(&[15, 0, 0]);
        // One line: plane 0 holds a single leading bit.
        let mut body = vec![0u8; LINE_BYTES];
        body[0] = 0x80;
        data.extend(&body);
        let image = decode_ebd(&data).unwrap();
        assert_eq!((image.width(), image.height()), (640, 1));
        assert_eq!(image.get(0, 0), 0xff0000);
        assert_eq!(image.get(1, 0), 0);
        data[0] = 16;
        assert!(decode_ebd(&data).is_err());
    }
}
