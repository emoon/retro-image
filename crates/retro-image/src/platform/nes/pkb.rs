//! NES pattern table packed with PackBits (`.pkb`), drawn as a sheet of
//! tiles like a `.chr`.
//!
//! Sources: Damian Yerrick's `pilbmp2nes.py`, <https://github.com/pinobatch/nrom-template>
//! `tools/pilbmp2nes.py` (Copyright 2014-2025 Damian Yerrick; "Copying and
//! distribution of this file, with or without modification, are permitted in
//! any medium without royalty provided the copyright notice and this notice
//! are preserved"). Its `--packbits` option, "use Apple PackBits RLE
//! compression", passes the CHR bytes through a PackBits encoder and writes
//! the result as it is. No code was taken from it, only that fact; the
//! PackBits layout is the one of [`crate::codec::packbits`].
//!
//! The file is the PackBits packets with no header and no length, so it is
//! unpacked to its end. The result is drawn as the `.chr` of those bytes would
//! be. No sample from a real project was available: the two samples in
//! `corpus/extra/nintendo-rom-icons/nes-pkb` were made from the template's
//! tile PNGs with that script's tile conversion and a PackBits encoder, so they
//! show that the unpacking is right, not that the encoder of the tool agrees.
//! This format is unverified.

use super::chr;
use crate::codec::packbits;
use crate::{DecodeError, Image};

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    let tiles = packbits::unpack_all(data, chr::MAX_LEN).ok_or(DecodeError::Unrecognized)?;
    chr::sheet(&tiles)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_packets_unpack_to_the_tiles_of_a_chr() {
        // Two tiles: one literal packet of 16 bytes, then a run of 16.
        let mut packed = alloc::vec![15u8];
        let mut tile = [0u8; 16];
        tile[0] = 0x80;
        packed.extend_from_slice(&tile);
        packed.extend([(1 - 16i8) as u8, 0xff]);
        let image = decode(&packed).unwrap();
        assert_eq!((image.width(), image.height()), (128, 8));
        assert_eq!(image.get(0, 0), 0x555555);
        // The second tile is all color 3.
        assert_eq!(image.get(8, 0), 0xffffff);
        assert_eq!(image.get(15, 7), 0xffffff);
        // Not a whole number of tiles, a cut packet, and nothing at all.
        assert!(decode(&packed[..packed.len() - 1]).is_err());
        assert!(decode(&[0, 1]).is_err());
        assert!(decode(&[]).is_err());
    }
}
