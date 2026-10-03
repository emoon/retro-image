//! Vertical Hires Interlace Editor (VHI).
//!
//! Sources: reverse engineered from the 3 samples in
//! `corpus/extra/commodore/csdb-vhi-editor` by mutating bytes and watching
//! `recoil2png`; no documentation was found (see docs/research/commodore.md).
//!
//! Memory image at `$2000` (the file's load address is not checked): two
//! hires bitmaps at `$2000` and `$4000`, one screen RAM at `$6000` shared by
//! both frames, then 3 bytes that do not affect the picture. The two frames
//! are blended to 320x200.
//!
//! A packed form has the same load address and a token stream: `01 count
//! value` repeats a byte (count 0 = 256) and `00 n` copies the next `n`
//! literal bytes (n 0 = 256). The stream is read only until the picture's
//! screen RAM is complete; whatever follows is ignored.

use super::prg::Prg;
use super::vic2::{BITMAP_LEN, Bitmap, Frame, SCREEN_LEN};
use crate::{DecodeError, Image};
use alloc::vec::Vec;

const LOAD: u16 = 0x2000;
const SECOND_BITMAP: u16 = 0x4000;
const SCREEN: u16 = 0x6000;
/// Bytes from `$2000` to the end of the screen RAM.
const PICTURE_LEN: usize = 0x4000 + SCREEN_LEN;
/// An unpacked file: load address, picture and the 3 trailing bytes.
const FILE_LEN: usize = 2 + PICTURE_LEN + 3;

pub(super) fn decode_vhi(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() == FILE_LEN {
        return decode_image(data);
    }
    let packed = data.get(2..).ok_or(DecodeError::Unrecognized)?;
    // `Prg` wants the load-address header in front of the memory image.
    let mut image = alloc::vec![0, 0];
    image.extend(unpack(packed)?);
    decode_image(&image)
}

fn decode_image(data: &[u8]) -> Result<Image, DecodeError> {
    let prg = Prg::new(data, LOAD);
    let screen = prg
        .at(SCREEN, SCREEN_LEN)
        .ok_or(DecodeError::Unrecognized)?;
    let frame = |addr| {
        let bitmap = prg.at(addr, BITMAP_LEN)?;
        Frame::hires(&Bitmap::hires(bitmap, screen), 200)
    };
    let (first, second) = frame(LOAD)
        .zip(frame(SECOND_BITMAP))
        .ok_or(DecodeError::Unrecognized)?;
    Ok(first.blend(&second, 0))
}

/// Expands the token stream until the picture is complete.
fn unpack(packed: &[u8]) -> Result<Vec<u8>, DecodeError> {
    let count = |byte: u8| if byte == 0 { 256 } else { usize::from(byte) };
    let mut out = Vec::with_capacity(PICTURE_LEN + 256);
    let mut rest = packed;
    while out.len() < PICTURE_LEN {
        match rest {
            [1, n, value, tail @ ..] => {
                out.extend(core::iter::repeat_n(*value, count(*n)));
                rest = tail;
            }
            [0, n, tail @ ..] => {
                let n = count(*n).min(tail.len());
                out.extend_from_slice(&tail[..n]);
                rest = &tail[n..];
            }
            _ => return Err(DecodeError::Unrecognized),
        }
    }
    out.truncate(PICTURE_LEN);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unpacks_runs_and_literals_until_complete() {
        // Two runs of 256 and a 0-count literal block shorter than 256.
        let mut packed = alloc::vec![1, 0, 7, 0, 3, 1, 2, 3];
        assert_eq!(unpack(&packed), Err(DecodeError::Unrecognized));
        packed.extend([1, 0, 9].repeat(PICTURE_LEN / 256));
        let out = unpack(&packed).unwrap();
        assert_eq!(out.len(), PICTURE_LEN);
        assert_eq!(out[..3], [7; 3]);
        assert_eq!(out[256..259], [1, 2, 3]);
    }

    #[test]
    fn rejects_unknown_tokens() {
        assert_eq!(unpack(&[2, 0, 0]), Err(DecodeError::Unrecognized));
    }
}
