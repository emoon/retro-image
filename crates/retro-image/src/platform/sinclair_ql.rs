//! Sinclair QL screens and pointer-environment area saves.
//!
//! Sources:
//! - Screen memory layout (128 bytes per line from $20000; per 16-bit
//!   word, the high byte at the even address holds G7-G0 in 512-pixel
//!   mode 4 and G3 F3 G2 F2 G1 F1 G0 F0 in 256-pixel mode 8, the low byte
//!   R7-R0 or R3 B3 R2 B2 R1 B1 R0 B0; bit 7 is the leftmost pixel; the
//!   flash bit toggles flashing): T. Tebby and D. Karlin, "Sinclair QL
//!   Software Developer's Guide" (1984), section 10.2, "Display Control",
//!   <https://www.sinclairql.net/downloads/1984-00_Sinclair_QL_Software_Developers_Guide_by_Tony_Tebby_and_David_Karlin-OCRed-SQPP.pdf>.
//! - Mode 4 colours black, red, green, white; mode 8 colours from the R, G,
//!   B bits; 32 768-byte files as 512x256 screens with no mode stored; PIC
//!   area saves (`$4AFC`, width in 512-pixel coordinates, height, line
//!   increment, mode 0/4 or 8, spare byte, then the lines) and PSA files
//!   (the same after 4 more bytes): Dilwyn Jones, "QL Graphics File
//!   Formats" (`graphics.txt`) and "Partial Screen Area Saves, or PIC/PSA
//!   Files" (`pics.txt`), from his QL pages' file formats section, read on
//!   the sinclairql.net mirror:
//!   <https://www.sinclairql.net/djw/docs/formats/graphics.zip>,
//!   <https://www.sinclairql.net/djw/docs/formats/pics.zip>.
//! - `.QS4` / `.QS8` for 32 KB mode 4 / mode 8 screens on PCs: C. Delhez,
//!   ShowQS (1992) documentation, `SHOWQS.TXT` in
//!   <https://www.sinclairql.net/djw/graphics/showqs.zip>.
//! - Padding after the last line of PIC files: observed in the QDesign clip
//!   art (`QDesign_ClipArt.zip` on the same mirror).
//! - Pixel shape (rows doubled so a 512x256 screen shows as 512x512 at the
//!   QL's 4:3 display aspect): from the 512x256 resolution on a 4:3 TV, as
//!   for other 2:1 screens in this crate.

use crate::bytes::be16;
use crate::{DecodeError, Format, Image};

const PLATFORM: &str = "Sinclair QL";

pub(super) static FORMATS: &[Format] = &[
    Format::new(PLATFORM, "Mode 4 screen", &["qs4"], |data| {
        decode_screen(data, Mode::Four)
    }),
    Format::new(PLATFORM, "Mode 8 screen", &["qs8"], |data| {
        decode_screen(data, Mode::Eight)
    }),
    Format::new(PLATFORM, "PIC area save", &["pic"], decode_pic).signature(),
    Format::new(PLATFORM, "PSA area save", &["psa"], decode_psa).signature(),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// 4 colours, 8 pixels per 16-bit word.
    Four,
    /// 8 colours, 4 pixels per word, each two mode 4 pixels wide.
    Eight,
}

/// Colours by G, R, B bits. Mode 4 has only black, red, green and white.
const COLORS: [u32; 8] = [
    0x000000, 0x0000ff, 0xff0000, 0xff00ff, 0x00ff00, 0x00ffff, 0xffff00, 0xffffff,
];

/// The colour at mode 4 pixel column `x` (0-7) of a screen word
/// (`even`, `odd` bytes). In mode 8, columns `2n` and `2n + 1` are pixel n.
fn pixel(mode: Mode, even: u8, odd: u8, x: usize) -> u32 {
    let index = match mode {
        Mode::Four => {
            let shift = 7 - x;
            match (even >> shift & 1, odd >> shift & 1) {
                (0, 0) => 0, // black
                (0, _) => 2, // red
                (_, 0) => 4, // green
                _ => 7,      // white
            }
        }
        Mode::Eight => {
            // Flash bits (even byte, odd positions) are ignored: pixels are
            // shown in their own colour, the steady phase of flashing.
            let shift = 7 - x / 2 * 2;
            (even >> shift & 1) << 2 | (odd >> shift & 1) << 1 | (odd >> (shift - 1) & 1)
        }
    };
    COLORS[usize::from(index)]
}

/// Renders `height` lines, `stride` bytes apart, of a picture `width` mode 4
/// pixels wide, with rows doubled for the display's pixel shape.
fn render(
    data: &[u8],
    mode: Mode,
    width: usize,
    height: usize,
    stride: usize,
) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let line_len = width.div_ceil(8) * 2;
    if width == 0 || height == 0 || stride < line_len {
        return Err(fail);
    }
    let needed = stride
        .checked_mul(height - 1)
        .and_then(|n| n.checked_add(line_len))
        .ok_or(fail)?;
    if data.len() < needed {
        return Err(fail);
    }
    let mut image = Image::new(width as u32, height as u32);
    for y in 0..height {
        let line = &data[y * stride..][..line_len];
        for x in 0..width {
            let word = &line[x / 8 * 2..];
            image.set(x as u32, y as u32, pixel(mode, word[0], word[1], x % 8));
        }
    }
    Ok(image.scaled(1, 2))
}

/// Bytes per line of the QL screen.
const LINE_BYTES: usize = 128;
/// The screen is 512 mode 4 pixels wide: the coordinate system of both modes.
const SCREEN_WIDTH: usize = 512;
const SCREEN_HEIGHT: usize = 256;

/// A 32 KB screen memory dump. Its mode is not stored: ShowQS's `.QS4` and
/// `.QS8` extensions name it.
fn decode_screen(data: &[u8], mode: Mode) -> Result<Image, DecodeError> {
    if data.len() != LINE_BYTES * SCREEN_HEIGHT {
        return Err(DecodeError::Unrecognized);
    }
    render(data, mode, SCREEN_WIDTH, SCREEN_HEIGHT, LINE_BYTES)
}

/// PIC area save: a 10-byte header, then the lines. The header is
/// validated strictly (flag, mode, a line increment that holds the width,
/// a file that ends with the last line, give or take a little zero
/// padding), so PIC files are also recognised by content.
fn decode_pic(data: &[u8]) -> Result<Image, DecodeError> {
    decode_area(data)
}

/// PSA area save: 4 undefined bytes, then a PIC file.
fn decode_psa(data: &[u8]) -> Result<Image, DecodeError> {
    decode_area(data.get(4..).ok_or(DecodeError::Unrecognized)?)
}

fn decode_area(data: &[u8]) -> Result<Image, DecodeError> {
    const HEADER_LEN: usize = 10;
    const MAX_PADDING: usize = 7;
    let fail = DecodeError::Unrecognized;
    let word = |at| be16(data, at).map(usize::from).ok_or(fail);
    if word(0)? != 0x4afc || data.get(9) != Some(&0) {
        return Err(fail);
    }
    let (width, height, stride) = (word(2)?, word(4)?, word(6)?);
    let mode = match data[8] {
        0 | 4 => Mode::Four,
        8 => Mode::Eight,
        _ => return Err(fail),
    };
    let end = stride
        .checked_mul(height)
        .and_then(|n| n.checked_add(HEADER_LEN))
        .ok_or(fail)?;
    // QDesign clip art (`_cut` files) ends with 2-6 zero bytes of padding.
    let padding = data.get(end..).ok_or(fail)?;
    if padding.len() > MAX_PADDING || padding.iter().any(|&b| b != 0) {
        return Err(fail);
    }
    render(&data[HEADER_LEN..], mode, width, height, stride)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_4_words_hold_green_then_red_bits() {
        // Pixel 0: green + red = white; pixel 1: green; pixel 7: red.
        let (even, odd) = (0b1100_0000, 0b1000_0001);
        assert_eq!(pixel(Mode::Four, even, odd, 0), 0xffffff);
        assert_eq!(pixel(Mode::Four, even, odd, 1), 0x00ff00);
        assert_eq!(pixel(Mode::Four, even, odd, 7), 0xff0000);
        assert_eq!(pixel(Mode::Four, even, odd, 3), 0x000000);
    }

    #[test]
    fn mode_8_words_hold_green_flash_and_red_blue_pairs() {
        // Pixel 0: G, R, B = white; pixel 1: flash only (black);
        // pixel 2: blue; pixel 3: green + red = yellow.
        let even = 0b10_01_00_10;
        let odd = 0b11_00_01_10;
        let colors: [u32; 8] = core::array::from_fn(|x| pixel(Mode::Eight, even, odd, x));
        assert_eq!(
            colors,
            [
                0xffffff, 0xffffff, 0, 0, 0x0000ff, 0x0000ff, 0xffff00, 0xffff00
            ]
        );
    }

    #[test]
    fn render_doubles_rows_and_honours_width_and_stride() {
        // Two lines of 2 words, 6 bytes apart; 12 pixels wide.
        let data = [0xff, 0x00, 0x00, 0xff, 9, 9, 0x00, 0xff, 0xff, 0xff];
        let image = render(&data, Mode::Four, 12, 2, 6).unwrap();
        assert_eq!((image.width(), image.height()), (12, 4));
        assert_eq!((image.get(0, 1), image.get(8, 0)), (0x00ff00, 0xff0000));
        assert_eq!((image.get(0, 2), image.get(11, 3)), (0xff0000, 0xffffff));
        assert!(render(&data, Mode::Four, 12, 3, 6).is_err());
        assert!(
            render(&data, Mode::Four, 17, 1, 2).is_err(),
            "stride too short"
        );
    }

    #[test]
    fn screens_are_32k_with_128_byte_lines() {
        let mut screen = alloc::vec![0; 32768];
        screen[128] = 0x80; // line 1, mode 8 pixel 0 green
        let image = decode_screen(&screen, Mode::Eight).unwrap();
        assert_eq!((image.width(), image.height()), (512, 512));
        assert_eq!((image.get(1, 2), image.get(1, 3)), (0x00ff00, 0x00ff00));
        assert_eq!(image.get(2, 2), 0);
        assert!(decode_screen(&screen[1..], Mode::Four).is_err());
    }

    #[test]
    fn pic_header_is_validated() {
        let mut pic = alloc::vec![0x4a, 0xfc, 0, 16, 0, 1, 0, 4, 8, 0];
        pic.extend_from_slice(&[0, 0b1000_0000, 0, 0b0100_0000]);
        let image = decode_pic(&pic).unwrap();
        assert_eq!((image.width(), image.height()), (16, 2));
        assert_eq!(image.get(1, 0), 0xff0000, "mode 8 pixel 0: red");
        assert_eq!(image.get(8, 1), 0x0000ff, "mode 8 pixel 4: blue");
        let mut psa = alloc::vec![1, 2, 3, 4];
        psa.extend_from_slice(&pic);
        assert_eq!(decode_psa(&psa), Ok(image));
        let mut bad_mode = pic.clone();
        bad_mode[8] = 16;
        assert!(decode_pic(&bad_mode).is_err());
        assert!(decode_pic(&pic[..pic.len() - 1]).is_err());
        let mut padded = pic.clone();
        padded.extend_from_slice(&[0; 6]);
        assert!(decode_pic(&padded).is_ok(), "zero padding");
        padded.extend_from_slice(&[0; 2]);
        assert!(decode_pic(&padded).is_err(), "too much padding");
        let mut trailing = pic.clone();
        trailing.push(1);
        assert!(decode_pic(&trailing).is_err());
    }
}
