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

use super::common::{be16, be32, interleaved_index, st_rgb, uses_ste_bits, words};
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
    render_lines(bitmap, palettes, depth, LINES)
}

/// Renders `lines` interleaved lines with 48 palette words per line.
fn render_lines(bitmap: &[u8], palettes: &[u16], depth: ColorDepth, lines: usize) -> Option<Image> {
    let bitmap = bitmap.get(..lines * LINE_LEN)?;
    let palettes = palettes.get(..lines * 48)?;
    let mut image = Image::new(320, lines as u32);
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

/// Spectrum 512 Extended: `SPX`, version, two compression flags, screen
/// count, author and description strings, data sizes, then a blank line,
/// `screens * 199` lines and their palettes (unpacked, each part packed
/// with Pack-Ice, or version 2's backward LZ packer over both).
pub(super) fn decode_spx(data: &[u8]) -> Result<Image, DecodeError> {
    decode_spx_inner(data).ok_or(DecodeError::Unrecognized)
}

fn decode_spx_inner(data: &[u8]) -> Option<Image> {
    if data.get(..3)? != b"SPX" {
        return None;
    }
    let version = *data.get(3)?;
    let packed = (*data.get(4)?, *data.get(5)?) != (0, 0);
    let lines = usize::from(*data.get(6)?) * LINES;
    if lines == 0 {
        return None;
    }
    // Skip the two NUL-terminated strings.
    let mut pos = 10;
    for _ in 0..2 {
        pos += data.get(pos..)?.iter().position(|&b| b == 0)? + 1;
    }
    let gfx_len = be32(data, pos)? as usize;
    let palette_len = be32(data, pos + 4)? as usize;
    let body = data.get(pos + 8..)?;
    let (bitmap, palettes) = match (version, packed) {
        (2, true) => {
            let unpacked = unpack_spx2(body)?;
            let split = unpacked.len().checked_sub(lines * 96)?;
            let (bitmap, palettes) = unpacked.split_at(split);
            (bitmap.to_vec(), palettes.to_vec())
        }
        (1, _) | (_, false) => {
            // Each part may be packed with Pack-Ice on its own.
            let part = |data: &[u8], packed: u8| {
                if packed != 0 {
                    super::pack_ice::unpack(data)
                } else {
                    Some(data.to_vec())
                }
            };
            let bitmap = part(body.get(..gfx_len)?, data[4])?;
            let palettes = body.get(gfx_len..gfx_len.checked_add(palette_len)?)?;
            (bitmap, part(palettes, data[5])?)
        }
        _ => return None,
    };
    let palettes = words(&palettes);
    render_lines(
        bitmap.get(LINE_LEN..)?,
        &palettes,
        st_depth(&palettes),
        lines,
    )
}

/// SPX version 2 packer, per the Spectrum 512 Extended page: unpacked and
/// packed sizes, then a bit stream read backwards a long at a time (most
/// significant bit first) that fills the output from its end: `0` =
/// literals (count, bytes) normally followed by a match, `1` = match
/// (offset, length - 3). Counts and offsets are 4, 8, 12 or 16 bits, as
/// given by a 2-bit prefix. Match offsets are relative to the current
/// output position: derived from sample files.
fn unpack_spx2(data: &[u8]) -> Option<Vec<u8>> {
    let unpacked_len = be32(data, 0)? as usize;
    let packed_len = be32(data, 4)? as usize;
    let packed = data.get(8..8usize.checked_add(packed_len)?)?;
    // Bound the allocation: a match of at most 37 bits yields up to 65538
    // bytes.
    if unpacked_len > packed.len().saturating_mul(16384) || unpacked_len > 1 << 26 {
        return None;
    }
    let mut bits = BackwardBits {
        data: packed,
        pos: packed.len(),
        buffer: 0,
        left: 0,
    };
    let mut out = alloc::vec![0u8; unpacked_len];
    let mut dst = unpacked_len;
    let mut after_literals = false;
    while dst > 0 {
        let is_match = after_literals || bits.read(1)? == 1;
        after_literals = false;
        if is_match {
            let offset = bits.read_sized()? as usize;
            let count = bits.read_sized()? as usize + 3;
            for _ in 0..count {
                dst = dst.checked_sub(1)?;
                out[dst] = *out.get(dst + offset)?;
            }
        } else {
            let count = bits.read_sized()?;
            for _ in 0..count {
                dst = dst.checked_sub(1)?;
                out[dst] = bits.read(8)? as u8;
            }
            after_literals = count != 0xffff;
        }
    }
    Some(out)
}

struct BackwardBits<'a> {
    data: &'a [u8],
    pos: usize,
    buffer: u32,
    left: u32,
}

impl BackwardBits<'_> {
    fn read(&mut self, count: u32) -> Option<u32> {
        let mut value = 0;
        for _ in 0..count {
            if self.left == 0 {
                self.pos = self.pos.checked_sub(4)?;
                self.buffer = be32(self.data, self.pos)?;
                self.left = 32;
            }
            value = value << 1 | self.buffer >> 31;
            self.buffer <<= 1;
            self.left -= 1;
        }
        Some(value)
    }

    /// A 2-bit size prefix n, then a 4 * (n + 1)-bit value.
    fn read_sized(&mut self) -> Option<u32> {
        let size = 4 * (self.read(2)? + 1);
        self.read(size)
    }
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
