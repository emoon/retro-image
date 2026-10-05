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
//!   conventional palette (10 groups of 16 12-bit colors; `litks13.kcf` of
//!   the corpus has 16, so any whole number up to 16 is read).
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
//! alpha) are composited onto [`TRANSPARENT_FILL`]. A cel finds a palette file
//! only by its own stem, so a doll's cels, whose palettes have other names, are
//! gray alone; the set's `.cnf` file, read by `set.rs`, gives them their
//! palettes and places them.

use alloc::vec::Vec;

use crate::bytes::le16;
use crate::image::{TRANSPARENT_FILL, check_size, over};
use crate::{Companions, DecodeError, Format, Image};

mod set;

pub(super) static FORMATS: &[Format] = &[
    Format::with_companions("KiSS", "Cel", &["cel"], decode_cel).signature(),
    // No header to check: only the exact size tells it from other `.cel` files.
    Format::with_companions("KiSS", "Conventional cel", &["cel"], decode_old_cel),
    // A text file naming the cels and palettes it places; no signature.
    Format::with_companions("KiSS", "Set", &["cnf"], set::decode_set),
];

const FAIL: DecodeError = DecodeError::Unrecognized;
const HEADER_LEN: usize = 32;
const CEL_MARK: u8 = 0x20;
const CKISS_MARK: u8 = 0x21;
const PALETTE_MARK: u8 = 0x10;
/// A group of 16 colors of 2 bytes: the headerless palette file is a whole
/// number of them, 10 by the specification, up to 16 in the wild.
const OLD_GROUP_LEN: usize = 32;
const MAX_OLD_GROUPS: usize = 16;
/// Palette groups a headered palette file can hold.
const MAX_GROUPS: usize = 10;

/// A cel as read from its file, before a palette is applied.
struct Cel {
    width: usize,
    height: usize,
    /// Where the cel sits relative to its object's top left corner.
    x: usize,
    y: usize,
    pixels: Pixels,
}

enum Pixels {
    /// One palette index per pixel, from 4-bit or 8-bit data. Index 0 is
    /// transparent.
    Indexed { bits: u8, indices: Vec<u8> },
    /// Cherry KiSS pixels: blue, green, red, alpha.
    Direct(Vec<[u8; 4]>),
}

impl Cel {
    /// How many bits an index has, which sets the gray ramp used without a
    /// palette file. `None` for pixels that carry their own colors.
    fn index_bits(&self) -> Option<u8> {
        match self.pixels {
            Pixels::Indexed { bits, .. } => Some(bits),
            Pixels::Direct(_) => None,
        }
    }

    /// Draws the cel with its top left at (`x`, `y`) on `canvas`, clipped to
    /// it. Transparent pixels leave the canvas as it is. `palette` has the 256
    /// colors of an indexed cel.
    fn draw(&self, canvas: &mut Image, x: i64, y: i64, palette: &[u32]) {
        let (canvas_width, canvas_height) = (i64::from(canvas.width()), i64::from(canvas.height()));
        for row in 0..self.height {
            let target_y = y + row as i64;
            if !(0..canvas_height).contains(&target_y) {
                continue;
            }
            for column in 0..self.width {
                let target_x = x + column as i64;
                if !(0..canvas_width).contains(&target_x) {
                    continue;
                }
                let (tx, ty) = (target_x as u32, target_y as u32);
                match &self.pixels {
                    Pixels::Indexed { indices, .. } => {
                        let index = indices[row * self.width + column];
                        if index != 0 {
                            canvas.set(tx, ty, palette[usize::from(index)]);
                        }
                    }
                    Pixels::Direct(pixels) => {
                        let [b, g, r, alpha] = pixels[row * self.width + column];
                        let color = u32::from_be_bytes([0, r, g, b]);
                        if alpha != 0 {
                            canvas.set(tx, ty, over(canvas.get(tx, ty), color, alpha));
                        }
                    }
                }
            }
        }
    }
}

/// The cel of a file with the 32-byte header.
fn read_headered(data: &[u8]) -> Result<Cel, DecodeError> {
    if data.get(..4) != Some(b"KiSS") || !matches!(data.get(4), Some(&(CEL_MARK | CKISS_MARK))) {
        return Err(FAIL);
    }
    let bits = *data.get(5).ok_or(FAIL)?;
    let width = usize::from(le16(data, 8).ok_or(FAIL)?);
    let height = usize::from(le16(data, 10).ok_or(FAIL)?);
    check_size(width, height)?;
    let body = data.get(HEADER_LEN..).ok_or(FAIL)?;
    let pixels = match bits {
        4 | 8 => indexed(width, height, bits, body)?,
        32 => direct(width, height, body)?,
        _ => return Err(FAIL),
    };
    Ok(Cel {
        width,
        height,
        x: usize::from(le16(data, 12).ok_or(FAIL)?),
        y: usize::from(le16(data, 14).ok_or(FAIL)?),
        pixels,
    })
}

/// The cel of a file with either layout.
fn read_cel(data: &[u8]) -> Result<Cel, DecodeError> {
    read_headered(data).or_else(|_| read_conventional(data))
}

/// The cel of a file with no header: width and height, then 4-bit rows to the
/// end of the file.
fn read_conventional(data: &[u8]) -> Result<Cel, DecodeError> {
    let width = usize::from(le16(data, 0).ok_or(FAIL)?);
    let height = usize::from(le16(data, 2).ok_or(FAIL)?);
    check_size(width, height)?;
    let body = &data[4..];
    if body.len() != width.div_ceil(2) * height {
        return Err(FAIL);
    }
    Ok(Cel {
        width,
        height,
        x: 0,
        y: 0,
        pixels: indexed(width, height, 4, body)?,
    })
}

/// 4 or 8-bit rows as one index per pixel.
fn indexed(width: usize, height: usize, bits: u8, body: &[u8]) -> Result<Pixels, DecodeError> {
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
    Ok(Pixels::Indexed { bits, indices })
}

/// Cherry KiSS pixels, 4 bytes each.
fn direct(width: usize, height: usize, body: &[u8]) -> Result<Pixels, DecodeError> {
    let pixels = body.get(..width * height * 4).ok_or(FAIL)?;
    Ok(Pixels::Direct(pixels.as_chunks::<4>().0.to_vec()))
}

fn decode_cel(data: &[u8], companions: &dyn Companions) -> Result<Image, DecodeError> {
    picture(&read_headered(data)?, companions)
}

fn decode_old_cel(data: &[u8], companions: &dyn Companions) -> Result<Image, DecodeError> {
    picture(&read_conventional(data)?, companions)
}

/// The cel alone on the transparent fill, through the palette file with its
/// stem.
fn picture(cel: &Cel, companions: &dyn Companions) -> Result<Image, DecodeError> {
    let palette = full_palette(
        companions.get("kcf").and_then(|kcf| read_palette(&kcf, 0)),
        cel.index_bits(),
    );
    let fill = core::iter::repeat(TRANSPARENT_FILL);
    let mut image = Image::from_colors(cel.width as u32, cel.height as u32, fill);
    cel.draw(&mut image, 0, 0, &palette);
    Ok(image)
}

/// The 256 colors an indexed cel looks up: `colors` if there are some, else a
/// gray ramp, and black past the end of a short palette.
fn full_palette(colors: Option<Vec<u32>>, bits: Option<u8>) -> Vec<u32> {
    let mut palette = colors.unwrap_or_else(|| gray_ramp(bits.unwrap_or(8)));
    palette.resize(256, 0);
    palette
}

/// Colors of palette group `group` of a `.kcf` file (group 0 if the file has
/// fewer), `None` if it is neither the headered nor the conventional layout.
fn read_palette(kcf: &[u8], group: usize) -> Option<Vec<u32>> {
    let (bits, colors, groups, start) = if kcf.starts_with(b"KiSS") {
        let groups = usize::from(le16(kcf, 10)?);
        let colors = usize::from(le16(kcf, 8)?);
        if *kcf.get(4)? != PALETTE_MARK
            || !(1..=MAX_GROUPS).contains(&groups)
            || !(1..=256).contains(&colors)
        {
            return None;
        }
        (*kcf.get(5)?, colors, groups, HEADER_LEN)
    } else if kcf.len().is_multiple_of(OLD_GROUP_LEN)
        && (1..=MAX_OLD_GROUPS).contains(&(kcf.len() / OLD_GROUP_LEN))
    {
        (12, 16, kcf.len() / OLD_GROUP_LEN, 0)
    } else {
        return None;
    };
    let bytes = match bits {
        12 => 2,
        24 => 3,
        _ => return None,
    };
    let group = if group < groups { group } else { 0 };
    let from = start.checked_add(group * colors * bytes)?;
    let data = kcf.get(from..from.checked_add(colors * bytes)?)?;
    let expand = |v: u8| u32::from(v) * 17;
    Some(if bits == 12 {
        // `rrrr bbbb`, `0000 gggg`.
        let colors = data.as_chunks::<2>().0.iter();
        colors
            .map(|&[rb, g]| expand(rb >> 4) << 16 | expand(g & 15) << 8 | expand(rb & 15))
            .collect()
    } else {
        let colors = data.as_chunks::<3>().0.iter();
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
    fn a_conventional_palette_is_a_whole_number_of_groups_and_missing_groups_use_the_first() {
        // Two groups of 16 colors, `rrrr bbbb` and `0000 gggg` for each.
        let mut kcf = alloc::vec![0u8; 64];
        kcf[0] = 0xf0; // red 15, blue 0
        kcf[32] = 0x0f; // group 1: red 0, blue 15
        assert_eq!(read_palette(&kcf, 0).unwrap()[0], 0xff0000);
        assert_eq!(read_palette(&kcf, 1).unwrap()[0], 0x0000ff);
        // Group 5 does not exist: group 0.
        assert_eq!(read_palette(&kcf, 5).unwrap()[0], 0xff0000);
        assert!(read_palette(&kcf[..60], 0).is_none());
        assert_eq!(read_palette(&kcf, 0).unwrap().len(), 16);
    }

    #[test]
    fn palette_files_are_not_cels() {
        let kcf = header(PALETTE_MARK, 12, 4, 4);
        assert!(decode_cel(&kcf, &NoCompanions).is_err());
        assert!(decode_cel(&header(CEL_MARK, 16, 1, 1), &NoCompanions).is_err());
    }
}
