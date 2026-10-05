//! The "Super Hires" family: hires pictures with one or two layers of hires
//! sprites on top, some shown as two interlaced frames.
//!
//! Sources: no file documentation exists (Codebase64's grafix specs list,
//! <http://codebase.c64.org/doku.php?id=base:c64_grafix_files_specs_list_v0.03>,
//! only names the parts, e.g. "hires bitmap, 8 x screen RAM, 2 x hires
//! sprite layers"). Everything below was reverse engineered from the sample
//! files by feeding modified copies to `recoil2png` and watching which
//! pixels change:
//!
//! - Super Hires Interlace (SHI, 15355 bytes, load `$7FFF`): a zero byte,
//!   two 12×25-cell hires bitmaps (cells of 8 bytes, row by row), then the
//!   sprites of both frames (per frame ten 21-line bands of eight sprites:
//!   four for the upper layer, one per 24-pixel column, then four for the
//!   lower layer), one screen RAM of 12×25 cells shared by both frames, and
//!   the colors of the four upper and four lower sprite columns.
//! - Super Hires Interlace FLI (SIF): two packed sections, one per frame,
//!   then four color bytes (upper and lower sprite layer of the first
//!   frame, then of the second). Each section starts with `$9400` plus its
//!   own length, then an escape byte; the rest unpacks backwards
//!   (`value count escape` runs, count 0 = 256) to 8176 bytes of planes.
//! - Super Hires FLI (SHF) and SHF-XL (SHX), unpacked (load `$4000`): hires
//!   FLI (eight screen RAMs from `$4000`, bitmap `$6000`) with sprites whose
//!   block changes every line, as if the pointers came from the screen RAM
//!   of the previous raster line; line `y` of the picture shows row `y % 21`
//!   of the block. The blocks are the editors' fixed pointer layout (found
//!   in the samples); `recoil2png` ignores the pointer bytes in the file, and
//!   so do we. SHF: 26 columns from column 14, from bitmap line 1, 167
//!   lines, an upper layer (color `$43E8`) over a lower one (`$43E9`), four
//!   sprites each. SHX: 18 columns from column 11, 168 lines, one layer of
//!   six sprites (color `$43E9`).
//! - Packed SHF (any other size): two ignored bytes, an escape byte, then
//!   `escape count value` runs unpacking forwards to the planes of one SIF
//!   frame; the sprite colors are at offsets `$1FE8` and `$1FE9`.
//!   `recoil2png` uses the first for both layers; running Crest's Super
//!   Hires FLI Editor V1.0 (`SHF_V1_Fix.prg`,
//!   <http://c64.rulez.org/pub/c64/Tools/Graphics/Bitmap/>) in VICE as a
//!   black box showed the lower-layer sprites (4-7) take the second.
//! - Packed SHX (any other size): two ignored bytes, data packed backwards
//!   as in SIF, then the escape byte; it unpacks to three 3072-byte planes
//!   of 168 lines × 18 bytes (sprites, bitmap, colors), with the sprite
//!   color at `$BD1`.
//!
//! The planes of SIF and packed SHF are 2048 bytes apart, with 167 lines ×
//! 12 bytes each: upper sprite layer, lower sprite layer, hires bitmap, and
//! a color byte per 8-pixel cell per line (FLI).
//!
//! A set bit of an upper layer wins over lower layers, which win over the
//! bitmap; interlaced frames are averaged per channel.

use super::unpack::{Run, backward_rle_filled, escape_rle};
use super::vic2;
use crate::{DecodeError, Image};
use alloc::vec::Vec;

/// Width of the SHI, SIF and packed SHF pictures.
pub(super) const NARROW: usize = 96;
/// Bytes of a 96-pixel line or row of cells.
pub(super) const ROW: usize = NARROW / 8;

/// An image from `pixel(x, y)` color indices.
pub(super) fn render(width: usize, height: usize, pixel: impl Fn(usize, usize) -> u8) -> Image {
    let colors: Vec<u8> = (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .map(|(x, y)| pixel(x, y))
        .collect();
    vic2::image(width, height, colors)
}

/// Whether bit `x` (MSB first) of a line of bytes is set.
pub(super) fn bit(line: &[u8], x: usize) -> bool {
    line[x / 8] & (0x80 >> (x % 8)) != 0
}

/// A hires pixel: the high nibble of `color` if set, else the low nibble.
pub(super) fn hires(set: bool, color: u8) -> u8 {
    if set { color >> 4 } else { color & 15 }
}

/// Sprite layers over a hires bitmap, stored as planes of one byte per
/// eight pixels per line, `spacing` bytes apart.
struct Planes<'a> {
    data: &'a [u8],
    width: usize,
    height: usize,
    spacing: usize,
    /// Plane index and color of each sprite layer, topmost first.
    layers: &'a [(usize, u8)],
    bitmap: usize,
    /// Plane of color bytes: set pixels use the high nibble.
    colors: usize,
}

impl Planes<'_> {
    fn fits(&self) -> bool {
        let last = self.layers.iter().map(|&(plane, _)| plane);
        let last = last.chain([self.bitmap, self.colors]).max().unwrap_or(0);
        self.data.len() >= last * self.spacing + self.height * self.width / 8
    }

    fn pixel(&self, x: usize, y: usize) -> u8 {
        let row = self.width / 8;
        let line = |plane: usize| &self.data[plane * self.spacing + y * row..][..row];
        self.layers
            .iter()
            .find(|&&(plane, _)| bit(line(plane), x))
            .map_or_else(
                || hires(bit(line(self.bitmap), x), line(self.colors)[x / 8]),
                |&(_, color)| color,
            )
    }

    fn image(&self) -> Result<Image, DecodeError> {
        if !self.fits() {
            return Err(DecodeError::Unrecognized);
        }
        Ok(render(self.width, self.height, |x, y| self.pixel(x, y)))
    }
}

const SHI_LEN: usize = 15355;
const SHI_BITMAP: usize = 3;
const SHI_BITMAP_LEN: usize = 25 * ROW * 8;
const SHI_SPRITES: usize = SHI_BITMAP + 2 * SHI_BITMAP_LEN;
/// Ten bands of eight 64-byte sprites.
const SHI_SPRITES_LEN: usize = 10 * 8 * 64;
const SHI_SCREEN: usize = SHI_SPRITES + 2 * SHI_SPRITES_LEN;
const SHI_COLORS: usize = SHI_SCREEN + 25 * ROW;

/// Super Hires Interlace Editor.
pub(super) fn decode_shi(data: &[u8]) -> Result<Image, DecodeError> {
    // `recoil2png` reads other sizes, or a nonzero byte at `$7FFF`, some
    // other way; there are no such samples.
    if data.len() != SHI_LEN || data[2] != 0 {
        return Err(DecodeError::Unrecognized);
    }
    let colors = &data[SHI_COLORS..SHI_COLORS + 8];
    let frame = |frame: usize| {
        render(NARROW, 200, |x, y| {
            let (band, line) = (y / 21, y % 21);
            let column = x / 24;
            let sprite = |layer: usize| {
                let start = SHI_SPRITES
                    + frame * SHI_SPRITES_LEN
                    + band * 512
                    + layer * 256
                    + column * 64
                    + line * 3;
                bit(&data[start..start + 3], x % 24)
            };
            if sprite(0) {
                colors[column]
            } else if sprite(1) {
                colors[4 + column]
            } else {
                let cell = y / 8 * ROW + x / 8;
                let byte = data[SHI_BITMAP + frame * SHI_BITMAP_LEN + cell * 8 + y % 8];
                hires(bit(&[byte], x % 8), data[SHI_SCREEN + cell])
            }
        })
    };
    Ok(Image::blend(&[&frame(0), &frame(1)]))
}

/// Unpacked size of a SIF frame or a packed SHF picture.
const PLANES_LEN: usize = 4 * 2048 - 16;

/// One frame of SIF or a packed SHF picture.
fn four_planes(data: &[u8], colors: [u8; 2]) -> Result<Image, DecodeError> {
    Planes {
        data,
        width: NARROW,
        height: 167,
        spacing: 2048,
        layers: &[(0, colors[0]), (1, colors[1])],
        bitmap: 2,
        colors: 3,
    }
    .image()
}

/// Splits off one packed SIF section and unpacks it.
fn sif_section(data: &[u8]) -> Option<(Vec<u8>, &[u8])> {
    let len = usize::from(crate::bytes::le16(data, 0)?).checked_sub(0x9400)?;
    let (section, rest) = data.split_at_checked(len)?;
    let [_, _, escape, packed @ ..] = section else {
        return None;
    };
    let (planes, exact) = backward_rle_filled(packed, *escape, PLANES_LEN)?;
    exact.then_some((planes, rest))
}

/// Super Hires Interlace FLI Editor.
pub(super) fn decode_sif(data: &[u8]) -> Result<Image, DecodeError> {
    let (first, rest) = sif_section(data).ok_or(DecodeError::Unrecognized)?;
    let (second, rest) = sif_section(rest).ok_or(DecodeError::Unrecognized)?;
    let [c0, c1, c2, c3] = *rest else {
        return Err(DecodeError::Unrecognized);
    };
    Ok(Image::blend(&[
        &four_planes(&first, [c0, c1])?,
        &four_planes(&second, [c2, c3])?,
    ]))
}

/// A sprite layer of unpacked SHF/SHX: the block (pointer) of each 24-pixel
/// column for each screen RAM, and the address of the layer's color.
struct Layer {
    blocks: [&'static [u8]; 8],
    color: u16,
}

/// Unpacked SHF and SHX: hires FLI at `$4000` with sprites whose blocks
/// change every line.
struct SpriteFli {
    len: usize,
    first_column: usize,
    columns: usize,
    /// Bitmap line shown at the top.
    first_line: usize,
    height: usize,
    /// Sprite layers, topmost first.
    layers: &'static [Layer],
}

impl SpriteFli {
    fn decode(&self, data: &[u8]) -> Result<Image, DecodeError> {
        if data.len() != self.len {
            return Err(DecodeError::Unrecognized);
        }
        // The VIC bank `$4000-$7FFF`; sprite blocks may lie past the file.
        let mut mem = alloc::vec![0u8; 0x4000];
        let body = &data[2..];
        mem[..body.len()].copy_from_slice(body);
        Ok(render(self.columns * 8, self.height, |x, y| {
            let line = self.first_line + y;
            let previous = (line + 7) % 8;
            let column = x / 24;
            for layer in self.layers {
                let Some(&block) = layer.blocks[previous].get(column) else {
                    continue;
                };
                let start = usize::from(block) * 64 + y % 21 * 3;
                if bit(&mem[start..start + 3], x % 24) {
                    return mem[usize::from(layer.color) - 0x4000];
                }
            }
            let cell = line / 8 * 40 + self.first_column + x / 8;
            let byte = mem[0x2000 + cell * 8 + line % 8];
            hires(bit(&[byte], x % 8), mem[line % 8 * 0x400 + cell])
        }))
    }
}

const SHF: SpriteFli = SpriteFli {
    len: 2 + 0x3e00,
    first_column: 14,
    columns: 26,
    first_line: 1,
    height: 167,
    layers: &[
        Layer {
            blocks: [
                &[0x80, 0x84, 0x85, 0x89],
                &[0x94, 0x98, 0x99, 0x9d],
                &[0xa8, 0xac, 0xad, 0xb1],
                &[0xbc, 0xc0, 0xc1, 0xc5],
                &[0xd0, 0xd4, 0xd5, 0xd9],
                &[0xe4, 0xe8, 0xe9, 0xea],
                &[0xef, 0xf0, 0xf1, 0xf2],
                &[0xf7, 0x1e, 0x2e, 0x3e],
            ],
            color: 0x43e8,
        },
        Layer {
            blocks: [
                &[0x8a, 0x8e, 0x8f, 0x93],
                &[0x9e, 0xa2, 0xa3, 0xa7],
                &[0xb2, 0xb6, 0xb7, 0xbb],
                &[0xc6, 0xca, 0xcb, 0xcf],
                &[0xda, 0xde, 0xdf, 0xe3],
                &[0xeb, 0xec, 0xed, 0xee],
                &[0xf3, 0xf4, 0xf5, 0xf6],
                &[0x4e, 0x5e, 0x6e, 0x7e],
            ],
            color: 0x43e9,
        },
    ],
};

const SHX: SpriteFli = SpriteFli {
    len: 2 + 0x3c00,
    first_column: 11,
    columns: 18,
    first_line: 0,
    height: 168,
    layers: &[Layer {
        blocks: [
            &[0x80, 0x84, 0x85, 0x89, 0x8a, 0x8e],
            &[0x8f, 0x93, 0x94, 0x98, 0x99, 0x9d],
            &[0x9e, 0xa2, 0xa3, 0xa7, 0xa8, 0xac],
            &[0xad, 0xb1, 0xb2, 0xb6, 0xb7, 0xbb],
            &[0xbc, 0xc0, 0xc1, 0xc5, 0xc6, 0xca],
            &[0xcb, 0xcf, 0xd0, 0xd4, 0xd5, 0xd9],
            &[0xda, 0xde, 0xdf, 0xe3, 0xe4, 0xe8],
            &[0xe9, 0xea, 0xeb, 0xec, 0xed, 0xee],
        ],
        color: 0x43e9,
    }],
};

/// Super Hires FLI Editor: unpacked, or packed forwards.
pub(super) fn decode_shf(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() == SHF.len {
        return SHF.decode(data);
    }
    let [_, _, escape, packed @ ..] = data else {
        return Err(DecodeError::Unrecognized);
    };
    let mut planes = escape_rle(packed, *escape, Run::CountValue, PLANES_LEN)
        .ok_or(DecodeError::Unrecognized)?;
    // Short data is accepted as long as the colors are there.
    if planes.len() <= 0x1fe9 {
        return Err(DecodeError::Unrecognized);
    }
    planes.resize(PLANES_LEN, 0);
    // `recoil2png` paints both layers in the first color; the editor
    // itself uses the second for the lower layer (see the module notes).
    four_planes(&planes, [planes[0x1fe8], planes[0x1fe9]])
}

/// SHF-XL Edit: unpacked, or packed backwards with the escape byte last.
pub(super) fn decode_shx(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() == SHX.len {
        return SHX.decode(data);
    }
    let [_, _, packed @ .., escape] = data else {
        return Err(DecodeError::Unrecognized);
    };
    let (planes, _) =
        backward_rle_filled(packed, *escape, 3 * 3072 - 48).ok_or(DecodeError::Unrecognized)?;
    Planes {
        data: &planes,
        width: 144,
        height: 168,
        spacing: 3072,
        layers: &[(0, planes[0xbd1])],
        bitmap: 1,
        colors: 2,
    }
    .image()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mix(a: u8, b: u8) -> u32 {
        Image::blend(&[
            &vic2::image(1, 1, alloc::vec![a]),
            &vic2::image(1, 1, alloc::vec![b]),
        ])
        .get(0, 0)
    }

    #[test]
    fn shi_layers_cover_the_bitmap() {
        let mut data = alloc::vec![0u8; SHI_LEN];
        data[SHI_SCREEN] = 0x21;
        data[SHI_BITMAP] = 0x80; // frame 0, pixel (0, 0) set
        data[SHI_COLORS] = 5;
        data[SHI_COLORS + 4] = 7;
        // Frame 1, band 0: lower-layer sprite of column 0 sets pixel (0, 0).
        data[SHI_SPRITES + SHI_SPRITES_LEN + 256] = 0x80;
        // Frame 0 and 1: upper layer sets pixel (1, 0).
        data[SHI_SPRITES] = 0x40;
        data[SHI_SPRITES + SHI_SPRITES_LEN] = 0x40;
        let image = decode_shi(&data).unwrap();
        assert_eq!(image.get(0, 0), mix(2, 7));
        assert_eq!(image.get(1, 0), vic2::rgb(5));
        assert_eq!(image.get(2, 0), vic2::rgb(1));
    }

    /// A SIF section of `PLANES_LEN` zero bytes: 31 full runs and one of 240.
    fn zero_section() -> Vec<u8> {
        let mut section = alloc::vec![0, 0, 0xee];
        for _ in 0..31 {
            section.extend_from_slice(&[0, 0, 0xee]);
        }
        section.extend_from_slice(&[0, 240, 0xee]);
        let len = 0x9400 + section.len() as u16;
        section[..2].copy_from_slice(&len.to_le_bytes());
        section
    }

    #[test]
    fn sif_needs_two_exact_sections_and_colours() {
        let mut data = [zero_section(), zero_section()].concat();
        data.extend_from_slice(&[1, 2, 3, 4]);
        assert!(decode_sif(&data).is_ok());
        assert!(decode_sif(&data[..data.len() - 1]).is_err());
        data.push(0);
        assert!(decode_sif(&data).is_err());
    }

    #[test]
    fn sprite_blocks_follow_the_previous_line() {
        let mut data = alloc::vec![0u8; SHF.len];
        let mem = |addr: usize| addr - 0x4000 + 2;
        // Picture line 0 is bitmap line 1: screen 1 colors, screen 0's
        // blocks; the lower layer's column 0 is block $8A at $6280.
        data[mem(0x4400 + 14)] = 0x34;
        data[mem(0x6280)] = 0x40;
        data[mem(0x43e8)] = 1;
        data[mem(0x43e9)] = 2;
        let image = decode_shf(&data).unwrap();
        assert_eq!(image.get(0, 0), vic2::rgb(4));
        assert_eq!(image.get(1, 0), vic2::rgb(2));
    }

    #[test]
    fn packed_shf_needs_its_colours() {
        let mut data = alloc::vec![0, 0, 0xee];
        for _ in 0..31 {
            data.extend_from_slice(&[0xee, 0, 0]);
        }
        data.extend_from_slice(&[0xee, 232, 0, 5]);
        assert!(decode_shf(&data).is_err());
        data.push(6);
        assert!(decode_shf(&data).is_ok());
    }
}
