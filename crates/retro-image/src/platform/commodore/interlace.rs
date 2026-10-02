//! C64 interlace formats: two bitmaps shown on alternate frames, which the
//! eye (and `recoil2png`) blends.
//!
//! Sources (memory maps):
//! - Codebase64 "C64 Graphics File Format Specs" (CB),
//!   <http://codebase.c64.org/doku.php?id=base:c64_grafix_files_specs_list_v0.03>
//! - GoDot loader pages (GD): Drazpaint/Drazlace
//!   <https://www.godot64.de/german/l_draz.htm>, TruePaint
//!   <https://www.godot64.de/german/l_trupnt.htm>
//! - Interlace display: <http://www.studiostyle.sk/dmagic/gallery/gfxmodes.htm>
//!
//! | Format | Sources |
//! |---|---|
//! | Drazlace (DRL, DLP) | CB "Drazlace", GD Draz |
//! | True Paint (MCI) | CB "True Paint", GD TruePaint |
//! | Hires-Interlace (HLF) | CB "Hires-Interlace v1.0"; which screen RAM pairs with which bitmap checked against `recoil2png` output |

use super::bitmap::{Hires, Multicolor};
use super::prg::Prg;
use super::vic2::Frame;
use crate::{DecodeError, Image};

/// Blends two frames; `shift` moves the second one right by a hires pixel.
fn blend(first: Option<Frame>, second: Option<Frame>, shift: bool) -> Result<Image, DecodeError> {
    let (first, second) = first.zip(second).ok_or(DecodeError::Unrecognized)?;
    let second = if shift { second.shift_right() } else { second };
    Ok(first.blend(&second, 0))
}

const TRUE_PAINT: [Multicolor; 2] = [
    Multicolor {
        load: 0x9c00,
        sizes: &[19434],
        bitmap: 0xa000,
        screen: 0x9c00,
        color: 0xe400,
        background: 0x9fe8,
    },
    Multicolor {
        load: 0x9c00,
        sizes: &[19434],
        bitmap: 0xc000,
        screen: 0xe000,
        color: 0xe400,
        background: 0x9fe8,
    },
];

/// True Paint: MCI, the second frame shifted by one pixel.
pub(super) fn decode_true_paint(data: &[u8]) -> Result<Image, DecodeError> {
    if !TRUE_PAINT[0].sizes.contains(&data.len()) {
        return Err(DecodeError::Unrecognized);
    }
    let prg = Prg::new(data, 0x9c00);
    blend(TRUE_PAINT[0].frame(&prg), TRUE_PAINT[1].frame(&prg), true)
}

const DRAZLACE: [Multicolor; 2] = [
    Multicolor {
        load: 0x5800,
        sizes: &[18242],
        bitmap: 0x6000,
        screen: 0x5c00,
        color: 0x5800,
        background: 0x7f40,
    },
    Multicolor {
        load: 0x5800,
        sizes: &[18242],
        bitmap: 0x8000,
        screen: 0x5c00,
        color: 0x5800,
        background: 0x7f40,
    },
];
/// Bytes from `$5800` to the end of the second bitmap.
const DRAZLACE_LEN: usize = 0x4740;

/// Drazlace: plain, or packed behind a `DRAZLACE! 1.0` header.
pub(super) fn decode_drazlace(data: &[u8]) -> Result<Image, DecodeError> {
    let unpacked;
    let data = if data.len() == DRAZLACE[0].sizes[0] {
        data
    } else {
        unpacked = super::bitmap::draz_unpack(data, b"DRAZLACE! 1.0", DRAZLACE_LEN)?;
        &unpacked
    };
    let prg = Prg::new(data, 0x5800);
    let shift = prg.byte(0x7f42).ok_or(DecodeError::Unrecognized)? != 0;
    blend(DRAZLACE[0].frame(&prg), DRAZLACE[1].frame(&prg), shift)
}

const HIRES_INTERLACE: [Hires; 2] = [
    Hires {
        load: 0x2000,
        sizes: &[24578],
        bitmap: 0x2000,
        screen: 0x4800,
    },
    Hires {
        load: 0x2000,
        sizes: &[24578],
        bitmap: 0x6000,
        screen: 0x4400,
    },
];

/// Hires-Interlace (Feniks).
pub(super) fn decode_hires_interlace(data: &[u8]) -> Result<Image, DecodeError> {
    if !HIRES_INTERLACE[0].sizes.contains(&data.len()) {
        return Err(DecodeError::Unrecognized);
    }
    let prg = Prg::new(data, 0x2000);
    blend(
        HIRES_INTERLACE[0].frame(&prg),
        HIRES_INTERLACE[1].frame(&prg),
        false,
    )
}
