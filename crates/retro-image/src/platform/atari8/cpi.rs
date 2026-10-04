//! CPI: Marco Pixel Editor pictures saved in the editor's compressed form.
//!
//! Sources:
//! - Marco Pixel Editor 2.1 manual (MPE.DOC,
//!   <http://ftp.pigwa.net/stuff/collections/atari_forever/Tools%20-%20atr/MARCO%20PIXEL%20EDITOR%202.1/MPE.DOC>):
//!   Graphics 15 (160x192, 4 colours); the compressed save is `CPI`, the
//!   plain one `PIC`. The packer is not documented.
//! - The packer and colours were reverse engineered from the five corpus
//!   samples and from `recoil2png` output on hand-made files. A byte that
//!   equals the one before it is followed by a count `n`, and the run is `n +
//!   1` copies of the byte; every other byte is a literal. (After a run the
//!   next byte starts afresh.) The stream must unpack to at least 7936 bytes
//!   (31 pages; RECOIL rejects fewer, and all samples end in 255 zeros and one
//!   more byte); the first 7680 are the picture, 40 bytes per line with 4
//!   pixels per byte, leftmost in the high bits. Pixel values 0-3 are shown
//!   as the greys `00 0C 08 04` (the values 1 and 3 are not in the usual
//!   order). Nothing in the file chooses the colours: the tail after the
//!   picture has no effect.

use super::antic::Bitmap;
use super::palette::register_rgb;
use crate::{DecodeError, Image};
use alloc::vec::Vec;

const SCREEN: usize = 7680;
const UNPACKED: usize = 7936;
const COLORS: [u8; 4] = [0x00, 0x0c, 0x08, 0x04];

pub(super) fn decode_cpi(data: &[u8]) -> Result<Image, DecodeError> {
    let unpacked = unpack(data).ok_or(DecodeError::Unrecognized)?;
    let bitmap = Bitmap {
        data: &unpacked[..SCREEN],
        bytes_per_line: 40,
        lines: 192,
        bits: 2,
    };
    bitmap.render(2, 1, |_, value| register_rgb(COLORS[usize::from(value)]))
}

/// Unpacks the first [`UNPACKED`] bytes; `None` if the stream ends sooner.
fn unpack(data: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(UNPACKED + 256);
    let mut pos = 0;
    while out.len() < UNPACKED {
        let byte = *data.get(pos)?;
        if data.get(pos + 1) == Some(&byte) {
            let count = usize::from(*data.get(pos + 2)?);
            out.resize(out.len() + count + 1, byte);
            pos += 3;
        } else {
            out.push(byte);
            pos += 1;
        }
    }
    out.truncate(UNPACKED);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_byte_takes_a_count() {
        assert!(unpack(&[1, 2, 2, 3, 4, 5, 5, 0, 6, 0, 0, 255]).is_none());
        let mut packed = alloc::vec![1u8, 2, 2, 3, 4, 5, 5, 0, 6];
        packed.extend_from_slice(&[0, 0, 255].repeat(31));
        let out = unpack(&packed).unwrap();
        assert_eq!(out[..9], [1, 2, 2, 2, 2, 4, 5, 6, 0]);
        assert_eq!(out.len(), UNPACKED);
    }
}
