//! ECI Graphic Editor, packed (`.ecp`).
//!
//! Sources: the unpacked layout is ECI's (see `ifli.rs`). The packer is
//! undocumented; reverse engineered from `TESTP.ECP` by black-box probing of
//! `recoil2png` with repacked copies: load address `$4000`, then the escape
//! byte, then `ESC count value` runs (count 0 is an empty run, not 256)
//! and literal bytes. Unpacking stops after the 32768 bytes of `$4000-$BFFF`;
//! bytes left after that (3 in the sample) are ignored, and a stream that
//! ends earlier is rejected.

use super::ifli;
use crate::{DecodeError, Image};
use alloc::vec::Vec;

const UNPACKED_LEN: usize = 0x8000;

pub(super) fn decode_ecp(data: &[u8]) -> Result<Image, DecodeError> {
    if data.get(..2) != Some(&[0x00, 0x40]) {
        return Err(DecodeError::Unrecognized);
    }
    let (&escape, packed) = data[2..].split_first().ok_or(DecodeError::Unrecognized)?;
    let mut memory = Vec::with_capacity(UNPACKED_LEN + 2);
    memory.extend_from_slice(&data[..2]);
    let mut bytes = packed.iter().copied();
    while memory.len() < UNPACKED_LEN + 2 {
        let byte = bytes.next().ok_or(DecodeError::Unrecognized)?;
        if byte == escape {
            let count = bytes.next().ok_or(DecodeError::Unrecognized)?;
            let value = bytes.next().ok_or(DecodeError::Unrecognized)?;
            memory.extend(core::iter::repeat_n(value, usize::from(count)));
        } else {
            memory.push(byte);
        }
    }
    memory.truncate(UNPACKED_LEN + 2);
    ifli::decode_eci(&memory)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_count_is_an_empty_run_and_extra_bytes_are_ignored() {
        let mut data = alloc::vec![0x00, 0x40, 0xf3];
        data.extend_from_slice(&[0xf3, 0x00, 0xaa]); // empty run
        data.extend_from_slice(&[0xf3, 0xff, 0x00]);
        data.extend_from_slice(&[0xf3, 0x00, 0x00]);
        // 255 zeros so far; fill the rest with runs of 255 then literals.
        let mut produced = 255;
        while produced + 255 <= UNPACKED_LEN {
            data.extend_from_slice(&[0xf3, 0xff, 0x00]);
            produced += 255;
        }
        data.extend(core::iter::repeat_n(0u8, UNPACKED_LEN - produced));
        assert!(decode_ecp(&data).is_ok());
        data.extend_from_slice(&[1, 2, 3]);
        assert!(decode_ecp(&data).is_ok());
        data.truncate(data.len() - 4);
        assert!(decode_ecp(&data).is_err());
    }
}
