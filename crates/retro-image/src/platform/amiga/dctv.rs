//! Digital Creations DCTV pictures, stored as 3- or 4-plane hires ILBMs.
//!
//! No public description of the encoding was found (the survey is
//! `docs/research/amiga-apple-misc.md`, "Amiga DCTV" and "Wave 5"). Everything
//! here was reverse engineered from the samples under `corpus/` and by
//! black-box probing of `recoil2png` with hand-built ILBMs, and verified
//! pixel-for-pixel on all ten DCTV samples:
//!
//! - The picture is a digital stand-in for a composite video signal. Two
//!   neighbouring pixels `(a, b)` form one 8-bit sample whose bits are
//!   interleaved: `b0 a0 b1 a1 b2 a2 b3 a3` from the least significant bit.
//!   3-plane files are 4-plane files without the lowest plane, so each pixel is
//!   shifted left by one first.
//! - The first row (the first two rows when CAMG says interlaced) is a
//!   signature: the top bit-plane of its first 256 pixels spells a fixed 32
//!   bytes. The other planes of those rows are ignored.
//! - After that, every row is one scan line. Lines alternate between two
//!   phases, A and B. Phase A pairs pixels `(2j, 2j+1)` and puts the result at
//!   output `x = 2j, 2j+1`; phase B pairs `(2j+1, 2j+2)`, lands one pixel to the
//!   right, and always has one more (zero) pair at the end of the line.
//! - Luma is the mean of a sample and its predecessor, `(v - 64) * 8 / 5`.
//!   Chroma is the alternating-sign `[1, 2, 1] / 4` filter of the samples. A
//!   phase A line carries the red-difference component and a phase B line the
//!   blue-difference one; each line takes the other component from the line
//!   before it, so one scan line needs two lines of data.
//! - Interlaced pictures hold two such fields in alternate rows, each decoded
//!   independently. Non-interlaced pictures show every line twice.
//! - Chroma-to-RGB constants were fitted: all were narrowed to intervals a few
//!   thousandths wide, and the rationals used below lie inside every interval.

use alloc::vec::Vec;

use super::ilbm::{CAMG_HIRES, CAMG_LACE, read_ilbm};
use crate::image::check_size;
use crate::{DecodeError, Image};

/// The top-plane bit pattern of the signature, one bit per pixel, most
/// significant bit first.
const SIGNATURE: [u8; 32] = [
    0x00, 0x49, 0x87, 0x28, 0xde, 0x11, 0x0b, 0xef, 0xd2, 0x0c, 0x8e, 0x8b, 0x35, 0x5b, 0x75, 0xec,
    0xb8, 0x29, 0x6b, 0x03, 0xf9, 0x2b, 0xb4, 0x34, 0xee, 0x67, 0x1e, 0x7c, 0x4f, 0x53, 0x63, 0x15,
];

/// Whether `row` (the first pixel row of a bitmap with `planes` planes) starts
/// with the DCTV signature.
pub(super) fn has_signature(planes: usize, row: &[u32]) -> bool {
    let Some(row) = row.get(..SIGNATURE.len() * 8) else {
        return false;
    };
    matches!(planes, 3 | 4)
        && row.iter().enumerate().all(|(x, &pixel)| {
            let top = pixel >> (planes - 1) & 1;
            top == u32::from(SIGNATURE[x / 8] >> (7 - x % 8) & 1)
        })
}

pub(super) fn decode(contents: &[u8]) -> Result<Image, DecodeError> {
    let bitmap = read_ilbm(contents)?;
    let camg = bitmap.camg.unwrap_or(0);
    let lace = camg & CAMG_LACE != 0;
    let header = &bitmap.header;
    let signature_rows = if lace { 2 } else { 1 };
    let signed = (0..signature_rows).all(|y| {
        bitmap
            .row(y)
            .is_some_and(|row| has_signature(header.planes, row))
    });
    if camg & CAMG_HIRES == 0 || !signed || header.height <= signature_rows {
        return Err(DecodeError::Unrecognized);
    }
    let lines = header.height - signature_rows;
    let (width, fields) = (header.width, if lace { 2 } else { 1 });
    let shift = 4 - header.planes;
    let out_height = if lace { lines } else { lines * 2 };
    check_size(width, out_height)?;
    let mut image = Image::new(width as u32, out_height as u32)?;
    for field in 0..fields {
        let mut previous = Vec::new();
        for (index, row) in (field..lines).step_by(fields).enumerate() {
            let samples = bitmap
                .row(signature_rows + row)
                .ok_or(DecodeError::Unrecognized)?;
            let line = Line::new(samples, shift, index % 2 == 1);
            let chroma = line.chroma();
            let y = if lace { row } else { row * 2 };
            for (x, color) in line.pixels(&chroma, &previous, width) {
                image.set(x as u32, y as u32, color);
                if !lace {
                    image.set(x as u32, y as u32 + 1, color);
                }
            }
            previous = chroma;
        }
    }
    Ok(image)
}

/// One scan line: its 8-bit samples, and which of the two phases it is in.
struct Line {
    samples: Vec<i32>,
    /// Phase B lines are shifted one pixel to the right.
    odd: bool,
}

impl Line {
    fn new(pixels: &[u32], shift: usize, odd: bool) -> Self {
        let first = usize::from(odd);
        let nibble = |x: usize| pixels.get(x).map_or(0, |&p| (p << shift) as i32 & 15);
        let samples = (first..pixels.len())
            .step_by(2)
            .map(|x| {
                // A trailing pixel without a partner makes a zero sample.
                if x + 1 >= pixels.len() {
                    return 0;
                }
                (0..4).fold(0, |v, bit| {
                    v | (nibble(x + 1) >> bit & 1) << (2 * bit)
                        | (nibble(x) >> bit & 1) << (2 * bit + 1)
                })
            })
            .collect();
        Self { samples, odd }
    }

    /// Luma of the line, one value per sample.
    fn luma(&self, index: usize) -> i32 {
        let before = index.checked_sub(1).map_or(0, |i| self.samples[i]);
        let mean = (self.samples[index] + before) >> 1;
        ((mean - 64) * 8 / 5).clamp(0, 255)
    }

    /// The chroma component this line carries, one value per sample.
    fn chroma(&self) -> Vec<i32> {
        // Samples alternate in sign: the subcarrier flips every sample.
        let signed = |i: Option<usize>| {
            i.map_or(0, |i| match i % 2 {
                0 => self.samples[i],
                _ => -self.samples[i],
            })
        };
        (0..self.samples.len())
            .map(|m| {
                let sum = signed(Some(m)) + 2 * signed(m.checked_sub(1)) + signed(m.checked_sub(2));
                sum / 4
            })
            .collect()
    }

    /// `(x, 0xRRGGBB)` for every pixel the line covers; `previous` is the
    /// chroma of the line before it in the same field.
    fn pixels<'a>(
        &'a self,
        chroma: &'a [i32],
        previous: &'a [i32],
        width: usize,
    ) -> impl Iterator<Item = (usize, u32)> + 'a {
        let first = usize::from(self.odd);
        (0..self.samples.len()).flat_map(move |m| {
            let own = chroma[m];
            let other = previous.get(m).copied().unwrap_or(0);
            // Phase A carries red-difference, phase B blue-difference.
            let (red_diff, blue_diff) = if self.odd { (other, own) } else { (own, other) };
            let y = self.luma(m);
            let r = y - red_diff * 1164 / 640;
            let g = y + (red_diff * 593 - blue_diff * 404) / 640;
            let b = y + blue_diff * 4143 / 1280;
            let color = [r, g, b]
                .iter()
                .fold(0, |rgb, &c| rgb << 8 | c.clamp(0, 255) as u32);
            (0..2)
                .map(move |dx| first + 2 * m + dx)
                .filter(move |&x| x < width)
                .map(move |x| (x, color))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interleaves_pixel_pairs_into_samples() {
        // (a, b) = (8, 8) -> bits a3 and b3 -> 128 + 64.
        let line = Line::new(&[8, 8, 8, 8], 0, false);
        assert_eq!(line.samples, [192, 192]);
        // 3-plane pixels are shifted up one plane first.
        assert_eq!(Line::new(&[4, 4], 1, false).samples, [192]);
        // Phase B skips the first pixel; an unpaired last pixel is zero.
        assert_eq!(Line::new(&[0, 8, 8, 5], 0, true).samples, [192, 0]);
        // Gray: a steady 192 is (192 - 64) * 8 / 5 = 204.
        assert_eq!(line.luma(1), 204);
        assert_eq!(line.chroma(), [48, 48]);
    }

    #[test]
    fn doubled_height_over_the_pixel_cap_is_rejected() {
        let (width, height, planes) = (65535usize, 520usize, 3usize);
        let row_len = width.div_ceil(16) * 2;
        let mut body = alloc::vec![0u8; row_len * planes * height];
        // Top plane of the first row carries the signature.
        body[2 * row_len..2 * row_len + SIGNATURE.len()].copy_from_slice(&SIGNATURE);
        let mut bmhd = alloc::vec![0u8; 20];
        bmhd[..2].copy_from_slice(&(width as u16).to_be_bytes());
        bmhd[2..4].copy_from_slice(&(height as u16).to_be_bytes());
        bmhd[8] = planes as u8;
        let mut contents = Vec::new();
        for (id, data) in [
            (b"BMHD", bmhd),
            (b"CAMG", alloc::vec![0, 0, 0x80, 0]),
            (b"BODY", body),
        ] {
            contents.extend_from_slice(id);
            contents.extend_from_slice(&(data.len() as u32).to_be_bytes());
            contents.extend_from_slice(&data);
        }
        assert!(matches!(decode(&contents), Err(DecodeError::Unrecognized)));
    }

    #[test]
    fn signature_needs_the_top_plane_pattern() {
        let mut row: Vec<u32> = (0..256)
            .map(|x| u32::from(SIGNATURE[x / 8] >> (7 - x % 8) & 1) << 3)
            .collect();
        assert!(has_signature(4, &row));
        assert!(!has_signature(3, &row));
        row[255] ^= 8;
        assert!(!has_signature(4, &row));
    }
}
