//! Commodore 128 VDC pictures.
//!
//! Sources:
//! - VBM (VDC BitMap): Craig Bruce's format description,
//!   <http://csbruce.com/cbm/postings/csc19950906-1.txt>, and
//!   <http://fileformats.archiveteam.org/wiki/VBM_(VDC_BitMap)>. Only the
//!   uncompressed version 2 is supported (no version 3 samples to check
//!   against); set bits are black.
//! - BASIC 8 / IPaint `brus` pictures: GoDot BASIC 8 saver and IPaint
//!   saver pages, <https://www.godot64.de/german/s_b8mode1.htm>,
//!   <https://www.godot64.de/german/s_ipaint.htm> (header, color modes,
//!   attribute row interleave, RLE). The `COLR` chunk tag between the
//!   separately packed bitmap and colors was found in sample files; only
//!   packed files with color (modes 1-4) are supported. The attribute row
//!   interleave applies only to interlaced (taller than 200 lines)
//!   pictures: non-interlaced samples (BASIC 8 mode 1 slideshow pictures,
//!   152-line IPaint clip art) render with color fringes at every shape
//!   edge when interleaved and cleanly with one attribute row per cell.

use crate::image::{BitOrder, check_size};
use crate::{DecodeError, Image};
use alloc::vec::Vec;

/// VDC RGBI color: bit 3 red, bit 2 green, bit 1 blue, bit 0 intensity;
/// dark yellow shows as brown. Levels observed from `recoil2png` output.
pub(super) fn rgbi(color: u8) -> u32 {
    if color == 12 {
        return 0xaa5500;
    }
    let intensity = if color & 1 != 0 { 0x55 } else { 0 };
    let channel = |bit: u8| {
        if color & bit != 0 {
            0xaa + intensity
        } else {
            intensity
        }
    };
    channel(8) << 16 | channel(4) << 8 | channel(2)
}

/// Unpacks BASIC 8 RLE until `len` bytes: a control byte with bit 7 set
/// repeats the next byte (bits 0-6) times, otherwise that many literal
/// bytes follow. Returns the data and the number of bytes consumed.
fn unpack(packed: &[u8], len: usize) -> Option<(Vec<u8>, usize)> {
    let mut out = Vec::with_capacity(len);
    let mut i = 0;
    while out.len() < len {
        let control = *packed.get(i)?;
        let count = usize::from(control & 0x7f);
        if control & 0x80 != 0 {
            out.extend(core::iter::repeat_n(*packed.get(i + 1)?, count));
            i += 2;
        } else {
            out.extend_from_slice(packed.get(i + 1..i + 1 + count)?);
            i += 1 + count;
        }
    }
    (out.len() == len).then_some((out, i))
}

/// BASIC 8 / IPaint `brus` picture, packed, color modes 1-4: load
/// address, 16-byte header, the bitmap (width × height bytes), `COLR`,
/// then one background/foreground byte per attribute cell. In interlaced
/// pictures even and odd lines (the two fields) use alternate attribute
/// rows; otherwise each attribute row covers one cell's lines.
pub(super) fn decode_brus(data: &[u8]) -> Result<Image, DecodeError> {
    let header = data.get(..18).ok_or(DecodeError::Invalid)?;
    if &header[2..7] != b"BRUS\x04" || header[10] != 1 {
        return Err(DecodeError::Invalid);
    }
    let cell_height = match header[11] {
        1 => 2,
        2 => 4,
        3 => 8,
        4 => 16,
        _ => return Err(DecodeError::Invalid),
    };
    let columns = usize::from(header[12]);
    let height = usize::from(u16::from_le_bytes([header[13], header[14]]));
    let interlaced = height > 200;
    let attribute_rows = height.div_ceil(2 * cell_height) * 2;
    // The bitmap must fit in the VDC's 64K.
    if columns == 0 || height == 0 || columns * height > 0x10000 {
        return Err(DecodeError::Invalid);
    }
    let packed = &data[18..];
    let (bitmap, used) = unpack(packed, columns * height).ok_or(DecodeError::Invalid)?;
    let packed = packed[used..]
        .strip_prefix(b"COLR")
        .ok_or(DecodeError::Invalid)?;
    let (colors, _) = unpack(packed, columns * attribute_rows).ok_or(DecodeError::Invalid)?;
    let mut image = Image::new((columns * 8) as u32, height as u32)?;
    for y in 0..height {
        let row = if interlaced {
            y / (2 * cell_height) * 2 + y % 2
        } else {
            y / cell_height
        };
        for x in 0..columns * 8 {
            let attribute = colors[row * columns + x / 8];
            let set = bitmap[y * columns + x / 8] & (0x80 >> (x % 8)) != 0;
            let color = if set { attribute & 15 } else { attribute >> 4 };
            image.set(x as u32, y as u32, rgbi(color));
        }
    }
    Ok(image)
}

/// VBM version 2: `BM $CB $02`, big-endian width and height, then rows of
/// bits, most significant bit leftmost, each row padded to whole bytes.
pub(super) fn decode_vbm(data: &[u8]) -> Result<Image, DecodeError> {
    let [b'B', b'M', 0xcb, 2, w0, w1, h0, h1, bits @ ..] = data else {
        return Err(DecodeError::Invalid);
    };
    let width = usize::from(u16::from_be_bytes([*w0, *w1]));
    let height = usize::from(u16::from_be_bytes([*h0, *h1]));
    let stride = width.div_ceil(8);
    if bits.len() != stride * height {
        return Err(DecodeError::Invalid);
    }
    check_size(width, height)?;
    Image::from_bits(
        width as u32,
        height as u32,
        bits,
        stride,
        BitOrder::MsbFirst,
        [0xffffff, 0],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One 8-pixel column, color mode 1 (8×2 cells), all pixels set,
    /// attribute rows alternating foreground 1 and 2; `height` lines.
    fn brus(height: u16) -> Vec<u8> {
        let mut data = b"\0\0BRUS\x04\0\0\0\x01\x01\x01".to_vec();
        data.extend(height.to_le_bytes());
        data.extend([0, 0, 0]);
        for _ in 0..height {
            data.extend([0x81, 0xff]);
        }
        data.extend(b"COLR");
        for row in 0..usize::from(height).div_ceil(4) * 2 {
            data.extend([1, (row % 2 + 1) as u8]);
        }
        data
    }

    /// Foreground color index of each line (`rgbi` 1 is gray, 2 blue).
    fn line_colors(image: &Image) -> Vec<u8> {
        image
            .rgb()
            .as_chunks::<{ 8 * 3 }>()
            .0
            .iter()
            .map(|line| if line[2] == 0xaa { 2 } else { 1 })
            .collect()
    }

    #[test]
    fn non_interlaced_rows_cover_whole_cells() {
        let image = decode_brus(&brus(4)).unwrap();
        assert_eq!(line_colors(&image), [1, 1, 2, 2]);
    }

    #[test]
    fn interlaced_rows_alternate_per_field() {
        let image = decode_brus(&brus(204)).unwrap();
        assert_eq!(line_colors(&image)[..4], [1, 2, 1, 2]);
    }

    #[test]
    fn vbm_rejects_oversized_pictures() {
        let mut data = b"BM\xcb\x02\xff\xff\x04\x01".to_vec();
        data.resize(8 + 8192 * 0x401, 0);
        assert!(decode_vbm(&data).is_err());
    }
}
