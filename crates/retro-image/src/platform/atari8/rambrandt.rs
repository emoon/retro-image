//! Rambrandt (RM2, RM4).
//!
//! Sources:
//! - Just Solve "RAMbrandt" (<http://fileformats.archiveteam.org/wiki/RAMbrandt>)
//!   and gury.atari8.info software pages
//!   (<http://gury.atari8.info/software/476.php>,
//!   <http://gury.atari8.info/software/962.php>) for the program and its modes.
//!   The program's documentation disk (an ATR image) was not read.
//! - Everything else was reverse engineered from TITLE.RM2 and ADVANCED.RM4 and
//!   by black-box probing of `recoil2png` (one-byte changes to find which bytes
//!   are read, truncation and padding scans).
//!
//! RM2: exactly 8192 bytes: a Graphics 10 screen of 192 lines x 40 bytes
//! (4 bits per pixel, drawn 4 wide), then the 9 registers 704-712 (the
//! GTIA mode 10 colours), then 503 bytes that are not read.

use super::antic::Bitmap;
use super::screen::gtia10;
use crate::{DecodeError, Image};

pub(super) fn decode_rm2(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 8192 {
        return Err(DecodeError::Unrecognized);
    }
    let bitmap = Bitmap {
        data: &data[..7680],
        bytes_per_line: 40,
        lines: 192,
        bits: 4,
    };
    let registers = data[7680..7689]
        .try_into()
        .map_err(|_| DecodeError::Unrecognized)?;
    Ok(gtia10(bitmap, &registers))
}
