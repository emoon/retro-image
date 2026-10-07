//! Utah RLE (`.rle`), the format of the Utah Raster Toolkit.
//!
//! Sources:
//! - Spencer W. Thomas, "Design of the Utah RLE Format"
//!   (<https://sarnold.github.io/urt/docs/rle.pdf>, also at
//!   <https://paulbourke.net/dataformats/urt/>): little-endian header (magic,
//!   xpos, ypos, xsize, ysize, flags, ncolors, pixel bits, ncmap, cmaplen),
//!   the background color padded to an odd length, the color map (16-bit
//!   entries, left justified, one channel after the other), an optional
//!   comment block, then scanline operations from the bottom row up. Every
//!   operation is an even number of bytes; opcodes with a 0x40 bit have a
//!   16-bit operand after a filler byte. SkipLines ends the scanline, SetColor
//!   picks the channel (255 is alpha) and restarts the line, SkipPixels leaves
//!   background, PixelData and Run write pixels, EOF ends the file.
//! - The paper does not print the magic number, the opcode numbers or the flag
//!   bits, nor does it say that counts are one less than the pixel count.
//!   These come from the ten sample files in `corpus/extra/unix-rasters`
//!   (reverse engineered): magic `52 CC`; opcodes 1 SkipLines, 2 SetColor,
//!   3 SkipPixels, 5 PixelData, 6 Run, 7 EOF; flags 1 ClearFirst, 2
//!   NoBackground, 4 Alpha, 8 Comments; PixelData and Run have operand + 1
//!   pixels. Every sample parses to its EOF opcode with these values and each
//!   channel fills exactly the picture width. The Utah Raster Toolkit source
//!   was not read.
//!
//! Decoded: 8-bit pictures of one channel (gray, or an index through a one- or
//! three-channel color map) or three channels (RGB, through a color map of one
//! or three channels if present). The alpha channel is kept as the image's
//! alpha. Pixels no operation writes are background colored, with alpha 0 when
//! the file has alpha. The image is `xsize` by `ysize`; the offsets are not
//! used. Comments are ignored.
//!
//! Verification: no tool decodes the samples (no RECOIL, Pillow, Deark or
//! ffmpeg support), so the evidence is the structural check above and a
//! review by eye of all ten pictures against an independent Python decoder
//! (see the divergence file `unix-rasters.tsv`).

use alloc::vec::Vec;

use crate::bytes::le16;
use crate::image::check_size;
use crate::{DecodeError, Image};

/// How many times the picture size plus the operation bytes the pixel writes
/// may add up to before the file is refused.
const WORK_FACTOR: usize = 8;
const FAIL: DecodeError = DecodeError::Unrecognized;
const MAGIC: u16 = 0xcc52;
const FIXED_HEADER_LEN: usize = 15;
const CLEAR_FIRST: u8 = 1;
const NO_BACKGROUND: u8 = 2;
const ALPHA: u8 = 4;
const COMMENTS: u8 = 8;
/// Set in an opcode that has a 16-bit operand.
const LONG: u8 = 0x40;
const ALPHA_CHANNEL: u8 = 255;

const SKIP_LINES: u8 = 1;
const SET_COLOR: u8 = 2;
const SKIP_PIXELS: u8 = 3;
const PIXEL_DATA: u8 = 5;
const RUN: u8 = 6;
const EOF: u8 = 7;

/// A color map: `channels` tables of `len` 8-bit entries.
struct ColorMap {
    channels: usize,
    len: usize,
    entries: Vec<u8>,
}

impl ColorMap {
    /// Entry `value` of table `channel`; a one-table map serves every channel.
    fn get(&self, channel: usize, value: u8) -> u8 {
        let table = if self.channels == 1 { 0 } else { channel };
        let index = usize::from(value).min(self.len - 1);
        self.entries[table * self.len + index]
    }
}

/// The picture while the operations run: raw channel values in the image (a
/// one-channel picture keeps its value in the first byte of each pixel) and
/// an alpha plane if the file has one.
struct Canvas {
    image: Image,
    alpha: Option<Vec<u8>>,
    colors: usize,
}

impl Canvas {
    /// Writes `values` to channel `channel` of scanline `line` from `x` on.
    /// Scanline 0 is the bottom row; what falls outside the picture, or in a
    /// channel the file does not have, is dropped.
    /// Returns how many pixels it was asked to write inside the picture.
    fn write(
        &mut self,
        line: usize,
        x: usize,
        channel: u8,
        values: impl Iterator<Item = u8>,
    ) -> usize {
        let (width, height) = (self.image.width() as usize, self.image.height() as usize);
        if line >= height || x >= width {
            return 0;
        }
        let row = height - 1 - line;
        let values = values.take(width - x);
        let work = values.size_hint().1.unwrap_or(0);
        if channel == ALPHA_CHANNEL {
            if let Some(alpha) = &mut self.alpha {
                for (slot, value) in alpha[row * width + x..].iter_mut().zip(values) {
                    *slot = value;
                }
            }
        } else if usize::from(channel) < self.colors {
            let pixels = self.image.row_mut(row as u32)[x * 3..]
                .as_chunks_mut::<3>()
                .0;
            for (pixel, value) in pixels.iter_mut().zip(values) {
                pixel[usize::from(channel)] = value;
            }
        }
        work
    }
}

pub(super) fn decode_utah_rle(data: &[u8]) -> Result<Image, DecodeError> {
    if le16(data, 0) != Some(MAGIC) || data.len() < FIXED_HEADER_LEN {
        return Err(FAIL);
    }
    let width = usize::from(le16(data, 6).ok_or(FAIL)?);
    let height = usize::from(le16(data, 8).ok_or(FAIL)?);
    let [flags, colors, pixel_bits, map_channels, map_bits] = data[10..15] else {
        return Err(FAIL);
    };
    let (colors, map_channels) = (usize::from(colors), usize::from(map_channels));
    // A map has one table for all channels or one per color; a one-channel
    // picture may use a three-table map as a palette.
    if flags & 0xf0 != 0
        || !matches!(colors, 1 | 3)
        || pixel_bits != 8
        || !matches!(map_channels, 0 | 1 | 3)
        || map_bits > 8
    {
        return Err(FAIL);
    }
    check_size(width, height)?;

    // The background color. The paper pads it to an odd length, which a
    // channel count of 1 or 3 already is.
    let mut pos = FIXED_HEADER_LEN;
    let background = data.get(pos..pos + colors).ok_or(FAIL)?;
    pos += colors;
    let map = if map_channels == 0 {
        None
    } else {
        let len = 1usize << map_bits;
        let raw = data.get(pos..pos + map_channels * len * 2).ok_or(FAIL)?;
        pos += raw.len();
        // Entries are 16 bits, left justified: the high byte is the value.
        let entries = raw.as_chunks::<2>().0.iter().map(|pair| pair[1]).collect();
        Some(ColorMap {
            channels: map_channels,
            len,
            entries,
        })
    };
    if flags & COMMENTS != 0 {
        let length = usize::from(le16(data, pos).ok_or(FAIL)?);
        pos += 2 + length + length % 2;
    }
    let ops = data.get(pos..).ok_or(FAIL)?;

    let mut canvas = Canvas {
        image: Image::new(width as u32, height as u32),
        alpha: (flags & ALPHA != 0).then(|| alloc::vec![0; width * height]),
        colors,
    };
    if flags & CLEAR_FIRST != 0 && flags & NO_BACKGROUND == 0 {
        for y in 0..height as u32 {
            for pixel in canvas.image.row_mut(y).as_chunks_mut::<3>().0 {
                pixel[..colors].copy_from_slice(background);
            }
        }
    }
    run_operations(ops, &mut canvas)?;

    let Canvas {
        mut image, alpha, ..
    } = canvas;
    for y in 0..height {
        for x in 0..width {
            // The canvas holds raw channel values until now.
            let raw = image.get(x as u32, y as u32).to_be_bytes();
            let [r, g, b] = pixel_color(&raw[1..], colors, map.as_ref());
            let a = alpha.as_ref().map_or(255, |alpha| alpha[y * width + x]);
            image.set_argb(x as u32, y as u32, u32::from_be_bytes([a, r, g, b]));
        }
    }
    Ok(image)
}

/// The color of a pixel from its raw values and the color map.
fn pixel_color(raw: &[u8], colors: usize, map: Option<&ColorMap>) -> [u8; 3] {
    match (colors, map) {
        (1, None) => [raw[0]; 3],
        (1, Some(map)) if map.channels == 1 => [map.get(0, raw[0]); 3],
        (1, Some(map)) => [0, 1, 2].map(|channel| map.get(channel, raw[0])),
        (_, None) => [raw[0], raw[1], raw[2]],
        (_, Some(map)) => [0, 1, 2].map(|channel| map.get(channel, raw[channel])),
    }
}

/// Runs the scanline operations on the canvas.
fn run_operations(ops: &[u8], canvas: &mut Canvas) -> Result<(), DecodeError> {
    let (mut pos, mut line, mut x, mut channel) = (0, 0usize, 0usize, 0u8);
    // Runs may overwrite the same pixels again and again, so cap the pixels
    // written at a small multiple of the picture plus the operation bytes.
    let mut budget =
        WORK_FACTOR * (canvas.image.width() as usize * canvas.image.height() as usize + ops.len());
    while let Some(&op) = ops.get(pos) {
        let operand = |at: usize| -> Result<usize, DecodeError> {
            if op & LONG != 0 {
                le16(ops, at + 2).map(usize::from).ok_or(FAIL)
            } else {
                ops.get(at + 1).map(|&b| usize::from(b)).ok_or(FAIL)
            }
        };
        // The long form is a 4-byte head, the short form 2.
        let head = if op & LONG != 0 { 4 } else { 2 };
        match op & !LONG {
            EOF if op == EOF => break,
            SET_COLOR if op == SET_COLOR => {
                channel = *ops.get(pos + 1).ok_or(FAIL)?;
                x = 0;
                pos += 2;
            }
            SKIP_LINES => {
                line += operand(pos)?;
                x = 0;
                pos += head;
            }
            SKIP_PIXELS => {
                x += operand(pos)?;
                pos += head;
            }
            PIXEL_DATA => {
                // One more pixel than the operand, padded to an even length.
                let count = operand(pos)? + 1;
                let start = pos + head;
                let values = ops.get(start..start + count).ok_or(FAIL)?;
                let work = canvas.write(line, x, channel, values.iter().copied());
                budget = budget.checked_sub(work).ok_or(FAIL)?;
                x += count;
                pos = start + count + count % 2;
            }
            RUN => {
                let count = operand(pos)? + 1;
                let start = pos + head;
                let value = *ops.get(start).ok_or(FAIL)?;
                ops.get(start + 1).ok_or(FAIL)?;
                let work = canvas.write(line, x, channel, core::iter::repeat_n(value, count));
                budget = budget.checked_sub(work).ok_or(FAIL)?;
                x += count;
                pos = start + 2;
            }
            _ => return Err(FAIL),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::CLEAR;

    /// A header for a `width` x `height` picture of `colors` channels with
    /// the given flags, a black background and no color map or comments.
    fn header(width: u16, height: u16, flags: u8, colors: u8) -> Vec<u8> {
        let mut bytes = alloc::vec![0x52, 0xcc, 0, 0, 0, 0];
        bytes.extend_from_slice(&width.to_le_bytes());
        bytes.extend_from_slice(&height.to_le_bytes());
        bytes.extend_from_slice(&[flags, colors, 8, 0, 8]);
        bytes.extend(core::iter::repeat_n(0, usize::from(colors)));
        bytes
    }

    #[test]
    fn repeated_runs_over_one_row_are_bounded() {
        // 60000x1 gray, then 100000 repeats of SET_COLOR and a full-row run: 6 GB of
        // writes if unbounded.
        let mut file = header(60000, 1, 0, 1);
        for _ in 0..100_000 {
            file.extend_from_slice(&[SET_COLOR, 0, RUN | LONG, 0, 0x5f, 0xea, 7, 0]);
        }
        assert!(decode_utah_rle(&file).is_err());
    }

    #[test]
    fn rows_run_from_the_bottom_and_channels_are_set_separately() {
        // 3x2 RGB with no background. Bottom row: red channel 1 2 3 (pixel
        // data), green a run of 9. Then up one line, blue a run of 7.
        let mut file = header(3, 2, 0, 3);
        file.extend_from_slice(&[SET_COLOR, 0, PIXEL_DATA, 2, 1, 2, 3, 0]);
        file.extend_from_slice(&[SET_COLOR, 1, RUN, 2, 9, 0]);
        file.extend_from_slice(&[
            SKIP_LINES,
            1,
            SET_COLOR,
            2,
            RUN | LONG,
            0,
            2,
            0,
            7,
            0,
            EOF,
            0,
        ]);
        let image = decode_utah_rle(&file).unwrap();
        assert_eq!(image.get(0, 1), 0x010900);
        assert_eq!(image.get(2, 1), 0x030900);
        assert_eq!(image.get(1, 0), 0x000007);
    }

    #[test]
    fn background_skips_alpha_and_color_maps() {
        // 4x1 gray with ClearFirst, background 5 and alpha: pixels 1..3 get
        // value 20 and alpha 255; the others stay background with alpha 0.
        let mut file = header(4, 1, CLEAR_FIRST | ALPHA, 1);
        file[15] = 5;
        file.extend_from_slice(&[SET_COLOR, 0, SKIP_PIXELS, 1, RUN, 1, 20, 0]);
        file.extend_from_slice(&[SET_COLOR, ALPHA_CHANNEL, SKIP_PIXELS, 1, RUN, 1, 255, 0]);
        let image = decode_utah_rle(&file).unwrap();
        assert_eq!(image.get_argb(0, 0), CLEAR);
        assert_eq!(image.get_argb(1, 0), 0xff14_1414);
        assert_eq!(image.get_argb(3, 0), CLEAR);
    }

    #[test]
    fn a_three_table_map_is_a_palette_for_one_channel() {
        // 2x1 indexed picture, two entries per table (cmaplen 1); the high
        // byte of each 16-bit entry is the value.
        let mut file = header(2, 1, 0, 1);
        file[13] = 3;
        file[14] = 1;
        file.extend_from_slice(&[0, 0x11, 0, 0x22, 0, 0x33, 0, 0x44, 0, 0x55, 0, 0x66]);
        file.extend_from_slice(&[SET_COLOR, 0, PIXEL_DATA, 1, 1, 0]);
        let image = decode_utah_rle(&file).unwrap();
        assert_eq!((image.get(0, 0), image.get(1, 0)), (0x224466, 0x113355));
    }

    #[test]
    fn rejects_other_headers_and_unknown_opcodes() {
        let good = header(2, 2, 0, 3);
        assert!(decode_utah_rle(&good).is_ok());
        let mut opcode = good.clone();
        opcode.extend_from_slice(&[4, 0]);
        assert!(decode_utah_rle(&opcode).is_err());
        let mut cut = good.clone();
        cut.extend_from_slice(&[PIXEL_DATA, 5, 1]);
        assert!(decode_utah_rle(&cut).is_err());
        assert!(decode_utah_rle(&header(2, 2, 0, 2)).is_err());
        assert!(decode_utah_rle(&header(0, 2, 0, 3)).is_err());
        assert!(decode_utah_rle(&header(65535, 65535, 0, 3)).is_err());
    }
}
