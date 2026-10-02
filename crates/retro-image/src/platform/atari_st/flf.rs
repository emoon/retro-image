//! Turbo Rascal Syntax Error (TRSE) "Fluff" images for the Atari ST/STE.
//!
//! Sources: reverse engineered from `image1.flf`, `logo1.flf` and
//! `logo.flf` by black-box probing of `recoil2png`; TRSE's GPL-3 source was
//! not read. Container and layout: [`crate::codec::flf`].
//!
//! Types 0x0c and 0x16: 320x200, one palette index per byte with the
//! palette stored in the file (`codec::flf::decode_paletted`). `recoil2png`
//! draws these two types, and the PC's 0x1b, identically; which of them
//! belong to the ST is a guess from the 16-colour palette and the size.

use crate::codec::flf::{self, Fluff};
use crate::{DecodeError, Image};

pub(super) fn decode_flf(data: &[u8]) -> Result<Image, DecodeError> {
    let fluff = Fluff::parse(data)?;
    if !matches!(fluff.kind, 0x0c | 0x16) {
        return Err(DecodeError::Unrecognized);
    }
    flf::decode_paletted(&fluff, 320, 200)
}
