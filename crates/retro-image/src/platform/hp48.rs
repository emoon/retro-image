//! HP 48.
//!
//! Sources:
//! - GROB: hpcalc.org "GROB format" (<https://www.hpcalc.org/hp48/docs/programming/grobs.txt>)
//!   and the HP 48 FAQ ch. 8 (<https://www.hpcalc.org/hp48/docs/faq/48faq-8.html>):
//!   binary objects start with "HPHP48-" and a version letter, then 5-nibble
//!   fields (least significant nibble first) for the prolog `$02B1E`, length,
//!   height and width; rows are padded to whole bytes and the least
//!   significant bit of each nibble is the leftmost pixel. Text objects read
//!   `GROB width height hexdigits`, one hex digit per memory nibble.
//! - Set bit = black: observed from `recoil2png` output.
//! - Directory (`$02A96`) and program (`$02D9D`) objects holding GROBs: the
//!   prologs are in the HP 48 FAQ ch. 8; the GROBs inside are found by their
//!   prolog and a length field that matches their size (reverse engineered
//!   from hpcalc.org samples). They are drawn side by side, top-aligned, on
//!   white, like the objects of an AMOS bank.

use alloc::vec::Vec;

use crate::image::check_size;
use crate::{DecodeError, Format, Image};

pub(super) static FORMATS: &[Format] =
    &[Format::new("HP 48", "GROB", &["grb", "gro"], decode_grob).signature()];

const GROB: usize = 0x02b1e;
const DIRECTORY: usize = 0x02a96;
const PROGRAM: usize = 0x02d9d;
const WHITE: u32 = 0xffffff;

fn decode_grob(data: &[u8]) -> Result<Image, DecodeError> {
    let nibbles = if data.starts_with(b"HPHP48-") && data.len() > 8 {
        data[8..].iter().flat_map(|&b| [b & 15, b >> 4]).collect()
    } else {
        text_nibbles(data).ok_or(DecodeError::Unrecognized)?
    };
    let grobs = match field(&nibbles, 0) {
        Some(GROB) => Vec::from_iter(Grob::parse(&nibbles)),
        Some(DIRECTORY | PROGRAM) => embedded_grobs(&nibbles),
        _ => Vec::new(),
    };
    render(&grobs).ok_or(DecodeError::Unrecognized)
}

/// The nibbles of a text GROB, with a binary-style header synthesised.
///
/// The text starts with `GROB`, optionally after a `%%HP:` transfer header
/// line (HP 48 FAQ ch. 8).
fn text_nibbles(data: &[u8]) -> Option<Vec<u8>> {
    let mut text = data.trim_ascii_start();
    if text.starts_with(b"%%HP:") {
        let line_end = text.iter().position(|&b| b == b'\r' || b == b'\n')?;
        text = text[line_end..].trim_ascii_start();
    }
    let mut words = text
        .split(|b| b.is_ascii_whitespace())
        .filter(|w| !w.is_empty());
    if words.next()? != b"GROB" {
        return None;
    }
    let mut number = || -> Option<usize> { core::str::from_utf8(words.next()?).ok()?.parse().ok() };
    let (width, height) = (number()?, number()?);
    if width > 0xfffff || height > 0xfffff {
        return None;
    }
    let hex = words.next()?;
    let mut nibbles = Vec::with_capacity(20 + hex.len());
    for value in [GROB, 0, height, width] {
        nibbles.extend((0..5).map(|i| (value >> (4 * i) & 15) as u8));
    }
    for &c in hex {
        nibbles.push(char::from(c).to_digit(16)? as u8);
    }
    Some(nibbles)
}

/// A 5-nibble field, least significant nibble first.
fn field(nibbles: &[u8], at: usize) -> Option<usize> {
    let digits = nibbles.get(at..at.checked_add(5)?)?;
    Some(digits.iter().rev().fold(0, |v, &n| v << 4 | usize::from(n)))
}

struct Grob<'a> {
    width: usize,
    height: usize,
    /// The length field: the object's size in nibbles, minus the prolog.
    length: usize,
    pixels: &'a [u8],
}

impl<'a> Grob<'a> {
    /// The GROB at the start of `nibbles`.
    fn parse(nibbles: &'a [u8]) -> Option<Self> {
        if field(nibbles, 0)? != GROB {
            return None;
        }
        let (length, height, width) =
            (field(nibbles, 5)?, field(nibbles, 10)?, field(nibbles, 15)?);
        if width == 0 || height == 0 {
            return None;
        }
        check_size(width, height).ok()?;
        let pixel_len = width.div_ceil(8).checked_mul(2)?.checked_mul(height)?;
        let pixels = nibbles.get(20..pixel_len.checked_add(20)?)?;
        Some(Self {
            width,
            height,
            length,
            pixels,
        })
    }

    fn row_nibbles(&self) -> usize {
        self.width.div_ceil(8) * 2
    }

    /// Whether the length field matches the header and pixels.
    fn length_matches(&self) -> bool {
        self.length == 15 + self.pixels.len()
    }

    fn is_set(&self, x: usize, y: usize) -> bool {
        self.pixels[y * self.row_nibbles() + x / 4] >> (x % 4) & 1 != 0
    }
}

/// The GROBs stored in a directory or program, in order.
fn embedded_grobs(nibbles: &[u8]) -> Vec<Grob<'_>> {
    let mut grobs = Vec::new();
    let mut at = 5;
    while at + 20 <= nibbles.len() {
        match Grob::parse(&nibbles[at..]).filter(Grob::length_matches) {
            Some(grob) => {
                at += 5 + grob.length;
                grobs.push(grob);
            }
            None => at += 1,
        }
    }
    grobs
}

/// Draws `grobs` side by side, top-aligned on white; `None` if empty.
fn render(grobs: &[Grob]) -> Option<Image> {
    let width: usize = grobs.iter().map(|g| g.width).sum();
    let height = grobs.iter().map(|g| g.height).max()?;
    if width > 0xffff {
        return None;
    }
    // Each GROB passed the cap alone, but side by side they can exceed it.
    check_size(width, height).ok()?;
    let mut indices = alloc::vec![0u8; width * height];
    let mut left = 0;
    for grob in grobs {
        for y in 0..grob.height {
            for x in 0..grob.width {
                indices[y * width + left + x] = u8::from(grob.is_set(x, y));
            }
        }
        left += grob.width;
    }
    Image::from_indexed(width as u32, height as u32, &indices, &[WHITE, 0]).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A binary GROB object's nibbles; `length` overrides the length field.
    fn grob(width: usize, height: usize, length: Option<usize>, fill: u8) -> Vec<u8> {
        let pixel_len = width.div_ceil(8) * 2 * height;
        let length = length.unwrap_or(15 + pixel_len);
        let mut nibbles = Vec::new();
        for value in [GROB, length, height, width] {
            nibbles.extend((0..5).map(|i| (value >> (4 * i) & 15) as u8));
        }
        nibbles.resize(nibbles.len() + pixel_len, fill);
        nibbles
    }

    #[test]
    fn grobs_over_the_pixel_cap_are_rejected() {
        let mut big = alloc::vec![0u8; 20 + 65535usize.div_ceil(8) * 2 * 1025];
        for (i, value) in [GROB, 0, 1025, 65535].into_iter().enumerate() {
            for n in 0..5 {
                big[i * 5 + n] = (value >> (4 * n) & 15) as u8;
            }
        }
        assert!(Grob::parse(&big).is_none());
    }

    #[test]
    fn embedded_grobs_side_by_side_over_the_pixel_cap_are_rejected() {
        // Each is small, but the canvas is 65008 x 100000.
        let mut program = alloc::vec![0xd, 0x9, 0xd, 0x2, 0x0];
        program.extend(grob(8, 100_000, None, 0));
        program.extend(grob(65_000, 16, None, 0));
        let grobs = embedded_grobs(&program);
        assert_eq!(grobs.len(), 2);
        assert!(render(&grobs).is_none());
    }

    #[test]
    fn text_grobs_start_with_grob_or_a_transfer_header() {
        let size = |data: &[u8]| decode_grob(data).map(|i| (i.width(), i.height()));
        assert_eq!(size(b"GROB 8 1 F0"), Ok((8, 1)));
        assert_eq!(size(b"%%HP: T(3)A(D)F(.);\r\nGROB 8 1 F0"), Ok((8, 1)));
        assert!(size(b"see GROB 8 1 F0").is_err());
    }

    #[test]
    fn finds_grobs_in_a_program_by_their_length_field() {
        let mut program = alloc::vec![0xd, 0x9, 0xd, 0x2, 0x0, 0x7];
        program.extend(grob(8, 2, None, 0xf));
        program.extend([1, 2, 3]);
        program.extend(grob(4, 3, Some(99), 0));
        program.extend(grob(4, 3, None, 0));
        let grobs = embedded_grobs(&program);
        let sizes: Vec<_> = grobs.iter().map(|g| (g.width, g.height)).collect();
        assert_eq!(sizes, [(8, 2), (4, 3)]);
        let image = render(&grobs).unwrap();
        assert_eq!((image.width(), image.height()), (12, 3));
        assert_eq!(&image.rgb()[..3], [0, 0, 0]);
        // Below the shorter first GROB: white background.
        assert_eq!(&image.rgb()[2 * 12 * 3..2 * 12 * 3 + 3], [255, 255, 255]);
    }
}
