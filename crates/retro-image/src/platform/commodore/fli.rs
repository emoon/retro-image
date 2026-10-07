//! C64 FLI formats: a bitmap with a separate screen RAM for each of the
//! eight lines of a character row (multicolor FLI, hires AFLI), optionally
//! with a per-line background (`$D021`) table.
//!
//! Sources (memory maps):
//! - Codebase64 "C64 Graphics File Format Specs" (CB),
//!   <http://codebase.c64.org/doku.php?id=base:c64_grafix_files_specs_list_v0.03>
//! - GoDot loader pages (GD), <https://www.godot64.de/german/lstab.htm>
//! - FLI display mechanism: <https://www.cebix.net/VIC-Article.txt>
//!
//! | Format | Sources |
//! |---|---|
//! | FLI Designer (FD2, FLI) | CB "FLI Designer 1.1 & 2.0", GD FLI-Designer |
//! | FLI Graph (BML) | CB "FLI Graph 2.2", GD FLI Graph |
//! | AFLI-editor (AFL) | CB "AFLI-editor v2.0", GD AFLI |
//! | Hires FLI Designer (HFC, HFD) | CB "Hires FLI" |
//! | Flip (FBI), FLI Graph packed | GD Flip <https://www.godot64.de/german/l_flipr.htm>, CB "FLI Graph 2.2" |
//! | Hires Manager (HIM) | CB "Hires Manager", GD HiManRaw; for the packed form, the exclusive end address and literal lengths were checked against a sample that exists both packed and unpacked |
//! | FLI Profi (FPR) | CB "FLI-Profi" (load `$3780`, sprites from `$3780`, color `$3C00`, screens `$4000`, bitmap `$6000`); how the leftmost 24 pixels are drawn was reverse engineered by probing `recoil2png` with modified and random files (see [`decode_fli_profi`]) |
//! | Flimatic (FLM) | FLI Designer's memory map (color RAM `$3C00`, screens `$4000`, bitmap `$6000`) plus 64 bytes, packed with the escape-last RLE ([`escape_last_rle`]). Read from the save and load routines of Flimatic 3.7, which pack `$3C00-$7F7F`; the background color is the low nibble of `$7F7F` (the editor's own screen code reads it, and changing it in `recoil2png` input recolors the picture), and the bytes `$7F40-$7F7E` change nothing in `recoil2png` |
//! | CFLI Designer (CFLI) | Reverse engineered from 3 samples: load `$4000`, eight screen RAMs and no bitmap; the picture is hires FLI over a bitmap of `$AA` bytes, so each pixel pair shows both screen nibbles. Checked against `recoil2png` output |
//!
//! Picture heights of Hires FLI Designer (112 lines) and Hires Manager
//! (192 lines, starting at the second character row) observed from
//! `recoil2png` output.

use super::bitmap::with_header;
use super::prg::Prg;
use super::unpack::{backward_rle, escape_last_rle};
use super::vic2::{BITMAP_LEN, Background, Bitmap, FLI_BUG, Frame, SCREEN_LEN, Screens};
use crate::{DecodeError, Image};
use alloc::vec::Vec;

/// Bytes of eight screen RAMs, 1024 apart.
const SCREENS_LEN: usize = 7 * 1024 + SCREEN_LEN;

/// Memory map of a FLI picture.
pub(super) struct Fli {
    pub load: u16,
    pub sizes: &'static [usize],
    pub bitmap: u16,
    pub screens: u16,
    /// Color RAM; `None` for hires (AFLI).
    pub color: Option<u16>,
    pub background: Bg,
    pub height: usize,
    /// Bitmap lines above the picture that are not shown.
    pub skip: usize,
}

/// Where a FLI picture's background color (`$D021`) comes from.
#[derive(Clone, Copy)]
pub(super) enum Bg {
    Black,
    /// One byte at this address.
    Byte(u16),
    /// A table with one entry per line at this address.
    Table(u16),
    /// A per-line table split in two: `first` entries at the first
    /// address, then the rest at the second.
    SplitTable {
        first: u16,
        len: u8,
        rest: u16,
    },
}

impl Fli {
    pub(super) fn decode(&self, data: &[u8]) -> Result<Image, DecodeError> {
        if !self.sizes.contains(&data.len()) {
            return Err(DecodeError::Invalid);
        }
        self.decode_unchecked(data)
    }

    pub(super) fn decode_unchecked(&self, data: &[u8]) -> Result<Image, DecodeError> {
        self.frame(&Prg::new(data, self.load))
            .map(|frame| frame.skip_lines(self.skip).to_image(FLI_BUG))
            .ok_or(DecodeError::Invalid)
    }

    pub(super) fn frame(&self, prg: &Prg) -> Option<Frame> {
        let screens = Screens::Fli {
            data: prg.at(self.screens, SCREENS_LEN)?,
            stride: 1024,
        };
        let split;
        let background = match self.background {
            Bg::Black => Background::Fixed(0),
            Bg::Byte(addr) => Background::Fixed(prg.byte(addr)?),
            Bg::Table(addr) => Background::PerLine(prg.at(addr, self.height)?),
            Bg::SplitTable { first, len, rest } => {
                let len = usize::from(len);
                let mut table = prg.at(first, len)?.to_vec();
                table.extend_from_slice(prg.at(rest, self.height.saturating_sub(len))?);
                split = table;
                Background::PerLine(&split)
            }
        };
        let bitmap = Bitmap {
            bitmap: prg.at(self.bitmap, BITMAP_LEN)?,
            screens,
            color: match self.color {
                Some(addr) => prg.at(addr, SCREEN_LEN)?,
                None => &[],
            },
            background,
        };
        let height = self.skip + self.height;
        let mut frame = match self.color {
            Some(_) => Frame::multicolor(&bitmap, height),
            None => Frame::hires(&bitmap, height),
        }?;
        // The FLI bug columns show the background.
        frame.fill_left(FLI_BUG, |y| background.get(y));
        Some(frame)
    }
}

const FLI_DESIGNER: Fli = Fli {
    load: 0x3c00,
    sizes: &[17218, 17409, 17410],
    bitmap: 0x6000,
    screens: 0x4000,
    color: Some(0x3c00),
    background: Bg::Black,
    height: 200,
    skip: 0,
};
const FLI_GRAPH: Fli = Fli {
    load: 0x3b00,
    sizes: &[17474, 17665, 17666],
    background: Bg::Table(0x3b00),
    ..FLI_DESIGNER
};
const AFLI_EDITOR: Fli = Fli {
    load: 0x4000,
    sizes: &[16385],
    bitmap: 0x6000,
    screens: 0x4000,
    color: None,
    background: Bg::Black,
    height: 200,
    skip: 0,
};
const HIRES_FLI_DESIGNER: Fli = Fli {
    load: 0x4000,
    sizes: &[16386],
    bitmap: 0x4000,
    screens: 0x6000,
    color: None,
    background: Bg::Black,
    height: 112,
    skip: 0,
};
const HIRES_MANAGER: Fli = Fli {
    sizes: &[16385],
    height: 192,
    skip: 8,
    ..HIRES_FLI_DESIGNER
};

/// Files copied through CP/M-style tools are padded with `0x1A` up to a
/// multiple of 128 bytes (`kingsd.fli`: 17218 + 62); the padding is ignored.
pub(super) fn decode_fli_designer(data: &[u8]) -> Result<Image, DecodeError> {
    const PAD: u8 = 0x1a;
    let base = FLI_DESIGNER.sizes[0];
    let padded =
        (base + 1..base + 128).contains(&data.len()) && data[base..].iter().all(|&b| b == PAD);
    FLI_DESIGNER.decode(if padded { &data[..base] } else { data })
}

/// Unpacks the backward-RLE files of Flip and FLI Graph: load `$38F0`,
/// escape byte, end of packed data, end of unpacked data, zero padding to
/// `$3900`, then data packed backwards that unpacks to `start..=end`.
fn unpack_38f0(data: &[u8], start: u16) -> Result<Vec<u8>, DecodeError> {
    let header = data.get(..18).ok_or(DecodeError::Invalid)?;
    let packed_end = usize::from(u16::from_le_bytes([header[3], header[4]]));
    let unpacked_end = u16::from_le_bytes([header[5], header[6]]);
    if header[..2] != [0xf0, 0x38] || packed_end + 1 != 0x38f0 + data.len() - 2 {
        return Err(DecodeError::Invalid);
    }
    let len = usize::from(
        unpacked_end
            .checked_sub(start)
            .ok_or(DecodeError::Invalid)?,
    ) + 1;
    let unpacked = backward_rle(&data[18..], header[2], len).ok_or(DecodeError::Invalid)?;
    Ok(with_header(unpacked))
}

/// Flimatic: FLI Designer's layout with the background in `$7F7F`.
pub(super) fn decode_flimatic(data: &[u8]) -> Result<Image, DecodeError> {
    const FLIMATIC: Fli = Fli {
        background: Bg::Byte(0x7f7f),
        ..FLI_DESIGNER
    };
    let unpacked = escape_last_rle(data, 0x7f80 - 0x3c00).ok_or(DecodeError::Invalid)?;
    FLIMATIC.decode_unchecked(&with_header(unpacked))
}

/// Flip (FLI Painter): FLI Designer layout, plain or packed.
pub(super) fn decode_flip(data: &[u8]) -> Result<Image, DecodeError> {
    if FLI_DESIGNER.sizes.contains(&data.len()) {
        return FLI_DESIGNER.decode(data);
    }
    FLI_DESIGNER.decode_unchecked(&unpack_38f0(data, 0x3c00)?)
}

/// FLI Graph: plain, or packed like Flip.
pub(super) fn decode_fli_graph(data: &[u8]) -> Result<Image, DecodeError> {
    if FLI_GRAPH.sizes.contains(&data.len()) {
        return FLI_GRAPH.decode(data);
    }
    FLI_GRAPH.decode_unchecked(&unpack_38f0(data, 0x3b00)?)
}

pub(super) fn decode_afli_editor(data: &[u8]) -> Result<Image, DecodeError> {
    AFLI_EDITOR.decode(data)
}

pub(super) fn decode_hires_fli_designer(data: &[u8]) -> Result<Image, DecodeError> {
    HIRES_FLI_DESIGNER.decode(data)
}

/// CFLI Designer: eight screen RAMs from `$4000`, shown as hires FLI over
/// a fixed `$AA` bitmap.
pub(super) fn decode_cfli(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 2 + SCREENS_LEN || data[..2] != [0x00, 0x40] {
        return Err(DecodeError::Invalid);
    }
    let bitmap = [0xaa; BITMAP_LEN];
    let bitmap = Bitmap {
        bitmap: &bitmap,
        screens: Screens::Fli {
            data: &data[2..],
            stride: 1024,
        },
        color: &[],
        background: Background::Fixed(0),
    };
    Frame::hires(&bitmap, 200)
        .map(|frame| frame.to_image(FLI_BUG))
        .ok_or(DecodeError::Invalid)
}

const FLI_PROFI: Fli = Fli {
    load: 0x3780,
    sizes: &[2 + 0x47c0],
    bitmap: 0x6000,
    screens: 0x4000,
    color: Some(0x3c00),
    background: Bg::Black,
    height: 200,
    skip: 0,
};

/// FLI Profi: multicolor FLI shown at full width, with the FLI-bug columns
/// covered by a multicolor sprite. Sprite rows alternate between two
/// streams of 64-byte blocks (21 rows each): lines `y % 4` = 0 or 3 read
/// the next row from `$3780`, lines 1 and 2 from `$38C0`. Sprite bit pairs
/// `01` take the line's color from `$3A00 + y`, `10` the color at
/// `$3BC8`, `11` the one at `$3BC9`, and cover the bitmap. Under the sprite
/// the bitmap's `00` is black, `01` and `10` light gray (the FLI bug) and
/// `11` the high nibble of `$3B00 + y`.
pub(super) fn decode_fli_profi(data: &[u8]) -> Result<Image, DecodeError> {
    if !FLI_PROFI.sizes.contains(&data.len()) {
        return Err(DecodeError::Invalid);
    }
    let prg = Prg::new(data, FLI_PROFI.load);
    let fli = FLI_PROFI.frame(&prg).ok_or(DecodeError::Invalid)?;
    // All addresses below lie inside the size checked above.
    let byte = |addr: usize| prg.byte(addr as u16).unwrap_or(0);
    let frame = Frame::from_fn(200, |x, y| {
        if x >= FLI_BUG {
            return fli.get(x, y);
        }
        let (base, nth) = match y % 4 {
            0 | 3 => (0x3780, y / 4 * 2 + usize::from(y % 4 == 3)),
            _ => (0x38c0, y / 4 * 2 + y % 4 - 1),
        };
        let row = base + nth / 21 * 64 + nth % 21 * 3;
        let shift = 6 - (x & 6);
        match byte(row + x / 8) >> shift & 3 {
            1 => byte(0x3a00 + y),
            2 => byte(0x3bc8),
            3 => byte(0x3bc9),
            _ => {
                let cell = y / 8 * 40 + x / 8;
                match byte(0x6000 + cell * 8 + y % 8) >> shift & 3 {
                    0 => 0,
                    3 => byte(0x3b00 + y) >> 4,
                    _ => 15,
                }
            }
        }
    });
    Ok(frame.to_image(0))
}

/// Hires Manager: plain (`$FF` at `$4001`) or packed.
pub(super) fn decode_hires_manager(data: &[u8]) -> Result<Image, DecodeError> {
    if HIRES_MANAGER.sizes.contains(&data.len()) {
        return HIRES_MANAGER.decode(data);
    }
    HIRES_MANAGER.decode_unchecked(&unpack_hires_manager(data)?)
}

/// Packed Hires Manager: load `$4000`, end of packed data, end of unpacked
/// data, then data packed backwards: `$00 count value` runs, and literal
/// sequences `count+1 data...` (read backwards).
fn unpack_hires_manager(data: &[u8]) -> Result<Vec<u8>, DecodeError> {
    let header = data.get(..6).ok_or(DecodeError::Invalid)?;
    let packed_end = usize::from(u16::from_le_bytes([header[2], header[3]]));
    let unpacked_end = usize::from(u16::from_le_bytes([header[4], header[5]]));
    if header[..2] != [0x00, 0x40]
        || packed_end + 1 != 0x4000 + data.len() - 2
        || !(0x4000..0x8000).contains(&unpacked_end)
    {
        return Err(DecodeError::Invalid);
    }
    let mut out = alloc::vec![0u8; 0x8000 - 0x4000];
    // The stored end is exclusive.
    let mut end = unpacked_end - 0x4000;
    let packed = &data[6..];
    let mut pos = packed.len();
    let mut next = || {
        pos = pos.checked_sub(1)?;
        Some(packed[pos])
    };
    while end > 4 {
        let Some(byte) = next() else {
            break;
        };
        if byte == 0 {
            let count = usize::from(next().ok_or(DecodeError::Invalid)?);
            let value = next().ok_or(DecodeError::Invalid)?;
            let start = end.saturating_sub(count);
            out[start..end].fill(value);
            end = start;
        } else {
            for _ in 0..usize::from(byte) - 1 {
                let Some(value) = next() else {
                    break;
                };
                end = end.checked_sub(1).ok_or(DecodeError::Invalid)?;
                out[end] = value;
            }
        }
    }
    Ok(with_header(out))
}

/// FLI Editor: FLI Graph memory map with a fixed 17665-byte size
/// (reverse engineered from samples, compared against `recoil2png`).
pub(super) fn decode_fed(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 17665 {
        return Err(DecodeError::Invalid);
    }
    FLI_GRAPH.decode(data)
}
