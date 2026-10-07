//! SHP: Movie Maker shape sheets.
//!
//! Sources:
//! - Movie Maker (Reston): Just Solve "Movie Maker"
//!   (<http://fileformats.archiveteam.org/wiki/Movie_Maker>) and Wikipedia
//!   (<https://en.wikipedia.org/wiki/Movie_Maker_(Reston_Publishing)>): a
//!   160x96, 4-color picture. Reverse engineered from ACTORS.SHP and the
//!   dexvert/pigwa Movie Maker samples by flipping single bytes and reading
//!   back what `recoil2png` changes: 4384 bytes, the first 528 (a shape
//!   directory) have no effect, the rest is a Movie Maker background (BKG).

use super::screen::decode_bkg;
use crate::{DecodeError, Image};

/// Directory of the shapes on the sheet, which RECOIL ignores.
const DIRECTORY: usize = 528;

/// Movie Maker shapes: a 528-byte directory, then a BKG picture. Exactly
/// 4384 bytes.
pub(super) fn decode_movie_maker(data: &[u8]) -> Result<Image, DecodeError> {
    match data.split_at_checked(DIRECTORY) {
        Some((_, picture)) if data.len() == 4384 => decode_bkg(picture),
        _ => Err(DecodeError::Invalid),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn movie_maker_sheet_skips_the_directory() {
        let mut data = vec![0u8; 4384];
        data[DIRECTORY] = 0b1100_0000; // first pixel: playfield 2
        data[4384 - 16..4384 - 12].copy_from_slice(&[0x00, 0x28, 0xca, 0x94]);
        let image = decode_movie_maker(&data).unwrap();
        assert_eq!((image.width(), image.height()), (320, 192));
        assert_ne!(image.get(0, 0), image.get(4, 0));
        assert!(decode_movie_maker(&data[..4383]).is_err());
    }
}
