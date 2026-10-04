//! Windows icons (ICO) and cursors (CUR).
//!
//! Sources:
//! - Microsoft, "Icons" (ICONDIR and ICONDIRENTRY, DIB-based and PNG-based
//!   images):
//!   <https://learn.microsoft.com/en-us/previous-versions/ms997538(v=msdn.10)>
//!   and the Wikipedia article <https://en.wikipedia.org/wiki/ICO_(file_format)>
//!   (6-byte directory: reserved 0, type 1 icon or 2 cursor, count; 16-byte
//!   entries: width, height (0 means 256), colours, reserved, planes or
//!   hotspot x, bits per pixel or hotspot y, data size, data offset).
//! - Each DIB image is a headerless bitmap whose stored height is twice the
//!   picture height: the colour bitmap, then a 1-bit AND mask. See `bmp.rs`
//!   for the DIB layouts.
//! - Verified against Pillow's ICO and CUR decoders on icons found on the
//!   development machine (see the divergence file `pc-ico.tsv`).
//!
//! Only the largest DIB image is decoded (ties go to the higher bit depth).
//! `Image` has no alpha: the mask and the alpha channel of 32-bit images are
//! ignored, so the pixels of a cursor outline or a transparent area show
//! their stored colour. Images stored as PNG (Vista-style 256x256) are not
//! supported; a file with only PNG images is rejected.

use alloc::vec::Vec;

use super::bmp::decode_icon_dib;
use crate::bytes::{le16, le32};
use crate::{DecodeError, Image};

const FAIL: DecodeError = DecodeError::Unrecognized;
const DIR_LEN: usize = 6;
const ENTRY_LEN: usize = 16;
/// Size of the `BITMAPINFOHEADER` every DIB-based icon image starts with.
const INFO_HEADER_LEN: u32 = 40;

struct Entry<'a> {
    width: u32,
    height: u32,
    bpp: u16,
    image: &'a [u8],
}

pub(super) fn decode_ico(data: &[u8]) -> Result<Image, DecodeError> {
    let best = entries(data)?
        .into_iter()
        .filter(|e| le32(e.image, 0) == Some(INFO_HEADER_LEN))
        .max_by_key(|e| (e.width * e.height, e.bpp))
        .ok_or(FAIL)?;
    decode_icon_dib(best.image)
}

/// The directory entries, each with its image data. Fails unless the
/// directory is well formed and every image lies inside the file and starts
/// with a DIB or PNG header, which is what makes this a reliable signature.
fn entries(data: &[u8]) -> Result<Vec<Entry<'_>>, DecodeError> {
    if le16(data, 0) != Some(0) || !matches!(le16(data, 2), Some(1 | 2)) {
        return Err(FAIL);
    }
    let count = usize::from(le16(data, 4).ok_or(FAIL)?);
    if count == 0 {
        return Err(FAIL);
    }
    (0..count)
        .map(|i| {
            let at = DIR_LEN + i * ENTRY_LEN;
            let field = data.get(at..at + ENTRY_LEN).ok_or(FAIL)?;
            let size = le32(field, 8).ok_or(FAIL)? as usize;
            let offset = le32(field, 12).ok_or(FAIL)? as usize;
            let end = offset.checked_add(size).ok_or(FAIL)?;
            let image = data.get(offset..end).ok_or(FAIL)?;
            if le32(image, 0) != Some(INFO_HEADER_LEN) && !image.starts_with(b"\x89PNG") {
                return Err(FAIL);
            }
            // A size byte of 0 means 256.
            let dimension = |b: u8| if b == 0 { 256 } else { u32::from(b) };
            Ok(Entry {
                width: dimension(field[0]),
                height: dimension(field[1]),
                bpp: le16(field, 6).ok_or(FAIL)?,
                image,
            })
        })
        .collect()
}
