//! SymbOS graphics (SGX).
//!
//! Sources: SGX chunks, pixel encodings and fixed palettes: SymbOS wiki,
//! <https://github.com/Prodatron/symbos-wiki/wiki/Format-SGX-(Graphic)>
//! (documentation only). The 4-bit to 8-bit level
//! mapping (0, 0x80, 0xFF) and grey levels: observed from `recoil2png`
//! output. ZX0-compressed chunks are not supported (no sample).

use alloc::vec::Vec;

use crate::{DecodeError, Image};

/// One SGX graphic part, placed at (`x`, `y`) on the canvas.
struct Part<'a> {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    line_bytes: usize,
    sixteen_colors: bool,
    pixels: &'a [u8],
}

/// SGX: a sequence of graphic parts laid out left to right; 255 starts a
/// new line of parts below, 0 (or the end of the file) ends the picture.
pub(super) fn decode_sgx(data: &[u8]) -> Result<Image, DecodeError> {
    let mut parts = Vec::new();
    let (mut x, mut y, mut line_height) = (0, 0, 0);
    let (mut width, mut height) = (0, 0);
    let mut rest = data;
    while let Some(&kind) = rest.first() {
        let (header_len, line_bytes, part_width, part_height, sixteen_colors) = match kind {
            0 => break,
            255 => {
                rest = rest.get(3..).ok_or(DecodeError::Unrecognized)?;
                y += line_height;
                (x, line_height) = (0, 0);
                continue;
            }
            1..=63 => {
                let header = rest.get(..3).ok_or(DecodeError::Unrecognized)?;
                let (w, h) = (usize::from(header[1]), usize::from(header[2]));
                (3, usize::from(kind), w, h, false)
            }
            64 => {
                let header = rest.get(..8).ok_or(DecodeError::Unrecognized)?;
                let word = |i: usize| usize::from(u16::from_le_bytes([header[i], header[i + 1]]));
                let sixteen_colors = match header[1] {
                    0 => false,
                    5 => true,
                    _ => return Err(DecodeError::Unrecognized),
                };
                (8, word(2), word(4), word(6), sixteen_colors)
            }
            // Bit 7 marks ZX0-compressed parts.
            _ => return Err(DecodeError::Unrecognized),
        };
        let pixels_per_byte = if sixteen_colors { 2 } else { 4 };
        if part_width == 0 || part_height == 0 || part_width > line_bytes * pixels_per_byte {
            return Err(DecodeError::Unrecognized);
        }
        let len = line_bytes * part_height;
        let pixels = rest
            .get(header_len..header_len + len)
            .ok_or(DecodeError::Unrecognized)?;
        parts.push(Part {
            x,
            y,
            width: part_width,
            height: part_height,
            line_bytes,
            sixteen_colors,
            pixels,
        });
        x += part_width;
        line_height = line_height.max(part_height);
        width = width.max(x);
        height = height.max(y + part_height);
        rest = &rest[header_len + len..];
    }
    // Parts hold at most 4 pixels per byte; a canvas far larger than that
    // would be mostly gaps, so treat it as corrupt rather than allocate it.
    if parts.is_empty() || width * height > 8 * data.len() {
        return Err(DecodeError::Unrecognized);
    }
    let mut image = Image::new(width as u32, height as u32);
    for part in &parts {
        draw_part(&mut image, part);
    }
    Ok(image)
}

/// Fixed 16-colour palette (CPC Plus 4-bit RGB, 8 shown as 0x80).
const SGX_COLORS: [u32; 16] = [
    0xffff80, 0x000000, 0xff8000, 0x800000, 0x00ffff, 0x000080, 0x8080ff, 0x0000ff, 0xffffff,
    0x008000, 0x00ff00, 0xff00ff, 0xffff00, 0x808080, 0xff8080, 0xff0000,
];
/// Fixed 4-colour palette: white, black, light grey, dark grey.
const SGX_GREYS: [u32; 4] = [0xffffff, 0x000000, 0xaaaaaa, 0x555555];

fn draw_part(image: &mut Image, part: &Part) {
    for y in 0..part.height {
        let line = &part.pixels[y * part.line_bytes..][..part.line_bytes];
        for x in 0..part.width {
            let color = if part.sixteen_colors {
                let byte = line[x / 2];
                let pen = if x % 2 == 0 { byte >> 4 } else { byte & 15 };
                SGX_COLORS[usize::from(pen)]
            } else {
                // CPC mode 1: pixel n has its low bit at 7 - n, high bit at 3 - n.
                let byte = line[x / 4];
                let n = x % 4;
                let pen = (byte >> (7 - n)) & 1 | ((byte >> (3 - n)) & 1) << 1;
                SGX_GREYS[usize::from(pen)]
            };
            image.set((part.x + x) as u32, (part.y + y) as u32, color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sgx_places_parts_side_by_side() {
        let mut data = alloc::vec![1, 4, 1, 0x80];
        data.extend_from_slice(&[1, 4, 1, 0x08, 0, 0, 0]);
        let image = decode_sgx(&data).unwrap();
        assert_eq!((image.width(), image.height()), (8, 1));
        assert_eq!(&image.rgb()[..3], &[0, 0, 0]);
        assert_eq!(&image.rgb()[12..15], &[0xaa, 0xaa, 0xaa]);
    }
}
