//! ByteRun1, also known as Apple PackBits.
//!
//! Source: ILBM spec, Appendix C/D
//! (<https://wiki.amigaos.net/wiki/ILBM_IFF_Interleaved_Bitmap>).

use alloc::vec::Vec;

/// Unpacks ByteRun1 data from `src` until `len` bytes are produced.
///
/// Returns the unpacked bytes and the number of input bytes consumed, or
/// `None` if the input ends first. A run that overshoots `len` is truncated.
pub(crate) fn unpack(src: &[u8], len: usize) -> Option<(Vec<u8>, usize)> {
    let mut out = Vec::with_capacity(len);
    let mut pos = 0;
    while out.len() < len {
        let n = *src.get(pos)? as i8;
        pos += 1;
        match n {
            0..=127 => {
                let count = n as usize + 1;
                let literal = src.get(pos..pos + count)?;
                pos += count;
                out.extend_from_slice(literal);
            }
            -127..=-1 => {
                let value = *src.get(pos)?;
                pos += 1;
                out.resize(out.len() + (1 - n as isize) as usize, value);
            }
            -128 => {}
        }
    }
    out.truncate(len);
    Some((out, pos))
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
    fn truncated_input() {
        assert!(unpack(&[5, 1, 2], 6).is_none());
    }
}
