//! TRS-80 and TRS-80 Color Computer.
//!
//! Sources:
//! - HR: raw 640x240 1-bit screen of the Model 4 hi-res boards, 80 bytes per
//!   line: trs-80.org hardware pages,
//!   <http://www.trs-80.org/radio-shack-model-4-high-resolution-board/> and
//!   <http://www.trs-80.org/model-4-grafyx-solution.html>.
//! - CompuServe RLE: Brutman, "Rediscovering CompuServe RLE"
//!   (<http://www.brutman.com/RLE/RLE_Graphics.html>): `ESC G H` (256x192) or
//!   `ESC G M` (128x96), then characters giving run lengths plus 32,
//!   alternating background and foreground, starting with background.
//! - Color Computer PMODE 4 (256x192, 1 bit) and PMODE 1 (128x96, 2 bits)
//!   screens: Lomont, "Color Computer 1/2/3 Hardware Programming",
//!   <https://www.lomont.org/software/misc/coco/Lomont_CoCoHardware.pdf>;
//!   files are RS-DOS binaries with a 5-byte preamble (0, length, load
//!   address), see the platform survey `docs/research/amiga-apple-misc.md`.
//! - CLP: 40x56 1-bit clip with a 25-byte header: reverse engineered from a
//!   sample.
//! - Colours (set bit white, except black in CLP; the PMODE 1 colour set),
//!   doubled lines on HR, doubled pixels on PMODE 1 and RLE pixels left over
//!   at the end taking the last run's colour: observed from `recoil2png`
//!   output.
//! - Color Computer 3 HRS, MGE, RAT and VEF: see `coco3.rs`; survey
//!   `docs/research/next-zx-misc.md`.

mod coco3;
mod magicdraw;

use alloc::vec::Vec;

use crate::{BitOrder, DecodeError, Format, Image};

pub(super) static FORMATS: &[Format] = &[
    Format::new("TRS-80", "640x240", &["hr"], decode_hr),
    Format::new("TRS-80", "CompuServe RLE", &["rle"], decode_rle).signature(),
    Format::new("TRS-80 Color Computer", "40x56", &["clp"], decode_clp),
    Format::new(
        "TRS-80 Color Computer",
        "256x192",
        &["grf", "max", "p41", "pix"],
        decode_pmode4,
    ),
    Format::new("TRS-80 Color Computer", "128x96", &["p11"], decode_pmode1),
    // Wave 5: Amiga and misc
    Format::new("TRS-80", "MagicDraw", &["shr"], magicdraw::decode),
    // Color Computer 3 (sources in coco3.rs)
    Format::new(
        "TRS-80 Color Computer 3",
        "HRS",
        &["hrs"],
        coco3::decode_hrs,
    ),
    Format::new(
        "TRS-80 Color Computer 3",
        "MGE",
        &["mge"],
        coco3::decode_mge,
    ),
    Format::new(
        "TRS-80 Color Computer 3",
        "RAT",
        &["rat"],
        coco3::decode_rat,
    ),
    Format::new(
        "TRS-80 Color Computer 3",
        "VEF",
        &["vef"],
        coco3::decode_vef,
    ),
    Format::new(
        "TRS-80 Color Computer 3",
        "CM3",
        &["cm3"],
        coco3::cm3::decode,
    ),
];

const WHITE: u32 = 0xffffff;

/// Draws a 1-bit bitmap, most significant bit leftmost, `width / 8` bytes
/// per row.
fn mono(bitmap: &[u8], width: usize, height: usize, set: u32) -> Result<Image, DecodeError> {
    let colors = [set ^ WHITE, set];
    Image::from_bits(
        width as u32,
        height as u32,
        bitmap,
        width / 8,
        BitOrder::MsbFirst,
        colors,
    )
}

fn decode_hr(data: &[u8]) -> Result<Image, DecodeError> {
    const LEN: usize = 640 / 8 * 240;
    // Some files carry a few hundred bytes of trailing data.
    if !(LEN..=LEN + 1024).contains(&data.len()) {
        return Err(DecodeError::Unrecognized);
    }
    mono(data, 640, 240, WHITE)?.scaled(1, 2)
}

fn decode_rle(data: &[u8]) -> Result<Image, DecodeError> {
    let (width, height) = match data.get(..3) {
        Some(b"\x1bGH") => (256, 192),
        Some(b"\x1bGM") => (128, 96),
        _ => return Err(DecodeError::Unrecognized),
    };
    let total = (width * height) as usize;
    // Printable run characters up to an escape or the end, covering the
    // picture; one pixel short is accepted only before an escape. This is
    // what recoil2png accepts (probed as a black box), and all samples are
    // such 7-bit text.
    let end = data[3..].iter().position(|&c| c == 0x1b);
    let runs = &data[3..end.map_or(data.len(), |e| 3 + e)];
    let covered: usize = runs.iter().map(|&c| usize::from(c.wrapping_sub(32))).sum();
    let shortfall = total.saturating_sub(covered);
    if runs.iter().any(|c| !(0x20..=0x7f).contains(c)) || shortfall > usize::from(end.is_some()) {
        return Err(DecodeError::Unrecognized);
    }
    let mut image = Image::new(width, height);
    let mut pos = 0;
    let mut foreground = false;
    let mut color = 0;
    for &c in runs {
        if pos >= total {
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
    mono(bitmap, 40, 56, 0)
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
    mono(bitmap, 256, 192, WHITE)
}

/// Colour set 0 of the MC6847: green, yellow, blue, red.
const PMODE1_COLORS: [u32; 4] = [0x07ff00, 0xffff00, 0x3b08ff, 0xcc003b];

fn decode_pmode1(data: &[u8]) -> Result<Image, DecodeError> {
    let bitmap = rs_dos_data(data, 3072).ok_or(DecodeError::Unrecognized)?;
    let indices: Vec<u8> = bitmap
        .iter()
        .flat_map(|&b| [b >> 6, b >> 4 & 3, b >> 2 & 3, b & 3])
        .collect();
    Image::from_indexed(128, 96, &indices, &PMODE1_COLORS)?.scaled(2, 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 256x192 RLE header and runs covering `pixels`, then `tail`.
    fn rle(pixels: usize, tail: &[u8]) -> Vec<u8> {
        let mut data = b"\x1bGH".to_vec();
        let mut left = pixels;
        while left > 0 {
            let run = left.min(95);
            data.push(32 + run as u8);
            left -= run;
        }
        data.extend_from_slice(tail);
        data
    }

    #[test]
    fn rle_runs_must_cover_the_picture_in_printable_characters() {
        let full = 256 * 192;
        assert!(decode_rle(&rle(full, b"")).is_ok());
        assert!(decode_rle(&rle(full + 500, b"\x1bGN")).is_ok());
        assert!(decode_rle(&rle(full - 1, b"\x1bGN")).is_ok());
        assert!(decode_rle(&rle(full - 1, b"")).is_err());
        assert!(decode_rle(&rle(full - 2, b"\x1bGN")).is_err());
        let mut control = rle(1000, b"\x05");
        control.extend_from_slice(&rle(full, b"")[3..]);
        assert!(decode_rle(&control).is_err());
    }
}
