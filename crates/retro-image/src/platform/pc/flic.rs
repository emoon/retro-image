//! Autodesk Animator FLI and Animator Pro FLC animations, first frame only.
//!
//! Sources:
//! - CompuPhase, "The FLIC file format": <https://www.compuphase.com/flic.htm>
//!   (128-byte header with the `AF11` (FLI) or `AF12` (FLC) magic, frame
//!   chunks `F1FA`, and the color, delta and run-length sub-chunks).
//! - Jim Kent, Dr. Dobb's Journal, March 1993, "The FLIC file format":
//!   <https://jacobfilipp.com/DrDobbs/articles/DDJ/1993/9303/9303a/9303a.htm>
//!   (BYTE_RUN, DELTA_FLI, DELTA_FLC, COLOR_64, COLOR_256, BLACK, COPY).
//! - Deark `fli.c` (<https://github.com/jsummers/deark>, MIT license) for
//!   how real files deviate (frame variant `F5FA`, an optional prefix chunk,
//!   clamped chunk sizes, out-of-range pixels ignored); also the oracle for
//!   the sample files.
//!
//! The picture is the screen after the first frame that draws pixels (a
//! leading frame that only sets the palette is applied first, as Deark does).
//! The screen starts with every pixel at color 0, so a delta frame against
//! the blank screen works. The hi-color FLH variant
//! (`AF44`) is drawn by `flh.rs` on the same frame walk; the Animator Pro `PIC`
//! and `COL` files are rejected.
//!
//! Verification: no RECOIL oracle for this format; output was compared pixel
//! for pixel with Deark's PNG output on the sample files.

// Parts of this file follow Deark's modules/fli.c
// (Deark, https://github.com/jsummers/deark):
//
// Copyright (C) 2020 Jason Summers
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

use alloc::vec;
use alloc::vec::Vec;

use super::dac_rounded;
use super::flh::HiScreen;
use crate::bytes::{le16, le32};
use crate::image::check_size;
use crate::{DecodeError, Image};

const FAIL: DecodeError = DecodeError::Unrecognized;
const HEADER_LEN: usize = 128;
/// Bytes of a chunk header (size and type) and of a frame header before its
/// sub-chunks (header, sub-chunk count, 8 reserved bytes).
const CHUNK_HEADER_LEN: usize = 6;
const FRAME_HEADER_LEN: usize = 16;

const FLI: u16 = 0xaf11;
const FLC: u16 = 0xaf12;
const FLH: u16 = 0xaf44;
const FRAME: u16 = 0xf1fa;
const FRAME_VARIANT: u16 = 0xf5fa;
const PREFIX: u16 = 0xf100;

const COLOR_256: u16 = 4;
const DELTA_FLC: u16 = 7;
const COLOR_64: u16 = 11;
const DELTA_FLI: u16 = 12;
const BLACK: u16 = 13;
const BYTE_RUN: u16 = 15;
const COPY: u16 = 16;

/// What a chunk does to the canvas, so dead work can be skipped.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Effect {
    /// Changes only the palette.
    Palette,
    /// Overwrites every pixel (BLACK, COPY).
    Fill,
    /// Overwrites some pixels.
    Draw,
    /// Ignored.
    Nothing,
}

/// What a FLIC decoder draws chunks on; the 8-bit [`Screen`] here and the
/// hi-color screen in `flh.rs` share the file walking below.
pub(super) trait Canvas {
    /// How chunks of `kind` touch the canvas.
    fn effect(kind: u16) -> Effect;
    /// Draws the chunk of `kind` whose body runs from `start` to `end`.
    fn apply(&mut self, kind: u16, start: usize, end: usize);
    /// Whether any chunk has drawn pixels.
    fn drawn(&self) -> bool;
}

/// The 8-bit screen and its palette.
struct Screen<'a> {
    data: &'a [u8],
    width: usize,
    height: usize,
    pixels: Vec<u8>,
    palette: [u32; 256],
    /// Whether any chunk has drawn pixels.
    drawn: bool,
}

impl Canvas for Screen<'_> {
    fn effect(kind: u16) -> Effect {
        match kind {
            COLOR_256 | COLOR_64 => Effect::Palette,
            BLACK | COPY => Effect::Fill,
            BYTE_RUN | DELTA_FLI | DELTA_FLC => Effect::Draw,
            _ => Effect::Nothing,
        }
    }

    fn apply(&mut self, kind: u16, start: usize, end: usize) {
        self.draw(kind, start, end);
    }

    fn drawn(&self) -> bool {
        self.drawn
    }
}

impl Screen<'_> {
    /// The byte at `at` in the file, 0 past its end.
    fn byte(&self, at: usize) -> u8 {
        self.data.get(at).copied().unwrap_or(0)
    }

    fn word(&self, at: usize) -> usize {
        usize::from(self.byte(at)) | usize::from(self.byte(at + 1)) << 8
    }

    fn put(&mut self, x: usize, y: usize, index: u8) {
        if x < self.width && y < self.height {
            self.pixels[y * self.width + x] = index;
        }
    }

    /// Chunks of `kind` are drawn from `start` (just after the chunk header)
    /// to `end`.
    fn draw(&mut self, kind: u16, start: usize, end: usize) {
        match kind {
            COLOR_256 | COLOR_64 => self.colors(start, end, kind == COLOR_64),
            BLACK => {
                self.drawn = true;
                self.pixels.fill(0);
            }
            COPY => {
                self.drawn = true;
                // Bytes past the end of the file read as 0.
                let src = self.data.get(start..).unwrap_or(&[]);
                let copied = src.len().min(self.pixels.len());
                self.pixels[..copied].copy_from_slice(&src[..copied]);
                self.pixels[copied..].fill(0);
            }
            BYTE_RUN => self.byte_run(start, end),
            DELTA_FLI => self.delta_fli(start, end),
            DELTA_FLC => self.delta_flc(start, end),
            _ => {}
        }
    }

    fn colors(&mut self, start: usize, end: usize, six_bit: bool) {
        let packets = self.word(start);
        let mut pos = start + 2;
        let mut next = 0usize;
        for _ in 0..packets {
            if pos >= end {
                return;
            }
            next += usize::from(self.byte(pos));
            let count = match self.byte(pos + 1) {
                0 => 256,
                n => usize::from(n),
            };
            pos += 2;
            for _ in 0..count {
                let [r, g, b] = [0, 1, 2].map(|i| {
                    let v = self.byte(pos + i);
                    if six_bit {
                        dac_rounded(v)
                    } else {
                        u32::from(v)
                    }
                });
                if let Some(slot) = self.palette.get_mut(next) {
                    *slot = r << 16 | g << 8 | b;
                }
                next += 1;
                pos += 3;
            }
        }
    }

    /// Lines of packets: a count byte, then `code >= 128` copies `256 - code`
    /// literal pixels and any other code repeats the next byte `code` times.
    fn byte_run(&mut self, start: usize, end: usize) {
        self.drawn = true;
        let mut pos = start + 1;
        let (mut x, mut y) = (0, 0);
        while pos < end {
            let code = self.byte(pos);
            pos += 1;
            if code >= 128 {
                for _ in 0..256 - usize::from(code) {
                    let index = self.byte(pos);
                    pos += 1;
                    self.put(x, y, index);
                    x += 1;
                }
            } else {
                let index = self.byte(pos);
                pos += 1;
                for _ in 0..code {
                    self.put(x, y, index);
                    x += 1;
                }
            }
            if x >= self.width {
                x = 0;
                y += 1;
                pos += 1;
                if y >= self.height {
                    break;
                }
            }
        }
    }

    /// First line and line count, then per line a packet count and packets of
    /// skip, code: `code < 128` literal pixels, otherwise a run of
    /// `256 - code` pixels of the next byte.
    fn delta_fli(&mut self, start: usize, end: usize) {
        self.drawn = true;
        let lines = self.word(start + 2);
        let mut pos = start + 4;
        for y in (self.word(start)..).take(lines) {
            if pos >= end {
                return;
            }
            let packets = self.byte(pos);
            pos += 1;
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
                        let index = self.byte(pos);
                        pos += 1;
                        self.put(x, y, index);
                        x += 1;
                    }
                } else {
                    let index = self.byte(pos);
                    pos += 1;
                    for _ in 0..256 - usize::from(code) {
                        self.put(x, y, index);
                        x += 1;
                    }
                }
            }
        }
    }

    /// Line count, then per line some words that skip lines (`0xC000`
    /// range) or set the last pixel (`0x8000` range) before the packet count,
    /// and packets of skip, code: `code < 128` literal pixel pairs, otherwise
    /// `256 - code` repeats of a pair.
    fn delta_flc(&mut self, start: usize, end: usize) {
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
                    2 => {
                        let last = self.width.saturating_sub(1);
                        self.put(last, y, word as u8);
                    }
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
                    for _ in 0..usize::from(code) * 2 {
                        let index = self.byte(pos);
                        pos += 1;
                        self.put(x, y, index);
                        x += 1;
                    }
                } else {
                    let pair = [self.byte(pos), self.byte(pos + 1)];
                    pos += 2;
                    for _ in 0..256 - usize::from(code) {
                        self.put(x, y, pair[0]);
                        self.put(x + 1, y, pair[1]);
                        x += 2;
                    }
                }
            }
            y += 1;
        }
    }
}

/// Size and type of the chunk at `pos`, its size clamped to `end`.
/// `None` if the header doesn't fit or the size is below the header.
pub(super) fn chunk(data: &[u8], pos: usize, end: usize) -> Option<(usize, u16)> {
    if end.saturating_sub(pos) < CHUNK_HEADER_LEN {
        return None;
    }
    let size = usize::try_from(le32(data, pos)?).ok()?;
    (size >= CHUNK_HEADER_LEN).then(|| (size.min(end - pos), le16(data, pos + 4).unwrap_or(0)))
}

pub(super) fn decode_flic(data: &[u8]) -> Result<Image, DecodeError> {
    let magic = le16(data, 4).ok_or(FAIL)?;
    if ![FLI, FLC, FLH].contains(&magic) || data.len() < HEADER_LEN + CHUNK_HEADER_LEN {
        return Err(FAIL);
    }
    let first = le16(data, HEADER_LEN + 4).ok_or(FAIL)?;
    if ![FRAME, FRAME_VARIANT, PREFIX].contains(&first) {
        return Err(FAIL);
    }
    let width = usize::from(le16(data, 8).ok_or(FAIL)?);
    let height = usize::from(le16(data, 10).ok_or(FAIL)?);
    let depth = le16(data, 12).ok_or(FAIL)?;
    check_size(width, height)?;
    let declared = usize::try_from(le32(data, 0).ok_or(FAIL)?).map_err(|_| FAIL)?;
    let end = if declared == 0 {
        data.len()
    } else {
        declared.min(data.len())
    };

    if magic == FLH {
        let mut screen = HiScreen::new(data, width, height, depth)?;
        return if first_picture(data, end, &mut screen) {
            Ok(screen.into_image()?)
        } else {
            Err(FAIL)
        };
    }
    if depth != 8 && depth != 0 {
        return Err(FAIL);
    }
    let mut screen = Screen {
        data,
        width,
        height,
        pixels: vec![0; width * height],
        palette: [0; 256],
        drawn: false,
    };
    if first_picture(data, end, &mut screen) {
        Image::from_indexed(width as u32, height as u32, &screen.pixels, &screen.palette)
    } else {
        Err(FAIL)
    }
}

/// Draws frames onto `canvas` until one has drawn pixels.
fn first_picture(data: &[u8], end: usize, canvas: &mut impl Canvas) -> bool {
    let mut pos = HEADER_LEN;
    while let Some((size, kind)) = chunk(data, pos, end) {
        if kind == FRAME || kind == FRAME_VARIANT {
            apply_frame(data, canvas, pos, pos + size);
            if canvas.drawn() {
                return true;
            }
        }
        pos += size;
    }
    false
}

/// Applies the sub-chunks of the frame chunk at `start`..`end`.
///
/// Pixel chunks before the last full-frame fill are overwritten anyway, so
/// only their palette siblings run. That keeps the work linear in the input
/// instead of one frame's worth of pixels per 6-byte BLACK chunk.
fn apply_frame<C: Canvas>(data: &[u8], canvas: &mut C, start: usize, end: usize) {
    let count = usize::from(le16(data, start + CHUNK_HEADER_LEN).unwrap_or(0));
    let first = start + FRAME_HEADER_LEN;
    let chunks = || {
        let mut pos = first;
        core::iter::from_fn(move || {
            let (size, kind) = chunk(data, pos, end)?;
            pos += size;
            Some((pos - size, size, kind))
        })
        .take(count)
    };
    let last_fill = chunks()
        .filter(|&(_, _, kind)| C::effect(kind) == Effect::Fill)
        .last()
        .map_or(first, |(at, _, _)| at);
    for (pos, size, kind) in chunks() {
        if pos < last_fill && matches!(C::effect(kind), Effect::Fill | Effect::Draw) {
            continue;
        }
        canvas.apply(kind, pos + CHUNK_HEADER_LEN, pos + size);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk_bytes(kind: u16, body: &[u8]) -> Vec<u8> {
        let mut out = ((body.len() + CHUNK_HEADER_LEN) as u32)
            .to_le_bytes()
            .to_vec();
        out.extend_from_slice(&kind.to_le_bytes());
        out.extend_from_slice(body);
        out
    }

    /// A 4x2 FLC whose first frame sets two colors, then draws `drawing`.
    fn flc(drawing: &[u8]) -> Vec<u8> {
        let mut sub = chunk_bytes(COLOR_64, &[1, 0, 0, 2, 0, 0, 0, 63, 63, 63]);
        sub.extend(drawing);
        let mut frame = alloc::vec![2, 0];
        frame.extend_from_slice(&[0; 8]);
        frame.extend(sub);
        let frame = chunk_bytes(FRAME, &frame);
        let mut file = alloc::vec![0; HEADER_LEN];
        file[4..6].copy_from_slice(&FLC.to_le_bytes());
        file[8..10].copy_from_slice(&4u16.to_le_bytes());
        file[10..12].copy_from_slice(&2u16.to_le_bytes());
        file[12..14].copy_from_slice(&8u16.to_le_bytes());
        file.extend(frame);
        let len = file.len() as u32;
        file[..4].copy_from_slice(&len.to_le_bytes());
        file
    }

    #[test]
    fn byte_run_frame_paints_through_the_palette() {
        // Line 1: run of 4 x color 1 (code 4); line 2: 2 literals then a run.
        let draw = chunk_bytes(BYTE_RUN, &[1, 4, 1, 1, 0xfe, 0, 1, 2, 1]);
        let image = decode_flic(&flc(&draw)).unwrap();
        assert_eq!((image.width(), image.height()), (4, 2));
        assert_eq!(image.get(0, 0), 0xffffff);
        assert_eq!(image.get(0, 1), 0);
        assert_eq!(image.get(3, 1), 0xffffff);
    }

    #[test]
    fn delta_fli_skips_unchanged_pixels() {
        // One line at y = 1: skip 1 pixel, 2 literal pixels of color 1.
        let draw = chunk_bytes(DELTA_FLI, &[1, 0, 1, 0, 1, 1, 2, 1, 1]);
        let image = decode_flic(&flc(&draw)).unwrap();
        assert_eq!(image.get(0, 1), 0);
        assert_eq!(image.get(1, 1), 0xffffff);
        assert_eq!(image.get(2, 1), 0xffffff);
        assert_eq!(image.get(3, 1), 0);
    }

    #[test]
    fn many_full_frame_chunks_decode_quickly() {
        extern crate std;
        // 3000 BLACK chunks on an 8192x8192 screen used to cost seconds.
        let blacks = chunk_bytes(BLACK, &[]).repeat(3000);
        let mut file = flc(&blacks);
        file[8..10].copy_from_slice(&8192u16.to_le_bytes());
        file[10..12].copy_from_slice(&8192u16.to_le_bytes());
        let count = 3001u16.to_le_bytes();
        file[HEADER_LEN + CHUNK_HEADER_LEN..][..2].copy_from_slice(&count);
        let started = std::time::Instant::now();
        let image = decode_flic(&file).unwrap();
        assert_eq!(image.width(), 8192);
        assert!(started.elapsed().as_millis() < 1000);
    }

    #[test]
    fn a_frame_without_pixels_is_rejected() {
        assert!(decode_flic(&flc(&[])).is_err());
    }

    #[test]
    fn truncated_files_do_not_panic() {
        let file = flc(&chunk_bytes(BYTE_RUN, &[1, 4, 1, 1, 0xfe, 0, 1, 2, 1]));
        for len in 0..file.len() {
            let _ = decode_flic(&file[..len]);
        }
    }
}
