//! BSAVE screen dumps (`FD` files from GW-BASIC and BASICA, and from programs
//! that write the same header): CGA 320x200 pictures, MCGA 320x200 pictures
//! and 80x25 text screens.
//!
//! Sources:
//! - Deark `bsave.c` and `pcpaint.c` (<https://github.com/jsummers/deark>, MIT
//!   license): the 7-byte header (`FD`, 16-bit load segment, 16-bit offset,
//!   16-bit data length), the choice of layout from the load address and the
//!   length, the PCPaint settings kept in the unused part of CGA memory, and
//!   PCPaint v1.5's compressed BSAVE, whose header is `FD 00 B8 00 00 00 00`
//!   followed by the unpacked size, a block count and PCPaint's run-length
//!   blocks (`pcpaint.rs`). Deark calls its own detection "barely anything",
//!   and notes that a few files store the file size as the data length.
//! - Wikipedia, "Color Graphics Adapter" (`cga.rs`): even scan lines in the
//!   first 8192 bytes of video memory, odd lines in the second 8192; four
//!   2-bit pixels per byte.
//! - Reverse engineered from the 23 sample files (Sembiance's `bsave` and
//!   `bsaveCompressed` folders); the layouts below were checked by eye.
//!
//! Layouts (load address, data length):
//! - `B8000`, 16000 to 16384 bytes: CGA 320x200, 4 colors. A file of 16000
//!   bytes lacks the last 192 bytes of the second bank, so its bottom rows
//!   are black; 16192 bytes is what PCPaint writes (its settings fill the gap
//!   between the banks).
//!   A 640x200 two-color dump has the same memory layout and cannot be told
//!   from a four-color one, so everything is shown as four colors.
//! - `A0000`, 64000 bytes: MCGA mode 13h, 320x200, 256 colors.
//! - `B8000`, 4000 bytes: an 80x25 text screen (character, attribute).
//!
//! Palettes are guesses. A file with PCPaint's `PCPaint V1.x` mark at data
//! offset 8000 names its CGA palette and border color two bytes after the
//! mark (palette numbers as in `pcpaint.rs`, the border color is background
//! color 0, found on one sample file). Every other CGA picture uses palette 1
//! at high intensity (black, cyan, magenta, white), the usual CGA default and
//! the one Deark picks. MCGA pictures use the default VGA BIOS palette, since
//! the dump has none.
//!
//! Other BSAVE contents (program-specific layouts, packed or multi-record
//! files) are rejected. Only the first record of a multi-record file is read.
//! `FD` plus a plausible length is the only check, so the format is chosen by
//! extension.
//!
//! Verification: no RECOIL oracle for this format; output matches Deark's PNG
//! output on the sample files that Deark's own detection handles.

// Parts of this file follow Deark's modules/bsave.c and modules/pcpaint.c
// (Deark, https://github.com/jsummers/deark):
//
// Copyright (C) 2016 Jason Summers
// <jason1@pobox.com>
//
// Copyright (C) 2016 Jason Summers
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

use alloc::borrow::Cow;
use alloc::vec::Vec;

use super::CGA_PALETTE;
use super::cga::{BANK_STRIDE, deinterlace, unpack_2bit};
use super::pcpaint::{CGA_4, VGA_PALETTE, unpack_blocks};
use crate::bytes::le16;
use crate::platform::textmode;
use crate::{DecodeError, Image};

const FAIL: DecodeError = DecodeError::Unrecognized;
const MARKER: u8 = 0xfd;
const HEADER_LEN: usize = 7;
const CGA_MEMORY: usize = 0xb8000;
const MCGA_MEMORY: usize = 0xa0000;
const WIDTH: usize = 320;
const HEIGHT: usize = 200;
const ROW_LEN: usize = WIDTH / 4;
const SCREEN_LEN: usize = 2 * BANK_STRIDE;
/// CGA palette 1 at high intensity.
const DEFAULT_PALETTE: [u32; 4] = CGA_4[3];
/// Where PCPaint keeps its mark, in the gap between the CGA banks.
const PCPAINT_MARK: &[u8] = b"PCPaint V1.";
const PCPAINT_MARK_AT: usize = 8000;
/// A compressed file: the header up to the length field, which is zero.
const COMPRESSED_HEADER: [u8; 7] = [MARKER, 0x00, 0xb8, 0, 0, 0, 0];

/// The load address and the data of the first record.
fn read_record(data: &[u8]) -> Result<(usize, Cow<'_, [u8]>), DecodeError> {
    if data.first() != Some(&MARKER) {
        return Err(FAIL);
    }
    let address =
        usize::from(le16(data, 1).ok_or(FAIL)?) * 16 + usize::from(le16(data, 3).ok_or(FAIL)?);
    if data.starts_with(&COMPRESSED_HEADER) {
        let size = usize::from(le16(data, 7).ok_or(FAIL)?);
        let blocks = usize::from(le16(data, 9).ok_or(FAIL)?);
        let packed = data.get(11..).ok_or(FAIL)?;
        return Ok((
            address,
            Cow::Owned(unpack_blocks(packed, blocks, size).ok_or(FAIL)?),
        ));
    }
    // A length past the end of the file is a damaged header, not a dump.
    let length = usize::from(le16(data, 5).ok_or(FAIL)?);
    if length == 0 || length > data.len() {
        return Err(FAIL);
    }
    let body = &data[HEADER_LEN..];
    Ok((address, Cow::Borrowed(&body[..length.min(body.len())])))
}

pub(super) fn decode_bsave(data: &[u8]) -> Result<Image, DecodeError> {
    let (address, screen) = read_record(data)?;
    match (address, screen.len()) {
        (MCGA_MEMORY, 64000) => {
            Image::from_indexed(WIDTH as u32, HEIGHT as u32, &screen, &VGA_PALETTE)
        }
        (CGA_MEMORY, 16000..=SCREEN_LEN) => decode_cga(&screen),
        (CGA_MEMORY, 4000) => textmode::render_text_screen(&screen, 80),
        _ => Err(FAIL),
    }
}

/// PCPaint's palette number and border color, if the file carries them.
fn pcpaint_palette(screen: &[u8]) -> Option<[u32; 4]> {
    let mark = screen.get(PCPAINT_MARK_AT..PCPAINT_MARK_AT + PCPAINT_MARK.len())?;
    if mark != PCPAINT_MARK {
        return None;
    }
    let settings = screen.get(PCPAINT_MARK_AT + PCPAINT_MARK.len() + 1..)?;
    let (number, border) = (
        usize::from(*settings.first()?),
        usize::from(*settings.get(1)?),
    );
    let mut palette = *CGA_4.get(number).unwrap_or(&CGA_4[2]);
    palette[0] = CGA_PALETTE[border & 15];
    Some(palette)
}

fn decode_cga(screen: &[u8]) -> Result<Image, DecodeError> {
    let palette = pcpaint_palette(screen).unwrap_or(DEFAULT_PALETTE);
    // A dump cut short shows black where it ends.
    let mut memory: Vec<u8> = screen.to_vec();
    memory.resize(SCREEN_LEN, 0);
    let rows = deinterlace(&memory, HEIGHT, ROW_LEN, 2, BANK_STRIDE);
    Image::from_indexed(WIDTH as u32, HEIGHT as u32, &unpack_2bit(&rows), &palette)
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;

    fn dump(segment: u16, body: &[u8]) -> Vec<u8> {
        let mut file = vec![MARKER];
        file.extend_from_slice(&segment.to_le_bytes());
        file.extend_from_slice(&[0, 0]);
        file.extend_from_slice(&(body.len() as u16).to_le_bytes());
        file.extend_from_slice(body);
        file
    }

    #[test]
    fn cga_rows_come_from_both_banks_and_a_short_dump_is_padded() {
        let mut body = vec![0u8; 16000];
        body[0] = 0b0100_0000; // row 0, pixel 0: color 1
        body[BANK_STRIDE] = 0b1000_0000; // row 1, pixel 0: color 2
        let image = decode_bsave(&dump(0xb800, &body)).unwrap();
        assert_eq!((image.width(), image.height()), (320, 200));
        assert_eq!(image.get(0, 0), DEFAULT_PALETTE[1]);
        assert_eq!(image.get(0, 1), DEFAULT_PALETTE[2]);
        assert_eq!(image.get(0, 199), DEFAULT_PALETTE[0]);
    }

    #[test]
    fn the_pcpaint_mark_picks_the_palette_and_the_border_color() {
        let mut body = vec![0u8; 16192];
        body[PCPAINT_MARK_AT..PCPAINT_MARK_AT + 12].copy_from_slice(b"PCPaint V1.0");
        body[PCPAINT_MARK_AT + 12] = 1; // palette 0 at low intensity
        body[PCPAINT_MARK_AT + 13] = 9; // border: light blue
        let image = decode_bsave(&dump(0xb800, &body)).unwrap();
        assert_eq!(image.get(0, 0), CGA_PALETTE[9]);
        let mut green = body.clone();
        green[0] = 0b0100_0000;
        assert_eq!(
            decode_bsave(&dump(0xb800, &green)).unwrap().get(0, 0),
            CGA_4[1][1]
        );
    }

    #[test]
    fn mcga_uses_the_vga_palette_and_odd_layouts_are_rejected() {
        let mut body = vec![0u8; 64000];
        body[0] = 4;
        let image = decode_bsave(&dump(0xa000, &body)).unwrap();
        assert_eq!(image.get(0, 0), VGA_PALETTE[4]);
        assert!(decode_bsave(&dump(0xb800, &body[..5000])).is_err());
        assert!(decode_bsave(&dump(0x1234, &body[..16000])).is_err());
        // The length field must fit the file.
        let mut short = dump(0xa000, &body);
        short.truncate(100);
        assert!(decode_bsave(&short).is_err());
    }
}
