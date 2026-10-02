//! Dot Designer's Club (T&E Soft) `CMP` pictures: compressed Screen 5 pixels.
//!
//! No documentation was found. Reverse engineered from samples (`CARTOON.CMP`,
//! Sunrise Picture Disk `INTRO1.CMP`) against `recoil2png` output, with the
//! limits probed on synthesized files:
//! - header: bytes per line (1-128), lines (1-212), and the first line (1 to
//!   the line count) from which each line is XORed with the one above;
//! - then, for every 64 bytes of the XORed bitmap: a byte flagging which of
//!   its eight 8-byte groups are not all zero (bit 7 first), a mask byte per
//!   flagged group flagging its non-zero bytes, then those bytes;
//! - the stream must end exactly with the bitmap, and nothing may be flagged
//!   past its end.

use alloc::vec;
use alloc::vec::Vec;

/// A decompressed picture: `bytes_per_line` bytes of Screen 5 pixels per line.
pub(super) struct Unpacked {
    pub bytes_per_line: usize,
    pub lines: usize,
    pub bitmap: Vec<u8>,
}

/// Unpacks a `CMP` file, or `None` if it isn't one.
pub(super) fn unpack(data: &[u8]) -> Option<Unpacked> {
    let (&[bytes_per_line, lines, first_xor], mut stream) = data.split_first_chunk::<3>()?;
    let (bytes_per_line, lines, first_xor) =
        (bytes_per_line as usize, lines as usize, first_xor as usize);
    if !(1..=128).contains(&bytes_per_line)
        || !(1..=212).contains(&lines)
        || !(1..=lines).contains(&first_xor)
    {
        return None;
    }
    let len = bytes_per_line * lines;
    let mut bitmap = vec![0u8; len];
    let next = |stream: &mut &[u8]| {
        let (&byte, rest) = stream.split_first()?;
        *stream = rest;
        Some(byte)
    };
    for unit in (0..len).step_by(64) {
        let groups = next(&mut stream)?;
        let mut masks = [0u8; 8];
        for (group, mask) in masks.iter_mut().enumerate() {
            if groups & (0x80 >> group) != 0 {
                *mask = next(&mut stream)?;
            }
        }
        for (group, mask) in masks.into_iter().enumerate() {
            for bit in 0..8 {
                if mask & (0x80 >> bit) != 0 {
                    let byte = bitmap.get_mut(unit + group * 8 + bit)?;
                    *byte = next(&mut stream)?;
                }
            }
        }
    }
    if !stream.is_empty() {
        return None;
    }
    for i in first_xor * bytes_per_line..len {
        bitmap[i] ^= bitmap[i - bytes_per_line];
    }
    Some(Unpacked {
        bytes_per_line,
        lines,
        bitmap,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unpacks_masked_bytes_and_xors_lines() {
        // 2 lines of 4 bytes: line 0 = 00 12 00 00, line 1 XOR delta = 00 00 00 34.
        let data = [4, 2, 1, 0x80, 0b0100_0001, 0x12, 0x34];
        let unpacked = unpack(&data).unwrap();
        assert_eq!(unpacked.bitmap, [0, 0x12, 0, 0, 0, 0x12, 0, 0x34]);
        // No XOR when it starts at the line count.
        let data = [4, 2, 2, 0x80, 0b0100_0001, 0x12, 0x34];
        assert_eq!(
            unpack(&data).unwrap().bitmap,
            [0, 0x12, 0, 0, 0, 0, 0, 0x34]
        );
    }

    #[test]
    fn rejects_bad_streams() {
        let good = [4, 1, 1, 0x80, 0xf0, 1, 2, 3, 4];
        assert!(unpack(&good).is_some());
        assert!(unpack(&good[..8]).is_none(), "truncated");
        assert!(
            unpack(&[4, 1, 1, 0x80, 0xf8, 1, 2, 3, 4, 5]).is_none(),
            "past the end"
        );
        assert!(unpack(&[4, 1, 1, 0x00, 0]).is_none(), "trailing data");
        assert!(unpack(&[4, 1, 0, 0x00]).is_none(), "XOR from line 0");
        assert!(unpack(&[4, 1, 2, 0x00]).is_none(), "XOR past the last line");
        assert!(unpack(&[129, 1, 1, 0x00, 0x00, 0x00]).is_none(), "too wide");
        assert!(unpack(&[4, 0, 1]).is_none(), "no lines");
    }
}
