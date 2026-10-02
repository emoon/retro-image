//! Apple IIGS PackBytes.
//!
//! Sources: Apple IIGS Technical Note #94
//! (<https://apple2.gs/technotes/tn/iigs/TN.IIGS.094.txt>) and the CiderPress II
//! Super Hi-Res notes (<https://ciderpress2.com/formatdoc/SuperHiRes-notes.html>).
//! A flag byte's top two bits pick the mode and its low six bits hold a count
//! minus one: `00` count literal bytes, `01` one byte repeated count times,
//! `10` four bytes repeated count times, `11` one byte repeated count x 4 times.

use alloc::vec::Vec;

/// Unpacks from `src` until `src` ends or `len` bytes are produced.
/// Returns `None` if a run is cut short by the end of `src`.
pub(super) fn unpack(src: &[u8], len: usize) -> Option<Vec<u8>> {
    // A flag and one byte unpack to at most 256 bytes.
    let mut out = Vec::with_capacity(len.min(src.len().saturating_mul(128)));
    let mut pos = 0;
    while out.len() < len && pos < src.len() {
        let flag = src[pos];
        pos += 1;
        let count = usize::from(flag & 0x3f) + 1;
        match flag >> 6 {
            0 => {
                out.extend_from_slice(src.get(pos..pos + count)?);
                pos += count;
            }
            1 => {
                out.resize(out.len() + count, *src.get(pos)?);
                pos += 1;
            }
            2 => {
                let pattern = src.get(pos..pos + 4)?;
                pos += 4;
                for _ in 0..count {
                    out.extend_from_slice(pattern);
                }
            }
            _ => {
                out.resize(out.len() + count * 4, *src.get(pos)?);
                pos += 1;
            }
        }
    }
    out.truncate(len);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_modes() {
        let src = [0x01, 1, 2, 0x42, 7, 0x81, 1, 2, 3, 4, 0xc0, 9];
        let out = unpack(&src, 100).unwrap();
        assert_eq!(out, [1, 2, 7, 7, 7, 1, 2, 3, 4, 1, 2, 3, 4, 9, 9, 9, 9]);
    }
}
