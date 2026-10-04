//! Pictures whose scanlines alternate between luminance lines and GTIA mode
//! 11 hue lines, so the eye mixes them into more colours: APAC (Any Point,
//! Any Colour, 256 colours) and Champions' Interlace (CIN).
//!
//! Sources:
//! - Mode descriptions: AtariWiki "APAC Graphics Mode"
//!   (<https://atariwiki.org/wiki/Wiki.jsp?page=APAC+Graphics+Mode>),
//!   atari-owner.com "Atari Software Graphic Modes"
//!   (<https://atari-owner.com/club/articles/atari-software-graphic-modes.17/>),
//!   De Re Atari App. E (<https://www.atariarchives.org/dere/chaptE.php>).
//! - Sizes: Just Solve "AP*" (<http://fileformats.archiveteam.org/wiki/AP*>),
//!   "Digi Paint" (<http://fileformats.archiveteam.org/wiki/Digi_Paint>) and
//!   "Champions' Interlace"
//!   (<http://fileformats.archiveteam.org/wiki/Champions%27_Interlace>),
//!   AtariWiki File Suffix
//!   (<https://atariwiki.org/wiki/Wiki.jsp?page=File+Suffix>).
//! - Observed from `recoil2png` output: the plane layouts below, and how
//!   colours are formed. A luminance line takes its hue from the hue line
//!   above; a hue line takes the average (rounded down) of the luminance
//!   lines above and below; missing neighbours count as 0. Interlaced
//!   pictures show both line parities, one per frame, and are drawn as the
//!   average (rounded down) of the two frames' RGB values. In CIN the
//!   luminance lines are Graphics 15 lines; on scanline 0, which has no hue
//!   line above, the Graphics 15 colour is shown unchanged.

use super::palette::{register_rgb, rgb};
use super::screen::GREY_COLORS;
use crate::{DecodeError, Image};
use alloc::vec::Vec;

const LINE: usize = 40;

/// 80x96 APAC stored as 96 hue lines, then 96 luminance lines (256, AP2).
/// A 7684-byte file has 4 unused bytes at the end.
pub(super) fn decode_planar(data: &[u8]) -> Result<Image, DecodeError> {
    if !matches!(data.len(), 7680 | 7684) {
        return Err(DecodeError::Unrecognized);
    }
    let (hue, luminance) = (&data[..3840], &data[3840..7680]);
    apac_80x96(hue, luminance)
}

/// 80x96 APAC stored as alternating hue and luminance lines (APA, APC, PLM).
/// A 7720-byte file has 40 unused bytes at the end.
pub(super) fn decode_interleaved(data: &[u8]) -> Result<Image, DecodeError> {
    if !matches!(data.len(), 7680 | 7720) {
        return Err(DecodeError::Unrecognized);
    }
    let (hue, luminance) = deinterleave(&data[..7680]);
    apac_80x96(&hue, &luminance)
}

/// Interlaced 80x192 APAC: 192 luminance lines, then 192 hue lines at offset
/// 7680 (15360 or 15362 bytes) or 8192 (15872 bytes).
pub(super) fn decode_interlaced(data: &[u8]) -> Result<Image, DecodeError> {
    let hue_offset = match data.len() {
        15360 | 15362 => 7680,
        15872 => 8192,
        _ => return Err(DecodeError::Unrecognized),
    };
    let (luminance, hue) = (&data[..7680], &data[hue_offset..hue_offset + 7680]);
    let picture = Scanlines {
        lines: 192,
        luminance: |y, x| nibble(luminance, y, x / 2),
        hue: |y, x| nibble(hue, y, x / 2),
        top: |x| rgb(nibble(luminance, 0, x / 2)),
    };
    picture.render(INTERLACED)
}

/// Champions' Interlace: Graphics 15 luminance lines, then GTIA mode 11 hue
/// lines, interlaced. 16004 bytes: 200 lines of each, then background and
/// playfield 0-2. 16384 bytes: 192 lines of each, then four 256-byte
/// tables of per-line colours. 15360 bytes: 192 lines of each in greys.
pub(super) fn decode_cin(data: &[u8]) -> Result<Image, DecodeError> {
    let (lines, colors): (usize, &dyn Fn(usize, u8) -> u8) = match data.len() {
        16004 => (200, &|_, value| data[16000 + usize::from(value)]),
        16384 => (192, &|y, value| data[15360 + 256 * usize::from(value) + y]),
        15360 => (192, &|_, value| GREY_COLORS[usize::from(value)]),
        _ => return Err(DecodeError::Unrecognized),
    };
    let (gr15, hue) = data.split_at(lines * LINE);
    let register = |y: usize, x: usize| {
        let value = (gr15[y * LINE + x / 4] >> (6 - 2 * (x % 4))) & 3;
        colors(y, value) & 0xfe
    };
    let picture = Scanlines {
        lines,
        luminance: |y, x| register(y, x) & 0x0f,
        hue: |y, x| nibble(hue, y, x / 2),
        top: |x| register_rgb(register(0, x)),
    };
    picture.render(INTERLACED)
}

/// 80x96 APAC: every hue/luminance line pair is two scanlines, hue first.
pub(super) fn apac_80x96(hue: &[u8], luminance: &[u8]) -> Result<Image, DecodeError> {
    let picture = Scanlines {
        lines: 192,
        luminance: |y, x| nibble(luminance, y / 2, x / 2),
        hue: |y, x| nibble(hue, y / 2, x / 2),
        top: |x| rgb(nibble(luminance, 0, x / 2)),
    };
    picture.render(&[true])
}

/// Both frames of an interlaced picture: one with hue lines on even
/// scanlines, one on odd.
pub(super) const INTERLACED: &[bool] = &[false, true];

/// Splits alternating lines (hue first) into two planes.
pub(super) fn deinterleave(data: &[u8]) -> ([u8; 3840], [u8; 3840]) {
    let mut planes = ([0; 3840], [0; 3840]);
    for (i, pair) in data.as_chunks::<{ 2 * LINE }>().0.iter().enumerate() {
        planes.0[i * LINE..][..LINE].copy_from_slice(&pair[..LINE]);
        planes.1[i * LINE..][..LINE].copy_from_slice(&pair[LINE..]);
    }
    planes
}

/// 4-bit pixel `x` of line `y` in a plane of 40-byte lines.
pub(super) fn nibble(plane: &[u8], y: usize, x: usize) -> u8 {
    let byte = plane[y * LINE + x / 2];
    if x.is_multiple_of(2) {
        byte >> 4
    } else {
        byte & 0x0f
    }
}

/// A picture of `lines` scanlines, 160 half-pixels wide (2 output pixels
/// each). `luminance` and `hue` give the values of half-pixel `x` on
/// scanline `y` when shown as that kind of line; `top` gives the colour of
/// scanline 0 shown as a luminance line.
pub(super) struct Scanlines<L, H, T> {
    pub lines: usize,
    pub luminance: L,
    pub hue: H,
    pub top: T,
}

impl<L, H, T> Scanlines<L, H, T>
where
    L: Fn(usize, usize) -> u8,
    H: Fn(usize, usize) -> u8,
    T: Fn(usize) -> u32,
{
    /// Renders 320 pixels wide, averaging one frame per entry of `frames`;
    /// an entry tells whether even scanlines are hue lines in that frame.
    pub fn render(&self, frames: &[bool]) -> Result<Image, DecodeError> {
        let frames: Vec<Image> = frames
            .iter()
            .map(|&even_hue| {
                let mut image = Image::new(160, self.lines as u32);
                for y in 0..self.lines {
                    for x in 0..160 {
                        let rgb = self.color(y, x, (y % 2 == 0) == even_hue);
                        image.set(x as u32, y as u32, rgb);
                    }
                }
                image
            })
            .collect();
        let frames: Vec<&Image> = frames.iter().collect();
        Image::blend(&frames).scaled(2, 1)
    }

    /// Colour of half-pixel `x` on scanline `y`, shown as a hue or luminance line.
    fn color(&self, y: usize, x: usize, hue_line: bool) -> u32 {
        let luminance = |y: usize| {
            if y < self.lines {
                (self.luminance)(y, x)
            } else {
                0
            }
        };
        if hue_line {
            let above = y.checked_sub(1).map_or(0, luminance);
            let below = luminance(y + 1);
            rgb(((self.hue)(y, x) << 4) | ((above + below) / 2))
        } else {
            match y.checked_sub(1) {
                Some(above) => rgb(((self.hue)(above, x) << 4) | luminance(y)),
                None => (self.top)(x),
            }
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
        assert_eq!(image.get(0, 0), rgb(0x21));
        // Scanline 1 is a luminance line under hue 2.
        assert_eq!(image.get(0, 1), rgb(0x23));
    }

    #[test]
    fn cin_top_line_keeps_its_colour() {
        let mut data = [0u8; 16004];
        data[0] = 0xc0; // Graphics 15 line 0: playfield 2
        data[16000..].copy_from_slice(&[0x00, 0x24, 0x46, 0x88]);
        let image = decode_cin(&data).unwrap();
        // Frame 1 shows 0x88 itself, frame 2 a hue line with luminance (0 + 0) / 2.
        assert_eq!(image.get(0, 0), 0x244a76);
    }

    #[test]
    fn rejects_other_sizes() {
        assert!(decode_planar(&[0; 7720]).is_err());
        assert!(decode_interleaved(&[0; 7684]).is_err());
        assert!(decode_interlaced(&[0; 15361]).is_err());
        assert!(decode_cin(&[0; 16005]).is_err());
    }
}
