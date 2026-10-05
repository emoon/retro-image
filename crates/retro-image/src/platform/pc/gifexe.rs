//! GIFEXE: a GIF viewer program with the picture appended to it.
//!
//! Sources:
//! - Bob Eager, "Notes on the format of DOS .EXE files":
//!   <https://www.tavi.co.uk/phobos/exeformat.html> (the header words at 2
//!   and 4 give the size of the program image: the last page's byte count,
//!   zero for a full page, and the number of 512-byte pages; anything after
//!   that is an overlay the program may read from its own file).
//! - Reverse engineered from the 10 sample files (Sembiance's `gifexe`
//!   folder): the stub's header declares its own size, 16622, 18670 or 21556
//!   bytes in the three stub versions seen, and a GIF87a or GIF89a starts at
//!   that byte. One file (`SCREEN2.EXE`) holds three GIFs in a row; the
//!   first is shown.
//!
//! The picture is found only at the declared end of the program image, never
//! by searching for `GIF8`, so an executable passes only if it really has a
//! GIF appended where its own header says the program stops.
//!
//! Verification: the payload goes through the GIF decoder (`gif.rs`), which
//! was compared with Deark. No RECOIL or Deark module exists for GIFEXE; the
//! output equals Deark's decode of the GIF cut out of each file.

use super::gif::decode_gif;
use crate::bytes::le16;
use crate::{DecodeError, Image};

const FAIL: DecodeError = DecodeError::Unrecognized;
const PAGE: usize = 512;

/// Where the program image declared by the MZ header ends.
fn image_end(data: &[u8]) -> Option<usize> {
    if !data.starts_with(b"MZ") {
        return None;
    }
    let last = usize::from(le16(data, 2)?);
    let pages = usize::from(le16(data, 4)?);
    let last = if last == 0 { PAGE } else { last };
    Some(pages.checked_sub(1)? * PAGE + last)
}

pub(super) fn decode_gifexe(data: &[u8]) -> Result<Image, DecodeError> {
    let gif = data.get(image_end(data).ok_or(FAIL)?..).ok_or(FAIL)?;
    if !(gif.starts_with(b"GIF87a") || gif.starts_with(b"GIF89a")) {
        return Err(FAIL);
    }
    decode_gif(gif)
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;

    #[test]
    fn the_declared_size_locates_the_gif() {
        // 1x1 GIF87a, one color.
        let gif = b"GIF87a\x01\x00\x01\x00\x80\x00\x00\xff\xff\xff\x00\x00\x00\x2c\x00\x00\x00\x00\x01\x00\x01\x00\x00\x02\x02\x44\x01\x00\x3b";
        let mut exe = vec![0u8; 600];
        exe[..2].copy_from_slice(b"MZ");
        exe[2..4].copy_from_slice(&88u16.to_le_bytes()); // 512 + 88 = 600
        exe[4..6].copy_from_slice(&2u16.to_le_bytes());
        exe.extend_from_slice(gif);
        let image = decode_gifexe(&exe).unwrap();
        assert_eq!((image.width(), image.height()), (1, 1));
        // The same bytes with a header that ends the program elsewhere fail.
        exe[2..4].copy_from_slice(&89u16.to_le_bytes());
        assert!(decode_gifexe(&exe).is_err());
        // A full last page is stored as zero.
        assert_eq!(image_end(&[b'M', b'Z', 0, 0, 3, 0]), Some(1536));
        assert_eq!(image_end(&[b'M', b'Z', 0, 0, 0, 0]), None);
    }
}
