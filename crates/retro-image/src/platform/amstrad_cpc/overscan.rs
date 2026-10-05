//! Amstrad CPC overscan screens saved with a self-running loader inside
//! the picture: iMPdraw v2 and a second tool with the same layout.
//!
//! Sources:
//! - AMSDOS header (type at byte 18, load address at 21-22, entry at
//!   26-27): <https://cpctech.cpcwiki.de/docs/allhead.html>.
//! - Gate Array colours, firmware inks and the CRTC screen addressing:
//!   <https://cpctech.cpcwiki.de/docs/garray.html>,
//!   <https://cpctech.cpcwiki.de/docs/screen.html>. The ASIC palette word
//!   (low byte `R<<4 | B`, high byte `G`, 4 bits each) is the CPC Plus
//!   format from <https://www.cpcwiki.eu/index.php/Gate_Array>.
//! - Everything else is reverse engineered from the five samples
//!   (KDO2, PINUP2, DRAGON, HARLEY, RESET#30) by reading their loaders as
//!   data: the file is a memory image starting at the load address; the
//!   loader programs the CRTC for 96 bytes per line and 34 character rows
//!   starting at that address (the table in HARLEY and RESET#30 reads
//!   R1=0x30, R2=0x32, R6=0x22, R7=0x23, R12/R13=0x0d00), selects mode 0
//!   and sets the pens. A 96-byte line wraps inside its 2 KB block, and the
//!   rows after the first 2 KB continue 16 KB further on. RECOIL shows
//!   these files as Apple IIGS noise, so the layout was checked by eye.
//! - iMPdraw's screen mode is the number in its BASIC loader line
//!   `20 MODE n: CALL &01AD` (token `0xAD`, constants `0x0e`-`0x10` are
//!   0-2; the CPC BASIC token table is in the Locomotive BASIC manual).
//!   DRAGON.SCR says `MODE 2`, and only a mode 2 reading of it (768x272, two
//!   pens from 0x7f00) is a sharp picture of a dragon; modes 0 and 1 give
//!   dithered noise. Its flag byte 0x1ac is 1 with pixel data where the
//!   Plus palette would be, so that palette is used only when it is
//!   plausible (high bytes below 16).
//! - Palette sources: iMPdraw keeps 16 Gate Array values (`0x40 | colour`)
//!   at 0x7f00; when the byte at 0x1ac is 1 the loader instead copies 32
//!   bytes from 0x801 to the ASIC palette. The other tool keeps a mode
//!   byte at 0x800 and 16 firmware ink numbers after it.

use alloc::vec::Vec;

use super::amsdos::{amsdos_body, amsdos_extension};
use super::hardware::{Mode, firmware_color, hardware_color, render};
use crate::image::widen_channel;
use crate::{DecodeError, Image};

const LINE_BYTES: usize = 96;
const ROWS: usize = 34;
const LINES: usize = ROWS * 8;
/// Body lengths seen: iMPdraw (0x7e90) and the other tool (0x7cc0).
const IMPDRAW_LEN: usize = 0x7e90;
const OTHER_LEN: usize = 0x7cc0;
/// iMPdraw's BASIC loader: `10 ' iMP`.
const IMPDRAW_SIGNATURE: [u8; 10] = [0x0e, 0x00, 0x0a, 0x00, 0x01, 0xc0, 0x20, 0x69, 0x4d, 0x50];
/// Body offset of the mode constant in `20 MODE n`, after the line header
/// and the `MODE` token with its space.
const IMPDRAW_MODE_AT: usize = 0x14;
const IMPDRAW_MODE_PREFIX: [u8; 6] = [0x0d, 0x00, 0x14, 0x00, 0xad, 0x20];
const BASIC_ZERO: u8 = 0x0e;

/// Memory image of the file, addressed as on the CPC.
struct Memory<'a> {
    load: usize,
    bytes: &'a [u8],
}

impl Memory<'_> {
    fn get(&self, address: usize) -> u8 {
        address
            .checked_sub(self.load)
            .and_then(|i| self.bytes.get(i))
            .copied()
            .unwrap_or(0)
    }

    fn range(&self, address: usize, len: usize) -> Vec<u8> {
        (address..address + len).map(|a| self.get(a)).collect()
    }

    /// Byte `x` of line `y` of the overscan screen that starts at the load
    /// address.
    fn screen_byte(&self, x: usize, y: usize) -> u8 {
        let p = self.load + y / 8 * LINE_BYTES + x;
        self.get((p & 0x7ff) + (y & 7) * 0x800 + (p >> 11) * 0x4000)
    }
}

/// Colour of a CPC Plus palette word: 4 bits per channel.
fn plus_color(low: u8, high: u8) -> u32 {
    let scale = |n: u8| widen_channel(u32::from(n & 15), 4);
    scale(low >> 4) << 16 | scale(high) << 8 | scale(low)
}

/// Screen mode: iMPdraw's BASIC `MODE n`; the other tool's pictures are
/// all mode 0 (its mode byte at 0x800).
fn screen_mode(memory: &Memory, body: &[u8], impdraw: bool) -> Result<Mode, DecodeError> {
    let fail = DecodeError::Unrecognized;
    if !impdraw {
        return (memory.get(0x800) == 0).then_some(Mode::Zero).ok_or(fail);
    }
    if body.get(IMPDRAW_MODE_AT - IMPDRAW_MODE_PREFIX.len()..IMPDRAW_MODE_AT)
        != Some(&IMPDRAW_MODE_PREFIX)
    {
        return Err(fail);
    }
    let constant = body.get(IMPDRAW_MODE_AT).ok_or(fail)?;
    Mode::from_number(constant.wrapping_sub(BASIC_ZERO)).ok_or(fail)
}

/// Pens of the picture.
fn palette(memory: &Memory, impdraw: bool) -> Result<[u32; 16], DecodeError> {
    let mut pens = [0; 16];
    if impdraw {
        let words = memory.range(0x801, 32);
        let plus_palette = words.iter().skip(1).step_by(2).all(|&b| b < 16);
        let plus = memory.get(0x1ac) == 1;
        if plus && plus_palette {
            for (pen, word) in pens.iter_mut().zip(words.as_chunks::<2>().0) {
                *pen = plus_color(word[0], word[1]);
            }
        } else {
            for (i, pen) in pens.iter_mut().enumerate() {
                *pen = hardware_color(memory.get(0x7f00 + i));
            }
        }
    } else {
        for (i, pen) in pens.iter_mut().enumerate() {
            let ink = usize::from(memory.get(0x801 + i));
            if ink > 26 {
                return Err(DecodeError::Unrecognized);
            }
            *pen = firmware_color(ink);
        }
    }
    Ok(pens)
}

pub(super) fn decode_overscan(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let body = amsdos_body(data).ok_or(fail)?;
    if amsdos_extension(data) != Some(*b"SCR") {
        return Err(fail);
    }
    let field = |at: usize| usize::from(u16::from_le_bytes([data[at], data[at + 1]]));
    let (kind, load, entry) = (data[18], field(21), field(26));
    let impdraw = match (kind, load, entry, body.len()) {
        (0, 0x170, 0, IMPDRAW_LEN) if body.starts_with(&IMPDRAW_SIGNATURE) => true,
        (2, 0x200, 0x811, OTHER_LEN) => false,
        _ => return Err(fail),
    };
    let memory = Memory { load, bytes: body };
    let pens = palette(&memory, impdraw)?;
    let mode = screen_mode(&memory, body, impdraw)?;
    let lines: Vec<Vec<u8>> = (0..LINES)
        .map(|y| (0..LINE_BYTES).map(|x| memory.screen_byte(x, y)).collect())
        .collect();
    let width = LINE_BYTES * mode.pixels_per_byte();
    render(mode, width, LINES, |y| &lines[y], &pens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_step_2k_and_later_rows_continue_16k_on() {
        let mut bytes = alloc::vec![0u8; 0x8000];
        bytes[0x200 + 3 * 0x800 - 0x200] = 1;
        // Row 21 starts at p = 0x200 + 21 * 96 = 0x9e0, past the first 2 KB.
        bytes[0x41e0 - 0x200] = 2;
        let memory = Memory {
            load: 0x200,
            bytes: &bytes,
        };
        assert_eq!(memory.screen_byte(0, 3), 1);
        assert_eq!(memory.screen_byte(0, 21 * 8), 2);
    }

    #[test]
    fn plus_words_are_red_blue_then_green() {
        assert_eq!(plus_color(0xf0, 0x00), 0xff0000);
        assert_eq!(plus_color(0x0f, 0x00), 0x0000ff);
        assert_eq!(plus_color(0x00, 0x0f), 0x00ff00);
    }
}
