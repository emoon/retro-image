//! Atari 8-bit VBXE: DAP (SlideShow for VBXE).
//!
//! Sources:
//! - Just Solve "SlideShow for VBXE"
//!   (<http://fileformats.archiveteam.org/wiki/SlideShow_for_VBXE>): exactly
//!   77568 bytes; VBXE FX core programmer's manual
//!   (<https://www.mathyvannisselroy.nl/VBXE/VBXE%20fx_en.pdf>): 8 bits per
//!   pixel overlay, 256-entry RGB palette.
//! - Layout (320x240 pixels, then the palette as 256 red, 256 green and
//!   256 blue bytes, used unscaled): observed from `recoil2png` output.

use crate::{DecodeError, Image};

const WIDTH: usize = 320;
const HEIGHT: usize = 240;
const PIXELS: usize = WIDTH * HEIGHT;

pub(super) fn decode_dap(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != PIXELS + 768 {
        return Err(DecodeError::Invalid);
    }
    let (pixels, planes) = data.split_at(PIXELS);
    let palette: [u32; 256] = core::array::from_fn(|index| {
        let channel = |plane: usize| u32::from(planes[plane * 256 + index]);
        channel(0) << 16 | channel(1) << 8 | channel(2)
    });
    Image::from_indexed(WIDTH as u32, HEIGHT as u32, pixels, &palette)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_is_planar() {
        let mut data = alloc::vec![0u8; PIXELS + 768];
        data[0] = 5;
        data[PIXELS + 5] = 0x12;
        data[PIXELS + 256 + 5] = 0x34;
        data[PIXELS + 512 + 5] = 0x56;
        let image = decode_dap(&data).unwrap();
        assert_eq!(&image.rgb()[..3], &[0x12, 0x34, 0x56]);
        assert!(decode_dap(&data[1..]).is_err());
    }
}
