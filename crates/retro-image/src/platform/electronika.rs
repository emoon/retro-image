//! Electronika BK and Electronika MC 0515 screen dumps.
//!
//! Sources:
//! - BK-0010 screen (16 KB, 64 bytes per line, 256 lines; colour mode 2 bits
//!   per pixel, black/blue/green/red): Electronika BK hardware overviews
//!   listed in `docs/formats/sinclair-cpc-bbc-misc.md`.
//! - BK pixel order (lowest bits leftmost): reverse engineered from samples
//!   and `recoil2png` output.
//! - MC 0515 640x200 monochrome screen (16000 bytes): emuverse,
//!   <https://emuverse.ru/wiki/Электроника_МС_0515>. Line order, bit order
//!   and output with rows doubled to 640x400: observed from `recoil2png`
//!   output.

use crate::{DecodeError, Format, Image};

pub(super) static FORMATS: &[Format] = &[
    Format::new("Electronika BK", "Colour screen", &["pic"], decode_bk_pic),
    Format::new("Electronika MC 0515", "Screen", &["scr"], decode_mc0515),
];

const BK_SCREEN_LEN: usize = 16384;

/// 256x256, 4 pixels per byte, the lowest bit pair leftmost.
fn decode_bk_pic(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != BK_SCREEN_LEN {
        return Err(DecodeError::Unrecognized);
    }
    const COLORS: [u32; 4] = [0x000000, 0x0000ff, 0x00ff00, 0xff0000];
    let mut image = Image::new(256, 256);
    for (i, &byte) in data.iter().enumerate() {
        let (x, y) = ((i % 64 * 4) as u32, (i / 64) as u32);
        for n in 0..4 {
            image.set(x + n, y, COLORS[usize::from((byte >> (2 * n)) & 3)]);
        }
    }
    Ok(image)
}

const MC0515_LEN: usize = 16000;

/// 640x200 monochrome, 80 bytes per line, most significant bit leftmost.
fn decode_mc0515(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != MC0515_LEN {
        return Err(DecodeError::Unrecognized);
    }
    let mut image = Image::new(640, 400);
    for (i, &byte) in data.iter().enumerate() {
        let (x, y) = ((i % 80 * 8) as u32, (i / 80) as u32);
        for bit in 0..8 {
            let color = if byte & (0x80 >> bit) != 0 {
                0xffffff
            } else {
                0
            };
            image.set(x + bit, 2 * y, color);
            image.set(x + bit, 2 * y + 1, color);
        }
    }
    Ok(image)
}
