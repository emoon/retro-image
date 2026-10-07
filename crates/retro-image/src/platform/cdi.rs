//! Philips CD-i IFF images (`FORM`, `IMAG`): CLUT4, RL7 and DYUV coding.
//!
//! Sources: reverse engineered from the 11 sample files at
//! <https://sembiance.com/fileFormatSamples/image/cdiIFFImage/> (extensions
//! `.4c4`, `.6r7`, `.4dy`, `.6dy`, `.iff`; in `corpus/extra/small-consoles/cdi`),
//! with Deark's `cdi_imag` module (<https://github.com/jsummers/deark>, MIT)
//! run as a black box as the reference: the output of all 11 files is
//! identical to Deark's, pixel for pixel. Deark's source was not read.
//!
//! Provenance of two facts:
//! - The DYUV delta table (0, 1, 4, 9, 16, 27, 44, 79, 128, and the negatives
//!   of 79, 44, 27, 16, 9, 4 and 1 modulo 256) reached this project through
//!   its survey note `gaps-consoles.md`, section 3.10. That note cites
//!   Philips Technical Note 86 for it, a document with a "not to be
//!   duplicated" banner that the maintainer did not approve reading. The
//!   author of this decoder did not read that note. The table is confirmed
//!   by the four DYUV samples, which decode identically to Deark and use all
//!   16 entries as Y deltas.
//! - The DYUV color constants (1.371 and 1.733, and the green formula) come
//!   from fitting to Deark's output, not from any document or from Deark's
//!   source: with these values every pixel of the four samples matches.
//!
//! The layouts below are what the samples show; nothing else about the format
//! is known here.
//!
//! The file is `FORM`, a big-endian length, `IMAG`, then chunks of a 4-byte
//! tag, a big-endian length and the data, padded to an even length:
//! - `IHDR`, 14 bytes: width, line size in bytes and height (16-bit),
//!   then two 16-bit fields that tell the coding apart (3 and 8 for DYUV, 6
//!   and 4 for CLUT4, 8 and 8 for RL7), and four bytes of which the last three
//!   are the DYUV start values Y, U and V (16, 128, 128 in every sample).
//! - `PLTE`: a 16-bit first index, a 16-bit count, then RGB bytes (CLUT4
//!   has 16 colors, RL7 128).
//! - `IDAT`: the pixels.
//!
//! The codings:
//! - CLUT4: 4-bit palette indices, the high nibble on the left, in lines of
//!   `line size` bytes.
//! - RL7: per line, a byte below 0x80 is one pixel of that palette index; a
//!   byte of 0x80 or more is a run of the index in its low 7 bits, whose
//!   length is the next byte, 0 meaning to the end of the line.
//! - DYUV: a line of `width` bytes, a pair of bytes for a pair of pixels. A
//!   nibble is an index into the delta table. The first byte of a pair has
//!   the delta of U in its high nibble and of the first pixel's Y in its low
//!   one, the second byte has V's and the second pixel's Y. Y, U and V
//!   accumulate modulo 256 along the line from the start values, and both
//!   pixels of a pair share U and V.
//!
//! Colors: DYUV pixels become RGB with `R = Y + 1.371 V'`, `B = Y + 1.733 U'`
//! and G from the luma equation `Y = 0.299 R + 0.587 G + 0.114 B`, where V'
//! and U' are V and U minus 128, rounded to nearest and clamped. These
//! constants were fitted to Deark's output and reproduce it exactly on the
//! four DYUV samples. The pixels are used as stored, although a CD-i screen
//! does not have square pixels.
//!
//! Other codings (CLUT7, CLUT8, RGB 5-5-5, QHY, multi-plane files) have no
//! sample here and are rejected. Detection: the `FORM`/`IMAG`/`IHDR` start is
//! a signature.

use alloc::vec::Vec;

use crate::bytes::{be16, be32};
use crate::image::check_size;
use crate::{DecodeError, Format, Image};

pub(super) static FORMATS: &[Format] = &[Format::new(
    "Philips CD-i",
    "IFF image",
    &["4c4", "6r7", "4dy", "6dy"],
    decode,
)
.signature()];

/// What the two coding fields of `IHDR` select.
#[derive(Clone, Copy, PartialEq)]
enum Coding {
    Dyuv,
    Clut4,
    Rl7,
}

/// The change of Y, U or V for each DYUV nibble.
const DELTAS: [u8; 16] = [
    0, 1, 4, 9, 16, 27, 44, 79, 128, 177, 212, 229, 240, 247, 252, 255,
];

struct Header {
    width: usize,
    line_len: usize,
    height: usize,
    coding: Coding,
    /// Start values of a DYUV line: Y, U, V.
    start: [u8; 3],
}

fn header(ihdr: &[u8]) -> Option<Header> {
    if ihdr.len() != 14 {
        return None;
    }
    let coding = match (be16(ihdr, 6)?, be16(ihdr, 8)?) {
        (3, 8) => Coding::Dyuv,
        (6, 4) => Coding::Clut4,
        (8, 8) => Coding::Rl7,
        _ => return None,
    };
    Some(Header {
        width: usize::from(be16(ihdr, 0)?),
        line_len: usize::from(be16(ihdr, 2)?),
        height: usize::from(be16(ihdr, 4)?),
        coding,
        start: [ihdr[11], ihdr[12], ihdr[13]],
    })
}

/// The chunks inside the `FORM` as (tag, data).
fn chunks(data: &[u8]) -> Option<Vec<(&[u8], &[u8])>> {
    if data.get(..4)? != b"FORM" || data.get(8..12)? != b"IMAG" {
        return None;
    }
    let end = (be32(data, 4)? as usize).checked_add(8)?.min(data.len());
    let mut chunks = Vec::new();
    let mut at = 12;
    while at + 8 <= end {
        let len = be32(data, at + 4)? as usize;
        let body = data.get(at + 8..at.checked_add(8)?.checked_add(len)?)?;
        chunks.push((&data[at..at + 4], body));
        at += 8 + len + (len & 1);
    }
    Some(chunks)
}

/// The palette of a `PLTE` chunk, black before its first index.
fn palette(plte: &[u8]) -> Option<Vec<u32>> {
    let (first, count) = (usize::from(be16(plte, 0)?), usize::from(be16(plte, 2)?));
    let rgb = plte.get(4..4 + count * 3)?;
    let mut colors = alloc::vec![0; first];
    colors.extend(
        rgb.as_chunks::<3>()
            .0
            .iter()
            .map(|&[r, g, b]| u32::from_be_bytes([0, r, g, b])),
    );
    Some(colors)
}

/// Palette indices of a CLUT4 image.
fn clut4(h: &Header, idat: &[u8]) -> Option<Vec<u8>> {
    if h.line_len < h.width.div_ceil(2) {
        return None;
    }
    let lines = idat.get(..h.line_len.checked_mul(h.height)?)?;
    let mut indices = Vec::with_capacity(h.width * h.height);
    for line in lines.chunks_exact(h.line_len) {
        indices.extend((0..h.width).map(|x| line[x / 2] >> (4 - x % 2 * 4) & 15));
    }
    Some(indices)
}

/// Palette indices of an RL7 image.
fn rl7(h: &Header, idat: &[u8]) -> Option<Vec<u8>> {
    let mut indices = Vec::with_capacity(h.width * h.height);
    let mut bytes = idat.iter().copied();
    for _ in 0..h.height {
        let line_end = indices.len() + h.width;
        while indices.len() < line_end {
            let byte = bytes.next()?;
            let count = if byte < 0x80 {
                1
            } else {
                match bytes.next()? {
                    0 => line_end - indices.len(),
                    run => usize::from(run).min(line_end - indices.len()),
                }
            };
            indices.resize(indices.len() + count, byte & 0x7f);
        }
    }
    Some(indices)
}

/// A DYUV pixel as `0xRRGGBB`.
fn dyuv_color(y: u8, u: u8, v: u8) -> u32 {
    let (y, u, v) = (i32::from(y), i32::from(u) - 128, i32::from(v) - 128);
    // 1.371 and 1.733, rounded to nearest.
    let red = y + (1371 * v + 500).div_euclid(1000);
    let blue = y + (1733 * u + 500).div_euclid(1000);
    // (Y - 0.299 R - 0.114 B) / 0.587 with R and B before rounding, as one
    // exact fraction of 587000.
    let green = y + (293_500 - (299 * 1371 * v + 114 * 1733 * u)).div_euclid(587_000);
    let channel = |c: i32| c.clamp(0, 255) as u32;
    channel(red) << 16 | channel(green) << 8 | channel(blue)
}

fn dyuv(h: &Header, idat: &[u8]) -> Option<Image> {
    if h.line_len < h.width || !h.width.is_multiple_of(2) {
        return None;
    }
    let lines = idat.get(..h.line_len.checked_mul(h.height)?)?;
    let step = |value: u8, nibble: u8| value.wrapping_add(DELTAS[usize::from(nibble)]);
    let mut colors = Vec::with_capacity(h.width * h.height);
    for line in lines.chunks_exact(h.line_len) {
        let [mut y, mut u, mut v] = h.start;
        for pair in line[..h.width].as_chunks::<2>().0 {
            u = step(u, pair[0] >> 4);
            let y0 = step(y, pair[0] & 15);
            v = step(v, pair[1] >> 4);
            y = step(y0, pair[1] & 15);
            colors.extend([dyuv_color(y0, u, v), dyuv_color(y, u, v)]);
        }
    }
    Image::from_colors(h.width as u32, h.height as u32, colors.into_iter()).ok()
}

fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    const FAIL: DecodeError = DecodeError::Invalid;
    let chunks = chunks(data).ok_or(FAIL)?;
    let find = |tag: &[u8; 4]| chunks.iter().find(|(t, _)| t == tag).map(|(_, body)| *body);
    let h = header(find(b"IHDR").ok_or(FAIL)?).ok_or(FAIL)?;
    check_size(h.width, h.height)?;
    let idat = find(b"IDAT").ok_or(FAIL)?;
    if h.coding == Coding::Dyuv {
        return dyuv(&h, idat).ok_or(FAIL);
    }
    let palette = palette(find(b"PLTE").ok_or(FAIL)?).ok_or(FAIL)?;
    let indices = match h.coding {
        Coding::Clut4 => clut4(&h, idat),
        _ => rl7(&h, idat),
    };
    Image::from_indexed(
        h.width as u32,
        h.height as u32,
        &indices.ok_or(FAIL)?,
        &palette,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A file with an `IHDR` for the given coding fields, an optional `PLTE`
    /// of `colors` and the `IDAT` bytes.
    fn file(
        w: u16,
        line: u16,
        h: u16,
        coding: (u16, u16),
        colors: &[[u8; 3]],
        idat: &[u8],
    ) -> Vec<u8> {
        let mut body = Vec::new();
        let mut chunk = |tag: &[u8; 4], data: &[u8]| {
            body.extend_from_slice(tag);
            body.extend_from_slice(&(data.len() as u32).to_be_bytes());
            body.extend_from_slice(data);
            if !data.len().is_multiple_of(2) {
                body.push(0);
            }
        };
        let mut ihdr = Vec::new();
        for field in [w, line, h, coding.0, coding.1] {
            ihdr.extend_from_slice(&field.to_be_bytes());
        }
        ihdr.extend_from_slice(&[0, 16, 128, 128]);
        chunk(b"IHDR", &ihdr);
        if !colors.is_empty() {
            let mut plte = alloc::vec![0, 0, 0, colors.len() as u8];
            plte.extend(colors.iter().flatten());
            chunk(b"PLTE", &plte);
        }
        chunk(b"IDAT", idat);
        let mut out = b"FORM".to_vec();
        out.extend_from_slice(&(body.len() as u32 + 4).to_be_bytes());
        out.extend_from_slice(b"IMAG");
        out.extend(body);
        out
    }

    #[test]
    fn clut4_pixels_have_the_high_nibble_on_the_left_and_lines_may_be_padded() {
        let colors = [[0, 0, 0], [255, 0, 0], [0, 255, 0]];
        // 3 pixels, 2 bytes to a line.
        let image = decode(&file(3, 2, 2, (6, 4), &colors, &[0x12, 0x10, 0x21, 0x00])).unwrap();
        // Red, green, red; green, red, black.
        assert_eq!(
            image.rgb(),
            [
                255, 0, 0, 0, 255, 0, 255, 0, 0, 0, 255, 0, 255, 0, 0, 0, 0, 0
            ]
        );
    }

    #[test]
    fn rl7_runs_end_at_the_line_end_when_their_length_is_zero() {
        let colors = [[0, 0, 0], [255, 255, 255], [9, 9, 9]];
        // Line 1: one pixel of 1, then a run of 2 of index 2. Line 2: a run of 3.
        let idat = [1, 0x82, 2, 0x81, 0];
        let image = decode(&file(3, 3, 2, (8, 8), &colors, &idat)).unwrap();
        let line = |y| (0..3).map(|x| image.get(x, y)).collect::<Vec<_>>();
        assert_eq!(line(0), [0xffffff, 0x090909, 0x090909]);
        assert_eq!(line(1), [0xffffff; 3]);
        // A run past the end of the line is cut off, and a missing byte is an error.
        assert!(decode(&file(2, 2, 1, (8, 8), &colors, &[0x81, 9])).is_ok());
        assert!(decode(&file(3, 3, 2, (8, 8), &colors, &idat[..4])).is_err());
    }

    #[test]
    fn dyuv_accumulates_deltas_per_line_and_shares_chroma_in_a_pair() {
        // Start 16, 128, 128. First byte: U delta 0, Y delta 128 (nibble 8);
        // second byte: V delta 0, Y delta 0.
        let image = decode(&file(2, 2, 2, (3, 8), &[], &[0x08, 0x00, 0x00, 0x00])).unwrap();
        // Y goes 16 -> 144 with no chroma: gray 144, then 144 again.
        assert_eq!((image.get(0, 0), image.get(1, 0)), (0x909090, 0x909090));
        // The second line starts again from the start values.
        assert_eq!(image.get(0, 1), 0x101010);
        // U + 255 (nibble 15) is U - 1: a bit less blue than gray.
        let image = decode(&file(2, 2, 1, (3, 8), &[], &[0xf0, 0x00])).unwrap();
        assert_eq!(image.get(0, 0), dyuv_color(16, 127, 128));
        // Red and blue are lifted by the chroma and green is clamped to 0.
        assert_eq!(dyuv_color(16, 255, 255), 0xbe_00ec);
    }

    #[test]
    fn unknown_codings_and_odd_dyuv_widths_are_rejected() {
        assert!(decode(&file(2, 2, 1, (4, 8), &[], &[0, 0])).is_err());
        assert!(decode(&file(3, 3, 1, (3, 8), &[], &[0; 3])).is_err());
    }
}
