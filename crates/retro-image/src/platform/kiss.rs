//! KiSS (Kisekae Set System) cels, with the palette from a `.kcf` file.
//!
//! Sources:
//! - "KISS/GS" by ITO Takayuki, an English translation of `kissgs.doc`
//!   declared public domain (<http://otakuworld.com/kiss/download/kissfrmt.txt>,
//!   Wayback `https://web.archive.org/web/2023/http://otakuworld.com/kiss/download/kissfrmt.txt`):
//!   the 32-byte cel header (`KiSS`, mark `0x20`, 4 or 8 bits per pixel,
//!   width, height, offsets), pixel index 0 is transparent, 4-bit pixels
//!   high nibble first with an odd width padded by one pixel; the headerless
//!   conventional cel (width, height, 4-bit rows, offsets 0); the palette
//!   file (`KiSS`, mark `0x10`, 12 or 24 bits per color, colors per group,
//!   group count; `rrrr bbbb` `0000 gggg` or `R G B`) and the headerless
//!   conventional palette (10 groups of 16 12-bit colors).
//! - Just Solve the File Format Problem, KiSS CEL
//!   (<http://fileformats.archiveteam.org/wiki/KiSS_CEL>, CC0): Cherry KiSS
//!   (CKiSS) cels use bits per pixel 32 with mark `0x20` or `0x21`, carry
//!   alpha and need no palette file.
//! - Reverse engineered from the samples in `corpus/extra/misc-computers/kiss`
//!   (22 files from the dexvert sample set, ten free dolls from otakuworld):
//!   every 4-bit, 8-bit, conventional and CKiSS cel is exactly header plus
//!   pixels long; the CKiSS pixel order is blue, green, red, alpha (the
//!   Sailor Moon logo in `hmpstar2/hstars.cel` is pink and rainbow in that
//!   order, blue and teal in RGB order); alpha is not premultiplied.
//!
//! Choices of this crate: the picture is the cel alone, without its offsets;
//! the palette is group 0 of the `.kcf` with the cel's stem, a gray ramp
//! without one; palette colors past the end of a short palette are black;
//! a 12-bit channel `v` becomes `v * 17`; transparent pixels (index 0, or
//! alpha) are composited onto [`TRANSPARENT_FILL`]. A set's `.cnf` file is not
//! read, so a cel whose palette has another name is shown in gray.

use alloc::vec::Vec;

use crate::bytes::le16;
use crate::image::{TRANSPARENT_FILL, check_size};
use crate::{Companions, DecodeError, Format, Image};

pub(super) static FORMATS: &[Format] = &[
    Format::with_companions("KiSS", "Cel", &["cel"], decode_cel).signature(),
    // No header to check: only the exact size tells it from other `.cel` files.
    Format::with_companions("KiSS", "Conventional cel", &["cel"], decode_old_cel),
];

const FAIL: DecodeError = DecodeError::Unrecognized;
const HEADER_LEN: usize = 32;
const CEL_MARK: u8 = 0x20;
const CKISS_MARK: u8 = 0x21;
const PALETTE_MARK: u8 = 0x10;
/// 10 groups of 16 colors of 2 bytes: the headerless palette file.
const OLD_PALETTE_LEN: usize = 320;

fn decode_cel(data: &[u8], companions: &dyn Companions) -> Result<Image, DecodeError> {
    if data.get(..4) != Some(b"KiSS") || !matches!(data.get(4), Some(&(CEL_MARK | CKISS_MARK))) {
        return Err(FAIL);
    }
    let bits = *data.get(5).ok_or(FAIL)?;
    let width = usize::from(le16(data, 8).ok_or(FAIL)?);
    let height = usize::from(le16(data, 10).ok_or(FAIL)?);
    check_size(width, height)?;
    let body = data.get(HEADER_LEN..).ok_or(FAIL)?;
    match bits {
        4 | 8 => indexed(width, height, bits, body, companions),
        32 => blended(width, height, body),
        _ => Err(FAIL),
    }
}

/// Width and height, then 4-bit rows to the end of the file.
fn decode_old_cel(data: &[u8], companions: &dyn Companions) -> Result<Image, DecodeError> {
    let width = usize::from(le16(data, 0).ok_or(FAIL)?);
    let height = usize::from(le16(data, 2).ok_or(FAIL)?);
    check_size(width, height)?;
    let body = &data[4..];
    if body.len() != width.div_ceil(2) * height {
        return Err(FAIL);
    }
    indexed(width, height, 4, body, companions)
}

/// A 4 or 8-bit cel through the palette file next to it.
fn indexed(
    width: usize,
    height: usize,
    bits: u8,
    body: &[u8],
    companions: &dyn Companions,
) -> Result<Image, DecodeError> {
    let row_len = if bits == 4 { width.div_ceil(2) } else { width };
    let rows = body.get(..row_len * height).ok_or(FAIL)?;
    let mut indices = Vec::with_capacity(width * height);
    for row in rows.chunks_exact(row_len) {
        if bits == 4 {
            indices.extend((0..width).map(|x| {
                let pair = row[x / 2];
                if x % 2 == 0 { pair >> 4 } else { pair & 15 }
            }));
        } else {
            indices.extend_from_slice(row);
        }
    }
    let mut palette = companions
        .get("kcf")
        .and_then(|kcf| read_palette(&kcf))
        .unwrap_or_else(|| gray_ramp(bits));
    palette.resize(256, 0);
    palette[0] = TRANSPARENT_FILL;
    Image::from_indexed(width as u32, height as u32, &indices, &palette)
}

/// A Cherry KiSS cel: 4-byte pixels, blue, green, red, alpha.
fn blended(width: usize, height: usize, body: &[u8]) -> Result<Image, DecodeError> {
    let pixels = body.get(..width * height * 4).ok_or(FAIL)?;
    let colors = pixels.as_chunks::<4>().0.iter().map(|&[b, g, r, alpha]| {
        let fill = TRANSPARENT_FILL.to_be_bytes();
        let over = |c: u8, under: u8| {
            let (c, under, alpha) = (u32::from(c), u32::from(under), u32::from(alpha));
            (c * alpha + under * (255 - alpha) + 127) / 255
        };
        over(r, fill[1]) << 16 | over(g, fill[2]) << 8 | over(b, fill[3])
    });
    Ok(Image::from_colors(width as u32, height as u32, colors))
}

/// Colors of palette group 0 of a `.kcf` file, `None` if it is neither the
/// headered nor the conventional layout.
fn read_palette(kcf: &[u8]) -> Option<Vec<u32>> {
    let (bits, colors, start) = if kcf.starts_with(b"KiSS") {
        let groups = le16(kcf, 10)?;
        let colors = usize::from(le16(kcf, 8)?);
        if *kcf.get(4)? != PALETTE_MARK
            || !(1..=10).contains(&groups)
            || !(1..=256).contains(&colors)
        {
            return None;
        }
        (*kcf.get(5)?, colors, HEADER_LEN)
    } else if kcf.len() == OLD_PALETTE_LEN {
        (12, 16, 0)
    } else {
        return None;
    };
    let bytes = match bits {
        12 => 2,
        24 => 3,
        _ => return None,
    };
    let group = kcf.get(start..start.checked_add(colors * bytes)?)?;
    let expand = |v: u8| u32::from(v) * 17;
    Some(if bits == 12 {
        // `rrrr bbbb`, `0000 gggg`.
        let colors = group.as_chunks::<2>().0.iter();
        colors
            .map(|&[rb, g]| expand(rb >> 4) << 16 | expand(g & 15) << 8 | expand(rb & 15))
            .collect()
    } else {
        let colors = group.as_chunks::<3>().0.iter();
        colors
            .map(|&[r, g, b]| u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b))
            .collect()
    })
}

/// Black to white in `2^bits` steps: what a cel looks like without its palette.
fn gray_ramp(bits: u8) -> Vec<u32> {
    let top = (1u32 << bits) - 1;
    (0..=top).map(|i| i * 255 / top * 0x01_0101).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NoCompanions;

    struct Palette(Vec<u8>);

    impl Companions for Palette {
        fn get(&self, extension: &str) -> Option<Vec<u8>> {
            (extension == "kcf").then(|| self.0.clone())
        }

        fn get_named(&self, _file_name: &str) -> Option<Vec<u8>> {
            None
        }
    }

    fn header(mark: u8, bits: u8, width: u16, height: u16) -> Vec<u8> {
        let mut data = b"KiSS".to_vec();
        data.extend_from_slice(&[mark, bits, 0, 0]);
        data.extend_from_slice(&width.to_le_bytes());
        data.extend_from_slice(&height.to_le_bytes());
        data.resize(HEADER_LEN, 0);
        data
    }

    #[test]
    fn four_bit_rows_are_high_nibble_first_with_an_odd_width_padded() {
        let mut cel = header(CEL_MARK, 4, 3, 2);
        cel.extend_from_slice(&[0x1f, 0x20, 0xf1, 0x00]);
        let image = decode_cel(&cel, &NoCompanions).unwrap();
        assert_eq!((image.width(), image.height()), (3, 2));
        // Gray ramp: 1 = 0x11, 15 = white; index 0 is the transparent fill.
        let row = |y| [0, 1, 2].map(|x| image.get(x, y));
        assert_eq!(row(0), [0x111111, 0xffffff, 0x222222]);
        assert_eq!(row(1), [0xffffff, 0x111111, TRANSPARENT_FILL]);
    }

    #[test]
    fn a_twelve_bit_palette_swaps_green_and_blue_and_short_palettes_pad_black() {
        let mut kcf = header(PALETTE_MARK, 12, 0, 0);
        kcf[8..10].copy_from_slice(&2u16.to_le_bytes());
        kcf[10..12].copy_from_slice(&1u16.to_le_bytes());
        // Color 1: red 0xf, green 0x8, blue 0x1.
        kcf.extend_from_slice(&[0x00, 0x00, 0xf1, 0x08]);
        let mut cel = header(CEL_MARK, 8, 3, 1);
        cel.extend_from_slice(&[0, 1, 9]);
        let image = decode_cel(&cel, &Palette(kcf)).unwrap();
        assert_eq!(image.get(0, 0), TRANSPARENT_FILL);
        assert_eq!(image.get(1, 0), 0xff8811);
        assert_eq!(image.get(2, 0), 0);
    }

    #[test]
    fn the_conventional_cel_must_fill_the_file_exactly() {
        let cel = [3, 0, 1, 0, 0x12, 0x30];
        let image = decode_old_cel(&cel, &NoCompanions).unwrap();
        assert_eq!((image.width(), image.height()), (3, 1));
        assert!(decode_old_cel(&cel[..5], &NoCompanions).is_err());
        assert!(decode_old_cel(&[&cel[..], &[0]].concat(), &NoCompanions).is_err());
    }

    #[test]
    fn cherry_kiss_pixels_are_blue_first_and_blend_over_the_fill() {
        let mut cel = header(CKISS_MARK, 32, 3, 1);
        cel.extend_from_slice(&[0x10, 0x20, 0x30, 0xff, 0, 0, 0, 0, 0, 0, 0, 0x80]);
        let image = decode_cel(&cel, &NoCompanions).unwrap();
        assert_eq!(image.get(0, 0), 0x302010);
        assert_eq!(image.get(1, 0), TRANSPARENT_FILL);
        assert_eq!(image.get(2, 0), 0x606060);
    }

    #[test]
    fn palette_files_are_not_cels() {
        let kcf = header(PALETTE_MARK, 12, 4, 4);
        assert!(decode_cel(&kcf, &NoCompanions).is_err());
        assert!(decode_cel(&header(CEL_MARK, 16, 1, 1), &NoCompanions).is_err());
    }
}
