//! C64 OS Commodore Grafix (`.cgx`): RIFF files holding one or more bitmap
//! frames arranged in a matrix.
//!
//! Sources:
//! - Greg Naçu, "Image File Formats", <https://c64os.com/post/imageformats>:
//!   RIFF/`CGFX` container, `FRMT` (matrix, animation and data info) and
//!   `DATA` chunks, frame component order.
//! - Matrix frames drawn side by side, row by row, and the frame size taken
//!   as the `DATA` size divided by the frame count, observed from sample
//!   files and `recoil2png` output. Only 40×25-cell hires and multicolor
//!   bitmap frames without FLI or interlace attributes are supported.

use super::vic2::{BITMAP_LEN, Bitmap, Frame, SCREEN_LEN, rgb};
use crate::image::check_size;
use crate::{DecodeError, Image};

/// Finds a RIFF chunk in `chunks`.
fn chunk<'a>(mut chunks: &'a [u8], id: &[u8; 4]) -> Option<&'a [u8]> {
    while chunks.len() >= 8 {
        let size = u32::from_le_bytes(chunks[4..8].try_into().ok()?) as usize;
        let body = chunks.get(8..size.checked_add(8)?)?;
        if &chunks[..4] == id {
            return Some(body);
        }
        chunks = chunks
            .get(size.checked_add(8 + (size & 1))?..)
            .unwrap_or(&[]);
    }
    None
}

pub(super) fn decode_cgx(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() < 12 || &data[..4] != b"RIFF" || &data[8..12] != b"CGFX" {
        return Err(DecodeError::Unrecognized);
    }
    let chunks = &data[12..];
    let format = chunk(chunks, b"FRMT").ok_or(DecodeError::Unrecognized)?;
    let frames = chunk(chunks, b"DATA").ok_or(DecodeError::Unrecognized)?;
    // Matrix rows and columns, frame count, 25×40-cell bitmap frames
    // without attributes.
    let &[rows, columns, _, _, count, _, _, _, ref frame_info @ ..] = format else {
        return Err(DecodeError::Unrecognized);
    };
    let &[25, 40, mode @ (3 | 4), 0] = frame_info else {
        return Err(DecodeError::Unrecognized);
    };
    let (rows, columns, count) = (usize::from(rows), usize::from(columns), usize::from(count));
    if count == 0 || rows * columns < count || frames.len() % count != 0 {
        return Err(DecodeError::Unrecognized);
    }
    let frame_len = frames.len() / count;
    if frame_len < BITMAP_LEN + 2 * SCREEN_LEN + 2 {
        return Err(DecodeError::Unrecognized);
    }
    let (width, height) = (columns * 320, rows * 200);
    check_size(width, height)?;
    let mut image = Image::new(width as u32, height as u32);
    for (i, frame) in frames.chunks_exact(frame_len).enumerate() {
        let (bitmap, rest) = frame.split_at(BITMAP_LEN);
        let (screen, rest) = rest.split_at(SCREEN_LEN);
        let (color, rest) = rest.split_at(SCREEN_LEN);
        let background = rest[1];
        let frame = if mode == 3 {
            Frame::hires(&Bitmap::hires(bitmap, screen), 200)
        } else {
            Frame::multicolor(&Bitmap::multicolor(bitmap, screen, color, background), 200)
        }
        .ok_or(DecodeError::Unrecognized)?;
        let (left, top) = (i % columns * 320, i / columns * 200);
        for y in 0..200 {
            for x in 0..320 {
                image.set((left + x) as u32, (top + y) as u32, rgb(frame.get(x, y)));
            }
        }
    }
    Ok(image)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn huge_matrix_is_rejected_before_allocating() {
        let frame_len = BITMAP_LEN + 2 * SCREEN_LEN + 2;
        let mut data = b"RIFF\0\0\0\0CGFX".to_vec();
        data.extend(b"FRMT\x0c\0\0\0");
        data.extend([255, 255, 0, 0, 1, 0, 0, 0, 25, 40, 3, 0]);
        data.extend(b"DATA");
        data.extend((frame_len as u32).to_le_bytes());
        data.resize(data.len() + frame_len, 0);
        assert!(decode_cgx(&data).is_err());
    }
}
