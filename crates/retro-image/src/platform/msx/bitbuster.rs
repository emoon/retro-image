//! BitBuster-compressed data as stored in G9B and MIG files.
//!
//! Source: reverse engineered from the corpus G9B files against the pixels
//! `recoil2png` shows for them (no BitBuster documentation or code was read):
//! - a block count byte, then per block a LE16 compressed size and the data;
//! - each block is an LZ77 stream: flag bits, MSB first, are taken from a byte
//!   fetched from the stream whenever the previous 8 are used up; 0 means copy
//!   one literal byte, 1 means a match;
//! - a match is an interleaved Elias gamma length (start at 1; while a 1 bit is
//!   read, append the next bit) plus 1, then an offset byte: bits 0-6 are the
//!   offset; if bit 7 is set, 4 more bits from the bit stream are its bits 7-10;
//!   the copy starts offset + 1 bytes back;
//! - a length that overflows 16 bits ends the block.
//!
//! MIG files (reverse engineered from the corpus MIG files: their blocks then
//! unpack to exactly their stated sizes and consume exactly their stated
//! lengths) use the same flags and offsets but read the offset first and code
//! the length as plain Elias gamma: n 1 bits and a 0, then n bits below a
//! leading 1.

use alloc::vec::Vec;

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    bits: u8,
    left: u8,
}

impl Reader<'_> {
    fn byte(&mut self) -> Option<u8> {
        let byte = *self.data.get(self.pos)?;
        self.pos += 1;
        Some(byte)
    }

    fn bit(&mut self) -> Option<usize> {
        if self.left == 0 {
            self.bits = self.byte()?;
            self.left = 8;
        }
        let bit = self.bits >> 7;
        self.bits <<= 1;
        self.left -= 1;
        Some(bit as usize)
    }

    /// Interleaved gamma code, or `None` for the end marker or the end of data.
    fn interleaved_gamma(&mut self) -> Option<usize> {
        let mut value = 1;
        while self.bit()? == 1 {
            value = value << 1 | self.bit()?;
            if value > 0xffff {
                return None;
            }
        }
        Some(value)
    }

    /// Elias gamma code, or `None` for the end marker or the end of data.
    fn gamma(&mut self) -> Option<usize> {
        let mut bits = 0;
        while self.bit()? == 1 {
            bits += 1;
            if bits == 16 {
                return None;
            }
        }
        let mut value = 1;
        for _ in 0..bits {
            value = value << 1 | self.bit()?;
        }
        Some(value)
    }

    /// Match offset: a byte, extended by 4 bits when its bit 7 is set.
    fn offset(&mut self) -> Option<usize> {
        let low = self.byte()?;
        let mut offset = (low & 0x7f) as usize;
        if low & 0x80 != 0 {
            for shift in (7..11).rev() {
                offset |= self.bit()? << shift;
            }
        }
        Some(offset)
    }
}

/// Where the two BitBuster variants differ.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Variant {
    /// Interleaved gamma length, then the offset.
    G9b,
    /// Offset, then plain gamma length.
    Mig,
}

/// Appends one block to `out`, up to `limit` bytes in total; `None` if it
/// refers before its own start or stops on a truncated match.
pub(super) fn unpack_block(
    variant: Variant,
    block: &[u8],
    out: &mut Vec<u8>,
    limit: usize,
) -> Option<()> {
    let start = out.len();
    let mut reader = Reader {
        data: block,
        pos: 0,
        bits: 0,
        left: 0,
    };
    while out.len() < limit {
        let Some(flag) = reader.bit() else { break };
        if flag == 0 {
            let Some(byte) = reader.byte() else { break };
            out.push(byte);
            continue;
        }
        let (length, offset) = match variant {
            Variant::G9b => {
                let Some(length) = reader.interleaved_gamma() else {
                    break;
                };
                if reader.pos >= block.len() {
                    break;
                }
                (length, reader.offset()?)
            }
            Variant::Mig => {
                let offset = reader.offset()?;
                let Some(length) = reader.gamma() else { break };
                (length, offset)
            }
        };
        let distance = offset + 1;
        if distance > out.len() - start {
            return None;
        }
        for _ in 0..=length {
            out.push(out[out.len() - distance]);
        }
    }
    Some(())
}

/// Unpacks all blocks, stopping at `limit` bytes.
pub(super) fn unpack(data: &[u8], limit: usize) -> Option<Vec<u8>> {
    let (&count, mut rest) = data.split_first()?;
    let mut out = Vec::new();
    for _ in 0..count {
        let size = u16::from_le_bytes([*rest.first()?, *rest.get(1)?]) as usize;
        let block = rest.get(2..2 + size)?;
        unpack_block(Variant::G9b, block, &mut out, limit)?;
        rest = &rest[2 + size..];
    }
    out.truncate(limit);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn literal_then_run() {
        // Flags 0 (literal 7), 1 (match: gamma "0" = 1 -> length 2, then offset
        // byte 0 -> 1 back), then 1 with an overflowing gamma ends the block.
        let block = [0b0101_1111, 7, 0, 0xff, 0xff, 0xff];
        let mut data = vec![1, block.len() as u8, 0];
        data.extend(block);
        assert_eq!(unpack(&data, 100), Some(vec![7, 7, 7]));
    }

    #[test]
    fn mig_reads_offset_then_elias_gamma() {
        // Flags 0 (literal 7), 1 (match: offset byte 0, gamma "10" then "1"
        // = 3, so 4 bytes from 1 back).
        let block = [0b0110_1000, 7, 0];
        let mut out = Vec::new();
        assert_eq!(unpack_block(Variant::Mig, &block, &mut out, 5), Some(()));
        assert_eq!(out, [7; 5]);
    }

    #[test]
    fn rejects_offset_before_start() {
        let data = [1, 3, 0, 0b1000_0000, 5, 0];
        assert_eq!(unpack(&data, 100), None);
    }
}
