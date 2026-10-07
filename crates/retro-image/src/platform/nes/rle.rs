//! NES nametable in run-length form (`.rle`, as NES Screen Tool and the
//! neslib toolchain write them), drawn like a `.nam`.
//!
//! Sources: reverse engineered from one sample, `presets/nes/climbr_title.nam.rle`
//! of the 8bitworkshop repository (<https://github.com/sehugg/8bitworkshop>,
//! data file only; its code is GPL and was not read), in
//! `corpus/extra/nintendo-rom-icons/nes-screens/climbr-title`. No format
//! description and no encoder source was read. What the sample shows:
//! - The first byte is the tag. Every other byte is copied to the output,
//!   except the tag, which starts a pair.
//! - The tag followed by a 0 ends the data (the sample ends so).
//! - The tag followed by `n` (1 to 255) repeats the byte just written `n`
//!   more times. There is no value byte: the run follows a literal.
//!
//! The first reading tried here, a tag followed by `n` and a value `v` for
//! `n + 1` copies of `v`, also consumes the whole sample and also unpacks to
//! exactly 1024 bytes, so the length cannot tell the two apart. Drawn with
//! the tile set of the sample (`climbr_title.chr`), that reading breaks the
//! logo and shifts the text one tile; this one gives a clean title screen,
//! so it is the one taken. One sample is thin evidence: it is the layout of
//! the neslib `vram_unrle` routine as the author remembers it, checked
//! against the picture, not against any source.
//!
//! A data byte equal to the tag, which the sample does not contain, has no
//! known encoding, so the tag a writer picks is assumed to be unused.
//!
//! The data must unpack to 960 bytes (tile numbers) or 1024 (with
//! attributes) and carry its end mark; anything else is not a nametable. The
//! picture is then drawn as a `.nam` is, with the `.chr` and `.pal` that have
//! the same name.

use alloc::vec::Vec;

use super::nam::{self, WITH_ATTRIBUTES_LEN};
use crate::{Companions, DecodeError, Image};

pub(super) fn decode(data: &[u8], companions: &dyn Companions) -> Result<Image, DecodeError> {
    nam::draw_screen(&unpack(data)?, companions)
}

/// The bytes the data unpacks to, which stay within one nametable and its
/// attributes.
fn unpack(data: &[u8]) -> Result<Vec<u8>, DecodeError> {
    const FAIL: DecodeError = DecodeError::Invalid;
    let (&tag, mut rest) = data.split_first().ok_or(FAIL)?;
    let mut out = Vec::new();
    loop {
        let (&byte, after) = rest.split_first().ok_or(FAIL)?;
        rest = after;
        if byte != tag {
            out.push(byte);
        } else {
            let (&count, after) = rest.split_first().ok_or(FAIL)?;
            rest = after;
            if count == 0 {
                return Ok(out);
            }
            let &previous = out.last().ok_or(FAIL)?;
            out.resize(out.len() + usize::from(count), previous);
        }
        if out.len() > WITH_ATTRIBUTES_LEN {
            return Err(FAIL);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `.chr` and nothing else beside the file.
    struct Chr;

    impl Companions for Chr {
        fn get(&self, extension: &str) -> Option<alloc::borrow::Cow<'_, [u8]>> {
            (extension == "chr").then(|| alloc::vec![0; 4096].into())
        }

        fn get_named(&self, _file_name: &str) -> Option<alloc::borrow::Cow<'_, [u8]>> {
            None
        }
    }

    /// Data with tag 9 that unpacks to `len` copies of the byte 1: the
    /// literal, then runs of at most 255 more.
    fn stream(len: usize) -> Vec<u8> {
        let mut data = alloc::vec![9, 1];
        let mut left = len - 1;
        while left > 0 {
            let run = left.min(255);
            data.extend([9, run as u8]);
            left -= run;
        }
        data.extend([9, 0]);
        data
    }

    #[test]
    fn a_run_repeats_the_byte_before_it_and_zero_ends_the_data() {
        let data = [9, 5, 6, 9, 3, 8, 9, 0];
        assert_eq!(unpack(&data).unwrap(), [5, 6, 6, 6, 6, 8]);
        // No end mark, an empty file, a run with nothing before it and more
        // than a nametable.
        assert!(unpack(&data[..data.len() - 2]).is_err());
        assert!(unpack(&[9, 9]).is_err());
        assert!(unpack(&[]).is_err());
        assert!(unpack(&[9, 9, 3, 9, 0]).is_err());
        assert!(unpack(&stream(1025)).is_err());
    }

    #[test]
    fn only_a_nametable_sized_result_is_accepted() {
        assert_eq!(unpack(&stream(960)).unwrap().len(), 960);
        assert!(decode(&stream(960), &Chr).is_ok());
        assert!(decode(&stream(1024), &Chr).is_ok());
        assert!(decode(&stream(959), &Chr).is_err());
        assert!(decode(&stream(1000), &Chr).is_err());
    }
}
