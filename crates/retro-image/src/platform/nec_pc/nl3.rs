//! Mapletown Network `NL3` pictures (NEC PC-98, 160x100, 64 colours), a text
//! format.
//!
//! Layout reverse engineered from `YOUKO.NL3` (Shift-JIS) and
//! `SAKI2_A_UTF-8.nl3` by black-box probing of `recoil2png`; see
//! `docs/research/gaps-corpus-other.md`, section 7. The only outside
//! reference is the Just Solve page for Mapletown Network
//! (<http://fileformats.archiveteam.org/wiki/Mapletown_Network>), which says
//! the file starts `20 20 78 25`.
//!
//! - Line breaks (CR, LF) are dropped. Every other character is a symbol:
//!   U+0020..U+007E are 0..94 and halfwidth katakana U+FF61..U+FF9F are
//!   96..158 (one byte 0xA1..0xDF in Shift-JIS, three bytes in UTF-8).
//!   Anything else is rejected.
//! - The first 128 symbols are 64 palette entries of two symbols: `v = s0 +
//!   128 * s1` (below 729), a base-9 colour with red `v / 81`, green
//!   `v / 9 % 9` and blue `v % 9`, each level `L` shown as `L * 255 / 8`.
//! - Then pixels, column by column (x outer, y inner): a symbol below 64 is
//!   one pixel of that palette entry; 64..127 starts a run of entry `s - 64`
//!   whose length is the next symbol plus 2. A final run may overshoot. The
//!   text after the 16000th pixel is ignored.

use alloc::vec::Vec;

use crate::{DecodeError, Image};

const WIDTH: usize = 160;
const HEIGHT: usize = 100;
const COLORS: usize = 64;

/// The symbols of the text, with line breaks removed.
struct Symbols<'a> {
    data: &'a [u8],
}

impl Symbols<'_> {
    fn next(&mut self) -> Result<u8, DecodeError> {
        loop {
            let (&first, rest) = self.data.split_first().ok_or(DecodeError::Unrecognized)?;
            self.data = rest;
            let symbol = match first {
                b'\r' | b'\n' => continue,
                0x20..=0x7e => first - 0x20,
                0xa1..=0xdf => 96 + (first - 0xa1),
                0xef => {
                    let (tail, rest) = self
                        .data
                        .split_at_checked(2)
                        .ok_or(DecodeError::Unrecognized)?;
                    self.data = rest;
                    let code = 0xf000 | u32::from(tail[0] & 0x3f) << 6 | u32::from(tail[1] & 0x3f);
                    match code.checked_sub(0xff61) {
                        Some(n) if n < 63 && tail[0] & 0xc0 == 0x80 && tail[1] & 0xc0 == 0x80 => {
                            96 + n as u8
                        }
                        _ => return Err(DecodeError::Unrecognized),
                    }
                }
                _ => return Err(DecodeError::Unrecognized),
            };
            // U+007F (symbol 95) is not valid; it is outside the ranges above.
            return Ok(symbol);
        }
    }
}

fn level(value: u32) -> u32 {
    value * 255 / 8
}

pub(in crate::platform) fn decode_nl3(data: &[u8]) -> Result<Image, DecodeError> {
    let mut symbols = Symbols { data };
    let mut palette = [0u32; COLORS];
    for entry in &mut palette {
        let low = u32::from(symbols.next()?);
        let value = low + 128 * u32::from(symbols.next()?);
        if value >= 729 {
            return Err(DecodeError::Unrecognized);
        }
        *entry = level(value / 81) << 16 | level(value / 9 % 9) << 8 | level(value % 9);
    }
    // Column-major indices, transposed below.
    let mut columns = Vec::with_capacity(WIDTH * HEIGHT);
    while columns.len() < WIDTH * HEIGHT {
        let symbol = symbols.next()?;
        if symbol < 64 {
            columns.push(symbol);
        } else if symbol < 128 {
            let length = usize::from(symbols.next()?) + 2;
            let left = WIDTH * HEIGHT - columns.len();
            columns.resize(columns.len() + length.min(left), symbol - 64);
        } else {
            return Err(DecodeError::Unrecognized);
        }
    }
    let mut indices = alloc::vec![0u8; WIDTH * HEIGHT];
    for (i, &index) in columns.iter().enumerate() {
        indices[i % HEIGHT * WIDTH + i / HEIGHT] = index;
    }
    Image::from_indexed(WIDTH as u32, HEIGHT as u32, &indices, &palette)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(entries: &[(u8, u8)], body: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        for i in 0..COLORS {
            let (a, b) = entries.get(i).copied().unwrap_or((0, 0));
            out.extend([a + 0x20, b + 0x20]);
        }
        out.extend_from_slice(body);
        out
    }

    /// Appends `count` pixels of palette entry `color` as runs.
    fn run(body: &mut Vec<u8>, color: u8, mut count: usize) {
        while count > 0 {
            let n = count.min(90);
            body.extend([0x20 + 64 + color, 0x20 + (n - 2) as u8]);
            count -= n;
            assert!(count != 1);
        }
    }

    #[test]
    fn runs_fill_columns_first() {
        // Colour 1 is v = 728 (white): low symbol 728 % 128 = 88, high 5.
        let mut body = Vec::new();
        run(&mut body, 1, 100); // the whole first column
        run(&mut body, 0, 15900);
        let image = decode_nl3(&text(&[(0, 0), (88, 5)], &body)).unwrap();
        assert_eq!(image.get(0, 99), 0xffffff);
        assert_eq!(image.get(1, 0), 0);
    }

    #[test]
    fn short_stream_is_rejected() {
        assert!(decode_nl3(&text(&[], b"!!")).is_err());
    }

    #[test]
    fn del_is_rejected() {
        assert!(decode_nl3(&[0x7f; 400]).is_err());
    }
}
