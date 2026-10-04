//! Hi-colour FLH animations (FLIC magic `0xAF44`), first picture only.
//!
//! Sources:
//! - CompuPhase, "The FLIC file format", section on FLH:
//!   <https://www.compuphase.com/flic.htm>. Header depth 16 means 5-6-5 words
//!   (`rrrrrggg gggbbbbb`), depth 15 means 5-5-5 (`0rrrrrgg gggbbbbb`), both
//!   little-endian. The chunks are DTA_BRUN (25, full-frame run-length), DTA_COPY
//!   (26, raw frame) and DTA_LC (27, delta), which work like their 8-bit
//!   counterparts but count pixels (words) instead of bytes.
//! - The 8-bit chunk layouts and the frame walking are in `flic.rs`.
//!
//! Checked on the one FLH in the corpus (`corpus/extra/next-amiga-pc/sembiance/flc/flyby.flh`,
//! 16-bit, first frame a DTA_BRUN chunk) by eye. DTA_LC follows the 8-bit delta
//! layout and is covered by a unit test only, because no sample's first picture
//! uses it. 24-bit files from the DTA program are not decoded.

use alloc::vec;
use alloc::vec::Vec;

use super::flic::Canvas;
use crate::{DecodeError, Image};

const DTA_BRUN: u16 = 25;
const DTA_COPY: u16 = 26;
const DTA_LC: u16 = 27;

/// A hi-colour screen of `0xRRGGBB` pixels.
pub(super) struct HiScreen<'a> {
    data: &'a [u8],
    width: usize,
    height: usize,
    /// Whether words are 5-6-5 (depth 16) rather than 5-5-5 (depth 15).
    green_six: bool,
    pixels: Vec<u32>,
    drawn: bool,
}

impl<'a> HiScreen<'a> {
    pub(super) fn new(
        data: &'a [u8],
        width: usize,
        height: usize,
        depth: u16,
    ) -> Result<Self, DecodeError> {
        if depth != 15 && depth != 16 {
            return Err(DecodeError::Unrecognized);
        }
        Ok(Self {
            data,
            width,
            height,
            green_six: depth == 16,
            pixels: vec![0; width * height],
            drawn: false,
        })
    }

    pub(super) fn into_image(self) -> Image {
        Image::from_colors(
            self.width as u32,
            self.height as u32,
            self.pixels.into_iter(),
        )
    }

    fn byte(&self, at: usize) -> u8 {
        self.data.get(at).copied().unwrap_or(0)
    }

    fn word(&self, at: usize) -> usize {
        usize::from(self.byte(at)) | usize::from(self.byte(at + 1)) << 8
    }

    /// The colour of the pixel word at `at`, bits widened to 8.
    fn color(&self, at: usize) -> u32 {
        let word = self.word(at) as u32;
        let widen = |value: u32, bits: u32| (value << (8 - bits)) | (value >> (2 * bits - 8));
        let blue = widen(word & 0x1f, 5);
        let (green, red) = if self.green_six {
            (widen(word >> 5 & 0x3f, 6), widen(word >> 11 & 0x1f, 5))
        } else {
            (widen(word >> 5 & 0x1f, 5), widen(word >> 10 & 0x1f, 5))
        };
        red << 16 | green << 8 | blue
    }

    fn put(&mut self, x: usize, y: usize, color: u32) {
        if x < self.width && y < self.height {
            self.pixels[y * self.width + x] = color;
        }
    }

    /// Lines of a packet count byte (ignored) and packets: a signed count,
    /// negative copies that many pixel words, positive repeats the next one.
    fn brun(&mut self, start: usize, end: usize) {
        self.drawn = true;
        let mut pos = start;
        for y in 0..self.height {
            if pos >= end {
                return;
            }
            pos += 1;
            let mut x = 0;
            while x < self.width && pos < end {
                let code = self.byte(pos);
                pos += 1;
                if code >= 128 {
                    for _ in 0..256 - usize::from(code) {
                        let color = self.color(pos);
                        pos += 2;
                        self.put(x, y, color);
                        x += 1;
                    }
                } else {
                    let color = self.color(pos);
                    pos += 2;
                    for _ in 0..code {
                        self.put(x, y, color);
                        x += 1;
                    }
                }
            }
        }
    }

    /// Line count, then per line the same line words as the 8-bit FLC delta
    /// (`0xC000` range skips lines, other flag words are ignored) and packets
    /// of skip, count: positive copies that many pixel words, negative
    /// repeats one word.
    fn lc(&mut self, start: usize, end: usize) {
        self.drawn = true;
        let lines = self.word(start);
        let mut pos = start + 2;
        let mut y = 0usize;
        for _ in 0..lines {
            let packets = loop {
                if pos >= end {
                    return;
                }
                let word = self.word(pos);
                pos += 2;
                match word >> 14 {
                    0 => break word,
                    3 => y += 65536 - word,
                    _ => {}
                }
            };
            let mut x = 0;
            for _ in 0..packets {
                if pos >= end {
                    return;
                }
                x += usize::from(self.byte(pos));
                let code = self.byte(pos + 1);
                pos += 2;
                if code < 128 {
                    for _ in 0..code {
                        let color = self.color(pos);
                        pos += 2;
                        self.put(x, y, color);
                        x += 1;
                    }
                } else {
                    let color = self.color(pos);
                    pos += 2;
                    for _ in 0..256 - usize::from(code) {
                        self.put(x, y, color);
                        x += 1;
                    }
                }
            }
            y += 1;
        }
    }
}

impl Canvas for HiScreen<'_> {
    fn apply(&mut self, kind: u16, start: usize, end: usize) {
        match kind {
            DTA_BRUN => self.brun(start, end),
            DTA_COPY => {
                self.drawn = true;
                for i in 0..self.pixels.len() {
                    self.pixels[i] = self.color(start + 2 * i);
                }
            }
            DTA_LC => self.lc(start, end),
            _ => {}
        }
    }

    fn drawn(&self) -> bool {
        self.drawn
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lc_delta_paints_a_copy_and_a_repeat() {
        let data = [
            1, 0, // one line
            2, 0, // two packets
            1, 2, 0x1f, 0, 0x00, 0xf8, // skip 1, copy blue, red (5-6-5)
            0, 0xfe, 0xe0, 0x07, // repeat green twice
        ];
        let mut screen = HiScreen::new(&data, 5, 1, 16).unwrap();
        screen.lc(0, data.len());
        assert_eq!(screen.pixels, [0, 0x0000ff, 0xff0000, 0x00ff00, 0x00ff00]);
    }
}
