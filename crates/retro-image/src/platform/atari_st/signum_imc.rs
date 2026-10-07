//! Signum! compressed monochrome images (`.IMC`, `.I01`, `.I02`, `.I04`,
//! `.PAC`), magic `bimc0002`.
//!
//! Sources:
//! - Layout and algorithm: "The Signum! image format (IMC)",
//!   <https://sdo.dseiler.eu/formats/bimc> (reverse engineered from a
//!   disassembly of another program; no license stated, facts only).
//! - Cross-check of the sub-chunk bit order and the XOR passes: the `signum`
//!   crate of sdo-tool, `src/images/imc.rs`
//!   (<https://github.com/Xiphoseer/sdo-tool>; its `Cargo.toml` declares
//!   `MIT OR Apache-2.0`). No code was copied.
//! - Verified by eye on the 15 sample files (all 640x400 line art and text).
//!
//! Header (big-endian, 40 bytes): `bimc0002`, file size, width, height,
//! chunks across, chunks down, size of the bit stream, size of the byte
//! stream, the final XOR word, 10 unknown bytes. The bit stream follows,
//! then the byte stream. The picture is 16x16 pixel chunks, left to right,
//! top to bottom:
//! - one bit per chunk row says whether the row holds any chunks, then one
//!   bit per chunk says whether it is present (absent chunks are blank);
//! - a present chunk has two strategy bits (0 to 3) and then, for 0 to 2,
//!   four flag bits, one per half-chunk of 8 rows by 1 byte (upper left,
//!   upper right, lower left, lower right). A flagged half takes a mask
//!   byte from the byte stream, whose set bits (high to low) say which of
//!   its 8 rows follow as one byte each. Strategy 1 then replaces every
//!   row of the chunk by its XOR with the previous row (rows are 2 bytes),
//!   strategy 2 does the same with 4-byte units (two rows). Strategy 3 is
//!   just 32 bytes, row by row;
//! - finally every byte of even pixel rows is XORed with the high byte of
//!   the XOR word, every byte of odd rows with its low byte.
//!
//! Set bits are black on white (ST high resolution video RAM).

use super::common::MONO_PALETTE;
use crate::bytes::{be16, be32};
use crate::image::check_size;
use crate::{BitOrder, DecodeError, Image};

const HEADER_LEN: usize = 40;
const CHUNK: usize = 16;

/// MSB-first reader over the bit stream.
struct Bits<'a> {
    data: &'a [u8],
    next: usize,
}

impl Bits<'_> {
    fn bit(&mut self) -> Option<bool> {
        let byte = *self.data.get(self.next / 8)?;
        let bit = byte >> (7 - self.next % 8) & 1;
        self.next += 1;
        Some(bit != 0)
    }
}

pub(super) fn decode_imc(data: &[u8]) -> Result<Image, DecodeError> {
    const FAIL: DecodeError = DecodeError::Invalid;
    if data.get(..8) != Some(b"bimc0002") {
        return Err(FAIL);
    }
    let field = |at| be16(data, at).map(usize::from).ok_or(FAIL);
    let (width, height) = (field(12)?, field(14)?);
    let (across, down) = (field(16)?, field(18)?);
    let bits_len = be32(data, 20).ok_or(FAIL)? as usize;
    let xor = be16(data, 28).ok_or(FAIL)?;
    if width == 0 || height == 0 || across * CHUNK < width || down * CHUNK < height {
        return Err(FAIL);
    }
    // Only as many chunks as the picture needs; a smaller grid would leave
    // part of the picture undefined.
    check_size(across * CHUNK, down * CHUNK)?;
    let body = data.get(HEADER_LEN..).ok_or(FAIL)?;
    let (bits, bytes) = (body.get(..bits_len).ok_or(FAIL)?, &body[bits_len..]);

    let row_len = across * 2;
    let mut bitmap = alloc::vec![0u8; row_len * down * CHUNK];
    let mut bits = Bits {
        data: bits,
        next: 0,
    };
    let mut bytes = bytes.iter().copied();
    for chunk_row in 0..down {
        if !bits.bit().ok_or(FAIL)? {
            continue;
        }
        for chunk_col in 0..across {
            if !bits.bit().ok_or(FAIL)? {
                continue;
            }
            let chunk = read_chunk(&mut bits, &mut bytes).ok_or(FAIL)?;
            for (line, pair) in chunk.as_chunks::<2>().0.iter().enumerate() {
                let at = (chunk_row * CHUNK + line) * row_len + chunk_col * 2;
                bitmap[at..at + 2].copy_from_slice(pair);
            }
        }
    }
    if xor != 0 {
        let [even, odd] = xor.to_be_bytes();
        for (y, row) in bitmap.chunks_exact_mut(row_len).enumerate() {
            let value = if y % 2 == 0 { even } else { odd };
            row.iter_mut().for_each(|byte| *byte ^= value);
        }
    }
    Image::from_bits(
        width as u32,
        height as u32,
        &bitmap,
        row_len,
        BitOrder::MsbFirst,
        MONO_PALETTE,
    )
}

/// Reads one chunk: 16 rows of 2 bytes.
fn read_chunk(bits: &mut Bits, bytes: &mut impl Iterator<Item = u8>) -> Option<[u8; 32]> {
    let strategy = u8::from(bits.bit()?) * 2 + u8::from(bits.bit()?);
    let mut chunk = [0u8; 32];
    if strategy == 3 {
        for byte in &mut chunk {
            *byte = bytes.next()?;
        }
        return Some(chunk);
    }
    // Upper left, upper right, lower left, lower right: 8 rows of one byte.
    for first in [0, 1, 16, 17] {
        if bits.bit()? {
            let mut mask = bytes.next()?;
            for row in 0..8 {
                if mask & 0x80 != 0 {
                    chunk[first + row * 2] = bytes.next()?;
                }
                mask <<= 1;
            }
        }
    }
    let unit = match strategy {
        1 => 2,
        2 => 4,
        _ => return Some(chunk),
    };
    let mut accumulator = [0u8; 4];
    for group in chunk.chunks_exact_mut(unit) {
        for (byte, acc) in group.iter_mut().zip(&mut accumulator) {
            *acc ^= *byte;
            *byte = *acc;
        }
    }
    Some(chunk)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    fn header(width: u16, height: u16, across: u16, down: u16, bits: u32, xor: u16) -> Vec<u8> {
        let mut data = b"bimc0002".to_vec();
        data.extend_from_slice(&0u32.to_be_bytes());
        for word in [width, height, across, down] {
            data.extend_from_slice(&word.to_be_bytes());
        }
        data.extend_from_slice(&bits.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&xor.to_be_bytes());
        data.resize(HEADER_LEN, 0);
        data
    }

    #[test]
    fn raw_chunk_is_copied_row_by_row() {
        // One chunk row, one chunk, strategy 3: bits 1 1 11 -> 0b1111_0000.
        let mut data = header(16, 16, 1, 1, 1, 0);
        data.push(0b1111_0000);
        data.extend((0..32u8).map(|i| if i == 0 { 0x80 } else { 0 }));
        let image = decode_imc(&data).unwrap();
        assert_eq!((image.width(), image.height()), (16, 16));
        assert_eq!(image.get(0, 0), 0x000000);
        assert_eq!(image.get(1, 0), 0xffffff);
        assert_eq!(image.get(0, 1), 0xffffff);
    }

    #[test]
    fn half_chunk_rows_and_xor_strategy() {
        // Strategy 1, only the upper-left half: mask selects rows 0 and 1.
        // Bits: 1 1 | 01 | 1000 -> 0b1101_1000.
        let mut data = header(16, 16, 1, 1, 1, 0);
        data.push(0b1101_1000);
        data.extend_from_slice(&[0b1100_0000, 0x80, 0x01]);
        let image = decode_imc(&data).unwrap();
        // Row 0 byte 0 = 0x80; row 1 byte 0 = 0x80 ^ 0x01 = 0x81 (XOR with
        // the row above); every later row repeats the accumulator.
        assert_eq!(image.get(0, 0), 0x000000);
        assert_eq!(image.get(7, 1), 0x000000);
        assert_eq!(image.get(0, 2), 0x000000);
        assert_eq!(image.get(1, 2), 0xffffff);
    }

    #[test]
    fn final_xor_inverts_even_rows_only() {
        let mut data = header(16, 16, 1, 1, 1, 0xff00);
        data.push(0);
        let image = decode_imc(&data).unwrap();
        assert_eq!(image.get(3, 0), 0x000000);
        assert_eq!(image.get(3, 1), 0xffffff);
    }

    #[test]
    fn truncated_streams_are_rejected() {
        let mut data = header(16, 16, 1, 1, 1, 0);
        data.push(0b1111_0000);
        assert!(decode_imc(&data).is_err());
    }
}
