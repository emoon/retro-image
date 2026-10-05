//! PICO-8 text cartridges (`.p8`), shown as their label or sprite sheet.
//!
//! Sources:
//! - The `.p8` layout: <https://github.com/dansanderson/picotool> (MIT,
//!   copyright 2015 Dan Sanderson), `pico8/game/formatter/p8.py` and
//!   `pico8/gfx/gfx.py`, read for the layout only and not copied: the first
//!   line `pico-8 cartridge // http://www.pico-8.com`, a line `version N`,
//!   then sections that start with a line `__name__`. `__gfx__` is 128
//!   lines of 128 hex digits, one pixel each, left to right, and
//!   `__label__` has the same shape.
//! - The 16 colors: the PICO-8 palette as listed by Lospec,
//!   <https://lospec.com/palette-list/pico-8>.
//! - Real samples: the five `.p8` files in `tests/testdata` of picotool, in
//!   `corpus/extra/small-consoles/pico8`. They are small test carts: four
//!   have a blank sprite sheet and one (`test_cart_with_label.p8`) has a
//!   label and a few sprite rows.
//!
//! The picture is the label (the cover of the cartridge) unless it is all
//! zero, else the sprite sheet, 128 x 128 pixels either way. A section may be
//! cut short at its end: missing rows and missing digits at the end of a row
//! are zero. A cartridge with neither section is rejected, and the other
//! sections are ignored. `.p8.png` cartridges hide their data in the low bits
//! of a PNG and need a PNG decoder, which this crate does not have: they are
//! not read.
//!
//! The 16 screen colors are used as they are. Pixels are hex digits, as in
//! every sample. Newer PICO-8 versions may write other digits (`g` to `v`,
//! for the "secret" colors 128 to 143) in a label; no sample has any, so that
//! is unverified: a label with a digit that is not hex is left out and the
//! sprite sheet is shown, while a sprite sheet with one is rejected.
//!
//! Detection: the first line is a signature, so `.signature()` is chained.

use crate::{DecodeError, Format, Image};

pub(super) static FORMATS: &[Format] =
    &[Format::new("PICO-8", "Cartridge", &["p8"], decode).signature()];

const HEADER: &[u8] = b"pico-8 cartridge // http://www.pico-8.com";
const SIZE: usize = 128;

#[rustfmt::skip]
const PALETTE: [u32; 16] = [
    0x000000, 0x1d2b53, 0x7e2553, 0x008751, 0xab5236, 0x5f574f, 0xc2c3c7, 0xfff1e8,
    0xff004d, 0xffa300, 0xffec27, 0x00e436, 0x29adff, 0x83769c, 0xff77a8, 0xffccaa,
];

/// A section of pixels: a color number for each of 128 x 128 pixels.
struct Section {
    pixels: [u8; SIZE * SIZE],
    rows: usize,
}

impl Section {
    fn new() -> Self {
        Self {
            pixels: [0; SIZE * SIZE],
            rows: 0,
        }
    }

    /// Adds a row of hex digits; `None` if there is no room or a digit is wrong.
    fn push(&mut self, line: &[u8]) -> Option<()> {
        if self.rows == SIZE || line.len() > SIZE {
            return None;
        }
        let row = &mut self.pixels[self.rows * SIZE..][..SIZE];
        for (pixel, &digit) in row.iter_mut().zip(line) {
            *pixel = (digit as char).to_digit(16)? as u8;
        }
        self.rows += 1;
        Some(())
    }

    fn is_blank(&self) -> bool {
        self.pixels.iter().all(|&p| p == 0)
    }
}

/// Which section the lines being read belong to.
enum Reading {
    Gfx,
    Label,
    Other,
}

fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let mut lines = data
        .split(|&b| b == b'\n')
        .map(|line| line.strip_suffix(b"\r").unwrap_or(line));
    if lines.next() != Some(HEADER) {
        return Err(fail);
    }
    let version = lines.next().and_then(|line| line.strip_prefix(b"version "));
    if !version.is_some_and(|v| !v.is_empty() && v.iter().all(u8::is_ascii_digit)) {
        return Err(fail);
    }
    let (mut gfx, mut label) = (None, None);
    let mut reading = Reading::Other;
    for line in lines {
        if let Some(name) = line.strip_prefix(b"__").and_then(|l| l.strip_suffix(b"__")) {
            reading = match name {
                b"gfx" => Reading::Gfx,
                b"label" => Reading::Label,
                _ => Reading::Other,
            };
            match reading {
                Reading::Gfx => gfx = Some(Section::new()),
                Reading::Label => label = Some(Section::new()),
                Reading::Other => {}
            }
        } else if !line.is_empty() {
            match reading {
                Reading::Gfx => {
                    if let Some(gfx) = gfx.as_mut() {
                        gfx.push(line).ok_or(fail)?;
                    }
                }
                Reading::Label => {
                    // A label that cannot be read is left out, not a reason
                    // to reject the cartridge.
                    if label.as_mut().is_some_and(|l| l.push(line).is_none()) {
                        label = None;
                        reading = Reading::Other;
                    }
                }
                Reading::Other => {}
            }
        }
    }
    let shown = label.filter(|l| !l.is_blank()).or(gfx).ok_or(fail)?;
    Image::from_indexed(SIZE as u32, SIZE as u32, &shown.pixels, &PALETTE)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cart(sections: &str) -> alloc::vec::Vec<u8> {
        alloc::format!(
            "{}\nversion 8\n__lua__\nprint(1)\n{sections}",
            core::str::from_utf8(HEADER).unwrap()
        )
        .into_bytes()
    }

    #[test]
    fn digits_are_pixels_left_to_right_and_short_rows_are_zero_filled() {
        let image = decode(&cart("__gfx__\n0f1\n\n2\n__gff__\nzz\n")).unwrap();
        assert_eq!((image.width(), image.height()), (128, 128));
        assert_eq!(image.get(1, 0), PALETTE[15]);
        assert_eq!(image.get(2, 0), PALETTE[1]);
        assert_eq!(image.get(3, 0), PALETTE[0]);
        assert_eq!(image.get(0, 1), PALETTE[2]);
    }

    #[test]
    fn a_label_wins_unless_it_is_blank() {
        let with_label = cart("__gfx__\n1\n__label__\n0\n3\n");
        assert_eq!(decode(&with_label).unwrap().get(0, 1), PALETTE[3]);
        assert_eq!(
            decode(&cart("__gfx__\n1\n__label__\n0\n"))
                .unwrap()
                .get(0, 0),
            PALETTE[1]
        );
        assert!(decode(&cart("__map__\n00\n")).is_err());
    }

    #[test]
    fn a_label_with_other_digits_is_left_out() {
        let cart = cart("__gfx__\n1\n__label__\n0\n3g\n__map__\n00\n");
        // The sprite sheet is shown, and the lines after the label are not
        // mistaken for its rows.
        assert_eq!(decode(&cart).unwrap().get(0, 0), PALETTE[1]);
        assert_eq!(decode(&cart).unwrap().get(0, 1), PALETTE[0]);
    }

    #[test]
    fn the_header_and_the_digits_are_checked() {
        let mut bad_header = cart("__gfx__\n1\n");
        bad_header[0] = b'P';
        assert!(decode(&bad_header).is_err());
        assert!(decode(&cart("__gfx__\n1g\n")).is_err());
        let long_row = alloc::format!("__gfx__\n{}\n", "1".repeat(129));
        assert!(decode(&cart(&long_row)).is_err());
        let too_many = alloc::format!("__gfx__\n{}", "1\n".repeat(129));
        assert!(decode(&cart(&too_many)).is_err());
    }
}
