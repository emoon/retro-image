//! Electronika BK and Electronika MC 0515 screen dumps.
//!
//! Sources:
//! - BK-0010 screen (16 KB, 64 bytes per line, 256 lines; color mode 2 bits
//!   per pixel, black/blue/green/red): Electronika BK hardware overviews,
//!   <https://en.wikipedia.org/wiki/Electronika_BK> and
//!   <https://alemorf.github.io/retro_computers/computer.html?id=BK0010>
//!   (more in `docs/research/sinclair-cpc-bbc-misc.md`).
//! - BK pixel order (lowest bits leftmost): reverse engineered from samples
//!   and `recoil2png` output.
//! - BKS screens (16384 bytes mono 512x256, or 16384 + a palette number 0-15
//!   for color; two of either blended as a flickering pair), white-on-black
//!   mono with rows doubled, and the colors of the BK-0011M's 16 palettes:
//!   observed from `recoil2png` output.
//! - MC 0515 640x200 monochrome screen (16000 bytes): emuverse,
//!   <https://emuverse.ru/wiki/Электроника_МС_0515>. Line order, bit order
//!   and output with rows doubled to 640x400: observed from `recoil2png`
//!   output.

use alloc::vec::Vec;

use crate::{DecodeError, Format, Image};

pub(super) static FORMATS: &[Format] = &[
    Format::new("Electronika BK", "Colour screen", &["pic"], decode_bk_pic),
    Format::new("Electronika BK", "Screen", &["bks"], decode_bks),
    Format::new("Electronika MC 0515", "Screen", &["scr"], decode_mc0515),
];

const BK_SCREEN_LEN: usize = 16384;

/// 256x256 color screen in the BK-0010 colors (BK-0011M palette 0).
fn decode_bk_pic(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != BK_SCREEN_LEN {
        return Err(DecodeError::Unrecognized);
    }
    Ok(bk_color(data, 0))
}

/// BKS: one or two screens (two are shown in alternation), followed in
/// color files by one palette number (0-15) per screen.
fn decode_bks(data: &[u8]) -> Result<Image, DecodeError> {
    let screens = data.len() / BK_SCREEN_LEN;
    let (pixels, palettes) = data.split_at(screens.min(2) * BK_SCREEN_LEN);
    let color = match palettes.len() {
        0 => false,
        n if n == screens => true,
        _ => return Err(DecodeError::Unrecognized),
    };
    if !(1..=2).contains(&screens)
        || palettes
            .iter()
            .any(|&p| usize::from(p) >= BK_PALETTES.len())
    {
        return Err(DecodeError::Unrecognized);
    }
    let frames: Vec<Image> = pixels
        .as_chunks::<BK_SCREEN_LEN>()
        .0
        .iter()
        .enumerate()
        .map(|(i, screen)| {
            if color {
                Ok(bk_color(screen, palettes[i]))
            } else {
                bk_mono(screen)
            }
        })
        .collect::<Result<_, DecodeError>>()?;
    let frames: Vec<&Image> = frames.iter().collect();
    Ok(Image::blend(&frames))
}

/// BK-0011M palettes: colors of pixel values 1-3 (0 is black).
const BK_PALETTES: [[u32; 3]; 16] = [
    [0x0000ff, 0x00ff00, 0xff0000],
    [0xffff00, 0xff00ff, 0xff0000],
    [0x00ffff, 0x0000ff, 0xff00ff],
    [0x00ff00, 0x00ffff, 0xffff00],
    [0xff00ff, 0x00ffff, 0xffffff],
    [0xffffff, 0xffffff, 0xffffff],
    [0xc00000, 0x800000, 0xff0000],
    [0x00ff00, 0x00ffff, 0xffff00],
    [0xc000c0, 0x8000ff, 0xff00ff],
    [0xffff00, 0x8000ff, 0xc00000],
    [0xffff00, 0xc000c0, 0xff0000],
    [0x00ffff, 0xffff00, 0xff0000],
    [0xff0000, 0x00ff00, 0x00ffff],
    [0x00ffff, 0xffff00, 0xffffff],
    [0xffff00, 0x00ff00, 0xffffff],
    [0x00ffff, 0x00ff00, 0xffffff],
];

/// 256x256, 4 pixels per byte, the lowest bit pair leftmost.
fn bk_color(screen: &[u8], palette: u8) -> Image {
    let colors = &BK_PALETTES[usize::from(palette)];
    let mut image = Image::new(256, 256);
    for (i, &byte) in screen.iter().enumerate() {
        let (x, y) = ((i % 64 * 4) as u32, (i / 64) as u32);
        for n in 0..4 {
            let color = match (byte >> (2 * n)) & 3 {
                0 => 0,
                value => colors[usize::from(value - 1)],
            };
            image.set(x + n, y, color);
        }
    }
    image
}

/// 512x256 mono, 8 pixels per byte, the lowest bit leftmost; shown with
/// rows doubled.
fn bk_mono(screen: &[u8]) -> Result<Image, DecodeError> {
    let mut image = Image::new(512, 256);
    for (i, &byte) in screen.iter().enumerate() {
        let (x, y) = ((i % 64 * 8) as u32, (i / 64) as u32);
        for bit in 0..8 {
            let color = if byte & (1 << bit) != 0 { 0xffffff } else { 0 };
            image.set(x + bit, y, color);
        }
    }
    image.scaled(1, 2)
}

const MC0515_LEN: usize = 16000;

/// 640x200 monochrome, 80 bytes per line, most significant bit leftmost.
fn decode_mc0515(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != MC0515_LEN {
        return Err(DecodeError::Unrecognized);
    }
    let mut image = Image::new(640, 200);
    for (i, &byte) in data.iter().enumerate() {
        let (x, y) = ((i % 80 * 8) as u32, (i / 80) as u32);
        for bit in 0..8 {
            let color = if byte & (0x80 >> bit) != 0 {
                0xffffff
            } else {
                0
            };
            image.set(x + bit, y, color);
        }
    }
    image.scaled(1, 2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn bks_sizes_select_mono_colour_and_pairs() {
        let mono = decode_bks(&[0xff; BK_SCREEN_LEN]).unwrap();
        assert_eq!((mono.width(), mono.height()), (512, 512));
        let mut color = vec![0x55; BK_SCREEN_LEN];
        color.push(0);
        assert_eq!(decode_bks(&color).unwrap().get(0, 0), 0x0000ff);
        let mut pair = vec![0x55; 2 * BK_SCREEN_LEN];
        pair.extend_from_slice(&[0, 5]);
        // Blue blended with white.
        assert_eq!(decode_bks(&pair).unwrap().get(0, 0), 0x7f7fff);
        color[BK_SCREEN_LEN] = 16;
        assert!(decode_bks(&color).is_err());
        assert!(decode_bks(&[0; 3 * BK_SCREEN_LEN]).is_err());
    }
}
