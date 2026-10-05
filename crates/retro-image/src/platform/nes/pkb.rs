//! NES pattern table packed with PackBits (`.pkb`), drawn as a sheet of
//! tiles like a `.chr`.
//!
//! Sources: Damian Yerrick's `pilbmp2nes.py`, <https://github.com/pinobatch/nrom-template>
//! `tools/pilbmp2nes.py` (Copyright 2014-2025 Damian Yerrick; "Copying and
//! distribution of this file, with or without modification, are permitted in
//! any medium without royalty provided the copyright notice and this notice
//! are preserved"). Its `--packbits` option, "use Apple PackBits RLE
//! compression", writes the size of the CHR bytes modulo 65536 as two bytes,
//! high byte first, and then the CHR bytes through a PackBits encoder. No
//! code was taken from it, only that layout; the PackBits layout is the one of
//! [`crate::codec::packbits`].
//!
//! The packets run to the end of the file, and the size must match what they
//! unpack to (modulo 65536), which makes the file check itself. The result is
//! drawn as the `.chr` of those bytes would be. No sample from a real project
//! was available: the two samples in `corpus/extra/nintendo-rom-icons/nes-pkb`
//! were made from the template's tile PNGs with that script's tile conversion
//! and a PackBits encoder, with the size prefix added as the script does, so
//! they show that the unpacking is right, not that the encoder of the tool
//! agrees. This format is unverified.

use super::chr;
use crate::bytes::be16;
use crate::codec::packbits;
use crate::{DecodeError, Image};

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let size = be16(data, 0).ok_or(fail)?;
    let tiles = packbits::unpack_all(&data[2..], chr::MAX_LEN).ok_or(fail)?;
    if tiles.len() % 0x1_0000 != usize::from(size) {
        return Err(fail);
    }
    chr::sheet(&tiles)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    /// The file for `packets`, which unpack to `len` bytes.
    fn file(len: usize, packets: &[u8]) -> Vec<u8> {
        let mut file = ((len % 0x1_0000) as u16).to_be_bytes().to_vec();
        file.extend_from_slice(packets);
        file
    }

    #[test]
    fn the_packets_unpack_to_the_tiles_of_a_chr() {
        // Two tiles: one literal packet of 16 bytes, then a run of 16.
        let mut packets = alloc::vec![15u8];
        let mut tile = [0u8; 16];
        tile[0] = 0x80;
        packets.extend_from_slice(&tile);
        packets.extend([(1 - 16i8) as u8, 0xff]);
        let image = decode(&file(32, &packets)).unwrap();
        assert_eq!((image.width(), image.height()), (128, 8));
        assert_eq!(image.get(0, 0), 0x555555);
        // The second tile is all color 3.
        assert_eq!(image.get(8, 0), 0xffffff);
        assert_eq!(image.get(15, 7), 0xffffff);
        // The size must match, the packets must be whole and not empty.
        assert!(decode(&file(16, &packets)).is_err());
        assert!(decode(&packets).is_err());
        assert!(decode(&file(32, &packets[..packets.len() - 1])).is_err());
        assert!(decode(&file(0, &[])).is_err());
        assert!(decode(&[0]).is_err());
    }

    #[test]
    fn the_size_is_taken_modulo_65536() {
        // 4112 runs of 16 zeros: 65792 bytes, which the size field holds as 256.
        let packets: Vec<u8> = (0..4112).flat_map(|_| [(1 - 16i8) as u8, 0]).collect();
        assert!(decode(&file(65792, &packets)).is_ok());
        assert!(decode(&file(65792 + 1, &packets)).is_err());
    }
}
