//! Canvas compressed pictures (`CPT`).
//!
//! Sources:
//! - <https://temlib.org/AtariForumWiki/index.php/Canvas_file_format>
//! - Canvas 1.17 manual: <http://cd.textfiles.com/crawlycrypt1/graphics/canvas17/manual.txt>

use super::common::{Resolution, SCREEN_LEN, be16, decode_screen, palette_words};
use crate::{DecodeError, Image};

pub(super) fn decode_cpt(data: &[u8]) -> Result<Image, DecodeError> {
    decode(data).ok_or(DecodeError::Unrecognized)
}

fn decode(data: &[u8]) -> Option<Image> {
    let words = palette_words(data, 0, 16)?;
    let resolution = Resolution::from_index(be16(data, 32)?)?;
    // One unit is a 16-pixel group: one word per plane.
    let unit = resolution.planes() as usize * 2;
    let units = SCREEN_LEN / unit;
    let mut bitmap = alloc::vec![0u8; SCREEN_LEN];
    let mut filled = alloc::vec![false; units];
    let mut pos = 34;
    loop {
        let count = be16(data, pos)?;
        let offset = usize::from(be16(data, pos + 2)?);
        let value = data.get(pos + 4..pos + 4 + unit)?;
        pos += 4 + unit;
        if count == 0xffff {
            break;
        }
        // The offset counts units, not bytes (derived from sample files).
        let run = offset..offset + usize::from(count) + 1;
        let target = bitmap.get_mut(run.start * unit..run.end * unit)?;
        for chunk in target.chunks_exact_mut(unit) {
            chunk.copy_from_slice(value);
        }
        filled[run].fill(true);
    }
    for index in (0..units).filter(|&i| !filled[i]) {
        let value = data.get(pos..pos + unit)?;
        pos += unit;
        bitmap[index * unit..(index + 1) * unit].copy_from_slice(value);
    }
    decode_screen(resolution, &bitmap, &words)
}
