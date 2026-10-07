//! Screens with a painted border, shown on a 384x304 canvas with the
//! 256x192 screen at (64, 64).
//!
//! Sources:
//! - BSC, BMC4 and BSP layouts (border byte packing, BSP header and border
//!   RLE): SpectraLab `ZX_SPECTRUM_GRAPHICS_GUIDE.md` (MIT), sections BSC,
//!   BMC4 and BSP, <https://github.com/Bedazzle/SpectraLab/blob/main/ZX_SPECTRUM_GRAPHICS_GUIDE.md>;
//!   zx-image README (CC0), <https://github.com/moroz1999/zx-image>.
//! - Border colors at normal intensity: observed from `recoil2png` output.

use super::screen::{
    ATTRIBUTES_LEN, BITMAP_LEN, COLUMNS, Frame, HEIGHT, SCR_LEN, WIDTH, attribute_color,
    bitmap_byte, blend, color, draw_scr,
};
use crate::{DecodeError, Image};

const CANVAS_WIDTH: usize = 384;
const CANVAS_HEIGHT: usize = 304;
const LEFT: usize = 64;
const TOP: usize = 64;
const BOTTOM: usize = 48;
/// Packed border: 64 top rows of 24 bytes, 192 side rows of 4 + 4 bytes,
/// 48 bottom rows of 24 bytes. Each byte holds two 8-pixel blocks, bits 2-0
/// the left one and bits 5-3 the right one.
const BORDER_LEN: usize = (TOP + BOTTOM) * 24 + HEIGHT * 8;

/// BSC: a 6912-byte screen followed by the packed border.
pub(super) fn decode_bsc(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != SCR_LEN + BORDER_LEN {
        return Err(DecodeError::Invalid);
    }
    let (scr, border) = data.split_at(SCR_LEN);
    let mut frame = Frame::new(CANVAS_WIDTH, CANVAS_HEIGHT)?;
    draw_packed_border(&mut frame, border);
    draw_scr(&mut frame, LEFT, TOP, scr);
    frame.into_image()
}

/// BMC4: interleaved bitmap, attributes for the upper and the lower 4 lines
/// of each character cell, then the packed border.
pub(super) fn decode_bmc4(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != BITMAP_LEN + 2 * ATTRIBUTES_LEN + BORDER_LEN {
        return Err(DecodeError::Invalid);
    }
    let (bitmap, rest) = data.split_at(BITMAP_LEN);
    let (attributes, border) = rest.split_at(2 * ATTRIBUTES_LEN);
    let mut frame = Frame::new(CANVAS_WIDTH, CANVAS_HEIGHT)?;
    draw_packed_border(&mut frame, border);
    frame.draw_screen(
        LEFT,
        TOP,
        |column, y| bitmap_byte(bitmap, column, y),
        |column, y, ink| {
            let bank = (y / 4) % 2 * ATTRIBUTES_LEN;
            attribute_color(attributes[bank + y / 8 * COLUMNS + column], ink)
        },
    );
    frame.into_image()
}

const BSP_HEADER_LEN: usize = 70;
const BSP_GIGASCREEN: u8 = 0x80;
const BSP_BORDER: u8 = 0x40;

/// BSP: `bsp` header (config byte at 3: bit 7 two screens, bit 6 border
/// data), one or two 6912-byte screens and run-length coded borders. With
/// two screens, a u16 offset of the second border precedes the screens.
/// Only files with border data are accepted.
pub(super) fn decode_bsp(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() < BSP_HEADER_LEN || !data.starts_with(b"bsp") {
        return Err(DecodeError::Invalid);
    }
    let config = data[3];
    if config & !(BSP_GIGASCREEN | BSP_BORDER) != 0 || config & BSP_BORDER == 0 {
        return Err(DecodeError::Invalid);
    }
    let frames = if config & BSP_GIGASCREEN != 0 {
        let second_border = data
            .get(BSP_HEADER_LEN..BSP_HEADER_LEN + 2)
            .map(|b| usize::from(u16::from_le_bytes([b[0], b[1]])))
            .ok_or(DecodeError::Invalid)?;
        let screens = BSP_HEADER_LEN + 2;
        let borders = screens + 2 * SCR_LEN;
        if second_border < borders || second_border > data.len() {
            return Err(DecodeError::Invalid);
        }
        alloc::vec![
            bsp_frame(&data[screens..][..SCR_LEN], &data[borders..second_border])?,
            bsp_frame(
                &data[screens + SCR_LEN..][..SCR_LEN],
                &data[second_border..]
            )?,
        ]
    } else {
        let borders = BSP_HEADER_LEN + SCR_LEN;
        let scr = data
            .get(BSP_HEADER_LEN..borders)
            .ok_or(DecodeError::Invalid)?;
        alloc::vec![bsp_frame(scr, &data[borders..])?]
    };
    Ok(blend(&frames))
}

fn bsp_frame(scr: &[u8], border: &[u8]) -> Result<Frame, DecodeError> {
    let mut frame = Frame::new(CANVAS_WIDTH, CANVAS_HEIGHT)?;
    draw_rle_border(&mut frame, border)?;
    draw_scr(&mut frame, LEFT, TOP, scr);
    Ok(frame)
}

/// BSP border runs, one per byte: color in bits 2-0, length code in bits
/// 7-3. Code 0 runs to the end of the segment (the line, or the left side
/// of a screen row), 1 takes the half-length from the next byte, 2 is 24
/// pixels, and higher codes are `(code + 13) * 2` pixels. The runs must use
/// up `border` exactly, as in every sample, which keeps the `bsp` signature
/// strict.
fn draw_rle_border(frame: &mut Frame, border: &[u8]) -> Result<(), DecodeError> {
    let mut bytes = border.iter().copied();
    let (mut x, mut y) = (0, 0);
    while y < CANVAS_HEIGHT {
        let byte = bytes.next().ok_or(DecodeError::Invalid)?;
        let screen_row = (TOP..TOP + HEIGHT).contains(&y);
        let segment_end = if screen_row && x < LEFT {
            LEFT
        } else {
            CANVAS_WIDTH
        };
        let mut length = match byte >> 3 {
            0 => segment_end - x,
            1 => 2 * usize::from(bytes.next().ok_or(DecodeError::Invalid)?),
            2 => 24,
            code => (usize::from(code) + 13) * 2,
        };
        let color = color(byte & 7, false);
        while length > 0 && y < CANVAS_HEIGHT {
            frame.set(x, y, color);
            length -= 1;
            x += 1;
            if x == LEFT && (TOP..TOP + HEIGHT).contains(&y) {
                x += WIDTH;
            }
            if x == CANVAS_WIDTH {
                x = 0;
                y += 1;
            }
        }
    }
    if bytes.next().is_some() {
        return Err(DecodeError::Invalid);
    }
    Ok(())
}

fn draw_packed_border(frame: &mut Frame, border: &[u8]) {
    let mut bytes = border.iter();
    for y in 0..CANVAS_HEIGHT {
        let mut x = 0;
        while x < CANVAS_WIDTH {
            if (TOP..TOP + HEIGHT).contains(&y) && x == LEFT {
                x += WIDTH;
            }
            let byte = bytes.next().copied().unwrap_or(0);
            frame.fill(x, y, 8, 1, color(byte & 7, false));
            frame.fill(x + 8, y, 8, 1, color((byte >> 3) & 7, false));
            x += 16;
        }
    }
}
