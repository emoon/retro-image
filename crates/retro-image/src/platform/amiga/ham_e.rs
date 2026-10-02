//! HAM-E pictures (Black Belt Systems): 4-plane hires ILBMs read as 8-bit
//! pixels.
//!
//! The survey is `docs/research/amiga-apple-misc.md` ("Amiga HAM-E" and
//! "Wave 5"), whose public sources only outline the mode. The layout below
//! was reverse engineered from the samples and by black-box probing of
//! `recoil2png` with hand-built ILBMs, and verified pixel-for-pixel on every
//! corpus sample:
//!
//! - A "palette line" starts with a 16-pixel cookie: `a 2 f 5 8 4 d c 6 d b 0 7 f 1`
//!   and then a mode pixel, 4 for register mode or 8 for hold-and-modify.
//!   The rest of the line carries 64 colours: 192 bytes, one byte per pixel
//!   pair (high nibble first), as R, G, B. Colours load into 4 banks in turn,
//!   one bank per palette line, wrapping after 4; the palette lines in an
//!   interlaced picture appear twice, one per field, and each field keeps its
//!   own banks. A palette line shows as black.
//! - Every other line is data: pixel pairs form bytes. Register mode takes the
//!   byte as an index into the 256 colours of the 4 banks. Hold-and-modify
//!   takes the top 2 bits as a control and the low 6 as data, like HAM8:
//!   0 = colour `data` of bank 0, 1 = blue, 2 = red, 3 = green set to `data << 2`.
//!   Each line starts from black.
//! - The picture is half as wide as the bitmap; interlaced pictures are shown
//!   with the width doubled again.
//! - The Amiga palette (CMAP) in these files is a fixed table that lets the
//!   hardware read the pixel value back from the colour; it carries nothing
//!   we need.
//! - Only hires pictures without the HAM flag are taken for HAM-E.

use super::ilbm::{CAMG_HAM, CAMG_HIRES, CAMG_LACE, ham, read_ilbm};
use crate::{DecodeError, Image};
use alloc::vec::Vec;

/// The first 15 pixels of a palette line.
const COOKIE: [u32; 15] = [10, 2, 15, 5, 8, 4, 13, 12, 6, 13, 11, 0, 7, 15, 1];
const REGISTER: u32 = 4;
const MODIFY: u32 = 8;
const BANK_SIZE: usize = 64;
const BANKS: usize = 4;

/// How data lines are read.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Mode {
    Register,
    Modify,
}

/// Whether `row` is a palette line.
pub(super) fn is_palette_line(row: &[u32]) -> bool {
    palette_line_mode(row).is_some()
}

/// The mode a palette line selects, or `None` if `row` is not a palette line.
fn palette_line_mode(row: &[u32]) -> Option<Mode> {
    let (cookie, rest) = row.split_at_checked(COOKIE.len())?;
    if cookie != COOKIE {
        return None;
    }
    match *rest.first()? {
        REGISTER => Some(Mode::Register),
        MODIFY => Some(Mode::Modify),
        _ => None,
    }
}

/// The colours and mode a field has loaded so far.
struct Field {
    banks: [[u32; BANK_SIZE]; BANKS],
    loaded: usize,
    mode: Mode,
}

impl Field {
    fn load(&mut self, mode: Mode, row: &[u32]) {
        let nibble = |i: usize| row.get(i).copied().unwrap_or(0) as u8 & 15;
        let byte = |i: usize| u32::from(nibble(16 + 2 * i) << 4 | nibble(17 + 2 * i));
        let bank = &mut self.banks[self.loaded % BANKS];
        for (entry, color) in bank.iter_mut().enumerate() {
            *color = byte(3 * entry) << 16 | byte(3 * entry + 1) << 8 | byte(3 * entry + 2);
        }
        self.loaded += 1;
        self.mode = mode;
    }

    fn colors(&self, row: &[u32], mut put: impl FnMut(usize, u32)) {
        let mut held = 0;
        for (x, pair) in row.as_chunks::<2>().0.iter().enumerate() {
            let byte = pair[0] << 4 | pair[1] & 15;
            let (control, data) = (byte >> 6, byte & 63);
            let color = match self.mode {
                Mode::Register => self.banks[control as usize][data as usize],
                Mode::Modify => ham(held, control, data << 2, self.banks[0][data as usize]),
            };
            held = color;
            put(x, color);
        }
    }
}

pub(super) fn decode(contents: &[u8]) -> Result<Image, DecodeError> {
    let bitmap = read_ilbm(contents)?;
    let camg = bitmap.camg.unwrap_or(0);
    let header = &bitmap.header;
    let first = bitmap.row(0).ok_or(DecodeError::Unrecognized)?;
    let mode = palette_line_mode(first).ok_or(DecodeError::Unrecognized)?;
    if header.planes != 4 || camg & CAMG_HIRES == 0 || camg & CAMG_HAM != 0 || header.width < 2 {
        return Err(DecodeError::Unrecognized);
    }
    let lace = camg & CAMG_LACE != 0;
    let mut fields: Vec<Field> = (0..if lace { 2 } else { 1 })
        .map(|_| Field {
            banks: [[0; BANK_SIZE]; BANKS],
            loaded: 0,
            mode,
        })
        .collect();
    let mut image = Image::new(header.width as u32 / 2, header.height as u32);
    for y in 0..header.height {
        let row = bitmap.row(y).ok_or(DecodeError::Unrecognized)?;
        let count = fields.len();
        let field = &mut fields[y % count];
        match palette_line_mode(row) {
            Some(mode) => field.load(mode, row),
            None => field.colors(row, |x, color| image.set(x as u32, y as u32, color)),
        }
    }
    Ok(if lace { image.scaled(2, 1) } else { image })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_lines_need_the_cookie_and_a_mode_pixel() {
        let mut row = [COOKIE.as_slice(), &[MODIFY]].concat();
        assert_eq!(palette_line_mode(&row), Some(Mode::Modify));
        row[15] = REGISTER;
        assert_eq!(palette_line_mode(&row), Some(Mode::Register));
        row[15] = 5;
        assert_eq!(palette_line_mode(&row), None);
        row[15] = REGISTER;
        row[3] ^= 1;
        assert!(!is_palette_line(&row));
    }

    #[test]
    fn banks_load_in_turn_and_wrap() {
        let mut field = Field {
            banks: [[0; BANK_SIZE]; BANKS],
            loaded: 0,
            mode: Mode::Register,
        };
        for line in 0..5u32 {
            // First colour red = `line + 1`: bytes 0x0N in nibbles 16 and 17.
            let mut row = [COOKIE.as_slice(), &[REGISTER]].concat();
            row.resize(16 + 2 * 192, 0);
            row[17] = line + 1;
            field.load(Mode::Register, &row);
        }
        assert_eq!(field.banks[0][0], 5 << 16);
        assert_eq!(field.banks[1][0], 2 << 16);
        assert_eq!(field.banks[3][0], 4 << 16);
    }
}
