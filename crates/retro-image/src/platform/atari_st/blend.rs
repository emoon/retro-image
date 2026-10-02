//! Two-screen pictures shown alternately so the eye blends them: Tobias
//! Richter's overscan slideshow (`PCI`), HighresMedium (`HRM`), PL4 and
//! D-GRAPH (`P3C`).
//!
//! Sources:
//! - Overscan Interlaced: <https://temlib.org/AtariForumWiki/index.php/Overscan_Interlaced_file_format>
//! - HighresMedium, including Hans Wessels' public-domain palette index
//!   function: <https://temlib.org/AtariForumWiki/index.php/HighresMedium_file_format>
//! - PL4: <https://temlib.org/AtariForumWiki/index.php/PL4_file_format>
//!   (16-word palettes, as the 64070-byte total requires)
//! - Observed from `recoil2png` output: the two screens are averaged per
//!   component; HighresMedium's 400 lines pair up into 200 doubled lines.
//!   `PCI` and `HRM` files may be packed with Pack-Ice.

use alloc::vec::Vec;

use super::common::{
    Resolution, SCREEN_LEN, be16, decode_screen, interleaved_index, mix, mix_images, palette_words,
    separate_planes_to_interleaved, st_rgb, uses_ste_bits, words,
};
use crate::{DecodeError, Image};

/// Unpacks Pack-Ice data; `None` if `data` is not packed.
fn unpack_ice(data: &[u8]) -> Result<Option<Vec<u8>>, DecodeError> {
    if !super::pack_ice::is_packed(data) {
        return Ok(None);
    }
    super::pack_ice::unpack(data)
        .map(Some)
        .ok_or(DecodeError::Unrecognized)
}

const PCI_WIDTH: usize = 352;
const PCI_HEIGHT: usize = 278;
const PCI_SCREEN_LEN: usize = PCI_WIDTH / 8 * PCI_HEIGHT * 4;
const PCI_PALETTE_LEN: usize = PCI_HEIGHT * 32;

/// Two 352x278 screens (separate plane blocks), then a 16-colour palette
/// per line for each.
pub(super) fn decode_pci(data: &[u8]) -> Result<Image, DecodeError> {
    let unpacked = unpack_ice(data)?;
    let data = unpacked.as_deref().unwrap_or(data);
    if data.len() != 2 * (PCI_SCREEN_LEN + PCI_PALETTE_LEN) {
        return Err(DecodeError::Unrecognized);
    }
    let (screens, palettes) = data.split_at(2 * PCI_SCREEN_LEN);
    let all = words(palettes);
    let ste = uses_ste_bits(all.iter().copied());
    let frame = |i: usize| {
        let screen =
            separate_planes_to_interleaved(&screens[i * PCI_SCREEN_LEN..][..PCI_SCREEN_LEN], 4);
        let palette = &all[i * PCI_HEIGHT * 16..][..PCI_HEIGHT * 16];
        let mut image = Image::new(PCI_WIDTH as u32, PCI_HEIGHT as u32);
        let stride = PCI_WIDTH / 2;
        for y in 0..PCI_HEIGHT {
            let line = &screen[y * stride..][..stride];
            for x in 0..PCI_WIDTH as u32 {
                let c = interleaved_index(line, x, 4);
                image.set(x, y as u32, st_rgb(palette[y * 16 + c], ste));
            }
        }
        image
    };
    Ok(mix_images(&frame(0), &frame(1)))
}

/// HighresMedium: 400 medium-resolution lines (pairs of alternating
/// lines), 35 palette words per line chosen by `find_hrm_index`.
pub(super) fn decode_hrm(data: &[u8]) -> Result<Image, DecodeError> {
    let unpacked = unpack_ice(data)?;
    let data = unpacked.as_deref().unwrap_or(data);
    if data.len() != 64000 + 28000 {
        return Err(DecodeError::Unrecognized);
    }
    let palettes = words(&data[64000..]);
    let ste = uses_ste_bits(palettes.iter().copied());
    let line_image = |y: usize| -> Option<Vec<u32>> {
        let line = &data[y * 160..][..160];
        (0..640)
            .map(|x| {
                let c = interleaved_index(line, x as u32, 2);
                let index = hrm_index(x, c)?;
                Some(st_rgb(*palettes.get(y * 35 + index)?, ste))
            })
            .collect()
    };
    let mut image = Image::new(640, 400);
    for y in 0..200 {
        let a = line_image(y * 2).ok_or(DecodeError::Unrecognized)?;
        let b = line_image(y * 2 + 1).ok_or(DecodeError::Unrecognized)?;
        for x in 0..640 {
            let color = mix(a[x], b[x]);
            image.set(x as u32, y as u32 * 2, color);
            image.set(x as u32, y as u32 * 2 + 1, color);
        }
    }
    Ok(image)
}

/// Hans Wessels' `find_hrm_index`; `None` where it would be negative.
fn hrm_index(x: usize, c: usize) -> Option<usize> {
    let x = x as isize + 80;
    let index = match c {
        0 => -1 + 4 * (x / 80),
        1 => 4 * ((x - 8) / 80),
        2 => 1 + 4 * ((x - 40) / 80),
        _ => 2 + 4 * ((x - 48) / 80),
    };
    usize::try_from(index).ok()
}

/// D-GRAPH (`P3C`): an ASCII size line, a palette and a CrackArt-packed
/// low-resolution screen, then a size line and a second packed screen
/// sharing the palette. Derived from sample files and `recoil2png`
/// output (the survey found no documentation).
pub(super) fn decode_p3c(data: &[u8]) -> Result<Image, DecodeError> {
    decode_p3c_inner(data).ok_or(DecodeError::Unrecognized)
}

fn decode_p3c_inner(data: &[u8]) -> Option<Image> {
    let (len, pos) = decimal_line(data, 0)?;
    let words = palette_words(data, pos, 16)?;
    let first = data.get(pos + 32..(pos + 32).checked_add(len)?)?;
    let (len2, pos2) = decimal_line(data, (pos + 32).checked_add(len)?)?;
    let second = data.get(pos2..pos2.checked_add(len2)?)?;
    let frame = |packed: &[u8]| {
        let screen = super::crackart::unpack(packed, SCREEN_LEN)?;
        decode_screen(Resolution::Low, &screen, &words)
    };
    Some(mix_images(&frame(first)?, &frame(second)?))
}

/// Decimal digits ended by CR LF; returns the value and the next position.
fn decimal_line(data: &[u8], start: usize) -> Option<(usize, usize)> {
    let end = start + data.get(start..)?.iter().position(|&b| b == b'\r')?;
    let digits = core::str::from_utf8(data.get(start..end)?).ok()?;
    if digits.is_empty() || data.get(end + 1) != Some(&b'\n') {
        return None;
    }
    Some((digits.parse().ok()?, end + 2))
}

const PL4_LEN: usize = 64070;

/// PL4: an LZ4 frame holding two DEGAS-like low-resolution screens.
pub(super) fn decode_pl4(data: &[u8]) -> Result<Image, DecodeError> {
    let unpacked = super::lz4::decompress_frame(data, PL4_LEN).ok_or(DecodeError::Unrecognized)?;
    if unpacked.len() != PL4_LEN {
        return Err(DecodeError::Unrecognized);
    }
    let frame = |at: usize| {
        let words = palette_words(&unpacked, at + 2, 16)?;
        if be16(&unpacked, at)? != 0 {
            return None;
        }
        decode_screen(Resolution::Low, &unpacked[at + 34..][..SCREEN_LEN], &words)
    };
    let a = frame(0).ok_or(DecodeError::Unrecognized)?;
    let b = frame(34 + SCREEN_LEN + 2).ok_or(DecodeError::Unrecognized)?;
    Ok(mix_images(&a, &b))
}
