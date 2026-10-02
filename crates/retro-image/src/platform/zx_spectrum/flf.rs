//! Turbo Rascal Syntax Error (TRSE) "Fluff" images for the ZX Spectrum.
//!
//! Sources: reverse engineered from `image12.flf` by black-box probing of
//! `recoil2png`; TRSE's GPL-3 source was not read. Container:
//! [`crate::codec::flf`]. Colour values: the eight Spectrum colours at the
//! `0xcd` level, as `recoil2png` draws them.
//!
//! Type 0x1c: byte 12 is `0x0e`, then 256x192 colour numbers (one per byte)
//! from offset 13, then the optional closing block. Numbers 8-15 draw like
//! 0-7 and 16 and above draw black, as `recoil2png` does.

use crate::codec::flf::{self, Fluff, PAYLOAD};
use crate::{DecodeError, Image};

const WIDTH: usize = 256;
const HEIGHT: usize = 192;

pub(super) fn decode_flf(data: &[u8]) -> Result<Image, DecodeError> {
    let fluff = Fluff::parse(data)?;
    if fluff.kind != 0x1c || fluff.byte(PAYLOAD)? != 0x0e {
        return Err(DecodeError::Unrecognized);
    }
    let (pixels, rest) = fluff.split(PAYLOAD + 1, WIDTH * HEIGHT)?;
    flf::trailer(rest, false)?;
    let mut palette = [0u32; 256];
    for (n, color) in palette.iter_mut().enumerate().take(16) {
        let level = |bit: usize| if n & bit != 0 { 0xcd } else { 0 };
        *color = level(2) << 16 | level(4) << 8 | level(1);
    }
    Image::from_indexed(WIDTH as u32, HEIGHT as u32, pixels, &palette)
}
