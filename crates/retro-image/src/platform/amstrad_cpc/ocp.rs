//! Advanced OCP Art Studio screens (SCR) and windows (WIN), with the
//! palette (PAL) as a companion file.
//!
//! Sources:
//! - PAL layout (mode, animation flag and delay, then 12 animation colors
//!   of each of the 16 pens as `0x40 | hardware colour`, border colors,
//!   excluded and protected inks; 239 bytes), MJH compression and the WIN
//!   trailer: OCP Art Studio file formats,
//!   <https://cpctech.cpcwiki.de/docs/artstud.html>.
//! - MJH details (a screen is four `MJH` blocks of 4096 bytes, each with a
//!   little-endian unpacked length; `01 count value` repeats a byte, count 0
//!   meaning 256), the WIN trailer fields (width in mode 2 pixels as u16 at
//!   4 bytes from the end, height at 2 bytes from the end, data padded to
//!   whole bytes per line), the raw screen sizes (16384, or 16336 without
//!   the unused tail) and the PAL checks (exactly 239 bytes, mode 0-2, the
//!   first color of every pen in 0x40-0x5F): reverse engineered from
//!   samples and `recoil2png` output.
//!
//! Without a usable PAL file, pictures are shown the way the CPC shows a
//! screen loaded at power-on: mode 1 with the default inks.

use alloc::borrow::Cow;
use alloc::vec::Vec;

use super::amsdos::{amsdos_extension, strip_amsdos};
use super::hardware::{DEFAULT_PENS, Mode, hardware_color, render, screen_line_offset};
use crate::{Companions, DecodeError, Image};

const SCREEN_LEN: usize = 0x4000;
/// A screen without the unused 48 bytes after its last 2 KB line block.
const SHORT_SCREEN_LEN: usize = 7 * 0x800 + 25 * LINE_BYTES;
const LINE_BYTES: usize = 80;
const LINES: usize = 200;

/// SCR: a standard 16 KB screen, raw or MJH-compressed.
pub(super) fn decode_scr(data: &[u8], companions: &dyn Companions) -> Result<Image, DecodeError> {
    let screen = unpack(strip_amsdos(data))?;
    if screen.len() != SCREEN_LEN && screen.len() != SHORT_SCREEN_LEN {
        return Err(DecodeError::Unrecognized);
    }
    let palette = Palette::from_companions(companions);
    let width = LINE_BYTES * palette.mode.pixels_per_byte();
    let line = |y| &screen[screen_line_offset(y, LINE_BYTES)..][..LINE_BYTES];
    render(palette.mode, width, LINES, line, &palette.pens)
}

/// An SCR file recognised by content: its AMSDOS header names it `.SCR`.
/// (Other 16 KB files, such as the halves of iMPdraw `.GO1`/`.GO2`
/// interlaced pictures, are not screens of their own.)
pub(super) fn decode_amsdos_scr(
    data: &[u8],
    companions: &dyn Companions,
) -> Result<Image, DecodeError> {
    if amsdos_extension(data) != Some(*b"SCR") {
        return Err(DecodeError::Unrecognized);
    }
    decode_scr(data, companions)
}

const WIN_TRAILER_LEN: usize = 5;

/// WIN: lines of pixels, raw or MJH-compressed, then a 5-byte trailer
/// holding the width in mode 2 pixels and the height.
pub(super) fn decode_win(data: &[u8], companions: &dyn Companions) -> Result<Image, DecodeError> {
    let file = strip_amsdos(data);
    let packed = file.starts_with(MJH);
    let window = unpack(file)?;
    let pixels_len = window
        .len()
        .checked_sub(WIN_TRAILER_LEN)
        .ok_or(DecodeError::Unrecognized)?;
    let (pixels, trailer) = window.split_at(pixels_len);
    let bits = usize::from(u16::from_le_bytes([trailer[1], trailer[2]]));
    let height = usize::from(trailer[3]);
    let line_bytes = bits.div_ceil(8);
    // MJH-packed windows may carry surplus bytes between the pixels and the
    // trailer (`recoil2png` ignores them); raw ones must fit exactly.
    let needed = line_bytes * height;
    let fits = if packed {
        needed <= pixels.len()
    } else {
        needed == pixels.len()
    };
    if bits == 0 || height == 0 || !fits {
        return Err(DecodeError::Unrecognized);
    }
    let pixels = &pixels[..needed];
    let palette = Palette::from_companions(companions);
    let width = bits * palette.mode.pixels_per_byte() / 8;
    if width == 0 {
        return Err(DecodeError::Unrecognized);
    }
    let line = |y| &pixels[y * line_bytes..][..line_bytes];
    render(palette.mode, width, height, line, &palette.pens)
}

/// Unpacks MJH blocks, or returns uncompressed data as is.
fn unpack(data: &[u8]) -> Result<Cow<'_, [u8]>, DecodeError> {
    if !data.starts_with(MJH) {
        return Ok(Cow::Borrowed(data));
    }
    let mut out = Vec::new();
    let mut rest = data;
    while !rest.is_empty() {
        rest = unpack_mjh_block(rest, &mut out).ok_or(DecodeError::Unrecognized)?;
    }
    Ok(Cow::Owned(out))
}

const MJH: &[u8] = b"MJH";
/// No supported picture unpacks to more than a screen.
const MAX_UNPACKED: usize = SCREEN_LEN;

/// Unpacks one `MJH` block onto `out`; returns the data after it.
fn unpack_mjh_block<'a>(data: &'a [u8], out: &mut Vec<u8>) -> Option<&'a [u8]> {
    let header = data.get(..5).filter(|h| h.starts_with(MJH))?;
    let len = usize::from(u16::from_le_bytes([header[3], header[4]]));
    let end = out.len() + len;
    if end > MAX_UNPACKED {
        return None;
    }
    let mut bytes = data[5..].iter();
    while out.len() < end {
        let (count, value) = match *bytes.next()? {
            1 => {
                let count = *bytes.next()?;
                let count = if count == 0 { 256 } else { usize::from(count) };
                (count, *bytes.next()?)
            }
            literal => (1, literal),
        };
        if out.len() + count > end {
            return None;
        }
        out.resize(out.len() + count, value);
    }
    Some(bytes.as_slice())
}

struct Palette {
    mode: Mode,
    pens: [u32; 16],
}

const PAL_LEN: usize = 239;
const PAL_PENS: usize = 3;
const PAL_COLORS_PER_PEN: usize = 12;

impl Palette {
    /// The PAL companion, or mode 1 with the power-on inks.
    fn from_companions(companions: &dyn Companions) -> Self {
        companions
            .get("pal")
            .and_then(|pal| Self::parse(strip_amsdos(&pal)))
            .unwrap_or(Palette {
                mode: Mode::One,
                pens: DEFAULT_PENS,
            })
    }

    /// Mode, then the first animation color of each pen.
    fn parse(pal: &[u8]) -> Option<Self> {
        if pal.len() != PAL_LEN {
            return None;
        }
        let mode = Mode::from_number(pal[0])?;
        let mut pens = [0; 16];
        for (pen, color) in pens.iter_mut().enumerate() {
            let value = pal[PAL_PENS + pen * PAL_COLORS_PER_PEN];
            if !(0x40..=0x5f).contains(&value) {
                return None;
            }
            *color = hardware_color(value);
        }
        Some(Palette { mode, pens })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NoCompanions;

    #[test]
    fn mjh_repeats_and_copies_literals() {
        let mut out = Vec::new();
        let rest = unpack_mjh_block(b"MJH\x05\x00\x01\x03\xaa\x07\x08tail", &mut out);
        assert_eq!(rest, Some(&b"tail"[..]));
        assert_eq!(out, [0xaa, 0xaa, 0xaa, 7, 8]);
        let mut out = Vec::new();
        unpack_mjh_block(b"MJH\x00\x01\x01\x00\x55", &mut out).unwrap();
        assert_eq!(out.len(), 256);
    }

    #[test]
    fn mjh_run_past_block_end_is_rejected() {
        let mut out = Vec::new();
        assert_eq!(unpack_mjh_block(b"MJH\x02\x00\x01\x03\xaa", &mut out), None);
    }

    #[test]
    fn win_trailer_gives_size() {
        // 12 mode 2 pixels wide (2 bytes per line), 2 lines high.
        let data = [0xff, 0xf0, 0, 0, 0, 12, 0, 2, 0];
        let image = decode_win(&data, &NoCompanions).unwrap();
        // Mode 1 without a PAL: 6 pixels per line.
        assert_eq!((image.width(), image.height()), (6, 2));
    }

    #[test]
    fn pal_rejects_colours_without_gate_array_bit() {
        let mut pal = alloc::vec![0x54; PAL_LEN];
        pal[0] = 0;
        assert!(Palette::parse(&pal).is_some());
        pal[PAL_PENS + 15 * PAL_COLORS_PER_PEN] = 0x14;
        assert!(Palette::parse(&pal).is_none());
    }
}
