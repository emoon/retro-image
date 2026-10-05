//! X bitmap (`.xbm`): a monochrome picture written as C source.
//!
//! Sources:
//! - Xlib Programming Manual, "Manipulating Bitmaps"
//!   (<https://www.x.org/releases/current/doc/libX11/libX11/libX11.html>):
//!   the X11 format is `#define name_width W`, `#define name_height H`,
//!   optional `name_x_hot` and `name_y_hot`, then
//!   `static [unsigned] char name_bits[] = { 0xNN, ... };`, rows padded to whole
//!   bytes. The manual does not give the bit order: the leftmost pixel is the
//!   least significant bit, observed on the samples.
//! - The older X10 format uses `static short name_bits[]` with 16-bit values
//!   and rows padded to 16 bits. No document for it was found; it is decoded
//!   as the low byte of each value first, leftmost pixel in the least
//!   significant bit, which makes `iv.X` come out as a clean symmetric icon
//!   (reviewed by eye). `3270.icon` is also a `short` array, but its picture
//!   is only legible with the two bytes of every value exchanged (it reads
//!   "IBM 3278-2" then), so it looks like the work of a big-endian writer and
//!   decodes scrambled. Nothing tells the two apart, so there is no heuristic.
//!
//! A set bit is black, the foreground of a bitmap shown on a white
//! background. Pillow does the opposite (a set bit is white in its mode "1"
//! image), so its output is the inverse of ours on every sample. Hotspots
//! and any extra `#define` lines are ignored. Comments (`/* */` and `//`) are
//! skipped. Values may be hexadecimal (`0x`) or decimal.
//!
//! The file must open with a `#define`: that, plus both dimensions and
//! enough values, is the signature.
//!
//! Verification: no RECOIL oracle. On the 19 X11 samples the output matches
//! Pillow's XBM reader pixel for pixel after inverting it (five of them
//! after removing the leading comment, which Pillow does not accept), and
//! `abydos.xbm` and `sample_1920x1280.xbm` equal the PBM files of the same
//! pictures. The two X10 samples are checked by eye (see the divergence file
//! `unix-rasters.tsv`).

use alloc::vec::Vec;

use super::c_source::Tokens;
use crate::image::check_size;
use crate::{BitOrder, DecodeError, Image};

const FAIL: DecodeError = DecodeError::Unrecognized;
const WHITE: u32 = 0xff_ffff;
const BLACK: u32 = 0;

fn number(token: &[u8]) -> Option<u32> {
    let text = core::str::from_utf8(token).ok()?;
    match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        Some(hex) => u32::from_str_radix(hex, 16).ok(),
        None => text.parse().ok(),
    }
}

pub(super) fn decode_xbm(data: &[u8]) -> Result<Image, DecodeError> {
    let mut tokens = Tokens { data, pos: 0 };
    // The signature: the first thing in the file is a `#define`.
    if tokens.next() != Some(b"#define") {
        return Err(FAIL);
    }
    tokens.pos = 0;
    let (mut width, mut height, mut wide) = (None, None, false);
    while let Some(token) = tokens.next() {
        match token {
            b"#define" => {
                let name = tokens.next().ok_or(FAIL)?;
                let value = tokens.next().and_then(number);
                if name.ends_with(b"_width") {
                    width = width.or(value);
                } else if name.ends_with(b"_height") {
                    height = height.or(value);
                }
            }
            b"short" => wide = true,
            b"{" => {
                let (width, height) = (width.ok_or(FAIL)? as usize, height.ok_or(FAIL)? as usize);
                return decode_bits(&mut tokens, width, height, wide);
            }
            _ => {}
        }
    }
    Err(FAIL)
}

/// The values of the `_bits` array, after its opening brace.
fn decode_bits(
    tokens: &mut Tokens,
    width: usize,
    height: usize,
    wide: bool,
) -> Result<Image, DecodeError> {
    check_size(width, height)?;
    let unit = if wide { 2 } else { 1 };
    let row_len = width.div_ceil(8 * unit) * unit;
    let values = row_len / unit * height;
    // Every value takes at least two characters, a number and a separator.
    if values > tokens.data.len() {
        return Err(FAIL);
    }
    let mut bitmap = Vec::with_capacity(row_len * height);
    while bitmap.len() < row_len * height {
        let value = number(tokens.next().ok_or(FAIL)?).ok_or(FAIL)?;
        bitmap.push(value as u8);
        if wide {
            bitmap.push((value >> 8) as u8);
        }
        // A comma follows every value except possibly the last.
        match tokens.next() {
            Some(b",") => {}
            Some(b"}") if bitmap.len() == row_len * height => break,
            _ => return Err(FAIL),
        }
    }
    Image::from_bits(
        width as u32,
        height as u32,
        &bitmap,
        row_len,
        BitOrder::LsbFirst,
        [WHITE, BLACK],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leftmost_pixel_is_the_low_bit_and_set_is_black() {
        let image = decode_xbm(
            b"/* c */\n#define t_width 10\n#define t_height 2\n#define t_x_hot 1\n\
              static unsigned char t_bits[] = {\n 0x01, 0x02, // row 0\n 0x80, 0x00 };",
        )
        .unwrap();
        assert_eq!((image.width(), image.height()), (10, 2));
        assert_eq!((image.get(0, 0), image.get(1, 0)), (BLACK, WHITE));
        assert_eq!((image.get(9, 0), image.get(7, 1)), (BLACK, BLACK));
    }

    #[test]
    fn short_arrays_hold_sixteen_pixels_low_bit_first() {
        // A 17-pixel row takes two words, stored low byte first.
        let image = decode_xbm(
            b"#define t_width 17\n#define t_height 1\nstatic short t_bits[] = {\n\
              0x0201, 0x0001 };",
        )
        .unwrap();
        let set: Vec<u32> = (0..17).filter(|&x| image.get(x, 0) == BLACK).collect();
        assert_eq!(set, [0, 9, 16]);
    }

    #[test]
    fn rejects_other_text_and_short_data() {
        assert!(decode_xbm(b"static char x_bits[] = { 0 };").is_err());
        let few = b"#define t_width 16\n#define t_height 2\nstatic char t_bits[] = { 1, 2, 3 };";
        assert!(decode_xbm(few).is_err());
        let huge = b"#define t_width 60000\n#define t_height 60000\nstatic char t_bits[] = { 1 };";
        assert!(decode_xbm(huge).is_err());
        assert!(decode_xbm(b"#define t_width 8\n#define t_height 1\n").is_err());
    }
}
