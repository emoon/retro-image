//! Vector-06C.
//!
//! Sources:
//! - Screen: 4 bit planes of 32 columns, each column 256 bytes of 8 pixels
//!   (most significant bit leftmost) running bottom to top; 16 colors from
//!   a palette of 8-bit `BBGGGRRR` values: Wikipedia,
//!   <https://en.wikipedia.org/wiki/Vector-06C>.
//! - SPR layout (16 palette bytes, then a run-length stream read backwards
//!   from the end of the file into the screen, filled from its end), the
//!   plane order, the edge cases (trailing bytes, unused bytes after the
//!   palette, overrunning the screen) and the color levels: reverse
//!   engineered from the sample files of the `vector-06c-spr2bmp` repository,
//!   <https://github.com/drilnet/vector-06c-spr2bmp>
//!   (data only; its code and the `Info SPR` archive were not read) and
//!   `recoil2png` output, including hand-made probe files.

use alloc::vec;
use alloc::vec::Vec;

use crate::image::widen_channel;
use crate::{DecodeError, Format, Image};

pub(super) static FORMATS: &[Format] = &[Format::new(
    "Vector-06C",
    "Graphics file",
    &["spr"],
    decode_spr,
)];

const PALETTE_LEN: usize = 16;
const PLANE_LEN: usize = 32 * 256;
const SCREEN_LEN: usize = 4 * PLANE_LEN;

/// SPR: 16 palette bytes, then the screen packed backwards: reading from the
/// last byte of the file towards the palette, a byte `0x80 | n` repeats the
/// byte before it `n` times and a byte `n` < 0x80 takes the `n` bytes before
/// it (nearest first). Output fills the screen from its last byte down;
/// decoding stops once the screen is full, so bytes left between the
/// palette and the stream are ignored.
fn decode_spr(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() <= PALETTE_LEN {
        return Err(DecodeError::Unrecognized);
    }
    let (palette, stream) = data.split_at(PALETTE_LEN);
    let screen = unpack_backwards(stream)?;
    let palette: Vec<u32> = palette.iter().map(|&v| color(v)).collect();
    let mut indices = vec![0u8; 256 * 256];
    for (i, index) in indices.iter_mut().enumerate() {
        let (x, y) = (i % 256, i / 256);
        let offset = x / 8 * 256 + (255 - y);
        let shift = 7 - x % 8;
        // The first plane in the file holds the index's highest bit.
        *index = (0..4).fold(0, |acc, plane| {
            acc << 1 | (screen[plane * PLANE_LEN + offset] >> shift & 1)
        });
    }
    Image::from_indexed(256, 256, &indices, &palette)
}

/// Unpacks the backwards run-length stream into a full screen, failing if
/// the stream runs out first.
fn unpack_backwards(stream: &[u8]) -> Result<Vec<u8>, DecodeError> {
    let mut screen = vec![0u8; SCREEN_LEN];
    let mut out = SCREEN_LEN;
    let mut bytes = stream.iter().rev().copied();
    let mut next = || bytes.next().ok_or(DecodeError::Unrecognized);
    while out > 0 {
        let control = next()?;
        let count = usize::from(control & 0x7f).min(out);
        if control & 0x80 != 0 {
            let value = next()?;
            screen[out - count..out].fill(value);
            out -= count;
        } else {
            for _ in 0..count {
                out -= 1;
                screen[out] = next()?;
            }
        }
    }
    Ok(screen)
}

/// A `BBGGGRRR` palette byte as RGB. Red and blue repeat their bits, so red
/// spreads its 8 levels evenly over 0-255 and blue steps by 85; green steps
/// by 36, which makes its top level 252.
fn color(v: u8) -> u32 {
    let r = widen_channel(u32::from(v & 7), 3);
    let g = u32::from(v >> 3 & 7) * 36;
    let b = widen_channel(u32::from(v >> 6), 2);
    r << 16 | g << 8 | b
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 16 palette bytes, then `runs` runs of 127 zeros and a literal of
    /// `tail`, which (read first) ends the screen.
    fn spr(palette: [u8; 16], tail: &[u8], runs: usize) -> Vec<u8> {
        let mut data = palette.to_vec();
        for _ in 0..runs {
            data.extend_from_slice(&[0, 0xff]);
        }
        data.extend_from_slice(tail);
        data.push(tail.len() as u8);
        data
    }

    #[test]
    fn spr_unpacks_backwards_into_bottom_up_columns() {
        let mut palette = [0; 16];
        palette[1] = 0x07; // Lowest index bit set: full red.
        // The screen's last two bytes: the last plane's last column, rows
        // stored bottom up, so they are the top two rows.
        let runs = (SCREEN_LEN - 2).div_ceil(127);
        let image = decode_spr(&spr(palette, &[0x01, 0x80], runs)).unwrap();
        assert_eq!(image.get(248, 0), 0xff0000);
        assert_eq!(image.get(255, 1), 0xff0000);
        assert_eq!(image.get(255, 0), 0);
        assert_eq!(color(0xff), 0xfffcff);
    }

    #[test]
    fn spr_rejects_a_stream_that_runs_out() {
        let data = spr([0; 16], &[1, 2], (SCREEN_LEN - 2) / 127 - 1);
        assert!(decode_spr(&data).is_err());
    }
}
