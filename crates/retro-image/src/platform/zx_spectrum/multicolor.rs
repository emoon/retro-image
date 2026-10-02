//! Multicolor screens: attributes for cells shorter than 8 pixels, single
//! frame or MultiArtist gigascreen.
//!
//! Sources:
//! - IFL, MLT, MG1/MG2/MG4/MG8 (MGH header and attribute layout):
//!   SpectraLab `ZX_SPECTRUM_GRAPHICS_GUIDE.md` (MIT), sections IFL, MLT and
//!   MGH/Multiartist, <https://github.com/Bedazzle/SpectraLab/blob/main/ZX_SPECTRUM_GRAPHICS_GUIDE.md>;
//!   zx-image README (CC0), <https://github.com/moroz1999/zx-image>.
//! - MC layout (linear bitmap and attributes): zx-image README, confirmed
//!   against `recoil2png` output.

use alloc::vec::Vec;

use super::screen::{
    BITMAP_LEN, COLUMNS, Frame, HEIGHT, WIDTH, attribute_color, bitmap_byte, blend,
};
use crate::{DecodeError, Image};

/// MLT: interleaved bitmap, then 8x1 attributes row by row.
pub(super) fn decode_mlt(data: &[u8]) -> Result<Image, DecodeError> {
    decode_single(data, 1)
}

/// IFL: interleaved bitmap, then 8x2 attributes row by row.
pub(super) fn decode_ifl(data: &[u8]) -> Result<Image, DecodeError> {
    decode_single(data, 2)
}

/// MC: 192 linear bitmap rows of 32 bytes, then 8x1 attributes row by row.
pub(super) fn decode_mc(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 2 * BITMAP_LEN {
        return Err(DecodeError::Unrecognized);
    }
    let (bitmap, attributes) = data.split_at(BITMAP_LEN);
    let mut frame = Frame::new(WIDTH, HEIGHT);
    frame.draw_screen(
        0,
        0,
        |column, y| bitmap[y * COLUMNS + column],
        |column, y, ink| attribute_color(attributes[y * COLUMNS + column], ink),
    );
    Ok(frame.into_image())
}

fn decode_single(data: &[u8], cell_height: usize) -> Result<Image, DecodeError> {
    if data.len() != BITMAP_LEN + attributes_len(cell_height) {
        return Err(DecodeError::Unrecognized);
    }
    let (bitmap, attributes) = data.split_at(BITMAP_LEN);
    Ok(multicolor_frame(bitmap, attributes, cell_height).into_image())
}

fn attributes_len(cell_height: usize) -> usize {
    HEIGHT / cell_height * COLUMNS
}

fn multicolor_frame(bitmap: &[u8], attributes: &[u8], cell_height: usize) -> Frame {
    let mut frame = Frame::new(WIDTH, HEIGHT);
    frame.draw_screen(
        0,
        0,
        |column, y| bitmap_byte(bitmap, column, y),
        |column, y, ink| attribute_color(attributes[y / cell_height * COLUMNS + column], ink),
    );
    frame
}

const MGH_HEADER_LEN: usize = 256;
const MGH_BITMAPS: usize = MGH_HEADER_LEN + 2 * BITMAP_LEN;
/// MG1 attributes: 8x1 for the 16 middle columns, 8x8 for the 8 columns at
/// each side.
const MG1_INNER_LEN: usize = HEIGHT * 16;
const MG1_OUTER_LEN: usize = HEIGHT / 8 * 16;

/// MultiArtist (`MGH` header): two bitmaps and two attribute sets shown as
/// gigascreen. The header's mode byte gives the attribute cell height.
pub(super) fn decode_mgh(data: &[u8], cell_height: u8) -> Result<Image, DecodeError> {
    let header_ok = data.len() >= MGH_BITMAPS
        && data.starts_with(b"MGH")
        && data[3] == 1
        && data[4] == cell_height;
    if !header_ok {
        return Err(DecodeError::Unrecognized);
    }
    let bitmaps = &data[MGH_HEADER_LEN..MGH_BITMAPS];
    let attributes = &data[MGH_BITMAPS..];
    let frames: Vec<Frame> = if cell_height == 1 {
        if attributes.len() != 2 * (MG1_INNER_LEN + MG1_OUTER_LEN) {
            return Err(DecodeError::Unrecognized);
        }
        let (inner, outer) = attributes.split_at(2 * MG1_INNER_LEN);
        (0..2)
            .map(|i| {
                let bitmap = &bitmaps[i * BITMAP_LEN..][..BITMAP_LEN];
                let inner = &inner[i * MG1_INNER_LEN..][..MG1_INNER_LEN];
                let outer = &outer[i * MG1_OUTER_LEN..][..MG1_OUTER_LEN];
                mg1_frame(bitmap, inner, outer)
            })
            .collect()
    } else {
        let len = attributes_len(usize::from(cell_height));
        if attributes.len() != 2 * len {
            return Err(DecodeError::Unrecognized);
        }
        bitmaps
            .as_chunks::<BITMAP_LEN>()
            .0
            .iter()
            .zip(attributes.chunks_exact(len))
            .map(|(bitmap, attributes)| {
                multicolor_frame(bitmap, attributes, usize::from(cell_height))
            })
            .collect()
    };
    Ok(blend(&frames))
}

fn mg1_frame(bitmap: &[u8], inner: &[u8], outer: &[u8]) -> Frame {
    let mut frame = Frame::new(WIDTH, HEIGHT);
    frame.draw_screen(
        0,
        0,
        |column, y| bitmap_byte(bitmap, column, y),
        |column, y, ink| {
            let attribute = match column {
                8..24 => inner[y * 16 + column - 8],
                0..8 => outer[y / 8 * 16 + column],
                _ => outer[y / 8 * 16 + column - 16],
            };
            attribute_color(attribute, ink)
        },
    );
    frame
}
