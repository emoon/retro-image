//! Koala MicroIllustrator PIC: compressed Graphics 15 picture.
//!
//! Source: "Koala picture files" by Jiri Bernasek (BEWESOFT),
//! <http://ftp.pigwa.net/stuff/collections/atari_forever/Tools%20-%20atr/KOALA%20PICTURE%20FILES.txt>:
//! magic `FF 80 C9 C7`, header length - 1 at offset 4, compression method at
//! 7, window (start X byte, end X byte, start Y, end Y) at 9-12, colours
//! 708-712 at 13-17. Method 0 stores the window raw, method 1 packed in
//! vertical order (even lines, then odd lines, column by column), method 2
//! packed in line order. Packed entries: bit 7 set = literal block, clear =
//! one repeated byte; the low 7 bits are the length, 0 meaning a 16-bit
//! big-endian length follows.
//!
//! Observed from `recoil2png` output: header byte 8 is the ANTIC mode;
//! 0x0E is Graphics 15, 0x0F is drawn as GTIA mode 9 with the background
//! register. Mode 2 (ANTIC text mode 2) stores screen codes for the
//! window, given in character rows (0, 40, 0, 24), drawn with the OS ROM
//! font: paper is playfield 2, ink is playfield 2's hue with playfield 1's
//! luminance; sample OPIS.PIC. Other modes are rejected here.
//!
//! Not marked as a signature format: Rambrandt (RM0-RM4) files start with
//! the same Koala header followed by extra colour data, so content
//! detection would misdecode them.

use super::antic::Bitmap;
use super::palette::register_rgb;
use super::screen::gtia9;
use super::text::mode2_colored;
use crate::{DecodeError, Image};
use alloc::vec::Vec;

const LINE: usize = 40;
const LINES: usize = 192;
const HEADER_MIN: usize = 18;
/// Graphics 15 (4 colours).
pub(super) const ANTIC_E: u8 = 0x0e;
/// Text mode 2 (Graphics 0): screen codes.
const ANTIC_2: u8 = 0x02;
/// Shown as GTIA mode 9 (16 luminances).
const ANTIC_F: u8 = 0x0f;

/// A decoded Koala file: the 40 x 192 byte screen, the ANTIC mode byte and
/// the colours 708-712 of the header.
pub(super) struct Pic {
    pub screen: [u8; LINE * LINES],
    pub mode: u8,
    pub colors: [u8; 5],
}

/// Reads the header and unpacks the screen.
pub(super) fn parse(data: &[u8]) -> Result<Pic, DecodeError> {
    if data.len() < HEADER_MIN || data[..4] != [0xff, 0x80, 0xc9, 0xc7] {
        return Err(DecodeError::Unrecognized);
    }
    let header_len = usize::from(u16::from_le_bytes([data[4], data[5]])) + 1;
    let body = data.get(header_len..).ok_or(DecodeError::Unrecognized)?;
    let window = Window::new(data[9], data[10], data[11], data[12])?;
    let mut screen = [0u8; LINE * LINES];
    let positions = window.positions(data[7] == 1);
    match data[7] {
        0 => {
            for (&byte, &position) in body.iter().zip(&positions) {
                screen[position] = byte;
            }
        }
        1 | 2 => unpack(body, &mut screen, &positions)?,
        _ => return Err(DecodeError::Unrecognized),
    }
    Ok(Pic {
        screen,
        mode: data[8],
        colors: [data[13], data[14], data[15], data[16], data[17]],
    })
}

pub(super) fn decode_pic(data: &[u8]) -> Result<Image, DecodeError> {
    let pic = parse(data)?;
    if pic.mode == ANTIC_2 {
        let [_, pf1, pf2, ..] = pic.colors;
        let paper = register_rgb(pf2);
        let ink = register_rgb(pf2 & 0xf0 | pf1 & 0x0f);
        return Ok(mode2_colored(&pic.screen[..40 * 24], 40, paper, ink));
    }
    let bitmap = Bitmap {
        data: &pic.screen,
        bytes_per_line: LINE,
        lines: LINES,
        bits: 2,
    };
    let background = pic.colors[4];
    match pic.mode {
        ANTIC_E => {
            let colors = [background, pic.colors[0], pic.colors[1], pic.colors[2]];
            bitmap.render(2, 1, |_, value| register_rgb(colors[usize::from(value)]))
        }
        ANTIC_F => gtia9(bitmap, background),
        _ => Err(DecodeError::Unrecognized),
    }
}

/// The area of screen memory the picture covers, in bytes and lines.
struct Window {
    left: usize,
    right: usize,
    top: usize,
    bottom: usize,
}

impl Window {
    fn new(left: u8, right: u8, top: u8, bottom: u8) -> Result<Self, DecodeError> {
        let window = Self {
            left: left.into(),
            right: right.into(),
            top: top.into(),
            bottom: bottom.into(),
        };
        if window.left < window.right
            && window.right <= LINE
            && window.top < window.bottom
            && window.bottom <= LINES
        {
            Ok(window)
        } else {
            Err(DecodeError::Unrecognized)
        }
    }

    /// Screen offsets in storage order.
    fn positions(&self, vertical: bool) -> Vec<usize> {
        let mut positions = Vec::new();
        if vertical {
            for x in self.left..self.right {
                let even = (self.top..self.bottom).step_by(2);
                let odd = (self.top + 1..self.bottom).step_by(2);
                positions.extend(even.chain(odd).map(|y| y * LINE + x));
            }
        } else {
            for y in self.top..self.bottom {
                positions.extend((self.left..self.right).map(|x| y * LINE + x));
            }
        }
        positions
    }
}

/// Unpacks run-length entries until every position is filled.
fn unpack(mut packed: &[u8], screen: &mut [u8], positions: &[usize]) -> Result<(), DecodeError> {
    let mut next = || -> Result<u8, DecodeError> {
        let (&byte, rest) = packed.split_first().ok_or(DecodeError::Unrecognized)?;
        packed = rest;
        Ok(byte)
    };
    let mut pending = positions.iter().peekable();
    while pending.peek().is_some() {
        let control = next()?;
        let mut len = usize::from(control & 0x7f);
        if len == 0 {
            len = usize::from(next()?) << 8 | usize::from(next()?);
        }
        let repeated = if control & 0x80 == 0 {
            Some(next()?)
        } else {
            None
        };
        for _ in 0..len {
            let byte = match repeated {
                Some(byte) => byte,
                None => next()?,
            };
            match pending.next() {
                Some(&position) => screen[position] = byte,
                None => return Ok(()),
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(method: u8) -> Vec<u8> {
        let mut data = alloc::vec![0xff, 0x80, 0xc9, 0xc7, 0x1a, 0, 1, method, 0x0e];
        data.extend_from_slice(&[0, 40, 0, 192, 0x28, 0xca, 0x94, 0x46, 0x00]);
        data.resize(27, 0);
        data
    }

    #[test]
    fn unpacks_vertical_order() {
        let mut data = header(1);
        data.extend_from_slice(&[0x81, 0xff]); // 1 literal byte
        data.extend_from_slice(&[0x00, 0x1d, 0xff, 0x00]); // 7679 zeros
        let image = decode_pic(&data).unwrap();
        // The first byte goes to line 0, column 0: four pixels of playfield 2.
        assert_eq!(&image.rgb()[..3], &[0x00, 0x63, 0x74]);
        // The second goes to line 2, not line 1.
        let line1 = 320 * 3;
        assert_eq!(&image.rgb()[line1..line1 + 3], &[0, 0, 0]);
    }

    #[test]
    fn rejects_truncated_data() {
        let mut data = header(2);
        data.extend_from_slice(&[0x05, 0xaa]);
        assert!(decode_pic(&data).is_err());
        assert!(decode_pic(&data[..10]).is_err());
    }
}
