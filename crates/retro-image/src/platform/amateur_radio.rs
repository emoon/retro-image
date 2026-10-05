//! Slow-scan television (amateur radio) `.hrz` pictures.
//!
//! Sources:
//! - Just Solve the File Format Problem, Slow-scan television
//!   (<http://fileformats.archiveteam.org/wiki/Slow-scan_television>, CC0): a
//!   headerless RGB bitmap of 256 x 240 pixels, 184320 bytes.
//! - ImageMagick's HRZ coder, `coders/hrz.c`
//!   (<https://github.com/ImageMagick/ImageMagick/blob/main/coders/hrz.c>,
//!   Apache-2.0; cited for the facts, no code is copied): 256 x 240, red,
//!   green, blue in that order, each sample multiplied by 4.
//! - Checked on the one sample in `corpus/extra/misc-computers/sstv`: every
//!   byte is 0 to 64, so a sample is 6 bits with 64 as full scale, and four
//!   times the value shows a normally exposed photograph with natural colors
//!   (a wooden table is orange, not blue). The product 256 of a sample of 64
//!   is capped at 255 here, a choice of this crate.
//!
//! Recognized by extension and size only. ImageMagick was not available to
//! run, so there is no second decoder; the sample was reviewed by eye.

use crate::{DecodeError, Format, Image};

pub(super) static FORMATS: &[Format] = &[Format::new(
    "Amateur radio",
    "Slow-scan television HRZ",
    &["hrz"],
    decode_hrz,
)];

const WIDTH: usize = 256;
const HEIGHT: usize = 240;

fn decode_hrz(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != WIDTH * HEIGHT * 3 {
        return Err(DecodeError::Unrecognized);
    }
    let level = |sample: u8| u32::from(sample).saturating_mul(4).min(255);
    let colors = data
        .as_chunks::<3>()
        .0
        .iter()
        .map(|&[r, g, b]| level(r) << 16 | level(g) << 8 | level(b));
    Ok(Image::from_colors(WIDTH as u32, HEIGHT as u32, colors))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_are_six_bit_red_green_blue_and_the_size_is_exact() {
        let mut data = alloc::vec![0u8; WIDTH * HEIGHT * 3];
        data[..3].copy_from_slice(&[63, 32, 1]);
        data[3..6].copy_from_slice(&[64, 0, 0]);
        let image = decode_hrz(&data).unwrap();
        assert_eq!((image.width(), image.height()), (256, 240));
        assert_eq!(image.get(0, 0), 0xfc_8004);
        assert_eq!(image.get(1, 0), 0xff_0000);
        assert!(decode_hrz(&data[1..]).is_err());
    }
}
