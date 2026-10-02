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

use alloc::vec::Vec;

use crate::{DecodeError, Format, Image};

pub(super) static FORMATS: &[Format] =
    &[Format::new("HP 48", "GROB", &["grb", "gro"], decode_grob)];

const PROLOG: usize = 0x02b1e;

fn decode_grob(data: &[u8]) -> Result<Image, DecodeError> {
    let nibbles = if data.starts_with(b"HPHP48-") && data.len() > 8 {
        data[8..].iter().flat_map(|&b| [b & 15, b >> 4]).collect()
    } else {
        text_nibbles(data).ok_or(DecodeError::Unrecognized)?
    };
    render(&nibbles).ok_or(DecodeError::Unrecognized)
}

/// The nibbles of a text GROB, with a binary-style header synthesised.
fn text_nibbles(data: &[u8]) -> Option<Vec<u8>> {
    let start = data.windows(5).position(|w| w == b"GROB ")? + 5;
    let mut words = data[start..]
        .split(|b| b.is_ascii_whitespace())
        .filter(|w| !w.is_empty());
    let mut number = || -> Option<usize> { core::str::from_utf8(words.next()?).ok()?.parse().ok() };
    let (width, height) = (number()?, number()?);
    let hex = words.next()?;
    let mut nibbles = Vec::with_capacity(20 + hex.len());
    for value in [PROLOG, 0, height, width] {
        nibbles.extend((0..5).map(|i| (value >> (4 * i) & 15) as u8));
    }
    for &c in hex {
        nibbles.push(char::from(c).to_digit(16)? as u8);
    }
    Some(nibbles)
}

fn render(nibbles: &[u8]) -> Option<Image> {
    let field = |at: usize| -> Option<usize> {
        let digits = nibbles.get(at..at + 5)?;
        Some(digits.iter().rev().fold(0, |v, &n| v << 4 | usize::from(n)))
    };
    if field(0)? != PROLOG {
        return None;
    }
    let (height, width) = (field(10)?, field(15)?);
    let row_nibbles = width.div_ceil(8) * 2;
    let pixels = nibbles.get(20..20 + row_nibbles.checked_mul(height)?)?;
    if width == 0 || height == 0 {
        return None;
    }
    let mut image = Image::new(width as u32, height as u32);
    for y in 0..height {
        for x in 0..width {
            let set = pixels[y * row_nibbles + x / 4] >> (x % 4) & 1 != 0;
            image.set(x as u32, y as u32, if set { 0 } else { 0xffffff });
        }
    }
    Some(image)
}
