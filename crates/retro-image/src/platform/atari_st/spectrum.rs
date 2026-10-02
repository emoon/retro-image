//! Spectrum 512 pictures: uncompressed (`SPU`, including the 15-bit
//! `5BIT` variant), compressed (`SPC`) and smooshed (`SPS`).
//!
//! Sources:
//! - <https://temlib.org/AtariForumWiki/index.php/Spectrum_512_file_format>
//!   (layout and Steve Belczyk's public-domain palette index function)
//! - <https://temlib.org/AtariForumWiki/index.php/Spectrum_512_Compressed_file_format>
//! - <https://temlib.org/AtariForumWiki/index.php/Spectrum_512_Smooshed_file_format>
//! - <https://temlib.org/AtariForumWiki/index.php/Spectrum_512_Enhanced_file_format>
//! - Output is 320x199 (the unused first line is dropped): observed from
//!   `recoil2png` output.

use alloc::vec::Vec;

use super::common::{be16, be32, interleaved_index, st_rgb, uses_ste_bits};
use crate::{DecodeError, Image};

const LINES: usize = 199;
const LINE_LEN: usize = 160;
const BITMAP_LEN: usize = LINES * LINE_LEN;
const PALETTE_WORDS: usize = LINES * 48;
const SPU_LEN: usize = LINE_LEN + BITMAP_LEN + PALETTE_WORDS * 2;

/// How palette words map to colors.
#[derive(Clone, Copy)]
enum ColorDepth {
    /// ST or STE palette word, see [`st_rgb`].
    St { ste: bool },
    /// 15-bit `5BIT` variant: the top three bits carry each component's LSB.
    Fifteen,
}

fn color(word: u16, depth: ColorDepth) -> u32 {
    match depth {
        ColorDepth::St { ste } => st_rgb(word, ste),
        ColorDepth::Fifteen => {
            let component = |nibble: u16, low: u16| {
                // nibble bits: 3 = bit 1, 2..0 = bits 4..2; `low` holds bit 0.
                let v = u32::from((nibble & 7) << 2 | (nibble >> 3 & 1) << 1 | (word >> low) & 1);
                (v << 3) | (v >> 2)
            };
            component(word >> 8, 15) << 16 | component(word >> 4, 14) << 8 | component(word, 13)
        }
    }
}

/// Belczyk's `FindIndex`: which of a line's 48 colors pixel `x` with color
/// index `c` shows.
fn palette_index(x: usize, c: usize) -> usize {
    let x1 = 10 * c;
    let x1 = if c & 1 != 0 { x1 - 5 } else { x1 + 1 };
    if x >= x1 + 160 {
        c + 32
    } else if x >= x1 {
        c + 16
    } else {
        c
    }
}

/// Renders 199 interleaved lines with 48 palette words per line.
fn render(bitmap: &[u8], palettes: &[u16], depth: ColorDepth) -> Option<Image> {
    let bitmap = bitmap.get(..BITMAP_LEN)?;
    let palettes = palettes.get(..PALETTE_WORDS)?;
    let mut image = Image::new(320, LINES as u32);
    for (y, (line, palette)) in bitmap
        .chunks_exact(LINE_LEN)
        .zip(palettes.chunks_exact(48))
        .enumerate()
    {
        for x in 0..320 {
            let c = interleaved_index(line, x, 4);
            let word = palette[palette_index(x as usize, c)];
            image.set(x, y as u32, color(word, depth));
        }
    }
    Some(image)
}

fn st_depth(palettes: &[u16]) -> ColorDepth {
    ColorDepth::St {
        ste: uses_ste_bits(palettes.iter().copied()),
    }
}

fn words(data: &[u8]) -> Vec<u16> {
    data.chunks_exact(2)
        .map(|w| u16::from_be_bytes([w[0], w[1]]))
        .collect()
}

pub(super) fn decode_spu(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != SPU_LEN {
        return Err(DecodeError::Unrecognized);
    }
    let palettes = words(&data[LINE_LEN + BITMAP_LEN..]);
    let depth = if &data[..4] == b"5BIT" {
        ColorDepth::Fifteen
    } else {
        st_depth(&palettes)
    };
    render(&data[LINE_LEN..], &palettes, depth).ok_or(DecodeError::Unrecognized)
}

/// The `SP` header: data and color map lengths.
fn header(data: &[u8]) -> Option<(&[u8], &[u8])> {
    if be16(data, 0)? != 0x5350 {
        return None;
    }
    let data_len = be32(data, 4)? as usize;
    let color_len = be32(data, 8)? as usize;
    let body = data.get(12..)?;
    let bitmap = body.get(..data_len)?;
    let colors = body.get(data_len..data_len.checked_add(color_len)?)?;
    Some((bitmap, colors))
}

/// Converts four separate plane blocks (each 199 lines of 40 bytes) to
/// interleaved lines.
fn separate_planes_to_interleaved(planes: &[u8]) -> Vec<u8> {
    let mut out = alloc::vec![0; BITMAP_LEN];
    for (i, &b) in planes.iter().enumerate().take(BITMAP_LEN) {
        let plane = i / (LINES * 40);
        let y = i / 40 % LINES;
        let byte = i % 40;
        out[y * LINE_LEN + byte / 2 * 8 + plane * 2 + byte % 2] = b;
    }
    out
}

pub(super) fn decode_spc(data: &[u8]) -> Result<Image, DecodeError> {
    decode_spc_inner(data).ok_or(DecodeError::Unrecognized)
}

fn decode_spc_inner(data: &[u8]) -> Option<Image> {
    let (packed, colors) = header(data)?;
    let planes = unpack_spc(packed)?;
    let bitmap = separate_planes_to_interleaved(&planes);
    let mut palettes = Vec::with_capacity(PALETTE_WORDS);
    let mut pos = 0;
    for _ in 0..LINES * 3 {
        let mask = be16(colors, pos)?;
        pos += 2;
        for entry in 0..16 {
            if entry < 15 && mask & (1 << entry) != 0 {
                palettes.push(be16(colors, pos)?);
                pos += 2;
            } else {
                palettes.push(0);
            }
        }
    }
    render(&bitmap, &palettes, st_depth(&palettes))
}

/// SPC data RLE: `0..=127` literal `x + 1` bytes, negative: repeat `2 - x`.
fn unpack_spc(data: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(BITMAP_LEN);
    let mut pos = 0;
    while out.len() < BITMAP_LEN {
        let x = *data.get(pos)? as i8;
        pos += 1;
        if x >= 0 {
            let count = x as usize + 1;
            out.extend_from_slice(data.get(pos..pos + count)?);
            pos += count;
        } else {
            let value = *data.get(pos)?;
            pos += 1;
            out.extend(core::iter::repeat_n(value, (2 - isize::from(x)) as usize));
        }
    }
    out.truncate(BITMAP_LEN);
    Some(out)
}

pub(super) fn decode_sps(data: &[u8]) -> Result<Image, DecodeError> {
    decode_sps_inner(data).ok_or(DecodeError::Unrecognized)
}

fn decode_sps_inner(data: &[u8]) -> Option<Image> {
    let (packed, colors) = header(data)?;
    let unpacked = unpack_sps(packed)?;
    let planes = if colors.last()? & 1 != 0 {
        unpacked
    } else {
        strips_to_separate_planes(&unpacked)
    };
    let bitmap = separate_planes_to_interleaved(&planes);
    let mut bits = BitReader {
        data: colors,
        pos: 0,
    };
    let mut palettes = Vec::with_capacity(PALETTE_WORDS);
    for _ in 0..LINES * 3 {
        let mask = bits.read(14)?;
        palettes.push(0);
        for entry in 1..15 {
            if mask & (1 << (14 - entry)) != 0 {
                let rgb = bits.read(9)?;
                palettes.push((rgb >> 6 & 7) << 8 | (rgb >> 3 & 7) << 4 | rgb & 7);
            } else {
                palettes.push(0);
            }
        }
        palettes.push(0);
    }
    render(&bitmap, &palettes, st_depth(&palettes))
}

/// SPS data RLE: `0..=127` repeat `x + 3`, `128..=255` literal `x - 127`.
fn unpack_sps(data: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(BITMAP_LEN);
    let mut pos = 0;
    while out.len() < BITMAP_LEN {
        let x = *data.get(pos)?;
        pos += 1;
        if x < 128 {
            let value = *data.get(pos)?;
            pos += 1;
            out.extend(core::iter::repeat_n(value, usize::from(x) + 3));
        } else {
            let count = usize::from(x) - 127;
            out.extend_from_slice(data.get(pos..pos + count)?);
            pos += count;
        }
    }
    out.truncate(BITMAP_LEN);
    Some(out)
}

/// Reorders byte-wide vertical strips (per plane, per byte column, lines
/// top to bottom) into separate plane blocks of 40-byte lines.
fn strips_to_separate_planes(strips: &[u8]) -> Vec<u8> {
    let mut out = alloc::vec![0; BITMAP_LEN];
    for (i, &b) in strips.iter().enumerate().take(BITMAP_LEN) {
        let plane = i / (LINES * 40);
        let column = i / LINES % 40;
        let y = i % LINES;
        out[plane * LINES * 40 + y * 40 + column] = b;
    }
    out
}

struct BitReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl BitReader<'_> {
    fn read(&mut self, bits: u32) -> Option<u16> {
        let mut value = 0u16;
        for _ in 0..bits {
            let byte = *self.data.get(self.pos / 8)?;
            let bit = (byte >> (7 - self.pos % 8)) & 1;
            value = value << 1 | u16::from(bit);
            self.pos += 1;
        }
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn belczyk_index() {
        assert_eq!(palette_index(0, 0), 0);
        assert_eq!(palette_index(1, 0), 16);
        assert_eq!(palette_index(161, 0), 32);
        assert_eq!(palette_index(4, 1), 1);
        assert_eq!(palette_index(5, 1), 17);
    }

    #[test]
    fn fifteen_bit_color() {
        // 0x777 = 11100 per component.
        assert_eq!(color(0x0777, ColorDepth::Fifteen), 0xe7e7e7);
        assert_eq!(color(0xefff, ColorDepth::Fifteen), 0xffffff);
    }
}
