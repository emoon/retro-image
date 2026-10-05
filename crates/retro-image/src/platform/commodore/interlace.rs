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
//! | True Paint, self-running packed | reverse engineered from 5 samples: BASIC `SYS` stub, a 256-byte depacker holding the flag table, and the unpacked picture in True Paint's memory map moved down by `$8000`; see `unpack::flag_table_rle`. Checked pixel for pixel against `recoil2png` on the unpacked data |
//! | Interlace Hires Editor (IHE) | reverse engineered from 1 sample by mutating bytes and watching `recoil2png`: two bare bitmaps at `$2000` and `$4000`, set bits black and clear bits gray (`$0C`) in both frames |
//! | Multi-Lace Editor (MLE) | reverse engineered from 1 sample by mutating bytes and watching `recoil2png`: two 2048-byte multicolor bitmaps at `$2000` and `$2800` (6 rows of cells and 16 cells of the 7th, 56 lines), fixed colors; the first frame is shown one pixel to the right |
//! | Interlaced Logo Editor (ILE) | reverse engineered by probing `recoil2png` with synthetic 4098-byte files (no sample file exists): two 2048-byte multicolor frames of 6 rows of cells; four bytes at the end of the second frame hold the background and the `01`, `10` and `11` colors; the first frame is shown one pixel to the right |
//! | Hires-Interlace (HLF) | CB "Hires-Interlace v1.0"; which screen RAM pairs with which bitmap checked against `recoil2png` output |

use super::bitmap::{Hires, Multicolor};
use super::prg::Prg;
use super::unpack::flag_table_rle;
use super::vic2::{BITMAP_LEN, Bitmap, Frame, SCREEN_LEN};
use crate::{DecodeError, Image};

/// Blends two frames; `shift` moves the second one right by a hires pixel,
/// bringing in the given background color at the left edge.
fn blend(
    first: Option<Frame>,
    second: Option<Frame>,
    shift: Option<u8>,
) -> Result<Image, DecodeError> {
    let (first, second) = first.zip(second).ok_or(DecodeError::Unrecognized)?;
    let second = match shift {
        Some(background) => second.shift_right(background),
        None => second,
    };
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
    true_paint(data)
}

/// Size of the plain True Paint image without its load address.
const TRUE_PAINT_BODY: usize = 19432;
/// BASIC `2059 SYS` line and the start of the loader that follows it.
const PACKED_STUB: [u8; 17] = [
    0x01, 0x08, 0x0b, 0x08, 0x09, 0x00, 0x9e, 0x32, 0x30, 0x35, 0x39, 0x00, 0xa2, 0x00, 0x78, 0xbd,
    0x1c,
];
/// Offsets in the packed file: flag table, its value table, and the payload,
/// which starts with the stream's end code (read last).
const FLAGS_AT: usize = 0x81;
const VALUES_AT: usize = 0x8a;
const PAYLOAD_AT: usize = 0x8e;

/// True Paint saved as a self-running program: the picture is packed
/// backwards after a depacker stub. The end code (`00` and the fifth
/// flag) at `$8E` is checked before unpacking.
pub(super) fn decode_true_paint_packed(data: &[u8]) -> Result<Image, DecodeError> {
    let bad = DecodeError::Unrecognized;
    if data.len() < PAYLOAD_AT + 2
        || data.len() >= 19434
        || data[..PACKED_STUB.len()] != PACKED_STUB
    {
        return Err(bad);
    }
    let flags: &[u8; 9] = data[FLAGS_AT..FLAGS_AT + 9].try_into().map_err(|_| bad)?;
    let values: &[u8; 4] = data[VALUES_AT..VALUES_AT + 4].try_into().map_err(|_| bad)?;
    if data[PAYLOAD_AT..PAYLOAD_AT + 2] != [0, flags[4]] {
        return Err(bad);
    }
    // The output is a 130-byte viewer, then the image.
    let unpacked =
        flag_table_rle(&data[PAYLOAD_AT..], flags, values, 2 * TRUE_PAINT_BODY).ok_or(bad)?;
    let body = unpacked
        .len()
        .checked_sub(TRUE_PAINT_BODY)
        .map(|start| &unpacked[start..])
        .ok_or(bad)?;
    true_paint(&super::bitmap::with_header(body.to_vec()))
}

fn true_paint(data: &[u8]) -> Result<Image, DecodeError> {
    let prg = Prg::new(data, 0x9c00);
    let background = prg.byte(0x9fe8).ok_or(DecodeError::Unrecognized)?;
    blend(
        TRUE_PAINT[0].frame(&prg),
        TRUE_PAINT[1].frame(&prg),
        Some(background),
    )
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

/// Drazlace, unpacked.
pub(super) fn decode_drazlace(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != DRAZLACE[0].sizes[0] {
        return Err(DecodeError::Unrecognized);
    }
    decode_drazlace_unchecked(data)
}

/// Drazlace, packed behind a `DRAZLACE! 1.0` header.
pub(super) fn decode_drazlace_packed(data: &[u8]) -> Result<Image, DecodeError> {
    let unpacked = super::bitmap::draz_unpack(data, &[b"DRAZLACE! 1.0"], DRAZLACE_LEN)?;
    decode_drazlace_unchecked(&unpacked)
}

fn decode_drazlace_unchecked(data: &[u8]) -> Result<Image, DecodeError> {
    let prg = Prg::new(data, 0x5800);
    let shift = prg.byte(0x7f42).ok_or(DecodeError::Unrecognized)? != 0;
    let shift = shift.then_some(prg.byte(0x7f40).ok_or(DecodeError::Unrecognized)?);
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
        None,
    )
}

const HIRESLACE: [Hires; 2] = [
    Hires {
        load: 0x4000,
        sizes: &[32770],
        bitmap: 0x4000,
        screen: 0x6000,
    },
    Hires {
        load: 0x4000,
        sizes: &[32770],
        bitmap: 0xa000,
        screen: 0x8000,
    },
];

/// Hireslace Editor (Hires-Lace v1.5).
pub(super) fn decode_hireslace(data: &[u8]) -> Result<Image, DecodeError> {
    if !HIRESLACE[0].sizes.contains(&data.len()) {
        return Err(DecodeError::Unrecognized);
    }
    let prg = Prg::new(data, 0x4000);
    blend(HIRESLACE[0].frame(&prg), HIRESLACE[1].frame(&prg), None)
}

/// Interlace Hires Editor: two 8000-byte bitmaps at `$2000` and `$4000`
/// (the 192 bytes between them are unused), with fixed colors.
pub(super) fn decode_interlace_hires_editor(data: &[u8]) -> Result<Image, DecodeError> {
    const LEN: usize = 2 + 0x3f40;
    const SECOND: usize = 2 + 0x2000;
    if data.len() != LEN || data[..2] != [0x00, 0x20] {
        return Err(DecodeError::Unrecognized);
    }
    // Set bits use the screen's high nibble (black), clear bits its low one.
    let screen = [0x0c; SCREEN_LEN];
    let frame = |start: usize| {
        Frame::hires(
            &Bitmap::hires(&data[start..start + BITMAP_LEN], &screen),
            200,
        )
    };
    blend(frame(2), frame(SECOND), None)
}

/// Multi-Lace Editor: two multicolor bitmaps of 256 cells each (the cells
/// after them, up to 7 rows, are blank) at `$2000` and `$2800`, drawn in
/// fixed colors: `01` brown, `10` orange, `11` green on black. The first
/// frame is shifted one pixel right.
pub(super) fn decode_multi_lace(data: &[u8]) -> Result<Image, DecodeError> {
    const FRAME_LEN: usize = 0x800;
    const HEIGHT: usize = 56;
    if data.len() != 2 + 2 * FRAME_LEN || data[..2] != [0x00, 0x20] {
        return Err(DecodeError::Unrecognized);
    }
    let screen = [0x98; SCREEN_LEN];
    let color = [5; SCREEN_LEN];
    let frame = |start: usize| {
        let mut bitmap = data[start..start + FRAME_LEN].to_vec();
        bitmap.resize(HEIGHT / 8 * 40 * 8, 0);
        Frame::multicolor(&Bitmap::multicolor(&bitmap, &screen, &color, 0), HEIGHT)
    };
    blend(frame(2 + FRAME_LEN), frame(2), Some(0))
}

/// Interlaced Logo Editor: two multicolor logo frames of 40×6 cells (1920
/// bytes each, followed by 128 bytes that only matter in the second frame).
/// The second frame ends with the colors: bytes 2044 to 2047 are the
/// background and the colors of bit pairs `01`, `10` and `11` (the last
/// one is a color RAM color, so 0 to 7). The first frame is shown one
/// pixel to the right.
pub(super) fn decode_interlaced_logo_editor(data: &[u8]) -> Result<Image, DecodeError> {
    const FRAME_LEN: usize = 0x800;
    const BITMAP_LEN: usize = 40 * 6 * 8;
    const HEIGHT: usize = 48;
    const COLORS_AT: usize = 2 + FRAME_LEN + 2044;
    if data.len() != 2 + 2 * FRAME_LEN {
        return Err(DecodeError::Unrecognized);
    }
    let [background, multi1, multi2, color_ram] = [0, 1, 2, 3].map(|i| data[COLORS_AT + i]);
    let screen = [multi1 << 4 | multi2 & 15; SCREEN_LEN];
    let color = [color_ram & 7; SCREEN_LEN];
    let frame = |start: usize| {
        Frame::multicolor(
            &Bitmap::multicolor(
                &data[start..start + BITMAP_LEN],
                &screen,
                &color,
                background & 15,
            ),
            HEIGHT,
        )
    };
    blend(frame(2 + FRAME_LEN), frame(2), Some(background & 15))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel(image: &Image, x: usize, y: usize) -> [u8; 3] {
        let at = (y * image.width() as usize + x) * 3;
        image.rgb()[at..at + 3].try_into().unwrap()
    }

    fn blended(a: u8, b: u8) -> [u8; 3] {
        let image = Image::blend(&[
            &super::super::vic2::image(1, 1, alloc::vec![a]),
            &super::super::vic2::image(1, 1, alloc::vec![b]),
        ]);
        pixel(&image, 0, 0)
    }

    #[test]
    fn logo_editor_colours_and_shift() {
        let mut data = alloc::vec![0u8; 4098];
        // Second frame: bit pair `11` at the left edge, whose color is the
        // last of the four color bytes (color RAM, so 3 bits: 0x0b is 3).
        data[2 + 0x800] = 0b1100_0000;
        data[2 + 0x800 + 2047] = 0x0b;
        let image = decode_interlaced_logo_editor(&data).unwrap();
        assert_eq!((image.width(), image.height()), (320, 48));
        // The first frame is shifted right, so the background fills its
        // left edge and the second frame's color blends with it.
        assert_eq!(pixel(&image, 0, 0), blended(3, 0));
        assert_eq!(pixel(&image, 2, 0), blended(0, 0));
        assert!(decode_interlaced_logo_editor(&data[..4097]).is_err());
    }

    #[test]
    fn hireslace_uses_both_screens() {
        let mut data = alloc::vec![0u8; 32770];
        data[2 + 0x4000] = 0x20; // second screen: set color 2
        data[2 + 0x6000] = 0x80; // second bitmap: first pixel set
        let image = decode_hireslace(&data).unwrap();
        assert_eq!((image.width(), image.height()), (320, 200));
        assert_eq!(pixel(&image, 0, 0), blended(0, 2));
        assert_eq!(pixel(&image, 1, 0), blended(0, 0));
        assert!(decode_hireslace(&data[..32769]).is_err());
    }
}
