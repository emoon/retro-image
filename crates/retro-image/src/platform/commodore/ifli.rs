//! C64 interlaced FLI formats: two FLI (or AFLI) frames shown alternately.
//!
//! Sources (memory maps):
//! - Codebase64 "C64 Graphics File Format Specs" (CB),
//!   <http://codebase.c64.org/doku.php?id=base:c64_grafix_files_specs_list_v0.03>
//! - GoDot IFLI loader page (GD), <https://www.godot64.de/german/l_ifli.htm>,
//!   and Pixel Perfect saver page, <https://www.godot64.de/german/s_pixperfect.htm>
//!
//! | Format | Sources |
//! |---|---|
//! | Gunpaint (GUN, IFL) | CB "Gunpaint", GD IFLI |
//! | Funpaint II (FUN, FP2) | CB "Funpaint 2", GD IFLI |
//! | Pixel Perfect (PP) | GD IFLI, GD Pixel Perfect saver |
//! | ECI Graphic Editor (ECI) | CB "ECI Graphic Editor v1.0" |
//! | Flash FLI (FFLI), Big FLI (BFLI) | Pasi Ojala's `ffli.doc`, `bfli.doc` and "BFLI - New graphics modes 2" (linecrunch counter wrap-around) in C64Gfx, <http://www.zimmers.net/anonftp/pub/cbm/crossplatform/graphics/Amiga/C64Gfx.lha> (documentation only) |
//!
//! Observed from `recoil2png` output: the second frame of the multicolour
//! formats is shown one pixel to the right; Gunpaint and Funpaint use a
//! black background (their `$D021` tables are unused here).

use super::fli::{Bg, Fli};
use super::prg::Prg;
use super::unpack::{Run, escape_rle};
use super::vic2::{FLI_BUG, Frame};
use crate::{DecodeError, Image};
use alloc::vec::Vec;

/// Two FLI frames in one memory image.
struct Ifli {
    load: u16,
    sizes: &'static [usize],
    frames: [Fli; 2],
    /// Whether the second frame is shown one pixel to the right.
    shift: bool,
}

impl Ifli {
    fn decode(&self, data: &[u8]) -> Result<Image, DecodeError> {
        if !self.sizes.contains(&data.len()) {
            return Err(DecodeError::Unrecognized);
        }
        self.decode_unchecked(data)
    }

    fn decode_unchecked(&self, data: &[u8]) -> Result<Image, DecodeError> {
        let prg = Prg::new(data, self.load);
        let [first, second] = &self.frames;
        let (first, second) = first
            .frame(&prg)
            .zip(second.frame(&prg))
            .ok_or(DecodeError::Unrecognized)?;
        let second = if self.shift {
            second.shift_right()
        } else {
            second
        };
        Ok(first.blend(&second, FLI_BUG))
    }
}

const fn fli(bitmap: u16, screens: u16, color: Option<u16>, background: Bg) -> Fli {
    Fli {
        load: 0,
        sizes: &[],
        bitmap,
        screens,
        color,
        background,
        height: 200,
        skip: 0,
    }
}

const GUNPAINT: Ifli = Ifli {
    load: 0x4000,
    sizes: &[33603],
    frames: [
        fli(0x6000, 0x4000, Some(0x8000), Bg::Black),
        fli(0xa400, 0x8400, Some(0x8000), Bg::Black),
    ],
    shift: true,
};

pub(super) fn decode_gunpaint(data: &[u8]) -> Result<Image, DecodeError> {
    GUNPAINT.decode(data)
}

/// Gunpaint's layout one byte shorter, as found in generic `.vic` dumps.
pub(super) fn decode_gunpaint_dump(data: &[u8]) -> Result<Image, DecodeError> {
    Ifli {
        sizes: &[33602],
        ..GUNPAINT
    }
    .decode(data)
}

const PIXEL_PERFECT: Ifli = Ifli {
    load: 0x3c00,
    sizes: &[33602],
    frames: [
        fli(0x6000, 0x4000, Some(0x3c00), Bg::Byte(0x7f7f)),
        fli(0xa000, 0x8000, Some(0x3c00), Bg::Byte(0x7f7f)),
    ],
    shift: true,
};

pub(super) fn decode_pixel_perfect(data: &[u8]) -> Result<Image, DecodeError> {
    PIXEL_PERFECT.decode(data)
}

/// Pixel Perfect packed: load `$3BFC`, `$10 $10 $10`, escape byte, then
/// `ESC count-1 value` RLE from `$3C00`.
pub(super) fn decode_pixel_perfect_packed(data: &[u8]) -> Result<Image, DecodeError> {
    let (header, packed) = data.split_at_checked(6).ok_or(DecodeError::Unrecognized)?;
    if header[..5] != [0xfc, 0x3b, 0x10, 0x10, 0x10] {
        return Err(DecodeError::Unrecognized);
    }
    let len = PIXEL_PERFECT.sizes[0] - 2;
    let unpacked = escape_rle(packed, header[5], Run::CountMinusOneValue, len)
        .ok_or(DecodeError::Unrecognized)?;
    PIXEL_PERFECT.decode_unchecked(&super::bitmap::with_header(unpacked))
}

const ECI: Ifli = Ifli {
    load: 0x4000,
    sizes: &[32770],
    frames: [
        fli(0x4000, 0x6000, None, Bg::Black),
        fli(0x8000, 0xa000, None, Bg::Black),
    ],
    shift: false,
};

pub(super) fn decode_eci(data: &[u8]) -> Result<Image, DecodeError> {
    ECI.decode(data)
}

/// Flash FLI: one bitmap shown with two sets of FLI screens and
/// backgrounds.
const FFLI: Ifli = Ifli {
    load: 0x3aff,
    sizes: &[26115],
    frames: [
        fli(0x6000, 0x4000, Some(0x3c00), Bg::Table(0x3b00)),
        fli(0x6000, 0x8000, Some(0x3c00), Bg::Table(0xa000)),
    ],
    shift: false,
};

pub(super) fn decode_ffli(data: &[u8]) -> Result<Image, DecodeError> {
    if data.get(..3) != Some(&[0xff, 0x3a, b'f']) {
        return Err(DecodeError::Unrecognized);
    }
    FFLI.decode(data)
}

/// Big FLI: a 400-line multicolour FLI picture shown with linecrunch. The
/// top half uses the bank at `$4000`, the bottom half the bank at `$8000`;
/// as the VIC-II's counters run on past the end of the first bank, the
/// bottom half's video matrix, colour RAM and bitmap offsets continue
/// from 1000 (8000 for the bitmap) and wrap at 1024 (8192).
pub(super) fn decode_bfli(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 33795 || data[..3] != [0xff, 0x3b, b'b'] {
        return Err(DecodeError::Unrecognized);
    }
    let memory = &data[3..];
    // Offsets from $3C00: colour RAM, then two banks of screens and bitmap.
    let color = &memory[..1024];
    let frame = Frame::from_fn(400, |x, y| {
        let bank = &memory[1024 + y / 200 * 0x4000..];
        let cell = (y / 8 * 40 + x / 8) & 1023;
        let screen = bank[(y & 7) * 1024 + cell];
        let byte = bank[0x2000 + ((cell * 8 + y % 8) & 8191)];
        match byte >> (6 - (x & 6)) & 3 {
            0 => 0,
            1 => screen >> 4,
            2 => screen & 15,
            _ => color[cell] & 15,
        }
    });
    Ok(frame.to_image(FLI_BUG))
}

const FUNPAINT: Ifli = Ifli {
    load: 0x3ff0,
    sizes: &[33694],
    frames: [
        fli(0x6000, 0x4000, Some(0x8000), Bg::Black),
        fli(0xa3e8, 0x83e8, Some(0x8000), Bg::Black),
    ],
    shift: true,
};

/// Funpaint II: `FUNPAINT (MT) ` header at `$3FF0`, then a pack flag and an
/// escape byte; packed data is `ESC count value` RLE from `$4000`.
pub(super) fn decode_funpaint(data: &[u8]) -> Result<Image, DecodeError> {
    let header = data.get(..18).ok_or(DecodeError::Unrecognized)?;
    if &header[2..16] != b"FUNPAINT (MT) " {
        return Err(DecodeError::Unrecognized);
    }
    if header[16] == 0 {
        return FUNPAINT.decode(data);
    }
    let len = FUNPAINT.sizes[0] - header.len();
    let unpacked = escape_rle(&data[18..], header[17], Run::CountValue, len)
        .ok_or(DecodeError::Unrecognized)?;
    let mut full = Vec::with_capacity(FUNPAINT.sizes[0]);
    full.extend_from_slice(header);
    full.extend(unpacked);
    FUNPAINT.decode_unchecked(&full)
}
