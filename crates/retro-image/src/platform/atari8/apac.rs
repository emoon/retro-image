//! APAC (Any Point, Any Colour): GTIA mode 9 luminance lines and mode 11 hue
//! lines alternate, so the eye mixes them into 256 colours.
//!
//! Sources:
//! - Mode description: AtariWiki "APAC Graphics Mode", atari-owner.com
//!   "Atari Software Graphic Modes", De Re Atari App. E.
//! - Sizes: Just Solve "AP*" and "Digi Paint", AtariWiki File Suffix.
//! - Observed from `recoil2png` output: the plane layouts below, and how
//!   colours are formed. A luminance line takes its hue from the hue line
//!   above; a hue line takes the average (rounded down) of the luminance
//!   lines above and below; missing neighbours count as 0. Interlaced
//!   pictures show both line parities, one per frame, and are drawn as the
//!   average (rounded down) of the two frames' RGB values.

use super::palette::{average, rgb};
use crate::{DecodeError, Image};

const LINE: usize = 40;
const SCANLINES: usize = 192;

/// 80x96 APAC stored as 96 hue lines, then 96 luminance lines (256, AP2).
/// A 7684-byte file has 4 unused bytes at the end.
pub(super) fn decode_planar(data: &[u8]) -> Result<Image, DecodeError> {
    if !matches!(data.len(), 7680 | 7684) {
        return Err(DecodeError::Unrecognized);
    }
    let picture = Apac {
        hue: &data[..3840],
        luminance: &data[3840..7680],
        line_shift: 1,
    };
    Ok(picture.render(&[true]))
}

/// 80x96 APAC stored as alternating hue and luminance lines (APA, APC, PLM).
/// A 7720-byte file has 40 unused bytes at the end.
pub(super) fn decode_interleaved(data: &[u8]) -> Result<Image, DecodeError> {
    if !matches!(data.len(), 7680 | 7720) {
        return Err(DecodeError::Unrecognized);
    }
    let (hue, luminance) = deinterleave(&data[..7680]);
    let picture = Apac {
        hue: &hue,
        luminance: &luminance,
        line_shift: 1,
    };
    Ok(picture.render(&[true]))
}

/// Interlaced 80x192 APAC: 192 luminance lines, then 192 hue lines at offset
/// 7680 (15360 or 15362 bytes) or 8192 (15872 bytes).
pub(super) fn decode_interlaced(data: &[u8]) -> Result<Image, DecodeError> {
    let hue_offset = match data.len() {
        15360 | 15362 => 7680,
        15872 => 8192,
        _ => return Err(DecodeError::Unrecognized),
    };
    let picture = Apac {
        hue: &data[hue_offset..hue_offset + 7680],
        luminance: &data[..7680],
        line_shift: 0,
    };
    Ok(picture.render(&[false, true]))
}

/// Splits alternating lines (hue first) into two planes.
fn deinterleave(data: &[u8]) -> ([u8; 3840], [u8; 3840]) {
    let mut planes = ([0; 3840], [0; 3840]);
    for (i, pair) in data.chunks_exact(2 * LINE).enumerate() {
        planes.0[i * LINE..][..LINE].copy_from_slice(&pair[..LINE]);
        planes.1[i * LINE..][..LINE].copy_from_slice(&pair[LINE..]);
    }
    planes
}

/// Hue and luminance planes of 4-bit pixels, 40 bytes per line. Scanline
/// `y` reads line `y >> line_shift` of each plane.
struct Apac<'a> {
    hue: &'a [u8],
    luminance: &'a [u8],
    line_shift: u32,
}

impl Apac<'_> {
    /// Renders 320x192, averaging one frame per entry of `frames`; an
    /// entry tells whether even scanlines are hue lines in that frame.
    fn render(&self, frames: &[bool]) -> Image {
        let mut image = Image::new(320, SCANLINES as u32);
        for y in 0..SCANLINES {
            for x in 0..80 {
                let colors = frames
                    .iter()
                    .map(|&even_hue| self.color(y, x, (y % 2 == 0) == even_hue));
                let rgb = colors.reduce(average).unwrap_or(0);
                for dx in 0..4 {
                    image.set(x as u32 * 4 + dx, y as u32, rgb);
                }
            }
        }
        image
    }

    /// Colour of pixel `x` on scanline `y`, shown as a hue or luminance line.
    fn color(&self, y: usize, x: usize, hue_line: bool) -> u32 {
        if hue_line {
            let above = y
                .checked_sub(1)
                .map_or(0, |y| self.nibble(self.luminance, y, x));
            let below = self.nibble(self.luminance, y + 1, x);
            rgb((self.nibble(self.hue, y, x) << 4) | ((above + below) / 2))
        } else {
            let hue = y.checked_sub(1).map_or(0, |y| self.nibble(self.hue, y, x));
            rgb(hue << 4 | self.nibble(self.luminance, y, x))
        }
    }

    /// Pixel `x` of `plane` on scanline `y`; 0 below the last line.
    fn nibble(&self, plane: &[u8], y: usize, x: usize) -> u8 {
        if y >= SCANLINES {
            return 0;
        }
        let byte = plane[(y >> self.line_shift) * LINE + x / 2];
        if x.is_multiple_of(2) {
            byte >> 4
        } else {
            byte & 0x0f
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hue_line_averages_luminance_neighbours() {
        let mut data = [0u8; 7680];
        data[0] = 0x20; // hue line 0, pixel 0: hue 2
        data[3840] = 0x30; // luminance line 0, pixel 0: 3
        let image = decode_planar(&data).unwrap();
        // Scanline 0 is a hue line: luminance (0 + 3) / 2 = 1.
        assert_eq!(&image.rgb()[..3], &rgb_bytes(rgb(0x21)));
        // Scanline 1 is a luminance line under hue 2.
        let i = 320 * 3;
        assert_eq!(&image.rgb()[i..i + 3], &rgb_bytes(rgb(0x23)));
    }

    fn rgb_bytes(rgb: u32) -> [u8; 3] {
        let [_, r, g, b] = rgb.to_be_bytes();
        [r, g, b]
    }

    #[test]
    fn rejects_other_sizes() {
        assert!(decode_planar(&[0; 7720]).is_err());
        assert!(decode_interleaved(&[0; 7684]).is_err());
        assert!(decode_interlaced(&[0; 15361]).is_err());
    }
}
