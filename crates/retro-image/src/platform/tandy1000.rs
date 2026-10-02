//! Tandy 1000.
//!
//! Sources:
//! - DeskMate Paint (`.PNT`): Deark `misc2.c` (<https://github.com/jsummers/deark>,
//!   MIT licence): signature `$13 "PNT"`, pixels from offset 22, 312x176 at
//!   4 bits per pixel (high nibble first), stored raw or as (value, count)
//!   byte pairs.
//! - Palette: observed from `recoil2png` output.

use alloc::vec::Vec;

use crate::{DecodeError, Format, Image};

/// DeskMate's 16 colours.
const PALETTE: [u32; 16] = [
    0x000000, 0x000099, 0x009900, 0x339999, 0x990000, 0xcc33cc, 0xcc6600, 0x999999, 0x996633,
    0x6633ff, 0x33cc00, 0x66cccc, 0xffcccc, 0xff99ff, 0xffff00, 0xffffff,
];

pub(super) static FORMATS: &[Format] =
    &[Format::new("Tandy 1000", "DeskMate Paint", &["pnt"], decode_pnt).signature()];

const WIDTH: usize = 312;
const HEIGHT: usize = 176;
const PIXELS_AT: usize = 22;

fn decode_pnt(data: &[u8]) -> Result<Image, DecodeError> {
    if !data.starts_with(b"\x13PNT") || data.len() <= PIXELS_AT {
        return Err(DecodeError::Unrecognized);
    }
    let len = WIDTH / 2 * HEIGHT;
    let src = &data[PIXELS_AT..];
    let pixels = if src.len() == len {
        src.to_vec()
    } else {
        unpack_runs(src, len).ok_or(DecodeError::Unrecognized)?
    };
    let indices: Vec<u8> = pixels.iter().flat_map(|&b| [b >> 4, b & 15]).collect();
    Image::from_indexed(WIDTH as u32, HEIGHT as u32, &indices, &PALETTE)
}

/// (value, count) byte pairs that must fill exactly `len` bytes. Every
/// sample written by DeskMate does; a zero count or a short or overlong
/// stream means a damaged file (e.g. a disk sector lost to zeros).
fn unpack_runs(src: &[u8], len: usize) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(len);
    let (pairs, rest) = src.as_chunks::<2>();
    let mut pairs = pairs.iter();
    while out.len() < len {
        let pair = pairs.next()?;
        let count = usize::from(pair[1]);
        if count == 0 || out.len() + count > len {
            return None;
        }
        out.resize(out.len() + count, pair[0]);
    }
    (pairs.len() == 0 && rest.is_empty()).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_must_fill_the_picture_exactly() {
        assert_eq!(unpack_runs(&[7, 2, 9, 1], 3), Some(alloc::vec![7, 7, 9]));
        assert_eq!(unpack_runs(&[7, 2], 3), None);
        assert_eq!(unpack_runs(&[7, 4], 3), None);
        assert_eq!(unpack_runs(&[7, 0, 7, 3], 3), None);
        assert_eq!(unpack_runs(&[7, 3, 1, 1], 3), None);
    }
}
