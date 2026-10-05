//! CTXB (`.ctxb`): a 3DS texture binary, as in Ocarina of Time 3D; its first
//! texture is drawn.
//!
//! Sources: GBATEK, "3DS Files - Video Texture Binary (CTXB)" and "3DS GPU
//! Texture Formats" (the OpenTK constants that stand for each GPU format),
//! <https://problemkaputt.de/gbatek.htm> (no licence, facts only), which also
//! points to <https://wiki.cloudmodding.com/oot/3D:CTXB_format>. No sample
//! file was available, so this is unverified.
//!
//! The file is `ctxb`, its size (4), a chunk size (0x10), the offset of
//! the textures (0x14, usually 0x48), the chunk `tex ` (0x18), its size
//! (0x1C), the number of textures (0x20), and then for the first texture the
//! size of its data (0x24), the number of mip levels (0x28, 16 bits), the
//! compression flag (0x2A), the cube map flag (0x2B), the width and height
//! (0x2C and 0x2E, powers of two), the OpenTK format and data type (0x30 and
//! 0x32, 16 bits each) and the offset of its data from the texture offset
//! (0x34). A cube map is rejected, since GBATEK does not say where its sides
//! are.

use super::texture::{Format, Texture};
use crate::bytes::{le16, le32};
use crate::{DecodeError, Image};

/// The GPU format for an OpenTK texture format and data type, as GBATEK's
/// table pairs them.
fn format(opentk_format: u16, data_type: u16) -> Option<Format> {
    Some(match (opentk_format, data_type) {
        (0x6752, 0x1401) => Format::Rgba8,
        (0x6754, 0x1401) => Format::Rgb8,
        (0x6752, 0x8034) => Format::Rgba5551,
        (0x6754, 0x8363) => Format::Rgb565,
        (0x6752, 0x8033) => Format::Rgba4,
        (0x6758, 0x1401) => Format::La8,
        (0x6759, 0x1401) => Format::Hilo8,
        (0x6757, 0x1401) => Format::L8,
        (0x6756, 0x1401) => Format::A8,
        (0x6758, 0x6760) => Format::La4,
        (0x6757, 0x6761) => Format::L4,
        (0x6756, 0x6761) => Format::A4,
        _ => return None,
    })
}

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    if !data.starts_with(b"ctxb")
        || data.get(0x18..0x1c) != Some(b"tex ")
        || le32(data, 4).ok_or(fail)? as usize != data.len()
        || le32(data, 0x20).ok_or(fail)? == 0
        || data.get(0x2b) != Some(&0)
    {
        return Err(fail);
    }
    let base = le32(data, 0x14).ok_or(fail)? as usize;
    let size = le32(data, 0x24).ok_or(fail)? as usize;
    let (width, height) = (
        usize::from(le16(data, 0x2c).ok_or(fail)?),
        usize::from(le16(data, 0x2e).ok_or(fail)?),
    );
    let format =
        format(le16(data, 0x30).ok_or(fail)?, le16(data, 0x32).ok_or(fail)?).ok_or(fail)?;
    let offset = base
        .checked_add(le32(data, 0x34).ok_or(fail)? as usize)
        .ok_or(fail)?;
    let pixels = data
        .get(offset..offset.checked_add(size).ok_or(fail)?)
        .ok_or(fail)?;
    Texture {
        format,
        width,
        height,
        data: pixels,
    }
    .image(width, height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    /// A CTXB with one texture of `width` x `height` pixels of the OpenTK
    /// format and type, and `pixels`.
    fn ctxb(width: u16, height: u16, opentk: (u16, u16), cubemap: u8, pixels: &[u8]) -> Vec<u8> {
        let mut file = Vec::new();
        file.extend_from_slice(b"ctxb");
        file.extend_from_slice(&((0x48 + pixels.len()) as u32).to_le_bytes());
        file.extend_from_slice(&[1, 0, 0, 0, 0, 0, 0, 0]);
        file.extend_from_slice(&0x18u32.to_le_bytes());
        file.extend_from_slice(&0x48u32.to_le_bytes());
        file.extend_from_slice(b"tex ");
        file.extend_from_slice(&0x30u32.to_le_bytes());
        file.extend_from_slice(&1u32.to_le_bytes());
        file.extend_from_slice(&(pixels.len() as u32).to_le_bytes());
        file.extend_from_slice(&[1, 0, 0, cubemap]);
        file.extend_from_slice(&width.to_le_bytes());
        file.extend_from_slice(&height.to_le_bytes());
        file.extend_from_slice(&opentk.0.to_le_bytes());
        file.extend_from_slice(&opentk.1.to_le_bytes());
        file.extend_from_slice(&0u32.to_le_bytes());
        file.resize(0x48, 0);
        file.extend_from_slice(pixels);
        file
    }

    #[test]
    fn the_first_texture_is_drawn_in_its_opentk_format() {
        let file = ctxb(8, 8, (0x6757, 0x1401), 0, &[0x20; 64]);
        let image = decode(&file).unwrap();
        assert_eq!((image.width(), image.height()), (8, 8));
        assert_eq!(image.get(0, 0), 0x20_2020);
        // RGB565 is stored as 16-bit words; the first stored pixel is the
        // bottom left one of the picture.
        let mut words = alloc::vec![0u8; 128];
        words[0] = 0x1f;
        let blue = decode(&ctxb(8, 8, (0x6754, 0x8363), 0, &words)).unwrap();
        assert_eq!(blue.get(0, 7), 0x00_00ff);
        assert_eq!(blue.get(0, 0), 0);
    }

    #[test]
    fn the_file_must_be_sound() {
        let good = ctxb(8, 8, (0x6757, 0x1401), 0, &[0; 64]);
        assert!(decode(&good).is_ok());
        assert!(decode(&ctxb(8, 8, (0x6757, 0x1401), 1, &[0; 64])).is_err());
        assert!(decode(&ctxb(8, 8, (0x6757, 0x0000), 0, &[0; 64])).is_err());
        assert!(decode(&good[..good.len() - 1]).is_err());
        let mut bad = good;
        bad[0x18] = b'x';
        assert!(decode(&bad).is_err());
    }
}
