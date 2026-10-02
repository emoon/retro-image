//! Apple II "Sprites": a text file of 1-bit sprites placed on a 320x200 screen.
//!
//! No documentation was found (see `docs/research/amiga-apple-misc.md`,
//! "Wave 5"). Reverse engineered from `corpus/test.spr` and by black-box
//! probing of `recoil2png` with hand-written files:
//!
//! - The file is whitespace-separated numbers, decimal or `$` followed by hex.
//!   A sprite is `width height kind x y` followed by `width * height` data
//!   bytes, column by column. Width counts bytes of 8 pixels, most significant
//!   bit leftmost; `x` and `y` are in pixels. `kind` is ignored.
//! - A record with height 0 ends the file; anything after it is ignored. A
//!   file without one is rejected, as is a sprite with width 0 or one that
//!   does not fit on the screen.
//! - Sprites are ORed onto a black screen, set bits white.
//! - Every number must be below 320; data bytes keep their low 8 bits.

use crate::{DecodeError, Image};

const WIDTH: usize = 320;
const HEIGHT: usize = 200;
const WHITE: u32 = 0xff_ffff;

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    let mut numbers = data
        .split(|b| matches!(b, b' ' | b'\t' | b'\r' | b'\n'))
        .filter(|token| !token.is_empty())
        .map(number);
    let mut next = move || numbers.next().ok_or(DecodeError::Unrecognized)?;
    let mut image = Image::new(WIDTH as u32, HEIGHT as u32);
    loop {
        let (width, height, _kind) = (next()?, next()?, next()?);
        let (x, y) = (next()?, next()?);
        if height == 0 {
            return Ok(image);
        }
        if width == 0 || x + 8 * width > WIDTH || y + height > HEIGHT {
            return Err(DecodeError::Unrecognized);
        }
        for column in 0..width {
            for row in y..y + height {
                let byte = next()? & 0xff;
                for bit in (0..8).filter(|bit| byte >> (7 - bit) & 1 != 0) {
                    image.set((x + column * 8 + bit) as u32, row as u32, WHITE);
                }
            }
        }
    }
}

/// A decimal number or `$` and hex digits, below 320.
fn number(token: &[u8]) -> Result<usize, DecodeError> {
    let (digits, radix) = match token.strip_prefix(b"$") {
        Some(hex) => (hex, 16),
        None => (token, 10),
    };
    if digits.is_empty() {
        return Err(DecodeError::Unrecognized);
    }
    digits.iter().try_fold(0usize, |value, &digit| {
        let digit = char::from(digit)
            .to_digit(radix)
            .ok_or(DecodeError::Unrecognized)?;
        Some(value * radix as usize + digit as usize)
            .filter(|&value| value < WIDTH)
            .ok_or(DecodeError::Unrecognized)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn places_and_ors_sprites() {
        let image = decode(b"1 2 2 10 5\r\n$80\r\n65\r\n1 1 2 10 5 $01 0 0 0 0 0").unwrap();
        let white = |x, y| image.get(x, y) == WHITE;
        assert!(white(10, 5) && white(17, 5));
        assert!(white(11, 6) && white(17, 6) && !white(10, 6));
    }

    #[test]
    fn data_runs_down_each_byte_column() {
        let image = decode(b"2 2 2 0 0 $80 $40 $20 $10 0 0 0 0 0").unwrap();
        let white = |x, y| image.get(x, y) == WHITE;
        assert!(white(0, 0) && white(10, 0) && white(1, 1) && white(11, 1));
    }

    #[test]
    fn rejects_malformed_files() {
        assert!(decode(b"1 2 2 0 0 128 65").is_err(), "no terminator");
        assert!(
            decode(b"1 1 2 313 0 1 0 0 0 0 0").is_err(),
            "off the screen"
        );
        assert!(
            decode(b"1 1 2 0 0 400 0 0 0 0 0").is_err(),
            "number too big"
        );
        assert!(decode(b"0 1 2 0 0 0 0 0 0 0").is_err(), "zero width");
        assert!(decode(b"1 1 2 0 0 0xff 0 0 0 0 0").is_err(), "not a number");
    }
}
