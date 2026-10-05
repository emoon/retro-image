//! BBC Micro screen memory dumps and LdPic pictures.
//!
//! Sources:
//! - Screen modes, memory sizes, character-cell layout (8 bytes per cell,
//!   cells left to right, then the next character row) and pixel packing:
//!   dfstudios, <https://www.dfstudios.co.uk/articles/retro-computing/bbc-micro-screen-formats/>.
//! - Default logical colors (2: black/white; 4: black/red/yellow/white;
//!   16: the 8 physical colors, then flashing ones): the same article.
//! - LdPic header, bit order, run-length coding and address stepping:
//!   <https://nerdoftheherd.com/projects/libbeebimage/ldpic/> (prose spec).
//! - Mode 7 teletext (raw screens, TTI, EP1): see `teletext.rs`.
//! - Pixel aspect (mode 0 rows doubled to 640x512, 160-pixel modes doubled
//!   horizontally to 320x256): observed from `recoil2png` output.

use crate::{DecodeError, Format, Image};

mod flf;
mod teletext;

pub(super) static FORMATS: &[Format] = &[
    Format::new("BBC Micro", "Mode 0 screen", &["bb0"], |data| {
        decode(data, &MODE0)
    }),
    Format::new("BBC Micro", "Mode 1 screen", &["bb1"], |data| {
        decode(data, &MODE1)
    }),
    Format::new("BBC Micro", "Mode 2 screen", &["bb2"], |data| {
        decode(data, &MODE2)
    }),
    Format::new("BBC Micro", "Mode 4 screen", &["bb4"], |data| {
        decode(data, &MODE4)
    }),
    Format::new("BBC Micro", "Mode 5 screen", &["bb5"], |data| {
        decode(data, &MODE5)
    }),
    Format::new("BBC Micro", "LdPic", &["bbg"], decode_ldpic).signature(),
    Format::new(
        "BBC Micro",
        "Turbo Rascal Syntax Error",
        &["flf"],
        flf::decode_flf,
    )
    .signature(),
    Format::new(
        "BBC Micro",
        "Mode 7 screen",
        &["bb7", "m7", "mode7"],
        teletext::decode_raw,
    ),
    Format::new(
        "BBC Micro",
        "Teletext page (TTI)",
        &["tti"],
        teletext::decode_tti,
    )
    .signature(),
    Format::new(
        "BBC Micro",
        "Teletext page (EP1)",
        &["ep1"],
        teletext::decode_ep1,
    )
    .signature(),
];

struct Mode {
    /// Default physical color of each logical color.
    palette: [u8; 16],
    /// Bytes per character row (80 cells of 8 bytes, or 40).
    row_bytes: usize,
    bits_per_pixel: usize,
    /// Output scale of each pixel (width, height).
    scale: (u32, u32),
}

impl Mode {
    fn screen_len(&self) -> usize {
        self.row_bytes * ROWS
    }
}

const TWO_COLORS: [u8; 16] = [0, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
const FOUR_COLORS: [u8; 16] = [0, 1, 3, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
const SIXTEEN_COLORS: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

const MODE0: Mode = Mode {
    palette: TWO_COLORS,
    row_bytes: 640,
    bits_per_pixel: 1,
    scale: (1, 2),
};
const MODE1: Mode = Mode {
    palette: FOUR_COLORS,
    row_bytes: 640,
    bits_per_pixel: 2,
    scale: (1, 1),
};
const MODE2: Mode = Mode {
    palette: SIXTEEN_COLORS,
    row_bytes: 640,
    bits_per_pixel: 4,
    scale: (2, 1),
};
const MODE4: Mode = Mode {
    palette: TWO_COLORS,
    row_bytes: 320,
    bits_per_pixel: 1,
    scale: (1, 1),
};
const MODE5: Mode = Mode {
    palette: FOUR_COLORS,
    row_bytes: 320,
    bits_per_pixel: 2,
    scale: (2, 1),
};

const ROWS: usize = 32;
const HEIGHT: usize = ROWS * 8;

fn decode(data: &[u8], mode: &Mode) -> Result<Image, DecodeError> {
    if data.len() != mode.screen_len() {
        return Err(DecodeError::Unrecognized);
    }
    render(data, mode, &mode.palette)
}

/// Renders screen memory with a logical-to-physical color mapping.
fn render(data: &[u8], mode: &Mode, palette: &[u8; 16]) -> Result<Image, DecodeError> {
    let pixels_per_byte = 8 / mode.bits_per_pixel;
    let width = mode.row_bytes / 8 * pixels_per_byte;
    let mut image = Image::new(width as u32, HEIGHT as u32);
    for y in 0..HEIGHT {
        for x in 0..width {
            let byte = data[y / 8 * mode.row_bytes + x / pixels_per_byte * 8 + y % 8];
            let logical = pixel(byte, x % pixels_per_byte, mode.bits_per_pixel);
            let color = physical_color(palette[usize::from(logical)]);
            image.set(x as u32, y as u32, color);
        }
    }
    let (sx, sy) = mode.scale;
    image.scaled(sx, sy)
}

/// Logical color of pixel `n` in a byte: its bits are interleaved, with
/// pixel 0 in the top bit of each group (bits 7, 5, 3, 1 for 4 bpp; bits
/// 7, 3 for 2 bpp; bit 7 for 1 bpp).
fn pixel(byte: u8, n: usize, bits_per_pixel: usize) -> u8 {
    let pixels = 8 / bits_per_pixel;
    (0..bits_per_pixel).fold(0, |value, bit| {
        let position = 7 - n - bit * pixels;
        value << 1 | (byte >> position) & 1
    })
}

/// Physical color bits: 0 red, 1 green, 2 blue. Colors 8-15 flash and
/// are shown in their first phase, color & 7.
fn physical_color(physical: u8) -> u32 {
    let channel = |bit: u8| if physical & bit != 0 { 0xff } else { 0 };
    channel(1) << 16 | channel(2) << 8 | channel(4)
}

/// LdPic (Acorn User, 1986): a bit stream whose fields are read MSB first
/// from each byte but have their bits reversed. Header: bits per stored
/// value (A, 8 bits), mode (8), 16 x 4-bit logical-to-physical colors from
/// color 15 down (64), address step (D, 8) and repeat-count width (E, 8).
/// Then runs: flag 1 = E-bit count and A-bit value, flag 0 = one A-bit
/// value. Values are written from screen offset D - 1, stepping by D; when
/// the address leaves the screen, writing restarts one byte lower (D - 2,
/// ...) with the same step, until offset 0 has been done. The spec's
/// wording suggests the step shrinks too, but the sample only decodes
/// fully with a constant step.
///
/// The stream must end in the file's last byte, as in every sample: with
/// the header checks, this makes random data fail, so LdPic is also
/// recognised by content.
fn decode_ldpic(data: &[u8]) -> Result<Image, DecodeError> {
    let mut bits = BitReader { data, position: 0 };
    let value_bits = bits.read(8)?;
    let mode = match bits.read(8)? {
        0 => &MODE0,
        1 => &MODE1,
        2 => &MODE2,
        4 => &MODE4,
        5 => &MODE5,
        _ => return Err(DecodeError::Unrecognized),
    };
    let mut palette = [0; 16];
    for logical in (0..16).rev() {
        palette[logical] = bits.read(4)? as u8;
    }
    let step = bits.read(8)? as usize;
    let count_bits = bits.read(8)?;
    if value_bits == 0 || value_bits > 8 || count_bits > 16 || step == 0 {
        return Err(DecodeError::Unrecognized);
    }
    let mut screen = alloc::vec![0u8; mode.screen_len()];
    let mut passes = step;
    let mut address = step - 1;
    while passes > 0 {
        let count = if bits.read(1)? == 1 {
            bits.read(count_bits)?
        } else {
            1
        };
        let value = bits.read(value_bits)? as u8;
        for _ in 0..count {
            screen[address] = value;
            address += step;
            if address >= screen.len() {
                passes -= 1;
                if passes == 0 {
                    break;
                }
                address = passes - 1;
            }
        }
    }
    if bits.position.div_ceil(8) != data.len() {
        return Err(DecodeError::Unrecognized);
    }
    render(&screen, mode, &palette)
}

struct BitReader<'a> {
    data: &'a [u8],
    position: usize,
}

impl BitReader<'_> {
    /// Reads `count` bits; the first bit read is the value's lowest.
    fn read(&mut self, count: u32) -> Result<u32, DecodeError> {
        let mut value = 0;
        for i in 0..count {
            let byte = self
                .data
                .get(self.position / 8)
                .ok_or(DecodeError::Unrecognized)?;
            let bit = (byte >> (7 - self.position % 8)) & 1;
            value |= u32::from(bit) << i;
            self.position += 1;
        }
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixel_bits_interleave() {
        assert_eq!(pixel(0b1000_1000, 0, 2), 3);
        assert_eq!(pixel(0b0100_0000, 1, 2), 2);
        assert_eq!(pixel(0b1010_1010, 0, 4), 15);
        assert_eq!(pixel(0b0000_0001, 1, 4), 1);
    }

    #[test]
    fn bit_reader_reverses_fields() {
        let mut bits = BitReader {
            data: &[0x10, 0x40],
            position: 0,
        };
        assert_eq!(bits.read(8), Ok(8));
        assert_eq!(bits.read(8), Ok(2));
        assert_eq!(bits.read(1), Err(DecodeError::Unrecognized));
    }
}
