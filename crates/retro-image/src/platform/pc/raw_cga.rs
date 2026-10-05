//! Headerless CGA screens (`.CGA`): 320x200 at 2 bits per pixel, 16000 bytes.
//!
//! Sources:
//! - Wikipedia, "Color Graphics Adapter" (`cga.rs`): the 4-color 320x200 mode
//!   has four 2-bit pixels per byte, leftmost in the high bits, 80 bytes per
//!   row.
//! - Reverse engineered from 11 files of exactly 16000 bytes (Sembiance's
//!   `vzi` folder, one of them `loadscrn.cga`). Six of them have the rows one
//!   after another, not split into the two CGA banks (the picture is smooth
//!   across row pairs that way and noise the other way, measured by the bits
//!   that differ between adjacent rows); the other five are noise or not
//!   clear either way. Only the linear layout is read.
//!
//! Nothing in the file says what it is, so only the `.cga` extension and the
//! exact size select it, and the palette is a guess: CGA palette 1 at high
//! intensity (black, cyan, magenta, white), as for BSAVE dumps (`bsave.rs`).
//! The one sample shows its intended colors that way.
//!
//! Verification: no RECOIL or Deark support; output reviewed by eye.

use super::cga::unpack_2bit;
use super::pcpaint::CGA_4;
use crate::{DecodeError, Image};

const WIDTH: usize = 320;
const HEIGHT: usize = 200;
const FILE_LEN: usize = WIDTH / 4 * HEIGHT;
/// CGA palette 1 at high intensity.
const PALETTE: [u32; 4] = CGA_4[3];

pub(super) fn decode_raw_cga(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != FILE_LEN {
        return Err(DecodeError::Unrecognized);
    }
    Image::from_indexed(WIDTH as u32, HEIGHT as u32, &unpack_2bit(data), &PALETTE)
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;

    #[test]
    fn rows_follow_each_other_and_only_the_exact_size_counts() {
        let mut screen = vec![0u8; FILE_LEN];
        screen[80] = 0b1100_0000; // row 1, pixel 0: color 3
        let image = decode_raw_cga(&screen).unwrap();
        assert_eq!(image.get(0, 1), PALETTE[3]);
        assert_eq!(image.get(0, 0), PALETTE[0]);
        assert!(decode_raw_cga(&screen[1..]).is_err());
    }
}
