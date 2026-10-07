//! CTPK (`.ctpk`): a 3DS texture package; its first 2D texture is drawn.
//!
//! Sources: GBATEK, "3DS Files - Video Texture Package (CTPK)"
//! (<https://problemkaputt.de/gbatek.htm>, no license, facts only). No sample
//! file was available, so this is unverified.
//!
//! The header is `CTPK`, the version 1 (16 bits), the number of textures (16
//! bits), the offset of the texture data (0x8), its size (0xC) and 4 more
//! fields. 0x20-byte texture entries follow from 0x20: the name offset (0),
//! the data size (4), the data offset from the start of the texture data
//! (8), the GPU format number (0xC), the width and height (0x10 and 0x12, 16
//! bits), the number of mip levels (0x14), and the type (0x15: 0 cube map, 1
//! 1D, 2 2D). The base level comes first in the data, mip levels after it. A
//! package holds unrelated pictures, so the first is drawn.

use super::texture::{Format, Texture};
use crate::bytes::{le16, le32};
use crate::{DecodeError, Image};

const ENTRIES_AT: usize = 0x20;
const ENTRY_LEN: usize = 0x20;
const TYPE_2D: u8 = 2;

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    const FAIL: DecodeError = DecodeError::Invalid;
    let count = usize::from(le16(data, 6).ok_or(FAIL)?);
    let entries_end = count
        .checked_mul(ENTRY_LEN)
        .and_then(|len| len.checked_add(ENTRIES_AT))
        .ok_or(FAIL)?;
    let texture_data = le32(data, 8).ok_or(FAIL)? as usize;
    if !data.starts_with(b"CTPK")
        || le16(data, 4) != Some(1)
        || count == 0
        || entries_end > data.len()
        || texture_data < entries_end
        || texture_data > data.len()
    {
        return Err(FAIL);
    }
    let entries = data[ENTRIES_AT..entries_end].as_chunks::<ENTRY_LEN>().0;
    let entry = entries
        .iter()
        .find(|entry| entry[0x15] == TYPE_2D && entry[0x14] > 0)
        .ok_or(FAIL)?;
    let size = le32(entry, 4).ok_or(FAIL)? as usize;
    let offset = (le32(entry, 8).ok_or(FAIL)? as usize)
        .checked_add(texture_data)
        .ok_or(FAIL)?;
    let format = Format::from_gpu(le32(entry, 0xc).ok_or(FAIL)?).ok_or(FAIL)?;
    let width = usize::from(le16(entry, 0x10).ok_or(FAIL)?);
    let height = usize::from(le16(entry, 0x12).ok_or(FAIL)?);
    let pixels = data
        .get(offset..offset.checked_add(size).ok_or(FAIL)?)
        .ok_or(FAIL)?;
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

    /// A package of textures `(format, width, height, type, data)`.
    fn ctpk(textures: &[(u32, u16, u16, u8, &[u8])]) -> Vec<u8> {
        let header_len = ENTRIES_AT + textures.len() * ENTRY_LEN;
        let mut file = Vec::new();
        file.extend_from_slice(b"CTPK\x01\x00");
        file.extend_from_slice(&(textures.len() as u16).to_le_bytes());
        file.extend_from_slice(&(header_len as u32).to_le_bytes());
        file.resize(ENTRIES_AT, 0);
        let mut offset = 0;
        for &(format, width, height, kind, data) in textures {
            file.extend_from_slice(&[0; 4]); // name offset
            file.extend_from_slice(&(data.len() as u32).to_le_bytes());
            file.extend_from_slice(&(offset as u32).to_le_bytes());
            file.extend_from_slice(&format.to_le_bytes());
            file.extend_from_slice(&width.to_le_bytes());
            file.extend_from_slice(&height.to_le_bytes());
            file.extend_from_slice(&[1, kind, 0, 0]);
            file.extend_from_slice(&[0; 8]);
            offset += data.len();
        }
        for &(_, _, _, _, data) in textures {
            file.extend_from_slice(data);
        }
        file
    }

    #[test]
    fn the_first_2d_texture_is_drawn() {
        // A cube map first (skipped), then an 8 x 8 L8 texture.
        let file = ctpk(&[(7, 8, 8, 0, &[1; 64]), (7, 8, 8, 2, &[0x40; 64])]);
        let image = decode(&file).unwrap();
        assert_eq!((image.width(), image.height()), (8, 8));
        assert_eq!(image.get(7, 7), 0x40_4040);
    }

    #[test]
    fn the_package_must_be_sound() {
        let file = ctpk(&[(7, 8, 8, 2, &[0; 64])]);
        assert!(decode(&file).is_ok());
        assert!(decode(&file[..file.len() - 1]).is_err());
        let mut bad = file.clone();
        bad[0] = b'X';
        assert!(decode(&bad).is_err());
        let mut bad = file.clone();
        bad[ENTRIES_AT + 0xc] = 14; // no such format
        assert!(decode(&bad).is_err());
        assert!(decode(&ctpk(&[])).is_err());
        assert!(decode(&ctpk(&[(7, 8, 8, 1, &[0; 64])])).is_err());
    }
}
