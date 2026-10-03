//! IFF ILBM pictures from Atari ST programs: DEGAS Elite blocks (`BL1`-`BL3`)
//! and NEOchrome Master pictures with rasters (`NEO`).
//!
//! Sources:
//! - <https://temlib.org/AtariForumWiki/index.php/DEGAS_Elite_Block_file_format>
//! - <https://temlib.org/AtariForumWiki/index.php/IFF_file_format> (FORM,
//!   BMHD, CMAP and BODY chunks, PackBits body)
//! - NEOchrome Master `RAST` chunks: <https://temlib.org/AtariForumWiki/index.php/Neochrome_Master>
//! - Observed from `recoil2png` output: colour map bytes keep their high
//!   nibble (times 0x11); pixels with an x:y aspect below 1:2 are shown with
//!   doubled lines.

use alloc::vec::Vec;

use crate::bytes::{be16, be32};
use crate::codec::packbits;
use crate::image::check_size;
use crate::{DecodeError, Image};

pub(super) fn decode_block(data: &[u8]) -> Result<Image, DecodeError> {
    decode(data).ok_or(DecodeError::Unrecognized)
}

/// A NEOchrome Master picture needs its rasters: without them `recoil2png`
/// renders the FORM as a plain (Amiga) ILBM, whatever the extension.
pub(super) fn decode_neochrome_master(data: &[u8]) -> Result<Image, DecodeError> {
    if !is_neochrome_master(data) {
        return Err(DecodeError::Unrecognized);
    }
    decode_block(data)
}

/// Whether an ILBM FORM is directly followed by a NEOchrome Master `RAST`
/// chunk (with or without a pad byte).
pub(in crate::platform) fn is_neochrome_master(data: &[u8]) -> bool {
    let Some(len) = be32(data, 4) else {
        return false;
    };
    let end = 8usize.saturating_add(len as usize);
    data.get(..4) == Some(b"FORM")
        && data.get(8..12) == Some(b"ILBM")
        && [end, end.saturating_add(1)]
            .iter()
            .any(|&at| data.get(at..).is_some_and(|rest| rest.starts_with(b"RAST")))
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
    // NEOchrome Master appends its `RAST` chunk after the FORM, so read
    // chunks up to the end of the file (derived from sample files).
    be32(data, 4)?;
    let end = data.len();
    let mut header = None;
    let mut palette = Vec::new();
    let mut body = None;
    let mut rasters = None;
    let mut pos = 12;
    while pos + 8 <= end {
        let id = &data[pos..pos + 4];
        let len = be32(data, pos + 4)? as usize;
        let Some(chunk) = data.get(pos + 8..(pos + 8).saturating_add(len)) else {
            break;
        };
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
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .map(|c| {
                        let level = |v: u8| u32::from(v >> 4) * 0x11;
                        level(c[0]) << 16 | level(c[1]) << 8 | level(c[2])
                    })
                    .collect();
            }
            b"BODY" => body = Some(chunk),
            b"RAST" => rasters = Some(chunk),
            _ => {}
        }
        pos += 8 + len;
        // Chunks are padded to even lengths, but NEOchrome Master omits the
        // pad byte (derived from sample files).
        if len & 1 != 0 && !data.get(pos..pos + 4).is_some_and(is_chunk_id) {
            pos += 1;
        }
    }
    let h = header?;
    if !(1..=8).contains(&h.planes) || h.width == 0 || h.height == 0 || h.width > 4096 {
        return None;
    }
    let y_scale = if h.x_aspect != 0 && u32::from(h.x_aspect) * 2 <= u32::from(h.y_aspect) {
        2
    } else {
        1
    };
    check_size(h.width, h.height * y_scale).ok()?;
    let row_len = h.width.div_ceil(16) * 2;
    let len = row_len * h.planes * h.height;
    let bitmap = match h.compression {
        0 => body?.get(..len)?.to_vec(),
        1 => packbits::unpack(body?, len)?.0,
        _ => return None,
    };
    let line_palettes = rasters
        .and_then(|chunk| super::rasters::line_palette_words(chunk, h.height))
        .map(|words| super::rasters::line_colors(&words, true));
    let mut image = Image::new(h.width as u32, (h.height * y_scale) as u32);
    for (y, line) in bitmap.chunks_exact(row_len * h.planes).enumerate() {
        for x in 0..h.width {
            let mut index = 0;
            for plane in 0..h.planes {
                let byte = line[plane * row_len + x / 8];
                index |= usize::from(byte >> (7 - x % 8) & 1) << plane;
            }
            let color = match &line_palettes {
                Some(palettes) => *palettes.get(y * 16 + index)?,
                None => *palette.get(index)?,
            };
            for dy in 0..y_scale {
                image.set(x as u32, (y * y_scale + dy) as u32, color);
            }
        }
    }
    Some(image)
}

/// Whether `id` looks like an IFF chunk id (four printable ASCII bytes).
fn is_chunk_id(id: &[u8]) -> bool {
    id.iter().all(|b| (b' '..=b'~').contains(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neochrome_master_needs_rast_right_after_the_form() {
        // FORM of odd length 5: "ILBM" + one byte, as NEOchrome Master writes it.
        let form = b"FORM\0\0\0\x05ILBMx";
        let unpadded = [&form[..], b"RAST\0\0\0\0"].concat();
        let padded = [&form[..], b"\0RAST\0\0\0\0"].concat();
        assert!(is_neochrome_master(&unpadded));
        assert!(is_neochrome_master(&padded));
        assert!(!is_neochrome_master(form));
        assert!(!is_neochrome_master(&[&form[..], b"CMAP\0\0\0\0"].concat()));
        let pbm = [&b"FORM\0\0\0\x05PBM x"[..], b"RAST\0\0\0\0"].concat();
        assert!(!is_neochrome_master(&pbm));
    }

    #[test]
    fn picture_over_the_pixel_cap_is_rejected() {
        // 4096 x 16385 pixels, one plane, packed 128:1 with runs of zeros.
        let mut bmhd = alloc::vec![0u8; 20];
        bmhd[..2].copy_from_slice(&4096u16.to_be_bytes());
        bmhd[2..4].copy_from_slice(&16385u16.to_be_bytes());
        bmhd[8] = 1;
        bmhd[10] = 1;
        let body = [0x81u8, 0].repeat(65_600);
        let mut data = b"FORM\0\0\0\0ILBM".to_vec();
        data.extend_from_slice(b"BMHD\0\0\0\x14");
        data.extend_from_slice(&bmhd);
        data.extend_from_slice(b"BODY");
        data.extend_from_slice(&(body.len() as u32).to_be_bytes());
        data.extend_from_slice(&body);
        assert!(decode(&data).is_none());
    }
}
