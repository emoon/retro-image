//! CLIM (`.bclim`): a 3DS layout texture, with its description at the end of
//! the file.
//!
//! Sources: GBATEK, "3DS Files - Video Layout Images (CLIM/FLIM)"
//! (<https://problemkaputt.de/gbatek.htm>, no license, facts only). FLIM, the
//! Wii U and eShop variant, is mirrored and rotated and is not decoded. No
//! sample file was available, so this is unverified.
//!
//! The texture data comes first, for a width and height each rounded up to a
//! power of two and at least 8. The picture is the upper left part of it
//! that the real width and height ask for, which can be odd. The 0x28-byte
//! footer at the end holds: `CLIM` (0), the byte order mark `0xFEFF` (4), the
//! footer size 0x14 (6), a version (8), the file size (0xC), the number of
//! blocks (0x10), `imag` (0x14), that chunk's size 0x10 (0x18), the width
//! (0x1C) and height (0x1E), the format (0x20, in CLIM's own numbering, see
//! [`Format::from_clim`]), and the offset of the footer (0x24). Together
//! that makes a strong signature, found at the end of the file.

use super::texture::{Format, Texture};
use crate::bytes::{le16, le32};
use crate::{DecodeError, Image};

const FOOTER_LEN: usize = 0x28;
const BYTE_ORDER_MARK: u16 = 0xfeff;

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let footer_at = data.len().checked_sub(FOOTER_LEN).ok_or(fail)?;
    let footer = &data[footer_at..];
    let sound = footer.starts_with(b"CLIM")
        && le16(footer, 4) == Some(BYTE_ORDER_MARK)
        && le16(footer, 6) == Some(0x14)
        && le32(footer, 0xc) == u32::try_from(data.len()).ok()
        && footer.get(0x14..0x18) == Some(b"imag")
        && le32(footer, 0x18) == Some(0x10)
        && le32(footer, 0x24) == u32::try_from(footer_at).ok();
    if !sound {
        return Err(fail);
    }
    let width = usize::from(le16(footer, 0x1c).ok_or(fail)?);
    let height = usize::from(le16(footer, 0x1e).ok_or(fail)?);
    let format = Format::from_clim(footer[0x20]).ok_or(fail)?;
    if width == 0 || height == 0 {
        return Err(fail);
    }
    let padded = |side: usize| side.next_power_of_two().max(8);
    Texture {
        format,
        width: padded(width),
        height: padded(height),
        data: &data[..footer_at],
    }
    .image(width, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A CLIM of `width` x `height` pixels in CLIM format `format` around
    /// `texture`.
    fn clim(width: u16, height: u16, format: u8, texture: &[u8]) -> alloc::vec::Vec<u8> {
        let mut file = texture.to_vec();
        let footer_at = file.len() as u32;
        file.extend_from_slice(b"CLIM\xff\xfe\x14\x00\x00\x00\x02\x02");
        file.extend_from_slice(&(footer_at + FOOTER_LEN as u32).to_le_bytes());
        file.extend_from_slice(&1u32.to_le_bytes());
        file.extend_from_slice(b"imag");
        file.extend_from_slice(&0x10u32.to_le_bytes());
        file.extend_from_slice(&width.to_le_bytes());
        file.extend_from_slice(&height.to_le_bytes());
        file.extend_from_slice(&[format, 0, 0, 0]);
        file.extend_from_slice(&footer_at.to_le_bytes());
        file
    }

    #[test]
    fn odd_sizes_are_cropped_from_the_power_of_two_texture() {
        // 12 x 3 pixels of L8 (CLIM format 0) in a 16 x 8 texture.
        let file = clim(12, 3, 0, &[0x80; 128]);
        let image = decode(&file).unwrap();
        assert_eq!((image.width(), image.height()), (12, 3));
        assert_eq!(image.get(0, 0), 0x80_8080);
        // The texture is too short for what the footer says.
        assert!(decode(&clim(12, 3, 0, &[0x80; 100])).is_err());
    }

    #[test]
    fn the_footer_must_be_sound() {
        let file = clim(8, 8, 0, &[0; 64]);
        assert!(decode(&file).is_ok());
        for (at, value) in [(0usize, b'X'), (4, 0), (6, 0x15), (0x14, b'x'), (0x24, 1)] {
            let mut bad = file.clone();
            bad[64 + at] = value;
            assert!(decode(&bad).is_err(), "byte {at:#x}");
        }
        // A format number that does not exist, a zero size, a file too short.
        assert!(decode(&clim(8, 8, 14, &[0; 64])).is_err());
        assert!(decode(&clim(0, 8, 0, &[0; 64])).is_err());
        assert!(decode(&file[..30]).is_err());
    }
}
