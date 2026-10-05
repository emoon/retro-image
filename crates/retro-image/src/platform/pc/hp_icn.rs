//! HP 100LX and 200LX palmtop icons (`.ICN`).
//!
//! Sources:
//! - Deark `misc2.c` (<https://github.com/jsummers/deark>, MIT license): the
//!   magic `01 00 01 00`, a little-endian 16-bit width at 4 and height at 6,
//!   then 1-bit rows of `(width + 7) / 8` bytes from offset 8, most
//!   significant bit leftmost. A set bit is black, as in the 44x32 icons the
//!   palmtops use; the pad bits at the end of a row are set.
//! - Reverse engineered from the 8 sample icons: all 44x32, 200 bytes, which
//!   is exactly `8 + 6 * 32`; the picture was checked by eye.
//!
//! Verification: no RECOIL oracle for this format; output matches Deark's
//! `hpicn` module on the sample files.

use crate::bytes::le16;
use crate::{BitOrder, DecodeError, Image};

const FAIL: DecodeError = DecodeError::Unrecognized;
const MAGIC: [u8; 4] = [1, 0, 1, 0];
const HEADER_LEN: usize = 8;
/// Largest width or height accepted, as in Deark.
const MAX_SIDE: usize = 2048;

/// The file size is part of the check: with the magic it makes this a
/// reliable signature.
pub(super) fn decode_icn(data: &[u8]) -> Result<Image, DecodeError> {
    if !data.starts_with(&MAGIC) {
        return Err(FAIL);
    }
    let side = |at| {
        le16(data, at)
            .map(usize::from)
            .filter(|s| (1..=MAX_SIDE).contains(s))
    };
    let (width, height) = (side(4).ok_or(FAIL)?, side(6).ok_or(FAIL)?);
    let row_len = width.div_ceil(8);
    if data.len() != HEADER_LEN + row_len * height {
        return Err(FAIL);
    }
    Image::from_bits(
        width as u32,
        height as u32,
        &data[HEADER_LEN..],
        row_len,
        BitOrder::MsbFirst,
        [0xffffff, 0x000000],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_bits_are_black_and_the_size_must_match() {
        // 3x2: rows 101 and 010, pad bits set.
        let icon = [1, 0, 1, 0, 3, 0, 2, 0, 0b1011_1111, 0b0101_1111];
        let image = decode_icn(&icon).unwrap();
        assert_eq!((image.width(), image.height()), (3, 2));
        assert_eq!(image.get(0, 0), 0);
        assert_eq!(image.get(1, 0), 0xffffff);
        assert_eq!(image.get(1, 1), 0);
        assert!(decode_icn(&icon[..9]).is_err());
        assert!(decode_icn(&[icon.as_slice(), &[0]].concat()).is_err());
    }
}
