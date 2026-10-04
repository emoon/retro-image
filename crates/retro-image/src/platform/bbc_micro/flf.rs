//! Turbo Rascal Syntax Error (TRSE) "Fluff" images for the BBC Micro.
//!
//! Sources: reverse engineered from `flower1.flf` and `proxima2.flf` by
//! black-box probing of `recoil2png`; TRSE's GPL-3 source was not read.
//! Container: [`crate::codec::flf`]. Colours: the BBC default logical
//! colours, see the parent module.
//!
//! Type 0x1a: byte 13 is the screen mode, 4 (320x256, black and white) or 5
//! (160x256 drawn twice as wide, black/red/yellow/white). The picture is
//! one colour number per byte, read from offset 13 on, so the mode byte
//! itself is the first pixel (it is out of the palette and draws black) and
//! the last stored byte, at the end of the picture, is not shown. That is
//! how `recoil2png` draws it; it looks like an off-by-one in one of the
//! two, which the samples can't settle. The closing block is required and
//! its ink table is ignored.

use super::{FOUR_COLORS, TWO_COLORS, physical_color};
use crate::codec::flf::{self, Fluff, PAYLOAD};
use crate::{DecodeError, Image};

const HEIGHT: usize = 256;
const MODE: usize = PAYLOAD + 1;

pub(super) fn decode_flf(data: &[u8]) -> Result<Image, DecodeError> {
    let fluff = Fluff::parse(data)?;
    if fluff.kind != 0x1a {
        return Err(DecodeError::Unrecognized);
    }
    let (width, logical, scale) = match fluff.byte(MODE)? {
        4 => (320, &TWO_COLORS[..2], 1),
        5 => (160, &FOUR_COLORS[..4], 2),
        _ => return Err(DecodeError::Unrecognized),
    };
    let (pixels, rest) = fluff.split(MODE, width * HEIGHT)?;
    // One more stored byte, then the closing block.
    let (_, rest) = rest.split_first().ok_or(DecodeError::Unrecognized)?;
    flf::trailer(rest, true)?;
    let mut palette = [0u32; 256];
    for (color, &physical) in palette.iter_mut().zip(logical) {
        *color = physical_color(physical);
    }
    let image = Image::from_indexed(width as u32, HEIGHT as u32, pixels, &palette)?;
    image.scaled(scale, 1)
}
