//! TRS-80 and TRS-80 Color Computer.
//!
//! Sources:
//! - HR: raw 640x240 1-bit screen of the Model 4 hi-res boards, 80 bytes per
//!   line (trs-80.org hardware pages listed in `docs/formats/amiga-apple-misc.md`).
//! - CompuServe RLE: Brutman, "Rediscovering CompuServe RLE"
//!   (<http://www.brutman.com/RLE/RLE_Graphics.html>): `ESC G H` (256x192) or
//!   `ESC G M` (128x96), then characters giving run lengths plus 32,
//!   alternating background and foreground, starting with background.
//! - Color Computer PMODE 4 (256x192, 1 bit) and PMODE 1 (128x96, 2 bits)
//!   screens: Lomont, "Color Computer 1/2/3 Hardware Programming"; files are
//!   RS-DOS binaries with a 5-byte preamble (0, length, load address).
//! - CLP: 40x56 1-bit clip with a 25-byte header: reverse engineered from a
//!   sample.
//! - Colours (set bit white, except black in CLP; the PMODE 1 colour set),
//!   doubled lines on HR, doubled pixels on PMODE 1 and RLE pixels left over
//!   at the end taking the last run's colour: observed from `recoil2png`
//!   output.

use crate::{DecodeError, Format, Image};

pub(super) static FORMATS: &[Format] = &[
    Format::new("TRS-80", "640x240", &["hr"], decode_hr),
    Format::new("TRS-80", "CompuServe RLE", &["rle"], decode_rle),
    Format::new("TRS-80 Color Computer", "40x56", &["clp"], decode_clp),
    Format::new(
        "TRS-80 Color Computer",
        "256x192",
        &["grf", "max", "p41", "pix"],
        decode_pmode4,
    ),
    Format::new("TRS-80 Color Computer", "128x96", &["p11"], decode_pmode1),
];

const WHITE: u32 = 0xffffff;

/// Draws a 1-bit bitmap (most significant bit leftmost), each line
/// `line_repeat` times.
fn mono(bitmap: &[u8], width: usize, height: usize, line_repeat: usize, set: u32) -> Image {
    let row_len = width / 8;
    let mut image = Image::new(width as u32, (height * line_repeat) as u32);
    for y in 0..height {
        for x in 0..width {
            let bit = bitmap[y * row_len + x / 8] & (0x80 >> (x % 8)) != 0;
            let color = if bit { set } else { set ^ WHITE };
            for r in 0..line_repeat {
                image.set(x as u32, (y * line_repeat + r) as u32, color);
            }
        }
    }
    image
}

fn decode_hr(data: &[u8]) -> Result<Image, DecodeError> {
    const LEN: usize = 640 / 8 * 240;
    // Some files carry a few hundred bytes of trailing data.
    if !(LEN..=LEN + 1024).contains(&data.len()) {
        return Err(DecodeError::Unrecognized);
    }
    Ok(mono(data, 640, 240, 2, WHITE))
}

fn decode_rle(data: &[u8]) -> Result<Image, DecodeError> {
    let (width, height) = match data.get(..3) {
        Some(b"\x1bGH") => (256, 192),
        Some(b"\x1bGM") => (128, 96),
        _ => return Err(DecodeError::Unrecognized),
    };
    let mut image = Image::new(width, height);
    let total = (width * height) as usize;
    let mut pos = 0;
    let mut foreground = false;
    let mut color = 0;
    for &c in &data[3..] {
        if c == 0x1b || pos >= total {
            break;
        }
        let run = usize::from(c.saturating_sub(32));
        color = if foreground { WHITE } else { 0 };
        for p in pos..(pos + run).min(total) {
            image.set(p as u32 % width, p as u32 / width, color);
        }
        pos += run;
        foreground = !foreground;
    }
    // Pixels left over take the last run's colour.
    for p in pos..total {
        image.set(p as u32 % width, p as u32 / width, color);
    }
    Ok(image)
}

fn decode_clp(data: &[u8]) -> Result<Image, DecodeError> {
    const HEADER_LEN: usize = 25;
    let bitmap = data
        .get(HEADER_LEN..HEADER_LEN + 5 * 56)
        .filter(|_| data[24] == 5 && data[16..18] == [0, 56])
        .ok_or(DecodeError::Unrecognized)?;
    Ok(mono(bitmap, 40, 56, 1, 0))
}

/// The data of an RS-DOS binary's first segment, which must hold at least
/// `len` bytes.
fn rs_dos_data(data: &[u8], len: usize) -> Option<&[u8]> {
    let header = data.get(..5)?;
    let segment_len = usize::from(u16::from_be_bytes([header[1], header[2]]));
    if header[0] != 0 || segment_len < len {
        return None;
    }
    data.get(5..5 + len)
}

fn decode_pmode4(data: &[u8]) -> Result<Image, DecodeError> {
    let bitmap = rs_dos_data(data, 6144).ok_or(DecodeError::Unrecognized)?;
    Ok(mono(bitmap, 256, 192, 1, WHITE))
}

/// Colour set 0 of the MC6847: green, yellow, blue, red.
const PMODE1_COLORS: [u32; 4] = [0x07ff00, 0xffff00, 0x3b08ff, 0xcc003b];

fn decode_pmode1(data: &[u8]) -> Result<Image, DecodeError> {
    let bitmap = rs_dos_data(data, 3072).ok_or(DecodeError::Unrecognized)?;
    let mut image = Image::new(256, 192);
    for y in 0..96 {
        for x in 0..128 {
            let byte = bitmap[y * 32 + x / 4];
            let color = PMODE1_COLORS[usize::from(byte >> (6 - x % 4 * 2) & 3)];
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                image.set((x * 2 + dx) as u32, (y * 2 + dy) as u32, color);
            }
        }
    }
    Ok(image)
}
