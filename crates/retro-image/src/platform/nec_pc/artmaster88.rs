//! ArtMaster88 `IMG` pictures (NEC PC-88, 640x200 shown at 640x400).
//!
//! The ArtMaster88 page on the archiveteam wiki only gives the signature
//! (<http://fileformats.archiveteam.org/wiki/ArtMaster88>), and Toda Takashi's
//! page on reading ArtMaster files (<http://nanyanen.jp/library/readart.html>,
//! prose only; its code was not read) just says the palette is stored as HSV.
//! The layout below was reverse engineered from `_REMSM4.IMG` by black-box
//! probing of `recoil2png` with mutated copies; see
//! `docs/research/msx-japanese.md`, "Wave 5: Japanese".
//!
//! Layout, little-endian:
//! - `"SS_SIF    0.00"` at 0, `"I"` at 0x10, `"B"` at 0x12 and `"BRG"` at 0x13 (other
//!   values select modes that were not decoded); 0x18 and 0x1A are 640 and 200.
//! - At 0x28 a chain of two records, each starting with its own length
//!   (including the two length bytes): the first leads to the second and the
//!   second to the pixel data. Their contents (brush shapes, HSV palette) do not
//!   affect the picture RECOIL draws.
//! - Three run-length packed bit planes of 80 * 200 bytes in the order blue,
//!   red, green, MSB first. Two equal bytes in a row are followed by a count
//!   (0 meaning 256) of copies of that byte; any other byte is a literal.
//! - Colours are the 8 digital PC-88 colours (0 or 255 per component). Bytes
//!   after the green plane are ignored.

use alloc::vec::Vec;

use super::pc88_planes::{self, PLANE_BYTES};
use crate::bytes::le16;
use crate::{DecodeError, Image};

const SIGNATURE: &[u8] = b"SS_SIF    0.00";
const RECORDS: usize = 0x28;

/// Unpacks one plane from `data[*pos..]`, advancing `pos` past it.
fn unpack_plane(data: &[u8], pos: &mut usize) -> Option<Vec<u8>> {
    let mut plane = Vec::with_capacity(PLANE_BYTES);
    while plane.len() < PLANE_BYTES {
        let byte = *data.get(*pos)?;
        if data.get(*pos + 1) == Some(&byte) {
            let count = match *data.get(*pos + 2)? {
                0 => 256,
                n => usize::from(n),
            };
            if plane.len() + count > PLANE_BYTES {
                return None;
            }
            plane.resize(plane.len() + count, byte);
            *pos += 3;
        } else {
            plane.push(byte);
            *pos += 1;
        }
    }
    Some(plane)
}

pub(in crate::platform) fn decode_artmaster88(data: &[u8]) -> Result<Image, DecodeError> {
    let bad = DecodeError::Unrecognized;
    if !data.starts_with(SIGNATURE)
        || data.get(0x10) != Some(&b'I')
        || data.get(0x12..0x16) != Some(b"BBRG")
        || le16(data, 0x18) != Some(pc88_planes::WIDTH as u16)
        || le16(data, 0x1a) != Some(pc88_planes::LINES as u16)
    {
        return Err(bad);
    }
    let mut pos = RECORDS;
    for _ in 0..2 {
        let length = usize::from(le16(data, pos).ok_or(bad)?);
        if length < 2 {
            return Err(bad);
        }
        pos += length;
    }
    let blue = unpack_plane(data, &mut pos).ok_or(bad)?;
    let red = unpack_plane(data, &mut pos).ok_or(bad)?;
    let green = unpack_plane(data, &mut pos).ok_or(bad)?;

    pc88_planes::image(&blue, &red, &green)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn plane_runs() {
        // Literal, a run of 3 zeros, a doubled byte with count 0 (256 copies).
        let mut data = vec![0x12, 0, 0, 3, 7, 7, 0];
        let mut pos = 0;
        assert!(unpack_plane(&data, &mut pos).is_none());
        // A plane of 16000 bytes: 62 runs of 256 and a final run of 128.
        data = Vec::new();
        for _ in 0..62 {
            data.extend([9, 9, 0]);
        }
        data.extend([5, 5, 128]);
        pos = 0;
        let plane = unpack_plane(&data, &mut pos).unwrap();
        assert_eq!((plane.len(), pos), (PLANE_BYTES, data.len()));
        assert_eq!((plane[0], plane[PLANE_BYTES - 1]), (9, 5));
        // A run that overshoots the plane is refused.
        data.truncate(data.len() - 1);
        data.push(129);
        pos = 0;
        assert!(unpack_plane(&data, &mut pos).is_none());
    }
}
