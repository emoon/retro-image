//! PhotoChrome pictures (`PCS`).
//!
//! Sources:
//! - <https://temlib.org/AtariForumWiki/index.php/PhotoChrome_file_format>
//!   (layout, compression and Hans Wessels' public-domain palette index
//!   function)
//! - <http://fileformats.archiveteam.org/wiki/PhotoChrome>
//! - Observed from `recoil2png` output: 320x199 output starting at the
//!   second scanline, which uses the first 48-colour palette; alternating
//!   screens are averaged per component.

use alloc::vec::Vec;

use super::common::{interleaved_index, separate_planes_to_interleaved, st_rgb, uses_ste_bits};
use crate::bytes::be16;
use crate::{DecodeError, Image};

const SCREEN_LEN: usize = 32000;
const PALETTE_LEN: usize = 9616;

pub(super) fn decode_pcs(data: &[u8]) -> Result<Image, DecodeError> {
    decode(data).ok_or(DecodeError::Unrecognized)
}

fn decode(data: &[u8]) -> Option<Image> {
    if be16(data, 0)? != 320 || be16(data, 2)? != 200 {
        return None;
    }
    let mode = *data.get(4)?;
    let mut pos = 6;
    let screen = unpack(data, &mut pos, SCREEN_LEN, 1)?;
    let palette = unpack(data, &mut pos, PALETTE_LEN * 2, 2)?;
    let first = Frame::new(&screen, &palette);
    if mode == 0 {
        return Some(first.render());
    }
    let mut screen2 = unpack(data, &mut pos, SCREEN_LEN, 1)?;
    if mode & 1 == 0 {
        screen2.iter_mut().zip(&screen).for_each(|(b, a)| *b ^= a);
    }
    let mut palette2 = unpack(data, &mut pos, PALETTE_LEN * 2, 2)?;
    if mode & 2 == 0 {
        palette2.iter_mut().zip(&palette).for_each(|(b, a)| *b ^= a);
    }
    let second = Frame::new(&screen2, &palette2);
    let a = first.render();
    let b = second.render();
    Some(Image::blend(&[&a, &b]))
}

struct Frame {
    /// Interleaved low-resolution screen.
    screen: Vec<u8>,
    palette: Vec<u16>,
}

impl Frame {
    fn new(planes: &[u8], palette: &[u8]) -> Self {
        let screen = separate_planes_to_interleaved(planes, 4);
        let palette = palette
            .as_chunks::<2>()
            .0
            .iter()
            .map(|w| u16::from_be_bytes([w[0], w[1]]))
            .collect();
        Self { screen, palette }
    }

    fn render(&self) -> Image {
        let ste = uses_ste_bits(self.palette.iter().copied());
        let mut image = Image::new(320, 199);
        for y in 1..200 {
            let line = &self.screen[y * 160..(y + 1) * 160];
            for x in 0..320 {
                let c = interleaved_index(line, x as u32, 4);
                let word = self.palette[(y - 1) * 48 + palette_index(x, c)];
                image.set(x as u32, (y - 1) as u32, st_rgb(word, ste));
            }
        }
        image
    }
}

/// Hans Wessels' `find_pcs_index`: which of 64 entries (from the line's
/// first) pixel `x` with colour `c` shows.
fn palette_index(x: usize, c: usize) -> usize {
    let mut index = c;
    let x1 = 4 * c;
    if x >= x1 {
        index += 16;
    }
    if x >= x1 + 64 + 12 && c < 14 {
        index += 16;
    }
    if x >= 132 + 16 && c == 14 {
        index += 16;
    }
    if x >= 132 + 20 && c == 15 {
        index += 16;
    }
    let mut x1 = 10 * c;
    if c & 1 == 1 {
        x1 -= 6;
    }
    if x >= 176 + x1 && c < 14 {
        index += 16;
    }
    index
}

/// Count of control bytes, then: `x < 0` copy `-x` literal units, `0`
/// repeat the next unit (word count), `1` copy (word count) units, `x > 1`
/// repeat the next unit `x` times. Units are `unit` bytes.
fn unpack(data: &[u8], pos: &mut usize, len: usize, unit: usize) -> Option<Vec<u8>> {
    let controls = be16(data, *pos)?;
    *pos += 2;
    let mut out = Vec::with_capacity(len);
    let take = |pos: &mut usize, n: usize| -> Option<&[u8]> {
        let slice = data.get(*pos..*pos + n)?;
        *pos += n;
        Some(slice)
    };
    for _ in 0..controls {
        let x = *take(pos, 1)?.first()? as i8;
        match x {
            i8::MIN..=-1 => {
                let n = usize::from(x.unsigned_abs());
                out.extend_from_slice(take(pos, n * unit)?);
            }
            0 => {
                let n = usize::from(be16(data, *pos)?);
                *pos += 2;
                let value = take(pos, unit)?;
                for _ in 0..n {
                    out.extend_from_slice(value);
                }
            }
            1 => {
                let n = usize::from(be16(data, *pos)?);
                *pos += 2;
                out.extend_from_slice(take(pos, n * unit)?);
            }
            _ => {
                let value = take(pos, unit)?;
                for _ in 0..x {
                    out.extend_from_slice(value);
                }
            }
        }
        if out.len() > len * 2 {
            return None;
        }
    }
    out.resize(len, 0);
    Some(out)
}
