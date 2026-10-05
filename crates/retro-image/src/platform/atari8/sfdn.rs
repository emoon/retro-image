//! Pictures packed with the "SFDN" compressor (files start with `S101`):
//! APP, APS, G9S/SFD, HPS, ILS, INS and PLS. Unpacked, each is a picture
//! of a format decoded elsewhere in this module.
//!
//! Sources:
//! - Just Solve "Apac3 APP" (<http://fileformats.archiveteam.org/wiki/Apac3_APP>)
//!   and "AP*" (<http://fileformats.archiveteam.org/wiki/AP*>): the `S101`
//!   magic and the formats that use it.
//! - Reverse engineered from samples and by black-box probing of
//!   `recoil2png` with hand-made files (no format documentation exists):
//!
//!   ```text
//!   0  "S101"
//!   4  unpacked length, little-endian
//!   6  16 nibble deltas, most frequent first
//!   22 bitstream, most significant bit first
//!   ```
//!
//!   The unpacked bytes are a stream of nibbles, high nibble first. The
//!   first nibble is stored as 4 raw bits. Every following nibble is the
//!   previous one minus a delta from the table (mod 16), picked by a rank
//!   coded as `k` one bits, a zero bit, then one more bit: rank `2k + bit`
//!   (so `00` = rank 0, `01` = 1, `100` = 2, ..., `111111101` = 15). Eight
//!   one bits in a row are invalid. Only the low nibble of a table entry
//!   counts.
//! - The unpacked length each extension takes, and the formats behind them
//!   (G9S/SFD: Graphics 9, PLS: interleaved APAC, APS: interleaved APAC with
//!   its 40-byte tail, APP: interlaced APAC with hue at 8192, ILS:
//!   interlaced APAC, INS: InterPainter, HPS: HIP with registers), observed
//!   from `recoil2png` output: every corpus sample, unpacked, renders
//!   identically through the unpacked format.

use super::{apac, hip, interlace, screen};
use crate::{DecodeError, Image};
use alloc::vec::Vec;

type Decoder = fn(&[u8]) -> Result<Image, DecodeError>;

/// G9S, SFD: Graphics 9. The MGV viewer's files have 4 more bytes
/// (`00 28 CA 00` in both known files), which are ignored; RECOIL rejects
/// them.
pub(super) fn decode_g9s(data: &[u8]) -> Result<Image, DecodeError> {
    packed(data, &[7680, 7684], screen::decode_gr9)
}

/// PLS (Plama 256): 80x96 APAC, alternating hue and luminance lines.
pub(super) fn decode_pls(data: &[u8]) -> Result<Image, DecodeError> {
    packed(data, &[7680], apac::decode_interleaved)
}

/// APS: 80x96 APAC, alternating lines, with a 40-byte tail.
pub(super) fn decode_aps(data: &[u8]) -> Result<Image, DecodeError> {
    packed(data, &[7720], apac::decode_interleaved)
}

/// APP (Apac3 Linker-Viewer): interlaced 80x192 APAC.
pub(super) fn decode_app(data: &[u8]) -> Result<Image, DecodeError> {
    packed(data, &[15872], apac::decode_interlaced)
}

/// ILS (APACVIEW): interlaced 80x192 APAC.
pub(super) fn decode_ils(data: &[u8]) -> Result<Image, DecodeError> {
    packed(data, &[15360], apac::decode_interlaced)
}

/// INS: InterPainter.
pub(super) fn decode_ins(data: &[u8]) -> Result<Image, DecodeError> {
    packed(data, &[16004], interlace::decode_inp)
}

/// HPS: Hard Interlace Picture with its color registers.
pub(super) fn decode_hps(data: &[u8]) -> Result<Image, DecodeError> {
    packed(data, &[16009], hip::decode_hip)
}

/// Unpacks `data`, which must unpack to one of `lens` bytes, and decodes the
/// result with `decode`.
fn packed(data: &[u8], lens: &[usize], decode: Decoder) -> Result<Image, DecodeError> {
    let unpacked = unpack(data).ok_or(DecodeError::Unrecognized)?;
    if !lens.contains(&unpacked.len()) {
        return Err(DecodeError::Unrecognized);
    }
    decode(&unpacked)
}

/// Unpacks an SFDN stream; `None` if it isn't one or is truncated.
fn unpack(data: &[u8]) -> Option<Vec<u8>> {
    let rest = data.strip_prefix(b"S101")?;
    let len = usize::from(crate::bytes::le16(rest, 0)?);
    let deltas = rest.get(2..18)?;
    let mut bits = Bits {
        data: rest.get(18..)?,
        pos: 0,
    };
    let mut nibble = bits.read(4)?;
    let mut out = Vec::with_capacity(len);
    for i in 0..2 * len {
        if i > 0 {
            let mut ones = 0;
            while bits.read(1)? == 1 {
                ones += 1;
                if ones == 8 {
                    return None;
                }
            }
            let rank = 2 * ones + usize::from(bits.read(1)?);
            nibble = nibble.wrapping_sub(deltas[rank]) & 0x0f;
        }
        if i % 2 == 0 {
            out.push(nibble << 4);
        } else if let Some(last) = out.last_mut() {
            *last |= nibble;
        }
    }
    Some(out)
}

/// Most-significant-bit-first reader.
struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
}

impl Bits<'_> {
    /// The next `count` (up to 8) bits.
    fn read(&mut self, count: usize) -> Option<u8> {
        let mut value = 0;
        for _ in 0..count {
            let byte = self.data.get(self.pos / 8)?;
            value = (value << 1) | ((byte >> (7 - self.pos % 8)) & 1);
            self.pos += 1;
        }
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(len: u16) -> Vec<u8> {
        let mut data = b"S101".to_vec();
        data.extend_from_slice(&len.to_le_bytes());
        data.extend(0..16u8);
        data
    }

    #[test]
    fn decodes_ranks() {
        let mut data = header(3);
        // Raw 5, then ranks 0 (00), 2 (100), 15 (111111101), 1 (01), 0 (00).
        data.extend_from_slice(&[0b0101_0010, 0b0111_1111, 0b0101_0000]);
        assert_eq!(unpack(&data).unwrap(), [0x55, 0x34, 0x33]);
    }

    #[test]
    fn rejects_escape_and_truncation() {
        let mut data = header(2);
        data.extend_from_slice(&[0x0f, 0xf0]);
        assert_eq!(unpack(&data), None);
        let mut data = header(200);
        data.push(0);
        assert_eq!(unpack(&data), None);
        assert_eq!(unpack(b"S101"), None);
    }
}
