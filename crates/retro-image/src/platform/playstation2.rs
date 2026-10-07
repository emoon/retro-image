//! Sony PlayStation 2: TIM2 pictures (`.tm2`, `.tim2`), the first picture
//! of a file.
//!
//! Sources:
//! - "TIM2 format specification ver.4", web technology Corp., 1999-12-02:
//!   file and picture headers, image types, CLUT types and the compound CLUT
//!   order of section 4.5. The copy read is `webtech/tim2v4b_e.zip` in
//!   <https://github.com/GirianSeed/tim2>. It carries a "`[CONFIDENTIAL]`"
//!   banner but has been public for years; the project owner approved
//!   reading it as specification prose, and nothing of the repository's
//!   sample code was read.
//! - Reverse engineered from samples, where the spec is silent or wrong: the
//!   opaque alpha is `0x80`, not `0xff` (every pixel of the spec's
//!   `i32.tm2`); the files are plain row-major, not swizzled for GS memory
//!   (three real 8-bit game files decode upright with the CLUT swap and
//!   scramble when a GS swizzle is added); the spec's 16-bit RGBA word has
//!   red in the low bits. The samples are the eleven pictures of the spec's
//!   `tim2img_e.zip` (one cat in every storage variant, which must decode to
//!   the same picture) and five game files from
//!   <https://sembiance.com/fileFormatSamples/image/tim2/>. A sixth file
//!   there, a 512x256 4-bit picture with a 320-color CLUT, decodes as noise
//!   with the first palette: 4-bit TIM2 files whose pixels are swizzled for
//!   GS memory are not handled and come out as noise. The corpus does not
//!   keep that file.
//! - Platform survey: `docs/research/gaps-consoles.md` section 3.2.
//!
//! Pictures with mipmaps show their largest level; a file with several
//! pictures shows the first. Alpha is kept.

use alloc::vec::Vec;

use crate::bytes::{le16, le32};
use crate::image::{bgr555, check_size};
use crate::{DecodeError, Format, Image};

pub(super) static FORMATS: &[Format] =
    &[Format::new("PlayStation 2", "TIM2", &["tm2", "tim2"], decode_tim2).signature()];

/// Size of the file header; pictures start after it, aligned to 16 or 128
/// bytes as the format id says.
const FILE_HEADER_LEN: usize = 16;
const PICTURE_HEADER_LEN: usize = 48;

/// The picture header fields the decoder uses.
struct Header {
    clut_size: usize,
    image_size: usize,
    header_size: usize,
    clut_colors: usize,
    mipmaps: u8,
    clut_type: u8,
    image_type: u8,
    width: usize,
    height: usize,
}

/// How the picture's pixels are stored.
enum Pixels {
    Direct { bytes_per_pixel: usize },
    Indexed { bits: usize },
}

fn read_header(data: &[u8], at: usize) -> Option<Header> {
    Some(Header {
        clut_size: le32(data, at + 4)? as usize,
        image_size: le32(data, at + 8)? as usize,
        header_size: usize::from(le16(data, at + 12)?),
        clut_colors: usize::from(le16(data, at + 14)?),
        mipmaps: *data.get(at + 17)?,
        clut_type: *data.get(at + 18)?,
        image_type: *data.get(at + 19)?,
        width: usize::from(le16(data, at + 20)?),
        height: usize::from(le16(data, at + 22)?),
    })
}

/// The pixel storage for an image type: 16, 24 and 32-bit direct color, 4
/// and 8-bit index.
fn pixel_storage(image_type: u8) -> Option<Pixels> {
    match image_type {
        1 => Some(Pixels::Direct { bytes_per_pixel: 2 }),
        2 => Some(Pixels::Direct { bytes_per_pixel: 3 }),
        3 => Some(Pixels::Direct { bytes_per_pixel: 4 }),
        4 => Some(Pixels::Indexed { bits: 4 }),
        5 => Some(Pixels::Indexed { bits: 8 }),
        _ => None,
    }
}

/// `0xAARRGGBB` from a 16-bit word: red in bits 0-4, green 5-9, blue 10-14,
/// and a set bit 15 means opaque.
fn color16(word: u16) -> u32 {
    let alpha = if word & 0x8000 != 0 { 0xff } else { 0 };
    alpha << 24 | bgr555(word)
}

/// `0xAARRGGBB` from bytes R, G, B and an alpha in which `0x80` is opaque.
fn color(r: u8, g: u8, b: u8, ps2_alpha: u8) -> u32 {
    let alpha = u32::from(ps2_alpha.min(0x80)) * 255 / 128;
    alpha << 24 | u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b)
}

/// Entry `index` of a CLUT whose colors are `depth` (the low bits of the
/// CLUT type: 1, 2 or 3 for 16, 24 or 32 bits) wide.
fn clut_entry(clut: &[u8], depth: u8, index: usize) -> Option<u32> {
    match depth {
        1 => le16(clut, index * 2).map(color16),
        2 => {
            let p = clut.get(index * 3..index * 3 + 3)?;
            Some(color(p[0], p[1], p[2], 0x80))
        }
        3 => {
            let p = clut.get(index * 4..index * 4 + 4)?;
            Some(color(p[0], p[1], p[2], p[3]))
        }
        _ => None,
    }
}

/// Where color `index` sits in a compound (CSM1) CLUT: the entries of every
/// 32 are stored with index bits 3 and 4 exchanged (spec section 4.5).
fn compound_position(index: usize) -> usize {
    index & !0x18 | (index & 8) << 1 | (index & 16) >> 1
}

/// The first `count` colors, in index order, of a picture's CLUT.
fn palette(header: &Header, clut: &[u8], count: usize) -> Option<Vec<u32>> {
    let depth = header.clut_type & 0x3f;
    let csm2 = header.clut_type & 0x80 != 0;
    // CSM1 8-bit CLUTs are always compound; 16-color ones only with bit 6.
    let compound = !csm2 && (count == 256 || header.clut_type & 0x40 != 0);
    if header.clut_colors < count || (compound && header.clut_colors < 32) {
        return None;
    }
    (0..count)
        .map(|i| clut_entry(clut, depth, if compound { compound_position(i) } else { i }))
        .collect()
}

fn decode_tim2(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    if data.get(..4) != Some(b"TIM2") || data.get(5).is_none_or(|&id| id > 1) {
        return Err(fail);
    }
    if le16(data, 6).ok_or(fail)? == 0 {
        return Err(fail);
    }
    let start = if data[5] == 0 { FILE_HEADER_LEN } else { 128 };
    let header = read_header(data, start).ok_or(fail)?;
    // A picture without image data holds only a CLUT (CLUT2 files).
    let storage = pixel_storage(header.image_type).ok_or(fail)?;
    if header.mipmaps == 0 || header.header_size < PICTURE_HEADER_LEN {
        return Err(fail);
    }
    check_size(header.width, header.height)?;
    let pixels = header.width * header.height;
    let image_len = match storage {
        Pixels::Direct { bytes_per_pixel } => pixels * bytes_per_pixel,
        // Odd counts of 4-bit pixels end in half a byte.
        Pixels::Indexed { bits } => (pixels * bits).div_ceil(8),
    };
    if image_len > header.image_size {
        return Err(fail);
    }
    let image_at = start.checked_add(header.header_size).ok_or(fail)?;
    let image = data.get(image_at..image_at + image_len).ok_or(fail)?;
    let clut_at = image_at.checked_add(header.image_size).ok_or(fail)?;
    let clut = data.get(clut_at..).unwrap_or_default();
    let clut = &clut[..header.clut_size.min(clut.len())];

    let argb: Vec<u32> = match storage {
        Pixels::Direct { bytes_per_pixel: 2 } => image
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&p| color16(u16::from_le_bytes(p)))
            .collect(),
        Pixels::Direct { bytes_per_pixel: 3 } => image
            .as_chunks::<3>()
            .0
            .iter()
            .map(|p| color(p[0], p[1], p[2], 0x80))
            .collect(),
        Pixels::Direct { .. } => image
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| color(p[0], p[1], p[2], p[3]))
            .collect(),
        Pixels::Indexed { bits: 4 } => {
            let colors = palette(&header, clut, 16).ok_or(fail)?;
            // The low nibble is the left pixel.
            image
                .iter()
                .flat_map(|&b| [colors[usize::from(b & 15)], colors[usize::from(b >> 4)]])
                .collect()
        }
        Pixels::Indexed { .. } => {
            let colors = palette(&header, clut, 256).ok_or(fail)?;
            image.iter().map(|&i| colors[usize::from(i)]).collect()
        }
    };
    Image::from_argb(header.width as u32, header.height as u32, argb.into_iter())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A one-picture file: 4 pixels wide, 1 high, with `image` and `clut`.
    fn file(image_type: u8, clut_type: u8, clut_colors: u16, image: &[u8], clut: &[u8]) -> Vec<u8> {
        sized_file(4, image_type, clut_type, clut_colors, image, clut)
    }

    /// The same for a picture `width` pixels wide.
    fn sized_file(
        width: u16,
        image_type: u8,
        clut_type: u8,
        clut_colors: u16,
        image: &[u8],
        clut: &[u8],
    ) -> Vec<u8> {
        let mut data = b"TIM2\x04\x00\x01\x00\0\0\0\0\0\0\0\0".to_vec();
        let total = 48 + image.len() + clut.len();
        data.extend_from_slice(&(total as u32).to_le_bytes());
        data.extend_from_slice(&(clut.len() as u32).to_le_bytes());
        data.extend_from_slice(&(image.len() as u32).to_le_bytes());
        data.extend_from_slice(&48u16.to_le_bytes());
        data.extend_from_slice(&clut_colors.to_le_bytes());
        data.extend_from_slice(&[0, 1, clut_type, image_type]);
        data.extend_from_slice(&width.to_le_bytes());
        data.extend_from_slice(&1u16.to_le_bytes());
        data.resize(16 + 48, 0);
        data.extend_from_slice(image);
        data.extend_from_slice(clut);
        data
    }

    #[test]
    fn opaque_alpha_is_0x80_and_zero_is_transparent() {
        let image = [0x10, 0x20, 0x30, 0x80, 0x10, 0x20, 0x30, 0x00];
        let image = decode_tim2(&file(3, 0, 0, &[image, image].concat(), &[])).unwrap();
        assert_eq!(image.get(0, 0), 0x102030);
        assert_eq!(image.get_argb(1, 0), crate::image::CLEAR);
    }

    #[test]
    fn csm1_8_bit_clut_swaps_index_bits_3_and_4() {
        // Index 8 is stored at position 16 and index 16 at position 8.
        let mut clut = alloc::vec![0u8; 256 * 4];
        for (position, gray) in [(0, 1u8), (16, 2), (8, 3), (24, 4)] {
            clut[position * 4..position * 4 + 4].copy_from_slice(&[gray, gray, gray, 0x80]);
        }
        let image = decode_tim2(&file(5, 3, 256, &[0, 8, 16, 24], &clut)).unwrap();
        let grays: Vec<u32> = (0..4).map(|x| image.get(x, 0) & 0xff).collect();
        assert_eq!(grays, [1, 2, 3, 4]);
    }

    #[test]
    fn csm2_clut_is_sequential_and_4_bit_pixels_start_low() {
        let mut clut = alloc::vec![0u8; 16 * 2];
        clut[2..4].copy_from_slice(&0x8000u16.to_le_bytes());
        clut[4..6].copy_from_slice(&0x801fu16.to_le_bytes());
        let image = decode_tim2(&file(4, 0x81, 16, &[0x21, 0x00], &clut)).unwrap();
        assert_eq!(image.get(0, 0), 0x000000, "index 1 is opaque black");
        assert_eq!(image.get(1, 0), 0xff0000, "index 2 is opaque red");
    }

    #[test]
    fn four_bit_pictures_with_an_odd_pixel_count_keep_their_last_pixel() {
        // 3x1 pixels: three nibbles take two bytes, and the third pixel is the
        // low nibble of the second byte.
        let mut clut = alloc::vec![0u8; 16 * 2];
        for (i, color) in [0x8000u16, 0x801f, 0x83e0, 0xfc00].iter().enumerate() {
            clut[i * 2..i * 2 + 2].copy_from_slice(&color.to_le_bytes());
        }
        let picture = sized_file(3, 4, 0x01, 16, &[0x21, 0x03], &clut);
        let image = decode_tim2(&picture).unwrap();
        assert_eq!(image.get(0, 0), 0xff0000, "index 1: red");
        assert_eq!(image.get(1, 0), 0x00ff00, "index 2: green");
        assert_eq!(image.get(2, 0), 0x0000ff, "index 3: blue, not black");
    }

    #[test]
    fn rejects_clut_only_pictures_and_truncation() {
        let mut data = file(3, 0, 0, &[0; 16], &[]);
        assert!(decode_tim2(&data[..data.len() - 1]).is_err());
        data[16 + 17] = 0;
        assert!(decode_tim2(&data).is_err());
    }
}
