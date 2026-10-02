//! Timex 2048 hi-colour and hi-res screens, and the ULAplus palette screen.
//!
//! Sources:
//! - Timex modes (hi-colour attribute bitmap at 0x6000, hi-res column
//!   alternation, port 0xFF colour pairs): WoS Timex technical reference,
//!   <https://worldofspectrum.org/faq/reference/tmxreference.htm>.
//! - File sizes (12288 hi-colour, 12289 hi-res, 24578 HRG = two hi-res
//!   screens) and ULAplus SCR (6912 + 64-byte palette): zx-image README (CC0)
//!   (<https://github.com/moroz1999/zx-image>) and SpectraLab `ZX_SPECTRUM_GRAPHICS_GUIDE.md` (MIT), ULA+
//!   section, <https://github.com/Bedazzle/SpectraLab/blob/main/ZX_SPECTRUM_GRAPHICS_GUIDE.md>.
//! - ULAplus GRB332 palette and CLUT selection: ULAplus specification,
//!   <https://sinclair.wiki.zxnet.co.uk/wiki/ULAplus>.
//! - Hi-res colours at bright intensity, rows doubled to 512x384, and the
//!   2-bit blue scaled by 0x55: observed from `recoil2png` output.

use super::screen::{
    BITMAP_LEN, COLUMNS, Frame, HEIGHT, SCR_LEN, WIDTH, attribute_color, bitmap_byte,
    bitmap_offset, blend, rgb_bits,
};
use crate::{DecodeError, Image};

const HICOLOR_LEN: usize = 2 * BITMAP_LEN;
const HIRES_LEN: usize = 2 * BITMAP_LEN + 1;

/// Hi-colour: interleaved bitmap, then 8x1 attributes in the same interleave.
pub(super) fn decode_hicolor(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != HICOLOR_LEN {
        return Err(DecodeError::Unrecognized);
    }
    let (bitmap, attributes) = data.split_at(BITMAP_LEN);
    let mut frame = Frame::new(WIDTH, HEIGHT);
    frame.draw_screen(
        0,
        0,
        |column, y| bitmap_byte(bitmap, column, y),
        |column, y, ink| attribute_color(bitmap_byte(attributes, column, y), ink),
    );
    Ok(frame.into_image())
}

/// Hi-res: 512x192 from two bitmaps, see [`draw_hires`].
pub(super) fn decode_hires(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != HIRES_LEN {
        return Err(DecodeError::Unrecognized);
    }
    Ok(draw_hires(data).into_image().scaled(1, 2))
}

/// HRG: two hi-res screens shown as gigascreen.
pub(super) fn decode_hrg(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 2 * HIRES_LEN {
        return Err(DecodeError::Unrecognized);
    }
    let (first, second) = data.split_at(HIRES_LEN);
    Ok(blend(&[draw_hires(first), draw_hires(second)]).scaled(1, 2))
}

/// One hi-res screen: 8-pixel columns alternate between the two bitmaps;
/// the trailing port 0xFF byte selects the ink (bits 5-3), paper is its
/// complement. Callers show it as 512x384, each row doubled.
fn draw_hires(data: &[u8]) -> Frame {
    let (bitmaps, port) = data.split_at(2 * BITMAP_LEN);
    let ink_index = (port[0] >> 3) & 7;
    let ink = rgb_bits(ink_index, 0xff);
    let paper = rgb_bits(7 - ink_index, 0xff);
    let mut frame = Frame::new(2 * WIDTH, HEIGHT);
    for y in 0..HEIGHT {
        for column in 0..2 * COLUMNS {
            let bitmap = &bitmaps[(column % 2) * BITMAP_LEN..];
            let byte = bitmap[bitmap_offset(y) + column / 2];
            for bit in 0..8 {
                let color = if byte & (0x80 >> bit) != 0 {
                    ink
                } else {
                    paper
                };
                frame.set(column * 8 + bit, y, color);
            }
        }
    }
    frame
}

const ULAPLUS_LEN: usize = SCR_LEN + 64;

/// ULAplus: a 6912-byte screen followed by a 64-entry GRB332 palette.
/// Attribute bits 7-6 pick one of 4 CLUTs of 8 inks then 8 papers.
pub(super) fn decode_ulaplus(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != ULAPLUS_LEN {
        return Err(DecodeError::Unrecognized);
    }
    let (bitmap, rest) = data.split_at(BITMAP_LEN);
    let (attributes, palette) = rest.split_at(COLUMNS * HEIGHT / 8);
    let mut frame = Frame::new(WIDTH, HEIGHT);
    frame.draw_screen(
        0,
        0,
        |column, y| bitmap_byte(bitmap, column, y),
        |column, y, ink| {
            let attribute = attributes[y / 8 * COLUMNS + column];
            let clut = usize::from(attribute >> 6) * 16;
            let entry = if ink {
                attribute & 7
            } else {
                8 | (attribute >> 3) & 7
            };
            grb332(palette[clut + usize::from(entry)])
        },
    );
    Ok(frame.into_image())
}

/// GRB332 palette byte: 3-bit green and red widen by repeating their bits,
/// 2-bit blue by multiplying by 0x55.
pub(super) fn grb332(value: u8) -> u32 {
    let widen3 = |v: u8| u32::from(v << 5 | v << 2 | v >> 1);
    let green = widen3(value >> 5);
    let red = widen3((value >> 2) & 7);
    let blue = u32::from(value & 3) * 0x55;
    red << 16 | green << 8 | blue
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grb332_widens_channels() {
        assert_eq!(grb332(0x00), 0x000000);
        assert_eq!(grb332(0xff), 0xffffff);
        assert_eq!(grb332(0x24), 0x242400);
        assert_eq!(grb332(0x02), 0x0000aa);
    }

    #[test]
    fn hires_alternates_bitmaps_and_uses_complement_paper() {
        let mut data = [0u8; HIRES_LEN];
        data[BITMAP_LEN] = 0x80; // first pixel of the second column
        data[2 * BITMAP_LEN] = 2 << 3; // red ink, cyan paper
        let image = decode_hires(&data).unwrap();
        let pixel = |x: usize| &image.rgb()[x * 3..x * 3 + 3];
        assert_eq!(pixel(0), &[0, 0xff, 0xff]);
        assert_eq!(pixel(8), &[0xff, 0, 0]);
    }
}
