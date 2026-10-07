//! HCB-editor: multicolor bitmap whose screen RAM changes every four lines.
//!
//! Sources: reverse engineered from `kemikal.hcb` and `fembot.hcb` by
//! black-box probing of `recoil2png` (byte flips in copies and synthetic
//! files). Load `$5000`, 12148 bytes. Memory map:
//! - `$5000-$57FF`: no effect on the picture.
//! - `$5800` and `$5C00`: two screen RAMs; the first serves lines 0-3 of
//!   every character row, the second lines 4-7.
//! - `$6000`: multicolor bitmap in the usual cell order.
//! - `$7F40`: 50 background colors, one per four lines.
//!
//! Bit pair `01` is the screen high nibble, `10` and `11` its low nibble.
//! The leftmost 24 pixels (the FLI bug area) are cut, as for the FLI formats.

use super::prg::Prg;
use super::vic2::{BITMAP_LEN, FLI_BUG, Frame, SCREEN_LEN};
use crate::{DecodeError, Image};

const LOAD: u16 = 0x5000;
const SIZE: usize = 2 + 0x2f72;
const HEIGHT: usize = 200;

pub(super) fn decode_hcb(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != SIZE {
        return Err(DecodeError::Invalid);
    }
    let prg = Prg::new(data, LOAD);
    let parts = (
        prg.at(0x5800, SCREEN_LEN),
        prg.at(0x5c00, SCREEN_LEN),
        prg.at(0x6000, BITMAP_LEN),
        prg.at(0x7f40, HEIGHT / 4),
    );
    let (Some(upper), Some(lower), Some(bitmap), Some(background)) = parts else {
        return Err(DecodeError::Invalid);
    };
    let frame = Frame::from_fn(HEIGHT, |x, y| {
        let cell = y / 8 * 40 + x / 8;
        let screen = if y % 8 < 4 { upper } else { lower }[cell];
        let bits = bitmap[cell * 8 + y % 8] >> (6 - (x & 6)) & 3;
        match bits {
            0 => background[y / 4],
            1 => screen >> 4,
            _ => screen & 15,
        }
    });
    Ok(frame.to_image(FLI_BUG))
}
