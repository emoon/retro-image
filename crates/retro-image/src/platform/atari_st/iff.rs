//! DEGAS Elite blocks (`BL1`-`BL3`): small IFF ILBM pictures.
//!
//! Sources:
//! - <https://temlib.org/AtariForumWiki/index.php/DEGAS_Elite_Block_file_format>
//! - <https://temlib.org/AtariForumWiki/index.php/IFF_file_format> (FORM,
//!   BMHD, CMAP and BODY chunks, PackBits body)
//! - Observed from `recoil2png` output: colour map bytes keep their high
//!   nibble (times 0x11); pixels with an x:y aspect below 1:2 are shown with
//!   doubled lines.

use alloc::vec::Vec;

use super::common::{be16, be32, unpack_bits};
use crate::{DecodeError, Image};

pub(super) fn decode_block(data: &[u8]) -> Result<Image, DecodeError> {
    decode(data).ok_or(DecodeError::Unrecognized)
}

struct Header {
    width: usize,
    height: usize,
    planes: usize,
    compression: u8,
    x_aspect: u8,
    y_aspect: u8,
}

fn decode(data: &[u8]) -> Option<Image> {
    if data.get(..4)? != b"FORM" || data.get(8..12)? != b"ILBM" {
        return None;
    }
    let end = (8 + be32(data, 4)? as usize).min(data.len());
    let mut header = None;
    let mut palette = Vec::new();
    let mut body = None;
    let mut pos = 12;
    while pos + 8 <= end {
        let id = &data[pos..pos + 4];
        let len = be32(data, pos + 4)? as usize;
        let chunk = data.get(pos + 8..(pos + 8).checked_add(len)?)?;
        match id {
            b"BMHD" => {
                header = Some(Header {
                    width: usize::from(be16(chunk, 0)?),
                    height: usize::from(be16(chunk, 2)?),
                    planes: usize::from(*chunk.get(8)?),
                    compression: *chunk.get(10)?,
                    x_aspect: *chunk.get(14)?,
                    y_aspect: *chunk.get(15)?,
                });
            }
            b"CMAP" => {
                palette = chunk
                    .chunks_exact(3)
                    .map(|c| {
                        let level = |v: u8| u32::from(v >> 4) * 0x11;
                        level(c[0]) << 16 | level(c[1]) << 8 | level(c[2])
                    })
                    .collect();
            }
            b"BODY" => body = Some(chunk),
            _ => {}
        }
        pos += 8 + len + (len & 1);
    }
    let h = header?;
    if !(1..=8).contains(&h.planes) || h.width == 0 || h.height == 0 || h.width > 4096 {
        return None;
    }
    let row_len = h.width.div_ceil(16) * 2;
    let len = row_len * h.planes * h.height;
    let bitmap = match h.compression {
        0 => body?.get(..len)?.to_vec(),
        1 => unpack_bits(body?, len)?.0,
        _ => return None,
    };
    let y_scale = if u32::from(h.x_aspect) * 2 <= u32::from(h.y_aspect) {
        2
    } else {
        1
    };
    let mut image = Image::new(h.width as u32, (h.height * y_scale) as u32);
    for (y, line) in bitmap.chunks_exact(row_len * h.planes).enumerate() {
        for x in 0..h.width {
            let mut index = 0;
            for plane in 0..h.planes {
                let byte = line[plane * row_len + x / 8];
                index |= usize::from(byte >> (7 - x % 8) & 1) << plane;
            }
            let color = *palette.get(index)?;
            for dy in 0..y_scale {
                image.set(x as u32, (y * y_scale + dy) as u32, color);
            }
        }
    }
    Some(image)
}
