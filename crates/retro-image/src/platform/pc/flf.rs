//! Turbo Rascal Syntax Error (TRSE) "Fluff" images for the PC.
//!
//! Sources: reverse engineered from `image2cga.flf` and `image2.flf` by
//! black-box probing of `recoil2png`; TRSE's GPL-3 source was not read.
//! Container: [`crate::codec::flf`]. CGA palettes: the IBM CGA 4-colour
//! palettes (green/red/brown, cyan/magenta/white, each low and high
//! intensity), <https://en.wikipedia.org/wiki/Color_Graphics_Adapter>,
//! with the levels `recoil2png` draws.
//!
//! - Type 0x0b (CGA): byte 12 selects the palette (2: cyan/magenta/grey,
//!   3: the same bright, 4: green/red/brown, 5: the same bright), then
//!   320x200 colour numbers (one per byte) from offset 13, then the closing
//!   block, which is required. Numbers 4 and up draw black.
//! - Type 0x1b (VGA): 320x200 paletted, see `codec::flf::decode_paletted`.

use super::cga_set;
use crate::codec::flf::{self, Fluff, PAYLOAD};
use crate::{DecodeError, Image};

const WIDTH: usize = 320;
const HEIGHT: usize = 200;

pub(super) fn decode_flf(data: &[u8]) -> Result<Image, DecodeError> {
    let fluff = Fluff::parse(data)?;
    match fluff.kind {
        0x0b => decode_cga(&fluff),
        0x1b => flf::decode_paletted(&fluff, WIDTH, HEIGHT),
        _ => Err(DecodeError::Unrecognized),
    }
}

fn decode_cga(fluff: &Fluff) -> Result<Image, DecodeError> {
    let set = match fluff.byte(PAYLOAD)? {
        2 => cga_set([3, 5, 7]),
        3 => cga_set([11, 13, 15]),
        4 => cga_set([2, 4, 6]),
        5 => cga_set([10, 12, 14]),
        _ => return Err(DecodeError::Unrecognized),
    };
    let (pixels, rest) = fluff.split(PAYLOAD + 1, WIDTH * HEIGHT)?;
    flf::trailer(rest, true)?;
    let mut palette = [0u32; 256];
    palette[1..4].copy_from_slice(&set[1..]);
    Image::from_indexed(WIDTH as u32, HEIGHT as u32, pixels, &palette)
}
