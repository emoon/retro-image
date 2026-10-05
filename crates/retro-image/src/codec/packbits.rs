//! PackBits (Apple), also known as ByteRun1 (Amiga IFF).
//!
//! Source: ILBM spec, Appendix C/D
//! (<https://wiki.amigaos.net/wiki/ILBM_IFF_Interleaved_Bitmap>).

use alloc::vec::Vec;

/// Unpacks ByteRun1 data from `src` until `len` bytes are produced.
///
/// Returns the unpacked bytes and the number of input bytes consumed, or
/// `None` if the input ends first. A run that overshoots `len` is truncated.
pub(crate) fn unpack(src: &[u8], len: usize) -> Option<(Vec<u8>, usize)> {
    // At most 128 bytes out per 2 bytes in: don't let a corrupt `len` from a
    // file header reserve more memory than the input could ever produce.
    let mut out = Vec::with_capacity(len.min(src.len().saturating_mul(64)));
    let mut pos = 0;
    while out.len() < len {
        pos = packet(src, pos, &mut out)?;
    }
    out.truncate(len);
    Some((out, pos))
}

/// Unpacks the packet at `pos` into `out` and returns the position after it,
/// or `None` if `src` ends inside it.
fn packet(src: &[u8], pos: usize, out: &mut Vec<u8>) -> Option<usize> {
    let n = *src.get(pos)? as i8;
    let pos = pos + 1;
    match n {
        0..=127 => {
            let count = n as usize + 1;
            out.extend_from_slice(src.get(pos..pos + count)?);
            Some(pos + count)
        }
        -127..=-1 => {
            let value = *src.get(pos)?;
            out.resize(out.len() + (1 - n as isize) as usize, value);
            Some(pos + 1)
        }
        -128 => Some(pos),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_and_run() {
        let (out, used) = unpack(&[2, 1, 2, 3, 0xfe, 9, 0x80, 0], 6).unwrap();
        assert_eq!(out, [1, 2, 3, 9, 9, 9]);
        assert_eq!(used, 6);
    }

    #[test]
    fn huge_requested_length_fails_without_reserving_it() {
        assert!(unpack(&[0xff, 1], usize::MAX).is_none());
    }

    #[test]
    fn truncated_input() {
        assert!(unpack(&[5, 1, 2], 6).is_none());
    }
}
