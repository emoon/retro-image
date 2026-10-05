//! Perfect Pix (Batman Group, 2016): two flickering frames (ODD and EVE
//! companions) described by a PPH header.
//!
//! Sources:
//! - PPH holds the mode, size and palette; ODD and EVE hold the two frames
//!   as linear lines; mode R uses a half-pixel shift, B0/B1 flicker two
//!   mode 0/1 frames: Perfect Pix manual (PDF on CPC-Power,
//!   <https://www.cpc-power.com/extra_lire_fichier.php?extra=notice&fiche=13139&slot=1&part=B&type=.pdf>;
//!   also in <https://archive.org/details/CPC-PerfectPix>), as summarised
//!   in `docs/research/sinclair-cpc-bbc-misc.md`.
//! - Byte layout (kind 3 = R, 4 = B0, 5 = B1; width in mode 1 pixels and
//!   height as u16; the number of palette zones, then per zone its pens as
//!   firmware colors followed by its line count, none after the last), the
//!   frame blend, and which frame is shifted on which line: reverse
//!   engineered from samples and `recoil2png` output.

use super::amsdos::strip_amsdos;
use super::hardware::{Mode, firmware_color};
use crate::{Companions, DecodeError, Image};

const HEADER_LEN: usize = 6;

/// PPH with its ODD and EVE companions; the header alone has no pixels.
pub(super) fn decode_pph(data: &[u8], companions: &dyn Companions) -> Result<Image, DecodeError> {
    let pph = strip_amsdos(data);
    let header = pph.get(..HEADER_LEN).ok_or(DecodeError::Unrecognized)?;
    let (mode, shifted) = match header[0] {
        3 => (Mode::Zero, true),
        4 => (Mode::Zero, false),
        5 => (Mode::One, false),
        _ => return Err(DecodeError::Unrecognized),
    };
    let width = usize::from(u16::from_le_bytes([header[1], header[2]]));
    let height = usize::from(u16::from_le_bytes([header[3], header[4]]));
    let zones = parse_zones(&pph[HEADER_LEN..], usize::from(header[5]), mode)?;
    let line_bytes = width.div_ceil(4);
    let frame = |extension| {
        let file = companions.get(extension).ok_or(DecodeError::Unrecognized)?;
        let pixels = strip_amsdos(&file);
        if width == 0 || height == 0 || pixels.len() != line_bytes * height {
            return Err(DecodeError::Unrecognized);
        }
        Ok(pixels.to_vec())
    };
    let frames = [frame("odd")?, frame("eve")?];

    // Each byte covers 4 output pixels: 2 mode 0 pixels drawn 2 wide, or 4
    // mode 1 pixels.
    let output_per_pixel = 4 / mode.pixels_per_byte();
    let mut images = [
        Image::new(width as u32, height as u32),
        Image::new(width as u32, height as u32),
    ];
    let mut remaining_zones = zones.iter();
    let mut zone = remaining_zones.next().ok_or(DecodeError::Unrecognized)?;
    let mut zone_end = zone.lines;
    for y in 0..height {
        while y >= zone_end {
            let Some(next) = remaining_zones.next() else {
                break;
            };
            zone = next;
            zone_end = zone_end.saturating_add(next.lines);
        }
        let pens = &zone.pens;
        for (f, (pixels, image)) in frames.iter().zip(&mut images).enumerate() {
            let line = &pixels[y * line_bytes..][..line_bytes];
            // Mode R: on even lines the second frame, on odd lines the first,
            // is shown half a pixel to the left; black fills the gap.
            let shift = usize::from(shifted && f == 1 - y % 2);
            for x in 0..width {
                let color = match x + shift {
                    sx if sx < width => {
                        let p = sx / output_per_pixel;
                        let per_byte = mode.pixels_per_byte();
                        pens[usize::from(mode.pen(line[p / per_byte], p % per_byte))]
                    }
                    _ => 0,
                };
                image.set(x as u32, y as u32, color);
            }
        }
    }
    Ok(Image::blend(&[&images[0], &images[1]]))
}

struct Zone {
    pens: [u32; 16],
    /// Lines the zone covers; the last zone covers the rest.
    lines: usize,
}

/// Palette zones: per zone 16 (mode 0) or 4 (mode 1) firmware colors and,
/// except after the last, a line count. Must fill `data` exactly.
fn parse_zones(
    data: &[u8],
    count: usize,
    mode: Mode,
) -> Result<alloc::vec::Vec<Zone>, DecodeError> {
    let pen_count = 1 << (8 / mode.pixels_per_byte());
    if count == 0 || data.len() != count * (pen_count + 1) - 1 {
        return Err(DecodeError::Unrecognized);
    }
    data.chunks(pen_count + 1)
        .map(|chunk| {
            let mut pens = [0; 16];
            for (pen, &color) in pens.iter_mut().zip(&chunk[..pen_count]) {
                if color > 26 {
                    return Err(DecodeError::Unrecognized);
                }
                *pen = firmware_color(usize::from(color));
            }
            let lines = chunk.get(pen_count).map_or(usize::MAX, |&n| usize::from(n));
            Ok(Zone { pens, lines })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use alloc::vec::Vec;

    struct Frames(Vec<u8>, Vec<u8>);

    impl Companions for Frames {
        fn get_named(&self, _file_name: &str) -> Option<Vec<u8>> {
            None
        }
        fn get(&self, extension: &str) -> Option<Vec<u8>> {
            match extension {
                "odd" => Some(self.0.clone()),
                "eve" => Some(self.1.clone()),
                _ => None,
            }
        }
    }

    /// Mode 1 header, 4 pixels by 2 lines, two zones of one line each.
    fn b1_header() -> Vec<u8> {
        let mut pph = vec![5, 4, 0, 2, 0, 2];
        pph.extend_from_slice(&[0, 26, 0, 0, 1]); // white pen 1, 1 line
        pph.extend_from_slice(&[0, 2, 0, 0]); // bright blue pen 1
        pph
    }

    #[test]
    fn frames_blend_and_zones_change_palette() {
        // First pixel pen 1 in the first frame only.
        let frames = Frames(vec![0x80, 0x80], vec![0, 0]);
        let image = decode_pph(&b1_header(), &frames).unwrap();
        assert_eq!(image.get(0, 0), 0x7f7f7f);
        assert_eq!(image.get(0, 1), 0x00007f);
        assert_eq!(image.get(1, 0), 0);
        assert!(decode_pph(&b1_header(), &crate::NoCompanions).is_err());
    }

    #[test]
    fn mode_r_shifts_one_frame_per_line() {
        // 4 output pixels (2 mode 0 pixels) by 2 lines, one zone: pen 1 white.
        let mut pph = vec![3, 4, 0, 2, 0, 1, 0, 26];
        pph.extend_from_slice(&[0; 14]);
        // Second mode 0 pixel is pen 1 in both frames.
        let frames = Frames(vec![0x40, 0x40], vec![0x40, 0x40]);
        let image = decode_pph(&pph, &frames).unwrap();
        // Line 0: the second frame is shifted, so x = 1 blends black and white.
        assert_eq!(image.get(1, 0), 0x7f7f7f);
        assert_eq!(image.get(2, 0), 0xffffff);
        // Rightmost pixel: the shifted frame has nothing there.
        assert_eq!(image.get(3, 1), 0x7f7f7f);
    }
}
