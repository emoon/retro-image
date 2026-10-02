//! ColorViewSquash (RGB): three frames shown through red, green and blue
//! tinted hues so that the eye mixes them into a colour picture.
//!
//! Sources:
//! - Format name, `RGB1` signature and the three-frame idea: Just Solve
//!   "ColorViewSquash" (<http://fileformats.archiveteam.org/wiki/ColorViewSquash>)
//!   and atari-owner.com "Atari Software Graphic Modes", ColorView
//!   (<https://atari-owner.com/club/articles/atari-software-graphic-modes.17/>).
//!   Neither describes the layout or the packer.
//! - Everything else was reverse engineered from the corpus samples (ASTROS,
//!   SEXY01, GOBLIN, CLOWN, BLUEYESD) and by black-box probing of `recoil2png`
//!   with hand-made files: one picture unit per token with the three frames'
//!   values set one at a time, size and header scans.
//!
//! Layout found:
//! - `RGB1`, a title length, the title, the mode (9 or 15), the width in
//!   4-pixel units (even, 2-80), the height (1-192) and the byte 1.
//! - The picture is a column-major list of pixels (192 per column), each
//!   three 4-bit values, one per frame, packed as a nibble stream:
//!   a nibble 1-7 is followed by a triple repeated 2-8 times; 0 is followed by
//!   a nibble N and a triple repeated N + 8 times; 9-15 is followed by 1-7
//!   literal triples; 8 is followed by N and N + 7 literal triples. Runs past
//!   the end of the picture are cut off, missing data is an error.
//! - Mode 9: each value is a luminance 0-15 of GTIA mode 9; mode 15: two
//!   2-bit pixels (high bits first) from the colours 0, 4, 10, 14 of the
//!   frame's hue, except that value 0 is black. The frames' hues are 3
//!   (red-orange), 12 (green) and 7 (blue); the screen colour is the average
//!   of the three frame colours (rounded down per channel). Mode 15 pictures
//!   are 2 pixels wide per value, so every unit is 4 pixels wide as well.

use super::palette::rgb;
use crate::{DecodeError, Image};
use alloc::vec::Vec;

const HUES: [u8; 3] = [0x30, 0xc0, 0x70];
/// The luminances of the four values of a Graphics 15 pixel.
const GR15_LUMINANCE: [u8; 4] = [0, 4, 10, 14];

pub(super) fn decode_rgb(data: &[u8]) -> Result<Image, DecodeError> {
    let rest = data
        .strip_prefix(b"RGB1")
        .ok_or(DecodeError::Unrecognized)?;
    let (&title, rest) = rest.split_first().ok_or(DecodeError::Unrecognized)?;
    let &[mode, width, height, 1, ..] = rest.get(usize::from(title)..).unwrap_or(&[]) else {
        return Err(DecodeError::Unrecognized);
    };
    let packed = &rest[usize::from(title) + 4..];
    let (width, height) = (usize::from(width), usize::from(height));
    if !matches!(mode, 9 | 15)
        || width == 0
        || !width.is_multiple_of(2)
        || width > 80
        || !(1..=192).contains(&height)
    {
        return Err(DecodeError::Unrecognized);
    }
    let pixels = unpack(packed, width * height)?;
    let mut image = Image::new(4 * width as u32, height as u32);
    for x in 0..4 * width {
        for y in 0..height {
            let [a, b, c] = pixels[x / 4 * height + y];
            let values = [a, b, c];
            let color = |frame: usize| {
                let value = values[frame];
                let luminance = if mode == 9 {
                    value
                } else {
                    // Two 2-bit pixels per value; 0 is black in every hue.
                    let index = usize::from(value >> (2 - 2 * (x / 2 % 2)) & 3);
                    match index {
                        0 => return 0,
                        i => GR15_LUMINANCE[i],
                    }
                };
                rgb(HUES[frame] | luminance)
            };
            image.set(x as u32, y as u32, average(color(0), color(1), color(2)));
        }
    }
    Ok(image)
}

/// The per-channel average of three `0xRRGGBB` colours, rounded down.
fn average(a: u32, b: u32, c: u32) -> u32 {
    let channel =
        |shift: u32| ((a >> shift & 0xff) + (b >> shift & 0xff) + (c >> shift & 0xff)) / 3;
    channel(16) << 16 | channel(8) << 8 | channel(0)
}

/// Unpacks `count` pixels of three frame values from the nibble stream.
fn unpack(packed: &[u8], count: usize) -> Result<Vec<[u8; 3]>, DecodeError> {
    let mut nibbles = packed.iter().flat_map(|&byte| [byte >> 4, byte & 0x0f]);
    let mut next = || nibbles.next().ok_or(DecodeError::Unrecognized);
    let mut pixels = Vec::with_capacity(count);
    while pixels.len() < count {
        let token = next()?;
        let (repeat, literals) = match token {
            1..=7 => (usize::from(token) + 1, 1),
            0 => (usize::from(next()?) + 8, 1),
            8 => (1, usize::from(next()?) + 7),
            _ => (1, usize::from(token) - 8),
        };
        // A run repeats its one triple; otherwise each literal triple is one pixel.
        for _ in 0..literals {
            let triple = [next()?, next()?, next()?];
            let room = count - pixels.len();
            pixels.extend(core::iter::repeat_n(triple, repeat.min(room)));
            if room <= repeat {
                break;
            }
        }
    }
    Ok(pixels)
}
