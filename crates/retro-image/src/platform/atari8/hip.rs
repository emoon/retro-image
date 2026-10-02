//! HIP (Hard Interlace Picture): a GTIA mode 9 frame and a mode 10 frame
//! shown alternately; mode 10 is shifted by half a pixel, which doubles the
//! horizontal resolution to 160. VZI does the same with two mode 9 frames.
//!
//! Sources:
//! - VZI: Just Solve "VertiZontal Interlacing" (16000 bytes, 2 frames);
//!   the frame order and shift direction are observed from `recoil2png`
//!   output.
//! - Just Solve "Hard Interlace Picture"; Mad Team TIP/HIP article;
//!   atari-owner.com "Atari Software Graphic Modes"; Altirra Hardware
//!   Reference Manual (GTIA mode 10 half-pixel shift).
//! - Observed from `recoil2png` output: the layouts (16000 bytes: mode 9 frame
//!   then mode 10 frame; 16009 bytes: plus registers 704-712; 16012 bytes:
//!   two DOS binary-load segments, mode 10 frame first), the default mode 10
//!   registers (0, 0, 2, 4, ..., 14), the frames mixed by averaging, and the
//!   placement: mode 9 pixels start 1 output pixel left of the 4-pixel grid,
//!   mode 10 pixels 1 to the right, with black beyond the edges.

use super::palette::{average, register_rgb, rgb};
use super::screen::gtia10_register;
use crate::{DecodeError, Image};

const FRAME: usize = 8000;
const DEFAULT_REGISTERS: [u8; 9] = [0x00, 0x00, 0x02, 0x04, 0x06, 0x08, 0x0a, 0x0c, 0x0e];

pub(super) fn decode_hip(data: &[u8]) -> Result<Image, DecodeError> {
    let (gtia9, gtia10, registers) = match data.len() {
        16000 => (&data[..FRAME], &data[FRAME..], DEFAULT_REGISTERS),
        16009 => {
            let registers = data[2 * FRAME..]
                .try_into()
                .map_err(|_| DecodeError::Unrecognized)?;
            (&data[..FRAME], &data[FRAME..2 * FRAME], registers)
        }
        16012 => {
            let first = binary_segment(data)?;
            let second = binary_segment(&data[FRAME + 6..])?;
            (second, first, DEFAULT_REGISTERS)
        }
        _ => return Err(DecodeError::Unrecognized),
    };
    Ok(half_pixel_pair(
        |y, x| rgb(nibble(&gtia9[y * 40..], x)),
        |y, x| register_rgb(registers[gtia10_register(nibble(&gtia10[y * 40..], x))]),
    ))
}

/// VertiZontal Interlacing: two GTIA mode 9 frames; the second is drawn
/// half a pixel left of the first.
pub(super) fn decode_vzi(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 2 * FRAME {
        return Err(DecodeError::Unrecognized);
    }
    let (first, second) = data.split_at(FRAME);
    Ok(half_pixel_pair(
        |y, x| rgb(nibble(&second[y * 40..], x)),
        |y, x| rgb(nibble(&first[y * 40..], x)),
    ))
}

/// Mixes two 80x200 frames given as `color(line, pixel)`: `left`'s pixels
/// start 1 output pixel left of the 4-pixel grid, `right`'s 1 pixel right.
fn half_pixel_pair(
    left: impl Fn(usize, usize) -> u32,
    right: impl Fn(usize, usize) -> u32,
) -> Image {
    let mut image = Image::new(320, 200);
    for y in 0..200 {
        for x in 0..320 {
            let left = (x + 1 < 320).then(|| left(y, (x + 1) / 4));
            let right = x.checked_sub(1).map(|x| right(y, x / 4));
            let rgb = average(left.unwrap_or(0), right.unwrap_or(0));
            image.set(x as u32, y as u32, rgb);
        }
    }
    image
}

/// The 8000 bytes of a DOS binary-load segment (`FF FF`, start, end).
fn binary_segment(data: &[u8]) -> Result<&[u8], DecodeError> {
    match data {
        [0xff, 0xff, _, _, _, _, rest @ ..] if rest.len() >= FRAME => Ok(&rest[..FRAME]),
        _ => Err(DecodeError::Unrecognized),
    }
}

fn nibble(line: &[u8], x: usize) -> u8 {
    let byte = line[x / 2];
    if x.is_multiple_of(2) {
        byte >> 4
    } else {
        byte & 0x0f
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn frames_are_offset_by_half_a_pixel() {
        let mut data = vec![0u8; 16000];
        data[0] = 0xf0; // mode 9 pixel 0: luminance 15
        let image = decode_hip(&data).unwrap();
        let pixel = |x: usize| image.rgb()[x * 3];
        // Mode 9 pixel 0 covers x = 0..=2 (one pixel left of the grid).
        assert_eq!([pixel(0), pixel(2), pixel(3)], [0x7f, 0x7f, 0x00]);
    }

    #[test]
    fn binary_load_needs_headers() {
        assert!(decode_hip(&[0; 16012]).is_err());
        assert!(decode_hip(&[0; 16001]).is_err());
    }
}
