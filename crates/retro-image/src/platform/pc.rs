//! IBM PC.
//!
//! Sources:
//! - Microsoft Paint (MSP): Encyclopedia of Graphics File Formats,
//!   <https://www.fileformat.info/format/mspaint/egff.htm> (32-byte header,
//!   "DanM" uncompressed version 1, "LinS" version 2 with a per-line size
//!   map and RLE). Set bit = white: observed from `recoil2png` output.

use alloc::vec;

use crate::{DecodeError, Format, Image};

pub(super) static FORMATS: &[Format] = &[Format::new(
    "PC",
    "Microsoft Paint version 1 or 2",
    &["msp"],
    decode_msp,
)];

const MSP_HEADER_LEN: usize = 32;

fn le16(b: &[u8]) -> usize {
    usize::from(u16::from_le_bytes([b[0], b[1]]))
}

fn decode_msp(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let header = data.get(..MSP_HEADER_LEN).ok_or(fail)?;
    let (width, height) = (le16(&header[4..6]), le16(&header[6..8]));
    if width == 0 || height == 0 {
        return Err(fail);
    }
    let row_len = width.div_ceil(8);
    let bitmap = match &header[..4] {
        b"DanM" => data
            .get(MSP_HEADER_LEN..MSP_HEADER_LEN + row_len * height)
            .ok_or(fail)?
            .to_vec(),
        b"LinS" => {
            let map_end = MSP_HEADER_LEN + height * 2;
            let map = data.get(MSP_HEADER_LEN..map_end).ok_or(fail)?;
            // A 3-byte run gives at most 255 bytes; this keeps corrupt
            // sizes from allocating huge bitmaps.
            if row_len * height > data.len().saturating_mul(85) {
                return Err(fail);
            }
            let mut bitmap = vec![0u8; row_len * height];
            let mut pos = map_end;
            for (y, size) in map.chunks_exact(2).map(le16).enumerate() {
                let line = data.get(pos..pos + size).ok_or(fail)?;
                pos += size;
                unpack_msp_line(line, &mut bitmap[y * row_len..(y + 1) * row_len]);
            }
            bitmap
        }
        _ => return Err(fail),
    };
    let mut image = Image::new(width as u32, height as u32);
    for y in 0..height {
        for x in 0..width {
            let set = bitmap[y * row_len + x / 8] & (0x80 >> (x % 8)) != 0;
            image.set(x as u32, y as u32, if set { 0xffffff } else { 0 });
        }
    }
    Ok(image)
}

/// `00 count value` is a run; any other byte `n` is followed by `n` literals.
/// Output beyond the line is dropped.
fn unpack_msp_line(src: &[u8], out: &mut [u8]) {
    let mut pos = 0;
    let mut x = 0;
    let mut put = |value: u8| {
        if let Some(slot) = out.get_mut(x) {
            *slot = value;
        }
        x += 1;
    };
    while pos < src.len() {
        let kind = src[pos];
        if kind == 0 {
            let (Some(&count), Some(&value)) = (src.get(pos + 1), src.get(pos + 2)) else {
                return;
            };
            (0..count).for_each(|_| put(value));
            pos += 3;
        } else {
            let literal = &src[pos + 1..src.len().min(pos + 1 + usize::from(kind))];
            literal.iter().for_each(|&b| put(b));
            pos += 1 + usize::from(kind);
        }
    }
}
