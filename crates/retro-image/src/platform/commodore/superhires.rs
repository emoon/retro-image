//! The "Super Hires" family: 96-pixel-wide hires pictures with hires sprite
//! layers on top, shown as two interlaced frames.
//!
//! Sources: no file documentation exists (Codebase64's grafix specs list,
//! <http://codebase.c64.org/doku.php?id=base:c64_grafix_files_specs_list_v0.03>,
//! only names the parts: "2 x hires bitmap, 2 x 2 x hires sprite layers").
//! Everything below was reverse engineered from the sample files by
//! feeding modified copies to `recoil2png` and watching which pixels change:
//!
//! - Super Hires Interlace (SHI, 15355 bytes, load `$7FFF`): a zero byte,
//!   two 12×25-cell hires bitmaps (cells of 8 bytes, row by row), then the
//!   sprites of both frames (per frame ten 21-line bands of eight sprites:
//!   four for the upper layer, one per 24-pixel column, then four for the
//!   lower layer), one screen RAM of 12×25 cells shared by both frames, and
//!   the colours of the four upper and four lower sprite columns.
//! - Super Hires Interlace FLI (SIF): two packed sections, one per frame,
//!   then four colour bytes (upper and lower sprite layer of the first
//!   frame, then of the second). Each section starts with `$9400` plus its
//!   own length, then an escape byte; the rest unpacks backwards
//!   (`value count escape` runs, count 0 = 256) to
//!   8176 bytes holding four 2048-byte planes of 167 lines × 12 bytes: upper
//!   sprite layer, lower sprite layer, hires bitmap, and one colour byte
//!   per 8-pixel cell per line (FLI).
//!
//! In both, a set upper-layer bit wins over the lower layer, which wins over
//! the bitmap; the frames are averaged per channel.

use super::unpack::backward_rle_exact;
use super::vic2;
use crate::{DecodeError, Image};
use alloc::vec::Vec;

/// Width of every Super Hires picture.
const WIDTH: usize = 96;
/// Bytes of a 96-pixel line or row of cells.
const ROW: usize = WIDTH / 8;

/// Renders both interlace frames with `pixel(frame, x, y)` and blends them.
fn interlace(height: usize, pixel: impl Fn(usize, usize, usize) -> u8) -> Image {
    let frame = |f| {
        let colors: Vec<u8> = (0..height)
            .flat_map(|y| (0..WIDTH).map(move |x| (x, y)))
            .map(|(x, y)| pixel(f, x, y))
            .collect();
        vic2::image(WIDTH, height, colors)
    };
    Image::blend(&[&frame(0), &frame(1)])
}

/// Whether bit `x` (MSB first) of a line of bytes is set.
fn bit(line: &[u8], x: usize) -> bool {
    line[x / 8] & (0x80 >> (x % 8)) != 0
}

/// A hires pixel: the high nibble of `color` if set, else the low nibble.
fn hires(set: bool, color: u8) -> u8 {
    if set { color >> 4 } else { color & 15 }
}

const SHI_LEN: usize = 15355;
const SHI_BITMAP: usize = 3;
const SHI_BITMAP_LEN: usize = 25 * ROW * 8;
const SHI_SPRITES: usize = SHI_BITMAP + 2 * SHI_BITMAP_LEN;
/// Ten bands of eight 64-byte sprites.
const SHI_SPRITES_LEN: usize = 10 * 8 * 64;
const SHI_SCREEN: usize = SHI_SPRITES + 2 * SHI_SPRITES_LEN;
const SHI_COLORS: usize = SHI_SCREEN + 25 * ROW;

/// Super Hires Interlace Editor.
pub(super) fn decode_shi(data: &[u8]) -> Result<Image, DecodeError> {
    // `recoil2png` reads other sizes, or a nonzero byte at `$7FFF`, some
    // other way; there are no such samples.
    if data.len() != SHI_LEN || data[2] != 0 {
        return Err(DecodeError::Unrecognized);
    }
    let colors = &data[SHI_COLORS..SHI_COLORS + 8];
    Ok(interlace(200, |frame, x, y| {
        let (band, line) = (y / 21, y % 21);
        let column = x / 24;
        let sprite = |layer: usize| {
            let start = SHI_SPRITES
                + frame * SHI_SPRITES_LEN
                + band * 512
                + layer * 256
                + column * 64
                + line * 3;
            bit(&data[start..start + 3], x % 24)
        };
        if sprite(0) {
            colors[column]
        } else if sprite(1) {
            colors[4 + column]
        } else {
            let cell = y / 8 * ROW + x / 8;
            let byte = data[SHI_BITMAP + frame * SHI_BITMAP_LEN + cell * 8 + y % 8];
            hires(bit(&[byte], x % 8), data[SHI_SCREEN + cell])
        }
    }))
}

const SIF_HEIGHT: usize = 167;
const SIF_PLANE: usize = 2048;
const SIF_UNPACKED: usize = 4 * SIF_PLANE - 16;

/// Splits off one packed SIF section and unpacks it.
fn sif_section(data: &[u8]) -> Option<(Vec<u8>, &[u8])> {
    let len = usize::from(crate::bytes::le16(data, 0)?).checked_sub(0x9400)?;
    let (section, rest) = data.split_at_checked(len)?;
    let [_, _, escape, packed @ ..] = section else {
        return None;
    };
    Some((backward_rle_exact(packed, *escape, SIF_UNPACKED)?, rest))
}

/// Super Hires Interlace FLI Editor.
pub(super) fn decode_sif(data: &[u8]) -> Result<Image, DecodeError> {
    let (first, rest) = sif_section(data).ok_or(DecodeError::Unrecognized)?;
    let (second, rest) = sif_section(rest).ok_or(DecodeError::Unrecognized)?;
    let [c0, c1, c2, c3] = *rest else {
        return Err(DecodeError::Unrecognized);
    };
    let frames = [(&first, [c0, c1]), (&second, [c2, c3])];
    Ok(interlace(SIF_HEIGHT, |frame, x, y| {
        let (planes, colors) = frames[frame];
        let line = |plane: usize| &planes[plane * SIF_PLANE + y * ROW..][..ROW];
        if bit(line(0), x) {
            colors[0]
        } else if bit(line(1), x) {
            colors[1]
        } else {
            hires(bit(line(2), x), line(3)[x / 8])
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shi_layers_cover_the_bitmap() {
        let mut data = alloc::vec![0u8; SHI_LEN];
        data[SHI_SCREEN] = 0x21;
        data[SHI_BITMAP] = 0x80; // frame 0, pixel (0, 0) set
        data[SHI_COLORS] = 5;
        data[SHI_COLORS + 4] = 7;
        // Frame 1, band 0: lower-layer sprite of column 0 sets pixel (0, 0).
        data[SHI_SPRITES + SHI_SPRITES_LEN + 256] = 0x80;
        // Frame 0 and 1: upper layer sets pixel (1, 0).
        data[SHI_SPRITES] = 0x40;
        data[SHI_SPRITES + SHI_SPRITES_LEN] = 0x40;
        let image = decode_shi(&data).unwrap();
        let mix = |a, b| {
            Image::blend(&[
                &vic2::image(1, 1, alloc::vec![a]),
                &vic2::image(1, 1, alloc::vec![b]),
            ])
            .get(0, 0)
        };
        assert_eq!(image.get(0, 0), mix(2, 7));
        assert_eq!(image.get(1, 0), vic2::rgb(5));
        assert_eq!(image.get(2, 0), vic2::rgb(1));
    }

    #[test]
    fn sif_needs_two_exact_sections_and_colours() {
        // One section: header, escape 0xEE, 8176 zero bytes as 31 full runs
        // plus a run of 240.
        let mut section = alloc::vec![0, 0, 0xee];
        for _ in 0..31 {
            section.extend_from_slice(&[0, 0, 0xee]);
        }
        section.extend_from_slice(&[0, 240, 0xee]);
        let len = 0x9400 + section.len() as u16;
        section[..2].copy_from_slice(&len.to_le_bytes());
        let mut data = [section.clone(), section].concat();
        data.extend_from_slice(&[1, 2, 3, 4]);
        assert!(decode_sif(&data).is_ok());
        assert!(decode_sif(&data[..data.len() - 1]).is_err());
        data.push(0);
        assert!(decode_sif(&data).is_err());
    }
}
