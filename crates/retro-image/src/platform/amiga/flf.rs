//! Turbo Rascal Syntax Error (TRSE) "Fluff" images for the Amiga.
//!
//! Sources: reverse engineered from `image1-1.flf` and `logobale.flf` by
//! black-box probing of `recoil2png`; TRSE's GPL-3 source was not read.
//! Container and layout: [`crate::codec::flf`].
//!
//! Type 0x0d: 320x256, one palette index per byte with the palette stored
//! in the file (`codec::flf::decode_paletted`).

use crate::codec::flf::{self, Fluff};
use crate::{DecodeError, Image};

pub(super) fn decode_flf(data: &[u8]) -> Result<Image, DecodeError> {
    let fluff = Fluff::parse(data)?;
    if fluff.kind != 0x0d {
        return Err(DecodeError::Invalid);
    }
    flf::decode_paletted(&fluff, 320, 256)
}
