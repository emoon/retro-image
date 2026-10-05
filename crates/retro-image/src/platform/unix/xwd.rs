//! X Window dump (`.xwd`), file version 7. Dumps named `.dmp` are found by content.
//!
//! Sources:
//! - `XWDFile.h` of the X Window System (xorgproto,
//!   <https://gitlab.freedesktop.org/xorg/proto/xorgproto/-/blob/master/include/X11/XWDFile.h>,
//!   notice below): a header of 25 big-endian 32-bit words (header size,
//!   version 7, pixmap format, depth, width, height, x offset, byte order,
//!   bitmap unit, bitmap bit order, bitmap pad, bits per pixel, bytes per
//!   line, visual class, red, green and blue masks, bits per RGB, colormap
//!   entries, number of colors, window geometry), the window name up to the
//!   header size, `ncolors` colormap entries of 12 bytes (pixel, red, green and
//!   blue as 16-bit values, flags, pad), then the image data. Bitmaps (depth 1
//!   with the X format) pad rows to the bitmap unit; pixmaps pad them to the
//!   bitmap pad. Pixel values follow the byte order, and the bit order
//!   applies to 1-bit pictures.
//! - Which of `ncolors` and `colormap_entries` sizes the colormap: Deark's
//!   `modules/xwd.c` (<https://github.com/jsummers/deark>, MIT license,
//!   notice below) says writers differ, and settles it from the file size.
//!   Here the image starts after `ncolors` entries, or after
//!   `colormap_entries` when only that count makes the file size add up.
//!
//! Decoded: ZPixmap pictures of 1, 8, 16, 24 and 32 bits per pixel and 1-bit
//! XYBitmaps. True and direct color pictures (visual class 4 and 5) use the
//! masks (a direct-color colormap is ignored); pictures of 8 bits or fewer
//! use the colormap by pixel value (by entry order when the entries do not
//! have distinct pixel values), or a gray ramp without one. XYPixmap
//! (several bit planes) is not decoded. A 1-bit file without a colormap
//! shows 0 as white and 1 as black, which is how ffmpeg reads the 1-bit dumps
//! it writes (Deark shows them inverted). The samples written by X programs
//! all have a colormap, so this case comes from other writers.
//!
//! The strict header (version 7, known formats and sizes, the image inside
//! the file) is the signature.
//!
//! `MARBLES.XWD` says 24 bits per pixel but stores 4 bytes per pixel (its
//! rows are `width * 4` bytes), which Deark also notices; such files are read
//! as 32 bits per pixel.
//!
//! Verification: no RECOIL oracle. Output matches Deark's `xwd` module pixel
//! for pixel on all 11 samples. ffmpeg's decoder (run as a black box) agrees
//! on 6; it cannot read the three `.dmp` files, shows 1-bit `screen.xwd`
//! inverted and reads `MARBLES.XWD` as 24 bits (see the divergence file
//! `unix-rasters.tsv`).

// XWDFile.h is distributed under this license:
//
// Copyright 1985, 1986, 1998  The Open Group
//
// Permission to use, copy, modify, distribute, and sell this software and its
// documentation for any purpose is hereby granted without fee, provided that
// the above copyright notice appear in all copies and that both that
// copyright notice and this permission notice appear in supporting
// documentation.
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.  IN NO EVENT SHALL THE
// OPEN GROUP BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN
// AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
// CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
//
// Except as contained in this notice, the name of The Open Group shall not be
// used in advertising or otherwise to promote the sale, use or other dealings
// in this Software without prior written authorization from The Open Group.

// The colormap size rule follows Deark's modules/xwd.c
// (Deark, https://github.com/jsummers/deark):
//
// Copyright (C) 2016-2026 Jason Summers
// <jason1@pobox.com>
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

use alloc::vec::Vec;

use super::to_byte;
use crate::bytes::be32;
use crate::image::check_size;
use crate::{DecodeError, Image};

const FAIL: DecodeError = DecodeError::Unrecognized;
const HEADER_LEN: usize = 100;
const COLOR_LEN: usize = 12;
const VERSION: u32 = 7;
const Z_PIXMAP: u32 = 2;
const XY_BITMAP: u32 = 0;
const TRUE_COLOR: u32 = 4;
const DIRECT_COLOR: u32 = 5;

struct Header {
    width: usize,
    height: usize,
    depth: u32,
    bits_per_pixel: usize,
    bytes_per_line: usize,
    /// Pixel values and bitmap units are stored most significant byte first.
    msb_bytes: bool,
    msb_bits: bool,
    bitmap_unit: usize,
    visual_class: u32,
    masks: [u32; 3],
    /// Where the colormap starts and how many entries it has.
    colors_at: usize,
    colors: usize,
    /// Where the image data starts.
    image_at: usize,
}

fn parse_header(data: &[u8]) -> Result<Header, DecodeError> {
    let word = |i: usize| be32(data, 4 * i).ok_or(FAIL);
    let header_size = word(0)? as usize;
    if data.len() < HEADER_LEN || word(1)? != VERSION || !(HEADER_LEN..=4096).contains(&header_size)
    {
        return Err(FAIL);
    }
    let (format, depth) = (word(2)?, word(3)?);
    let (width, height) = (word(4)? as usize, word(5)? as usize);
    let (byte_order, bit_order, bitmap_pad) = (word(7)?, word(9)?, word(10)? as usize);
    let (mut bitmap_unit, mut bits_per_pixel) = (word(8)? as usize, word(11)? as usize);
    let bytes_per_line = word(12)? as usize;
    let visual_class = word(13)?;
    let (entries, ncolors) = (word(18)? as usize, word(19)? as usize);
    let supported = match format {
        Z_PIXMAP => matches!(bits_per_pixel, 1 | 8 | 16 | 24 | 32),
        XY_BITMAP => depth == 1 && bits_per_pixel == 1,
        _ => false,
    };
    if !supported
        || !(1..=32).contains(&depth)
        || byte_order > 1
        || bit_order > 1
        || !matches!(bitmap_unit, 8 | 16 | 32)
        || visual_class > 5
        || ncolors > 1 << 16
        || entries > 1 << 16
    {
        return Err(FAIL);
    }
    check_size(width, height)?;
    // `MARBLES.XWD` says 24 bits per pixel but has 4 bytes per pixel. That is
    // told from a row of 24-bit pixels padded to `bitmap_pad` (which also
    // takes `width * 4` bytes at widths 1 to 3 with a pad of 32) by working
    // out the padded length.
    let padded = if bitmap_pad >= 8 && bitmap_pad % 8 == 0 {
        (width * 3).next_multiple_of(bitmap_pad / 8)
    } else {
        width * 3
    };
    if bits_per_pixel == 24 && bytes_per_line == width * 4 && padded != width * 4 {
        bits_per_pixel = 32;
    }
    // The bitmap unit applies to bitmaps only (`XWDFile.h`). The 1-bit pixels
    // of a ZPixmap are bytes whose bit order is `bitmap_bit_order`, and a row
    // is as long as `bytes_per_line` says: ffmpeg writes unit 32 with rows of
    // ceil(width / 8) bytes.
    if format == Z_PIXMAP {
        bitmap_unit = 8;
    }
    // A 1-bit row is made of whole bitmap units.
    let used = match bits_per_pixel {
        1 => width.div_ceil(bitmap_unit) * (bitmap_unit / 8),
        bits => width * (bits / 8),
    };
    if bytes_per_line < used {
        return Err(FAIL);
    }
    let image_len = bytes_per_line.checked_mul(height).ok_or(FAIL)?;
    // The colormap normally has `ncolors` entries; some writers size it by
    // `colormap_entries`, which shows in the file size.
    let colors_at = header_size;
    let room = data
        .len()
        .checked_sub(colors_at)
        .and_then(|rest| rest.checked_sub(image_len))
        .ok_or(FAIL)?;
    let colors = if entries != ncolors && entries * COLOR_LEN == room {
        entries
    } else {
        ncolors
    };
    if colors * COLOR_LEN > room {
        return Err(FAIL);
    }
    Ok(Header {
        width,
        height,
        depth,
        bits_per_pixel,
        bytes_per_line,
        msb_bytes: byte_order == 1,
        msb_bits: bit_order == 1,
        bitmap_unit,
        visual_class,
        masks: [word(14)?, word(15)?, word(16)?],
        colors_at,
        colors,
        image_at: colors_at + colors * COLOR_LEN,
    })
}

pub(super) fn decode_xwd(data: &[u8]) -> Result<Image, DecodeError> {
    let h = parse_header(data)?;
    let rows = &data[h.image_at..];
    let mut image = Image::new(h.width as u32, h.height as u32);
    let direct = matches!(h.visual_class, TRUE_COLOR | DIRECT_COLOR) && h.masks != [0; 3];
    if direct && h.bits_per_pixel < 8 {
        return Err(FAIL);
    }
    let palette = if h.bits_per_pixel <= 8 && !direct {
        Some(palette(data, &h))
    } else if direct {
        None
    } else {
        return Err(FAIL); // 16 bits or more need masks
    };
    for (y, row) in rows
        .chunks_exact(h.bytes_per_line)
        .take(h.height)
        .enumerate()
    {
        for x in 0..h.width {
            let value = pixel(row, x, &h);
            let color = match &palette {
                Some(palette) => palette[value as usize & 0xff],
                None => {
                    let [r, g, b] = h.masks.map(|mask| channel(value, mask));
                    r << 16 | g << 8 | b
                }
            };
            image.set(x as u32, y as u32, color);
        }
    }
    Ok(image)
}

/// The pixel value at column `x` of a row.
fn pixel(row: &[u8], x: usize, h: &Header) -> u32 {
    let bytes = h.bits_per_pixel / 8;
    if bytes == 0 {
        // Bits of bitmap units: the unit's byte order and bit order decide.
        let unit_bytes = h.bitmap_unit / 8;
        let bit = x % h.bitmap_unit;
        let unit = &row[x / h.bitmap_unit * unit_bytes..][..unit_bytes];
        let value = unit.iter().enumerate().fold(0u32, |value, (i, &b)| {
            let shift = if h.msb_bytes { unit_bytes - 1 - i } else { i };
            value | u32::from(b) << (8 * shift)
        });
        let shift = if h.msb_bits {
            h.bitmap_unit - 1 - bit
        } else {
            bit
        };
        return value >> shift & 1;
    }
    row[x * bytes..][..bytes]
        .iter()
        .enumerate()
        .fold(0, |value, (i, &b)| {
            let shift = if h.msb_bytes { bytes - 1 - i } else { i };
            value | u32::from(b) << (8 * shift)
        })
}

/// One color channel of a pixel, scaled to 8 bits.
fn channel(pixel: u32, mask: u32) -> u32 {
    if mask == 0 {
        return 0;
    }
    let shift = mask.trailing_zeros();
    u32::from(to_byte((pixel & mask) >> shift, mask >> shift))
}

/// The colors of a picture of 8 bits or fewer, by pixel value: the colormap
/// entries where there are any, otherwise a gray ramp over the depth.
fn palette(data: &[u8], h: &Header) -> Vec<u32> {
    let mut palette = alloc::vec![0; 256];
    if h.colors == 0 && h.depth == 1 {
        // Black and white in the order of the bitmap formats (PBM, XBM) and of
        // ffmpeg, which reads and writes its 1-bit dumps this way; Deark
        // shows them inverted.
        palette[0] = 0xff_ffff;
        return palette;
    }
    if h.colors == 0 {
        let max = (1u32 << h.depth.min(8)) - 1;
        for (value, color) in (0..).zip(palette.iter_mut().take(max as usize + 1)) {
            *color = u32::from(to_byte(value, max)) * 0x01_0101;
        }
        return palette;
    }
    let entries = data[h.colors_at..].as_chunks::<COLOR_LEN>().0;
    let entries = entries.iter().take(h.colors);
    // Colors are 16-bit X11 values; the high byte is the 8-bit value, as in
    // Deark and ffmpeg.
    let color = |entry: &[u8; COLOR_LEN]| {
        let [r, g, b] = [4, 6, 8].map(|at| u32::from(entry[at]));
        r << 16 | g << 8 | b
    };
    let pixel_of = |entry: &[u8; COLOR_LEN]| be32(entry, 0).unwrap_or(u32::MAX) as usize;
    // Entries name their pixel value, but some writers leave the field the
    // same in every entry (the `.dmp` samples). Then the order decides.
    let mut named = alloc::vec![false; 256];
    let by_pixel = entries.clone().all(|entry| {
        named
            .get_mut(pixel_of(entry))
            .is_some_and(|seen| !core::mem::replace(seen, true))
    });
    for (index, entry) in entries.enumerate() {
        let value = if by_pixel { pixel_of(entry) } else { index };
        if let Some(slot) = palette.get_mut(value) {
            *slot = color(entry);
        }
    }
    palette
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The header fields the tests vary; the rest are zero.
    #[derive(Default)]
    struct Spec {
        format: u32,
        depth: u32,
        size: [u32; 2],
        msb_bytes: bool,
        unit: u32,
        msb_bits: bool,
        bits_per_pixel: u32,
        bytes_per_line: u32,
        pad: u32,
        class: u32,
        masks: [u32; 3],
    }

    fn file(spec: &Spec, colors: &[[u16; 3]], image: &[u8]) -> Vec<u8> {
        let mut words = alloc::vec![0u32; 25];
        words[0] = 100;
        words[1] = VERSION;
        words[2] = spec.format;
        words[3] = spec.depth;
        words[4..6].copy_from_slice(&spec.size);
        words[7] = u32::from(spec.msb_bytes);
        words[8] = spec.unit;
        words[9] = u32::from(spec.msb_bits);
        words[10] = spec.pad;
        words[11] = spec.bits_per_pixel;
        words[12] = spec.bytes_per_line;
        words[13] = spec.class;
        words[14..17].copy_from_slice(&spec.masks);
        words[18] = colors.len() as u32;
        words[19] = colors.len() as u32;
        let mut bytes: Vec<u8> = words.iter().flat_map(|w| w.to_be_bytes()).collect();
        for (i, rgb) in colors.iter().enumerate() {
            bytes.extend_from_slice(&(i as u32).to_be_bytes());
            for v in rgb {
                bytes.extend_from_slice(&v.to_be_bytes());
            }
            bytes.extend_from_slice(&[0, 0]);
        }
        bytes.extend_from_slice(image);
        bytes
    }

    /// Columns of row `y` that are black.
    fn black(image: &Image, y: u32) -> Vec<u32> {
        (0..image.width())
            .filter(|&x| image.get(x, y) == 0)
            .collect()
    }

    #[test]
    fn bitmaps_follow_the_unit_byte_order_and_bit_order() {
        // 20 pixels in 16-bit units. Bytes 01 80, least significant byte
        // and bit first, are pixels 0 and 15.
        let colors = [[0xffff; 3], [0, 0, 0]];
        let lsb = Spec {
            depth: 1,
            size: [20, 1],
            unit: 16,
            bits_per_pixel: 1,
            bytes_per_line: 4,
            class: 2,
            ..Spec::default()
        };
        let bytes = file(&lsb, &colors, &[1, 0x80, 0, 0]);
        assert_eq!(black(&decode_xwd(&bytes).unwrap(), 0), [0, 15]);
        // The same bytes as a ZPixmap with the most significant byte and
        // bit first: the unit is 0x0180, so pixels 7 and 8.
        let msb = Spec {
            format: Z_PIXMAP,
            msb_bytes: true,
            msb_bits: true,
            ..lsb
        };
        let bytes = file(&msb, &colors, &[1, 0x80, 0, 0]);
        assert_eq!(black(&decode_xwd(&bytes).unwrap(), 0), [7, 8]);
    }

    #[test]
    fn one_bit_pixmaps_are_read_byte_by_byte_with_rows_of_whole_bytes() {
        // How ffmpeg writes them: bitmap unit 32 and pad 8 in the header,
        // but rows of ceil(width / 8) bytes. 20 pixels take 3 bytes; the
        // bit order (most significant first) decides, not the byte order.
        let spec = Spec {
            format: Z_PIXMAP,
            depth: 1,
            size: [20, 2],
            msb_bytes: false,
            unit: 32,
            msb_bits: true,
            bits_per_pixel: 1,
            bytes_per_line: 3,
            pad: 8,
            class: 0,
            masks: [0; 3],
        };
        // No colormap: 1 is black. Row 0 sets pixels 0 and 15 (bytes 80 01),
        // row 1 sets pixel 19 (third byte 0x10).
        let raster = [0x80, 0x01, 0x00, 0x00, 0x00, 0x10];
        let image = decode_xwd(&file(&spec, &[], &raster)).unwrap();
        assert_eq!(black(&image, 0), [0, 15]);
        assert_eq!(black(&image, 1), [19]);
    }

    #[test]
    fn indexed_pictures_use_the_colormap_by_pixel_value() {
        let spec = Spec {
            format: Z_PIXMAP,
            depth: 8,
            size: [2, 1],
            msb_bytes: true,
            unit: 32,
            msb_bits: true,
            bits_per_pixel: 8,
            bytes_per_line: 4,
            class: 3,
            ..Spec::default()
        };
        let colors = [[0, 0, 0], [0xffff, 0x8080, 0]];
        let image = decode_xwd(&file(&spec, &colors, &[1, 0, 9, 9])).unwrap();
        assert_eq!((image.get(0, 0), image.get(1, 0)), (0xff8000, 0));
    }

    #[test]
    fn true_color_uses_the_masks_in_the_stated_byte_order() {
        // 565 pixels: 0xf800 is red, 0x001f blue.
        let lsb = Spec {
            format: Z_PIXMAP,
            depth: 16,
            size: [2, 1],
            unit: 32,
            bits_per_pixel: 16,
            bytes_per_line: 4,
            class: TRUE_COLOR,
            masks: [0xf800, 0x07e0, 0x001f],
            ..Spec::default()
        };
        let image = decode_xwd(&file(&lsb, &[], &[0x00, 0xf8, 0x1f, 0x00])).unwrap();
        assert_eq!((image.get(0, 0), image.get(1, 0)), (0xff0000, 0x0000ff));
        let msb = Spec {
            msb_bytes: true,
            ..lsb
        };
        let image = decode_xwd(&file(&msb, &[], &[0xf8, 0x00, 0x00, 0x1f])).unwrap();
        assert_eq!((image.get(0, 0), image.get(1, 0)), (0xff0000, 0x0000ff));
    }

    #[test]
    fn colormap_entries_with_equal_pixel_values_are_taken_in_order() {
        let spec = Spec {
            format: Z_PIXMAP,
            depth: 1,
            size: [2, 1],
            msb_bytes: true,
            unit: 8,
            msb_bits: true,
            bits_per_pixel: 1,
            bytes_per_line: 1,
            class: 2,
            ..Spec::default()
        };
        let mut bytes = file(&spec, &[[0x1100; 3], [0x2200; 3]], &[0b0100_0000]);
        // Make both entries claim pixel value 1.
        bytes[100..104].copy_from_slice(&1u32.to_be_bytes());
        bytes[112..116].copy_from_slice(&1u32.to_be_bytes());
        let image = decode_xwd(&bytes).unwrap();
        assert_eq!((image.get(0, 0), image.get(1, 0)), (0x111111, 0x222222));
    }

    #[test]
    fn rows_of_four_bytes_override_a_24_bit_header() {
        // Like `MARBLES.XWD`: 24 bits per pixel, pad 8, rows of width * 4.
        let spec = Spec {
            format: Z_PIXMAP,
            depth: 24,
            size: [4, 1],
            msb_bytes: true,
            unit: 8,
            msb_bits: true,
            bits_per_pixel: 24,
            bytes_per_line: 16,
            pad: 8,
            class: TRUE_COLOR,
            masks: [0xff0000, 0xff00, 0xff],
        };
        let raster = [0, 1, 2, 3, 0, 4, 5, 6, 0, 7, 8, 9, 0, 10, 11, 12];
        let image = decode_xwd(&file(&spec, &[], &raster)).unwrap();
        assert_eq!((image.get(0, 0), image.get(3, 0)), (0x010203, 0x0a0b0c));
    }

    #[test]
    fn rows_of_two_and_three_padded_24_bit_pixels_stay_24_bit() {
        // Padded to 32 bits, 24-bit rows take 8 and 12 bytes at widths 2 and
        // 3, which is also width * 4.
        for (width, row_len) in [(2u8, 8u32), (3, 12)] {
            let spec = Spec {
                format: Z_PIXMAP,
                depth: 24,
                size: [u32::from(width), 1],
                msb_bytes: true,
                unit: 32,
                msb_bits: true,
                bits_per_pixel: 24,
                bytes_per_line: row_len,
                pad: 32,
                class: TRUE_COLOR,
                masks: [0xff0000, 0xff00, 0xff],
            };
            let mut raster: Vec<u8> = (1..=3 * width).collect();
            raster.resize(row_len as usize, 0);
            let image = decode_xwd(&file(&spec, &[], &raster)).unwrap();
            for x in 0..u32::from(width) {
                let at = 3 * x as u8;
                let expected = u32::from_be_bytes([0, at + 1, at + 2, at + 3]);
                assert_eq!(image.get(x, 0), expected, "width {width}, column {x}");
            }
        }
    }

    #[test]
    fn rejects_foreign_headers_and_short_images() {
        let spec = Spec {
            format: Z_PIXMAP,
            depth: 8,
            size: [2, 1],
            unit: 32,
            bits_per_pixel: 8,
            bytes_per_line: 4,
            class: 3,
            ..Spec::default()
        };
        let ok = file(&spec, &[[0; 3]], &[0; 4]);
        assert!(decode_xwd(&ok).is_ok());
        assert!(decode_xwd(&ok[..ok.len() - 1]).is_err());
        let mut version = ok.clone();
        version[7] = 6;
        assert!(decode_xwd(&version).is_err());
        let planes = Spec { format: 1, ..spec };
        assert!(decode_xwd(&file(&planes, &[], &[0; 4])).is_err());
    }
}
