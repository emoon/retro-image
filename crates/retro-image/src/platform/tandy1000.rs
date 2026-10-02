//! Tandy 1000.
//!
//! Sources:
//! - DeskMate Paint (`.PNT`): Deark `misc2.c` (<https://github.com/jsummers/deark>,
//!   MIT licence): signature `$13 "PNT"`, pixels from offset 22, 312x176 at
//!   4 bits per pixel (high nibble first), stored raw or as (value, count)
//!   byte pairs.
//! - Palette: observed from `recoil2png` output.

use alloc::vec::Vec;

use crate::{DecodeError, Format, Image};

/// DeskMate's 16 colours.
const PALETTE: [u32; 16] = [
    0x000000, 0x000099, 0x009900, 0x339999, 0x990000, 0xcc33cc, 0xcc6600, 0x999999, 0x996633,
    0x6633ff, 0x33cc00, 0x66cccc, 0xffcccc, 0xff99ff, 0xffff00, 0xffffff,
];

pub(super) static FORMATS: &[Format] = &[Format::new(
    "Tandy 1000",
    "DeskMate Paint",
    &["pnt"],
    decode_pnt,
)];

const WIDTH: usize = 312;
const HEIGHT: usize = 176;
const PIXELS_AT: usize = 22;

fn decode_pnt(data: &[u8]) -> Result<Image, DecodeError> {
    if !data.starts_with(b"\x13PNT") || data.len() <= PIXELS_AT {
        return Err(DecodeError::Unrecognized);
    }
    let len = WIDTH / 2 * HEIGHT;
    let src = &data[PIXELS_AT..];
    let pixels = if src.len() == len {
        src.to_vec()
    } else {
        let mut out = Vec::with_capacity(len);
        for pair in src.chunks_exact(2) {
            if out.len() >= len {
                break;
            }
            out.resize(out.len() + usize::from(pair[1]), pair[0]);
        }
        out.resize(len, 0);
        out
    };
    let mut image = Image::new(WIDTH as u32, HEIGHT as u32);
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let byte = pixels[(y * WIDTH + x) / 2];
            let index = if x % 2 == 0 { byte >> 4 } else { byte & 15 };
            image.set(x as u32, y as u32, PALETTE[usize::from(index)]);
        }
    }
    Ok(image)
}
