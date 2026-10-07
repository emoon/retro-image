//! Screens in the standard 256x192 attribute mode, single frame or
//! gigascreen/tricolor frame sequences.
//!
//! Sources:
//! - SCR, ATR, IMG, HLR, STL, `.3`: SpectraLab `ZX_SPECTRUM_GRAPHICS_GUIDE.md`
//!   (MIT), sections SCR, 53c/ATR, Gigascreen, HLR, STL and RGB3,
//!   <https://github.com/Bedazzle/SpectraLab/blob/main/ZX_SPECTRUM_GRAPHICS_GUIDE.md>.
//! - LCE (13824 bytes, two screens interlaced to 512x384, first on even
//!   lines, pixels doubled): zx-image README (CC0),
//!   <https://github.com/moroz1999/zx-image>.
//! - ATR dither pattern (`55 AA ...`, first row starting with paper) and the
//!   mono SCR colors: observed from `recoil2png` output.

use super::screen::{
    ATTRIBUTES_LEN, BITMAP_LEN, COLUMNS, Frame, HEIGHT, SCR_LEN, WIDTH, attribute_color,
    bitmap_byte, bitmap_offset, blend, draw_scr,
};
use crate::{DecodeError, Image};

/// Raw 6912-byte screen dump (bitmap then attributes), or a 6144-byte
/// bitmap-only dump shown white on black.
pub(super) fn decode_scr(data: &[u8]) -> Result<Image, DecodeError> {
    match data.len() {
        SCR_LEN => {
            let mut frame = Frame::new(WIDTH, HEIGHT)?;
            draw_scr(&mut frame, 0, 0, data);
            Ok(frame.into_image()?)
        }
        BITMAP_LEN => {
            let mut frame = Frame::new(WIDTH, HEIGHT)?;
            frame.draw_screen(
                0,
                0,
                |column, y| bitmap_byte(data, column, y),
                |_, _, ink| if ink { 0xffffff } else { 0 },
            );
            Ok(frame.into_image()?)
        }
        _ => Err(DecodeError::Invalid),
    }
}

/// 768 attribute bytes over a fixed checkered bitmap.
pub(super) fn decode_atr(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != ATTRIBUTES_LEN {
        return Err(DecodeError::Invalid);
    }
    attribute_frame(data, 8, |_, y| if y % 2 == 0 { 0x55 } else { 0xaa })?.into_image()
}

/// Gigascreen: two 6912-byte screens shown in alternation.
pub(super) fn decode_img(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 2 * SCR_LEN {
        return Err(DecodeError::Invalid);
    }
    let frames = data.as_chunks::<SCR_LEN>().0.iter().map(|scr| {
        let mut frame = Frame::new(WIDTH, HEIGHT)?;
        draw_scr(&mut frame, 0, 0, scr);
        Ok(frame)
    });
    Ok(blend(
        &frames.collect::<Result<alloc::vec::Vec<_>, DecodeError>>()?,
    ))
}

/// LCE (zx-image): two 6912-byte screens shown at once on an interlaced
/// 512x384 display, the first on even lines, pixels doubled horizontally.
pub(super) fn decode_lce(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 2 * SCR_LEN {
        return Err(DecodeError::Invalid);
    }
    let (first, second) = data.split_at(SCR_LEN);
    let field = |scr| {
        let mut frame = Frame::new(WIDTH, HEIGHT)?;
        draw_scr(&mut frame, 0, 0, scr);
        frame.into_image()
    };
    super::super::sam_coupe::interlace(&[field(first)?, field(second)?])
}

const HLR_LEN: usize = 1628;
const HLR_PATTERN: usize = 0x54;
const HLR_ATTRIBUTES: usize = 0x5c;

/// HLR: a Z80 viewer holding an 8-byte cell pattern and two attribute
/// screens shown as gigascreen.
pub(super) fn decode_hlr(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != HLR_LEN {
        return Err(DecodeError::Invalid);
    }
    let pattern = &data[HLR_PATTERN..HLR_ATTRIBUTES];
    let frames = data[HLR_ATTRIBUTES..]
        .as_chunks::<ATTRIBUTES_LEN>()
        .0
        .iter()
        .map(|attributes| attribute_frame(attributes, 8, |_, y| pattern[y % 8]));
    Ok(blend(
        &frames.collect::<Result<alloc::vec::Vec<_>, DecodeError>>()?,
    ))
}

const STL_LEN: usize = 3072;

/// Stellar: two 8x4-cell attribute screens over a fixed `0x0F` bitmap,
/// interleaved in the file two bytes at a time.
pub(super) fn decode_stl(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != STL_LEN {
        return Err(DecodeError::Invalid);
    }
    let frame = |first: usize| {
        let attributes: alloc::vec::Vec<u8> = data
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|group| [group[first], group[first + 1]])
            .collect();
        attribute_frame(&attributes, 4, |_, _| 0x0f)
    };
    Ok(blend(&[frame(0)?, frame(2)?]))
}

/// A frame of attribute cells `cell_height` pixels high, row-major, over a
/// fixed bitmap given by `pattern(column, y)`.
fn attribute_frame(
    attributes: &[u8],
    cell_height: usize,
    pattern: impl Fn(usize, usize) -> u8,
) -> Result<Frame, DecodeError> {
    let mut frame = Frame::new(WIDTH, HEIGHT)?;
    frame.draw_screen(0, 0, pattern, |column, y, ink| {
        attribute_color(attributes[y / cell_height * COLUMNS + column], ink)
    });
    Ok(frame)
}

const TRICOLOR_LEN: usize = 3 * BITMAP_LEN;

/// `.3` tricolor: planes in blue, red, green order (observed from
/// `recoil2png` output).
pub(super) fn decode_3(data: &[u8]) -> Result<Image, DecodeError> {
    decode_tricolor(data, [0x0000ff, 0xff0000, 0x00ff00])
}

/// `.RGB` tricolor: planes in red, green, blue order.
pub(super) fn decode_rgb(data: &[u8]) -> Result<Image, DecodeError> {
    decode_tricolor(data, [0xff0000, 0x00ff00, 0x0000ff])
}

/// Tricolor: three bitmaps shown on successive frames, each lighting one RGB
/// channel at full intensity (`channels` in file order). The channels add up
/// rather than average (observed from `recoil2png` output).
fn decode_tricolor(data: &[u8], channels: [u32; 3]) -> Result<Image, DecodeError> {
    if data.len() != TRICOLOR_LEN {
        return Err(DecodeError::Invalid);
    }
    let mut frame = Frame::new(WIDTH, HEIGHT)?;
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let offset = bitmap_offset(y) + x / 8;
            let color = data
                .as_chunks::<BITMAP_LEN>()
                .0
                .iter()
                .zip(channels)
                .filter(|(plane, _)| plane[offset] & (0x80 >> (x % 8)) != 0)
                .fold(0, |color, (_, channel)| color | channel);
            frame.set(x, y, color);
        }
    }
    frame.into_image()
}
