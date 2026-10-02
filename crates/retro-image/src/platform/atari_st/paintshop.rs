//! PaintShop compressed pictures (`PSC`).
//!
//! Source: <https://temlib.org/AtariForumWiki/index.php/PaintShop_file_format>.
//! Lines are `ceil(width / 8)` bytes, not always 80: derived from sample files.

use alloc::vec::Vec;

use super::common::mono_image;
use crate::bytes::be16;
use crate::{DecodeError, Image};

pub(super) fn decode_psc(data: &[u8]) -> Result<Image, DecodeError> {
    decode(data).ok_or(DecodeError::Unrecognized)
}

fn decode(data: &[u8]) -> Option<Image> {
    if data.get(..6)? != b"tm89PS" {
        return None;
    }
    let width = usize::from(be16(data, 10)?) + 1;
    let height = usize::from(be16(data, 12)?) + 1;
    if width > 640 || height > 400 {
        return None;
    }
    let line_len = width.div_ceil(8);
    let bitmap = unpack(&data[14..], line_len, height)?;
    mono_image(&bitmap, width as u32, height as u32, line_len)
}

/// Runs the scanline opcodes into `height` lines of `line_len` bytes.
fn unpack(data: &[u8], line_len: usize, height: usize) -> Option<Vec<u8>> {
    let total = line_len * height;
    let mut screen: Vec<u8> = Vec::with_capacity(total);
    let mut pos = 0;
    let mut next = || {
        let b = data.get(pos).copied();
        pos += 1;
        b
    };
    while screen.len() < total {
        match next()? {
            0 => screen.resize(screen.len() + line_len, 0),
            200 => screen.resize(screen.len() + line_len, 0xff),
            code @ (10 | 12) => {
                let mut count = usize::from(next()?) + 1;
                if code == 12 {
                    count += 256;
                }
                let start = screen.len().checked_sub(line_len)?;
                for _ in 0..count {
                    screen.extend_from_within(start..start + line_len);
                }
            }
            99 => {
                for _ in 0..total {
                    screen.push(next()?);
                }
            }
            100 => {
                let b = next()?;
                screen.resize(screen.len() + line_len, b);
            }
            102 => {
                let pattern = [next()?, next()?];
                screen.extend((0..line_len).map(|i| pattern[i % 2]));
            }
            110 => {
                for _ in 0..line_len {
                    screen.push(next()?);
                }
            }
            255 => break,
            _ => return None,
        }
    }
    screen.resize(total, 0);
    Some(screen)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opcodes() {
        let screen = unpack(&[200, 10, 0, 102, 0xaa, 0x55, 255], 80, 400).unwrap();
        assert_eq!(screen[0], 0xff);
        assert_eq!(screen[2 * 80 - 1], 0xff);
        assert_eq!(&screen[2 * 80..2 * 80 + 2], &[0xaa, 0x55]);
        assert_eq!(screen[3 * 80], 0);
    }

    #[test]
    fn repeat_needs_a_line_above() {
        assert_eq!(unpack(&[10, 0], 80, 400), None);
    }
}
