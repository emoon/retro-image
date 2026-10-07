//! MagicDraw (`.SHR`) pictures: the TRS-80 Model 4 640x240 screen, run-length
//! coded.
//!
//! No documentation was found (survey: `docs/research/amiga-apple-misc.md`,
//! "Wave 5"). Reverse engineered from `corpus/magicdrw.shr` and by black-box
//! probing of `recoil2png` with synthetic files:
//!
//! - A control byte with the top bit set repeats the next byte `control & 0x7f`
//!   times; otherwise `control` literal bytes follow. Zero counts are allowed.
//! - The first 19200 bytes are the screen (80 per line, most significant bit
//!   leftmost, set bit white); a shorter stream is rejected. The sample goes
//!   on for another 822 bytes, which `recoil2png` ignores, so we do too.
//! - Our own restriction, since `.SHR` is a shared extension and `recoil2png`
//!   takes any stream that reaches 19200 bytes: the operations must also run
//!   cleanly to the end of the file. Of the 19 `.SHR` files in the corpus
//!   only the sample and a 32 KB Apple IIGS dump pass that.
//! - Lines are doubled, as for the plain 640x240 screen.

use alloc::vec::Vec;

use super::{WHITE, mono};
use crate::{DecodeError, Image};

const WIDTH: usize = 640;
const HEIGHT: usize = 240;
const SCREEN_LEN: usize = WIDTH / 8 * HEIGHT;

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    const FAIL: DecodeError = DecodeError::Invalid;
    let mut screen = Vec::with_capacity(SCREEN_LEN);
    let mut decoded = 0;
    let mut input = data.iter().copied();
    while let Some(control) = input.next() {
        let count = usize::from(control & 0x7f);
        if control & 0x80 != 0 {
            let byte = input.next().ok_or(FAIL)?;
            screen.resize((screen.len() + count).min(SCREEN_LEN), byte);
        } else {
            let literal = input.by_ref().take(count);
            let room = SCREEN_LEN - screen.len();
            let mut taken = 0;
            for byte in literal {
                if taken < room {
                    screen.push(byte);
                }
                taken += 1;
            }
            if taken < count {
                return Err(FAIL);
            }
        }
        decoded += count;
    }
    if decoded < SCREEN_LEN {
        return Err(FAIL);
    }
    mono(&screen, WIDTH, HEIGHT, WHITE)?.scaled(1, 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_runs_and_literals_and_ignores_the_tail() {
        // One literal byte 0x80, then zero runs to fill the screen.
        let mut data = alloc::vec![1, 0x80, 0x80, 0x00];
        let mut left = SCREEN_LEN - 1;
        while left > 0 {
            let run = left.min(127);
            data.extend([0x80 | run as u8, 0]);
            left -= run;
        }
        data.extend([2, 9, 9]);
        let image = decode(&data).unwrap();
        assert_eq!((image.width(), image.height()), (640, 480));
        assert_eq!((image.get(0, 0), image.get(1, 0)), (0xffffff, 0));
        assert_eq!(image.get(0, 1), 0xffffff, "lines are doubled");
    }

    #[test]
    fn rejects_short_and_ragged_streams() {
        assert!(decode(&[0x80 | 100, 0]).is_err());
        assert!(decode(&[]).is_err());
        let mut full = alloc::vec![];
        full.extend([0xff, 0x00].repeat(SCREEN_LEN / 127 + 1));
        assert!(decode(&full).is_ok());
        full.extend([3, 1, 2]); // literal cut short at the end of the file
        assert!(decode(&full).is_err());
    }
}
