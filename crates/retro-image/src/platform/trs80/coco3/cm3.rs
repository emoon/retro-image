//! CoCoMax III CM3 pictures.
//!
//! Sources: the KAOS Toolkit by Chet Simpson, MIT license (notice below),
//! <https://github.com/ChetSimpson/KAOSToolkit>:
//! - `AssetFoo/docs/images/cm3.dox`: the 29-byte header, the optional
//!   243-byte pattern table and the page layout. Its page on compression is
//!   a TODO.
//! - `AssetFoo/src/images/cm3/cm3_image_reader.cpp` and
//!   `KAOSCore/include/kaos/core/utility/bit_reader.h`, read to learn the
//!   compression. Per page: a row count byte, then per row a control byte.
//!   Top bit set: 160 raw bytes. Otherwise 20 bytes of "horizontal" bits
//!   (one per byte of the row, most significant bit first) and `control`
//!   bytes of "vertical" bits. A clear horizontal bit repeats the previous
//!   byte; a set one takes a new byte from the stream if the next vertical
//!   bit is set, else keeps the byte at that position in the row above. The
//!   previous byte carries over from one row to the next. Rows are 160
//!   bytes of 4-bit pixels.
//! - Checked against the 19 KAOS test pictures in `corpus/extra/coco3`: every
//!   row must consume its streams and the file must end after the last
//!   row, with only zero padding (to a multiple of 128 bytes in six files)
//!   left over; the renders were reviewed by eye.
//!
//! Differences from the KAOS reader: it leaves raw rows (top bit set) out
//! of the picture, which looks like an omission. `mazda.cm3` contains raw
//! rows: read as 160 literal bytes each, the file ends exactly after row 192
//! and the picture is clean, so they are drawn like any other row here.
//! Patterns and the animation and cycling fields are skipped. No sample has
//! a second page (320x384). The color space is not stored and is assumed
//! to be RGB222, as for the other CoCo 3 formats.
//!
//! The KAOS Toolkit license:
//!
//! > MIT License
//! >
//! > Copyright (c) 2023 HyperTech Gaming and Chet Simpson
//! >
//! > Permission is hereby granted, free of charge, to any person obtaining a
//! > copy of this software and associated documentation files (the
//! > "Software"), to deal in the Software without restriction, including
//! > without limitation the rights to use, copy, modify, merge, publish,
//! > distribute, sublicense, and/or sell copies of the Software, and to permit
//! > persons to whom the Software is furnished to do so, subject to the
//! > following conditions:
//! >
//! > The above copyright notice and this permission notice shall be included
//! > in all copies or substantial portions of the Software.
//! >
//! > THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
//! > OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
//! > MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
//! > IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
//! > CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
//! > TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE
//! > SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use alloc::vec::Vec;

use super::{COLORMAP_LEN, picture};
use crate::{DecodeError, Image};

const HEADER_LEN: usize = 29;
const PATTERNS_LEN: usize = 243;
const PAGE_ROWS: usize = 192;
const ROW_LEN: usize = 160;
const HORIZONTAL_LEN: usize = ROW_LEN / 8;
const TWO_PAGES: u8 = 0x80;
const NO_PATTERNS: u8 = 0x01;
const RAW_ROW: u8 = 0x80;

/// Most-significant-bit-first reader over a byte slice.
struct Bits<'a> {
    bytes: &'a [u8],
    next: usize,
}

impl<'a> Bits<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Bits { bytes, next: 0 }
    }

    fn read(&mut self) -> Option<bool> {
        let byte = self.bytes.get(self.next / 8)?;
        let bit = byte >> (7 - self.next % 8) & 1;
        self.next += 1;
        Some(bit != 0)
    }
}

/// A cursor over the byte stream.
struct Stream<'a>(&'a [u8]);

impl<'a> Stream<'a> {
    fn take(&mut self, len: usize) -> Option<&'a [u8]> {
        let (head, rest) = self.0.split_at_checked(len)?;
        self.0 = rest;
        Some(head)
    }

    fn byte(&mut self) -> Option<u8> {
        self.take(1).map(|b| b[0])
    }
}

pub(in super::super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    const FAIL: DecodeError = DecodeError::Invalid;
    let header = data.get(..HEADER_LEN).ok_or(FAIL)?;
    let flags = header[0];
    let pages = if flags & TWO_PAGES != 0 { 2 } else { 1 };
    let patterns = if flags & NO_PATTERNS != 0 {
        0
    } else {
        PATTERNS_LEN
    };
    let mut stream = Stream(data.get(HEADER_LEN + patterns..).ok_or(FAIL)?);
    let mut packed = Vec::with_capacity(pages * PAGE_ROWS * ROW_LEN);
    // Both persist across rows and pages: the row above, and the last byte.
    let mut row = [0u8; ROW_LEN];
    let mut last = 0u8;
    for _ in 0..pages {
        if usize::from(stream.byte().ok_or(FAIL)?) != PAGE_ROWS {
            return Err(FAIL);
        }
        for _ in 0..PAGE_ROWS {
            unpack_row(&mut stream, &mut row, &mut last).ok_or(FAIL)?;
            packed.extend_from_slice(&row);
        }
    }
    // The file ends with the last row, maybe followed by zero padding up to
    // a multiple of 128 bytes (six of the 19 samples).
    if stream.0.iter().any(|&b| b != 0) {
        return Err(FAIL);
    }
    picture(
        320,
        (pages * PAGE_ROWS) as u32,
        4,
        &header[1..=COLORMAP_LEN],
        &packed,
    )
}

/// Replaces `row` (the previous row on entry) with the next row's bytes.
fn unpack_row(stream: &mut Stream, row: &mut [u8; ROW_LEN], last: &mut u8) -> Option<()> {
    let control = stream.byte()?;
    if control & RAW_ROW != 0 {
        row.copy_from_slice(stream.take(ROW_LEN)?);
        *last = row[ROW_LEN - 1];
        return Some(());
    }
    let mut horizontal = Bits::new(stream.take(HORIZONTAL_LEN)?);
    let mut vertical = Bits::new(stream.take(usize::from(control))?);
    for byte in row.iter_mut() {
        if !horizontal.read()? {
            *byte = *last;
        } else if vertical.read()? {
            *byte = stream.byte()?;
        }
        *last = *byte;
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;

    /// A header with the color map left black and no patterns.
    fn header() -> Vec<u8> {
        let mut data = vec![NO_PATTERNS];
        data.resize(HEADER_LEN, 0);
        data
    }

    #[test]
    fn rows_repeat_the_byte_above_and_the_previous_byte() {
        let mut data = header();
        data[1..=COLORMAP_LEN].copy_from_slice(&[0x3f; COLORMAP_LEN]);
        data[2] = 0; // color 1 black
        data.push(PAGE_ROWS as u8);
        // Row 0: all horizontal and vertical bits set, 160 new bytes.
        data.push(20);
        data.extend_from_slice(&[0xff; 20]);
        data.extend_from_slice(&[0xff; 20]);
        data.extend((0..160).map(|i| if i == 0 { 0x10 } else { 0x00 }));
        // Row 1: horizontal bits set, vertical bits clear: a copy of row 0.
        data.push(20);
        data.extend_from_slice(&[0xff; 20]);
        data.extend_from_slice(&[0; 20]);
        // Row 2: no horizontal bits: every byte repeats the previous one.
        data.push(0);
        data.extend_from_slice(&[0; 20]);
        for _ in 3..PAGE_ROWS {
            // Same again, nothing to read.
            data.push(0);
            data.extend_from_slice(&[0; 20]);
        }
        let image = decode(&data).unwrap();
        assert_eq!((image.width(), image.height()), (320, 192));
        let pixel = |x: usize, y: usize| image.rgb()[(y * 320 + x) * 3];
        // Row 0 and row 1 start with color 1 (black), then color 0.
        assert_eq!((pixel(0, 0), pixel(1, 0), pixel(0, 1)), (0, 255, 0));
        // Row 2 repeats row 1's last byte (color 0 pair), so it is white.
        assert_eq!((pixel(0, 2), pixel(319, 191)), (255, 255));
        data.extend_from_slice(&[0; 9]);
        assert!(decode(&data).is_ok(), "zero padding is allowed");
        data.push(1);
        assert!(decode(&data).is_err(), "other trailing data is rejected");
        data.truncate(data.len() - 12);
        assert!(decode(&data).is_err(), "truncated data is rejected");
    }

    #[test]
    fn raw_rows_are_kept() {
        let mut data = header();
        data.push(PAGE_ROWS as u8);
        for _ in 0..PAGE_ROWS {
            data.push(RAW_ROW);
            data.extend_from_slice(&[0x12; ROW_LEN]);
        }
        assert!(decode(&data).is_ok());
    }
}
