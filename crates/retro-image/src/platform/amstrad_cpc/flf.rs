//! Turbo Rascal Syntax Error (TRSE) "Fluff" images for the Amstrad CPC.
//!
//! Sources: reverse engineered from `titleScreen.flf` by black-box probing
//! of `recoil2png`; TRSE's GPL-3 source was not read. Container:
//! [`crate::codec::flf`]. Firmware color numbers:
//! [`super::hardware::firmware_color`].
//!
//! Type 0x18 (mode 0): byte 12 is `0x0b`, then 160x200 pen numbers (one per
//! byte, drawn twice as wide) from offset 13, then the closing block, which
//! is required here: its offsets 192-207 hold the firmware ink number of
//! pens 0-15 (27 and above show black).

use super::hardware::firmware_color;
use crate::codec::flf::{self, Fluff, PAYLOAD};
use crate::{DecodeError, Image};

const WIDTH: usize = 160;
const HEIGHT: usize = 200;
const INKS: usize = 192;

pub(super) fn decode_flf(data: &[u8]) -> Result<Image, DecodeError> {
    let fluff = Fluff::parse(data)?;
    if fluff.kind != 0x18 || fluff.byte(PAYLOAD)? != 0x0b {
        return Err(DecodeError::Invalid);
    }
    let (pens, rest) = fluff.split(PAYLOAD + 1, WIDTH * HEIGHT)?;
    let trailer = flf::trailer(rest, true)?;
    let mut palette = [0; 16];
    for (color, &ink) in palette.iter_mut().zip(&trailer[INKS..]) {
        *color = if ink < 27 {
            firmware_color(usize::from(ink))
        } else {
            0
        };
    }
    let image = Image::from_indexed(WIDTH as u32, HEIGHT as u32, pens, &palette)?;
    image.scaled(2, 1)
}
