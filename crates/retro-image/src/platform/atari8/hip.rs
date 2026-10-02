//! HIP (Hard Interlace Picture): a GTIA mode 9 frame and a mode 10 frame
//! shown alternately; mode 10 is shifted by half a pixel, which doubles the
//! horizontal resolution to 160. VZI does the same with two mode 9 frames.
//!
//! Sources:
//! - VZI: Just Solve "VertiZontal Interlacing"
//!   (<http://fileformats.archiveteam.org/wiki/VertiZontal_Interlacing>;
//!   16000 bytes, 2 frames); the frame order and shift direction are
//!   observed from `recoil2png` output.
//! - Just Solve "Hard Interlace Picture"
//!   (<http://fileformats.archiveteam.org/wiki/Hard_Interlace_Picture>);
//!   Mad Team TIP/HIP article
//!   (<https://madteam.atari8.info/index.php?atarynka=tip>); atari-owner.com
//!   "Atari Software Graphic Modes"
//!   (<https://atari-owner.com/club/articles/atari-software-graphic-modes.17/>);
//!   Altirra Hardware Reference Manual, GTIA mode 10 half-pixel shift
//!   (<https://www.virtualdub.org/downloads/Altirra%20Hardware%20Reference%20Manual.pdf>).
//! - Observed from `recoil2png` output: the layouts (16000 bytes: mode 9 frame
//!   then mode 10 frame; 16009 bytes: plus registers 704-712; 16012 bytes:
//!   two DOS binary-load segments, mode 10 frame first; 15372 bytes: the
//!   same with 192-line frames), the default mode 10
//!   registers (0, 0, 2, 4, ..., 14), the frames mixed by averaging, and the
//!   placement: mode 9 pixels start 1 output pixel left of the 4-pixel grid,
//!   mode 10 pixels 1 to the right, with black beyond the edges.

use super::palette::{register_rgb, rgb};
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
        16012 | 15372 => {
            let frame = data.len() / 2 - 6;
            let first = binary_segment(data, frame)?;
            let second = binary_segment(&data[frame + 6..], frame)?;
            (second, first, DEFAULT_REGISTERS)
        }
        _ => return Err(DecodeError::Unrecognized),
    };
    Ok(half_pixel_pair(
        320,
        gtia9.len() / 40,
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
        320,
        200,
        |y, x| rgb(nibble(&second[y * 40..], x)),
        |y, x| rgb(nibble(&first[y * 40..], x)),
    ))
}

/// Mixes two frames of `width` / 4 x `lines` given as `color(line, pixel)`:
/// `left`'s pixels start 1 output pixel left of the 4-pixel grid, `right`'s 1
/// pixel right.
pub(super) fn half_pixel_pair(
    width: usize,
    lines: usize,
    left: impl Fn(usize, usize) -> u32,
    right: impl Fn(usize, usize) -> u32,
) -> Image {
    let mut left_frame = Image::new(width as u32, lines as u32);
    let mut right_frame = Image::new(width as u32, lines as u32);
    for y in 0..lines {
        for x in 0..width {
            if x + 1 < width {
                left_frame.set(x as u32, y as u32, left(y, (x + 1) / 4));
            }
            if let Some(x1) = x.checked_sub(1) {
                right_frame.set(x as u32, y as u32, right(y, x1 / 4));
            }
        }
    }
    Image::blend(&[&left_frame, &right_frame])
}

/// The `len` bytes of a DOS binary-load segment (`FF FF`, start, end).
fn binary_segment(data: &[u8], len: usize) -> Result<&[u8], DecodeError> {
    match data {
        [0xff, 0xff, _, _, _, _, rest @ ..] if rest.len() >= len => Ok(&rest[..len]),
        _ => Err(DecodeError::Unrecognized),
    }
}

pub(super) fn nibble(line: &[u8], x: usize) -> u8 {
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
        let pixel = |x: u32| image.get(x, 0);
        // Mode 9 pixel 0 covers x = 0..=2 (one pixel left of the grid).
        assert_eq!([pixel(0), pixel(2), pixel(3)], [0x7f7f7f, 0x7f7f7f, 0]);
    }

    #[test]
    fn binary_load_needs_headers() {
        assert!(decode_hip(&[0; 16012]).is_err());
        assert!(decode_hip(&[0; 16001]).is_err());
    }
}
