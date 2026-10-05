//! NES nametable in run-length form (`.rle`, as NES Screen Tool and the
//! neslib toolchain write them), drawn like a `.nam`.
//!
//! Sources: reverse engineered from one sample, `presets/nes/climbr_title.nam.rle`
//! of the 8bitworkshop repository (<https://github.com/sehugg/8bitworkshop>,
//! data file only; its code is GPL and was not read), in
//! `corpus/extra/nintendo-rom-icons/nes-screens/climbr-title`. No format
//! description and no encoder source was read. What the sample shows:
//! - The first byte is the tag. Every other byte is copied to the output,
//!   except the tag, which starts a pair or a triple.
//! - The tag followed by a 0 ends the data (the sample ends so).
//! - The tag followed by `n` (1 to 255) and a value `v` is `n + 1` copies of
//!   `v`. With `n` copies the sample's 85 runs unpack to 939 bytes; with
//!   `n + 1` they unpack to exactly 1024, a nametable of 960 tile numbers
//!   and its 64 attribute bytes, so that reading is taken.
//!
//! A data byte equal to the tag, which the sample does not contain, has no
//! known encoding, so the tag a writer picks is assumed to be unused.
//!
//! The data must unpack to 960 bytes (tile numbers) or 1024 (with
//! attributes) and carry its end mark; anything else is not a nametable. The
//! picture is then drawn as a `.nam` is, with the `.chr` and `.pal` that have
//! the same name.

use alloc::vec::Vec;

use super::nam;
use crate::{Companions, DecodeError, Image};

/// Bytes of a nametable and of one with its attribute table.
const NAMES_LEN: usize = 960;
const WITH_ATTRIBUTES_LEN: usize = NAMES_LEN + 64;

pub(super) fn decode(data: &[u8], companions: &dyn Companions) -> Result<Image, DecodeError> {
    let screen = unpack(data)?;
    if screen.len() != NAMES_LEN && screen.len() != WITH_ATTRIBUTES_LEN {
        return Err(DecodeError::Unrecognized);
    }
    nam::draw_screen(&screen, companions)
}

/// The bytes the data unpacks to, which stay within one nametable and its
/// attributes.
fn unpack(data: &[u8]) -> Result<Vec<u8>, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let (&tag, mut rest) = data.split_first().ok_or(fail)?;
    let mut out = Vec::new();
    loop {
        let (&byte, after) = rest.split_first().ok_or(fail)?;
        rest = after;
        if byte != tag {
            out.push(byte);
        } else {
            let (&count, after) = rest.split_first().ok_or(fail)?;
            if count == 0 {
                return Ok(out);
            }
            let (&value, after) = after.split_first().ok_or(fail)?;
            rest = after;
            out.resize(out.len() + usize::from(count) + 1, value);
        }
        if out.len() > WITH_ATTRIBUTES_LEN {
            return Err(fail);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `.chr` and nothing else beside the file.
    struct Chr;

    impl Companions for Chr {
        fn get(&self, extension: &str) -> Option<Vec<u8>> {
            (extension == "chr").then(|| alloc::vec![0; 4096])
        }

        fn get_named(&self, _file_name: &str) -> Option<Vec<u8>> {
            None
        }
    }

    /// Data with tag 9 that unpacks to `runs` runs of 256 bytes and then
    /// `tail` more bytes, as one run of the value 1.
    fn stream(runs: usize, tail: usize) -> Vec<u8> {
        let mut data = alloc::vec![9];
        for _ in 0..runs {
            data.extend([9, 255, 1]);
        }
        data.extend([9, (tail - 1) as u8, 1, 9, 0]);
        data
    }

    #[test]
    fn runs_are_one_longer_than_their_count_and_zero_ends_the_data() {
        let data = [9, 5, 6, 9, 3, 7, 8, 9, 0];
        assert_eq!(unpack(&data).unwrap(), [5, 6, 7, 7, 7, 7, 8]);
        // No end mark, a cut run, an empty file and more than a nametable.
        assert!(unpack(&data[..data.len() - 2]).is_err());
        assert!(unpack(&[9, 9, 3]).is_err());
        assert!(unpack(&[]).is_err());
        assert!(unpack(&stream(5, 2)).is_err());
    }

    #[test]
    fn only_a_nametable_sized_result_is_accepted() {
        // 3 * 256 + 192 = 960 and + 64 = 1024 bytes.
        assert!(decode(&stream(3, 192), &Chr).is_ok());
        assert!(decode(&stream(4, 2), &Chr).is_err());
        assert!(decode(&stream(3, 191), &Chr).is_err());
        assert!(decode(&stream(3, 193), &Chr).is_err());
        assert!(decode(&stream(3, 256), &Chr).is_ok());
    }
}
