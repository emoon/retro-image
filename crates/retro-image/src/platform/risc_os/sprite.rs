//! RISC OS sprite files (filetype &FF9).
//!
//! Sources:
//! - Sprite area and sprite header layout, word-aligned rows, the least
//!   significant pixel of a word being the leftmost, palette entries as two
//!   `&BBGGRR00` words, masks: RISC OS PRM, "Sprites",
//!   <http://www.riscos.com/support/developers/prm/sprites.html>.
//! - A sprite file is a sprite area without its first word (the area
//!   size): PRM, "Appendix E: File formats",
//!   <http://www.riscos.com/support/developers/prm/fileformats.html>.
//! - New format sprites (RISC OS 3.5): the mode word (sprite type in bits
//!   27-31, vertical DPI in bits 14-26, horizontal DPI in bits 1-13, bit 0
//!   set), no left-hand wastage, sprite types 1-6 with the 16 bpp (red bits
//!   0-4, green 5-9, blue 10-14) and 32 bpp (red, green, blue bytes)
//!   pixels, DPI 180/90/45/22 as eigen factors 0-3, palettes for up to
//!   8 bpp from RISC OS 3.6: PRM volume 5a, "Video",
//!   <http://www.riscos.com/support/developers/prm/video.html>.
//! - Masks (PRM, "Sprites", "Masks"): the sprite's mask offset is the image
//!   offset when there is no mask. In a sprite with a mode number the mask
//!   has the image's layout (width in words, bits per pixel, left-hand
//!   wastage); in a RISC OS 3.5 sprite it has 1 bit per pixel, rows padded to
//!   whole words. A pixel is opaque when its mask bits are not all zero.
//!   A mask that does not fit in the file is ignored.
//! - Mode numbers 0-46 (pixel resolution, OS-unit resolution, colours):
//!   PRM volume 4, "Table B: Modes",
//!   <http://www.riscos.com/support/developers/prm/modes.html>; the eigen
//!   factors are log2(OS units / pixels).
//! - Default 256-colour palette (bits 0-1 tint, 2 red bit 2, 3 blue bit 2,
//!   4 red bit 3, 5 green bit 2, 6 green bit 3, 7 blue bit 3), and 16-entry
//!   (VIDC1) palettes of 256-colour sprites (the low 4 bits of a pixel pick
//!   the entry, the high 4 override red bit 3, green bits 2-3 and blue bit
//!   3): PRM, "VDU drivers",
//!   <http://www.riscos.com/support/developers/prm/vdu.html>. Such palettes
//!   were found in mode 28 sprites (sembiance samples `*.bin,FF9`), where
//!   they hold the default palette's first 16 entries.
//! - Palettes of sprites without one: the Wimp colours, as RISC OS's Paint
//!   shows them (2 colours: Wimp colours 0 and 7; 4 colours: 0, 2, 4, 7),
//!   PRM, "The Window Manager", "Colour handling",
//!   <http://www.riscos.com/support/developers/prm/wimp.html>. The Wimp
//!   colour values, including the RISC OS 3.5 dark blue `#4499FF` (RISC OS
//!   3's `*Desktop_SetPalette` example has `#004499`), and averaging the two
//!   (flash) colours of a palette entry, are those of Deark's `rosprite`
//!   module (`modules/rosprite.c`, MIT licence, notice below), which is also
//!   the black-box reference for this decoder.
//! - 16-bit pixels scaled to 8 bits by rounding `v * 255 / 31`, as Deark
//!   does (`de_bgr555_to_888` in `src/deark-data.c`, same licence).
//!   Bit replication (`image::bgr555`) differs from it for 4 of the 32 values.
//!
//! Portions derived from Deark (<https://entropymine.com/deark/>),
//! `modules/rosprite.c` and `src/deark-data.c`:
//!
//! Copyright (C) 2016-2026 Jason Summers <jason1@pobox.com>
//!
//! Permission is hereby granted, free of charge, to any person obtaining a
//! copy of this software and associated documentation files (the
//! "Software"), to deal in the Software without restriction, including
//! without limitation the rights to use, copy, modify, merge, publish,
//! distribute, sublicense, and/or sell copies of the Software, and to permit
//! persons to whom the Software is furnished to do so, subject to the
//! following conditions:
//!
//! The above copyright notice and this permission notice shall be included
//! in all copies or substantial portions of the Software.
//!
//! THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
//! OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
//! MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN
//! NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
//! DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR
//! OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE
//! USE OR OTHER DEALINGS IN THE SOFTWARE.

use alloc::vec::Vec;

use crate::bytes::{le16, le32};
use crate::image::check_size;
use crate::{DecodeError, Image};

/// Sprite header length; a palette, if any, follows it.
const HEADER_LEN: usize = 44;

/// Decodes the first sprite of a sprite file.
///
/// The file header and the first sprite's header are validated strictly
/// (offsets inside the file, a NUL-padded printable name, a known mode,
/// pixel-aligned used bits, image data inside the sprite), so the format is
/// also recognised by content: on disk, sprite files are often named
/// `name,ff9`, which has no extension.
pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let word = |at: usize| le32(data, at).map(|w| w as usize).ok_or(fail);
    // Offsets in the file are sprite-area offsets: 4 more than file offsets.
    let (count, first, free) = (word(0)?, word(4)?, word(8)?);
    if !(1..=10_000).contains(&count) || first < 16 || first % 4 != 0 || free % 4 != 0 {
        return Err(fail);
    }
    let start = first - 4;
    let end = free.checked_sub(4).ok_or(fail)?;
    if end > data.len() || start.checked_add(HEADER_LEN).is_none_or(|e| e > end) {
        return Err(fail);
    }
    let sprite = &data[start..];
    let size = word(start)?;
    if size < HEADER_LEN || size > sprite.len() {
        return Err(fail);
    }
    // Some files have a palette added without the sprite size and the file
    // header being updated, so the image may run past both: it is only
    // required to fit in the file.
    decode_sprite(sprite)
}

/// Decodes one sprite: its header, palette and image.
fn decode_sprite(sprite: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let word = |at: usize| le32(sprite, at).ok_or(fail);
    if !valid_name(&sprite[4..16]) {
        return Err(fail);
    }
    let width_words = word(16)?.checked_add(1).ok_or(fail)? as usize;
    let height = word(20)?.checked_add(1).ok_or(fail)? as usize;
    let (first_bit, last_bit) = (word(24)? as usize, word(28)? as usize);
    let image_at = word(32)? as usize;
    let format = PixelFormat::from_mode_word(word(40)?).ok_or(fail)?;
    let bpp = format.bits_per_pixel();
    // New format sprites have no left-hand wastage.
    let first_bit = if format.is_mode_number { first_bit } else { 0 };
    if first_bit > 31 || last_bit > 31 || first_bit % bpp != 0 || (last_bit + 1) % bpp != 0 {
        return Err(fail);
    }
    let row_bits = width_words.checked_mul(32).ok_or(fail)?;
    let width = row_bits
        .checked_sub(first_bit + (31 - last_bit))
        .ok_or(fail)?
        / bpp;
    let stride = width_words * 4;
    let pixels_len = stride.checked_mul(height).ok_or(fail)?;
    let pixels = image_at
        .checked_add(pixels_len)
        .filter(|_| image_at >= HEADER_LEN)
        .and_then(|end| sprite.get(image_at..end))
        .ok_or(fail)?;
    if width == 0 || width > u32::MAX as usize || height > u32::MAX as usize {
        return Err(fail);
    }
    // The palette runs from the header to the image.
    let (sx, sy) = format.pixel_scale;
    check_size(width * sx as usize, height * sy as usize)?;
    let palette = &sprite[HEADER_LEN..image_at];

    // The mask has the image's layout for a mode number, else 1 bit per pixel.
    let mask_at = word(36)? as usize;
    let (mask_bpp, mask_stride) = if format.is_mode_number {
        (bpp, stride)
    } else {
        (1, width.div_ceil(32) * 4)
    };
    let mask = (mask_at != image_at)
        .then(|| mask_stride.checked_mul(height)?.checked_add(mask_at))
        .flatten()
        .filter(|_| mask_at >= HEADER_LEN)
        .and_then(|end| sprite.get(mask_at..end));
    let mut image = Image::new(width as u32, height as u32);
    match format.kind {
        Kind::Indexed => {
            let colors = sprite_palette(palette, bpp);
            for (y, row) in pixels.chunks_exact(stride).enumerate() {
                for x in 0..width {
                    let index = indexed_pixel(row, first_bit + x * bpp, bpp);
                    let color = colors.color(index);
                    image.set(x as u32, y as u32, color);
                }
            }
        }
        Kind::Tbgr1555 => {
            for (y, row) in pixels.chunks_exact(stride).enumerate() {
                for x in 0..width {
                    let value = le16(row, x * 2).ok_or(fail)?;
                    image.set(x as u32, y as u32, rgb555(value));
                }
            }
        }
        Kind::Tbgr8888 => {
            for (y, row) in pixels.chunks_exact(stride).enumerate() {
                for (x, px) in row.as_chunks::<4>().0.iter().take(width).enumerate() {
                    let color = u32::from(px[0]) << 16 | u32::from(px[1]) << 8 | u32::from(px[2]);
                    image.set(x as u32, y as u32, color);
                }
            }
        }
    }
    if let Some(mask) = mask {
        let opaque =
            |row: &[u8], x: usize| indexed_pixel(row, first_bit + x * mask_bpp, mask_bpp) != 0;
        let alpha = mask
            .chunks_exact(mask_stride)
            .flat_map(|row| (0..width).map(move |x| if opaque(row, x) { 255 } else { 0 }))
            .collect();
        image = image.with_alpha(alpha);
    }
    image.scaled(sx, sy)
}

/// A sprite name: 1-12 printable characters, padded with NULs.
fn valid_name(name: &[u8]) -> bool {
    let len = name.iter().position(|&b| b == 0).unwrap_or(name.len());
    len > 0
        && name[..len].iter().all(|&b| b >= 0x20 && b != 0x7f)
        && name[len..].iter().all(|&b| b == 0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// 1, 2, 4 or 8 bits per pixel through a palette.
    Indexed,
    /// 16 bits: red in bits 0-4, green 5-9, blue 10-14.
    Tbgr1555,
    /// 32 bits: bytes red, green, blue, unused.
    Tbgr8888,
}

/// A 16 bpp pixel: red in bits 0-4, green 5-9, blue 10-14, each scaled to
/// 0-255 and rounded.
fn rgb555(value: u16) -> u32 {
    let scale = |shift: u16| (u32::from(value >> shift & 0x1f) * 255 + 15) / 31;
    scale(0) << 16 | scale(5) << 8 | scale(10)
}

/// The pixel format and shape a sprite's mode word describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PixelFormat {
    kind: Kind,
    /// Bits per pixel of indexed formats.
    indexed_bpp: usize,
    /// Output scale making the mode's pixels square (width, height).
    pixel_scale: (u32, u32),
    /// An old-style mode number rather than a new format mode word.
    is_mode_number: bool,
}

impl PixelFormat {
    fn bits_per_pixel(&self) -> usize {
        match self.kind {
            Kind::Indexed => self.indexed_bpp,
            Kind::Tbgr1555 => 16,
            Kind::Tbgr8888 => 32,
        }
    }

    fn from_mode_word(mode: u32) -> Option<Self> {
        if mode < 256 {
            let &(_, bpp, x_eig, y_eig) = MODES.iter().find(|m| u32::from(m.0) == mode)?;
            let mut format = Self::new(sprite_type_for_bpp(bpp), x_eig, y_eig)?;
            format.is_mode_number = true;
            return Some(format);
        }
        if mode & 1 == 0 {
            return None; // a pointer to a mode selector block: not valid in a file
        }
        // RISC OS 3.5: tttt tyyy yyyy yyyy yyxx xxxx xxxx xxx1.
        let sprite_type = mode >> 27;
        let x_dpi = mode >> 1 & 0x1fff;
        let y_dpi = mode >> 14 & 0x1fff;
        if x_dpi == 0 || y_dpi == 0 {
            return None;
        }
        let mut format = Self::new(sprite_type, 0, 0)?;
        format.pixel_scale = match (dpi_eigen(x_dpi), dpi_eigen(y_dpi)) {
            (Some(x), Some(y)) => square_pixels(x, y),
            _ => (1, 1),
        };
        Some(format)
    }

    fn new(sprite_type: u32, x_eig: u32, y_eig: u32) -> Option<Self> {
        let (kind, indexed_bpp) = match sprite_type {
            1..=4 => (Kind::Indexed, 1 << (sprite_type - 1)),
            5 => (Kind::Tbgr1555, 0),
            6 => (Kind::Tbgr8888, 0),
            _ => return None,
        };
        Some(Self {
            kind,
            indexed_bpp,
            pixel_scale: square_pixels(x_eig, y_eig),
            is_mode_number: false,
        })
    }
}

fn sprite_type_for_bpp(bpp: u8) -> u32 {
    u32::from(bpp).trailing_zeros() + 1
}

/// Eigen value of a DPI value: 180 DPI is 0, 90 is 1, 45 is 2, 22 or 23 is 3.
fn dpi_eigen(dpi: u32) -> Option<u32> {
    match dpi {
        180 => Some(0),
        90 => Some(1),
        45 => Some(2),
        22 | 23 => Some(3),
        _ => None,
    }
}

/// Repeats pixels so that a mode with eigen factors (`x_eig`, `y_eig`)
/// gets square pixels: a pixel is `2^eig` OS units in each direction.
fn square_pixels(x_eig: u32, y_eig: u32) -> (u32, u32) {
    let low = x_eig.min(y_eig);
    // Differences beyond 4:1 aren't real display modes; leave them alone.
    if x_eig - low > 2 || y_eig - low > 2 {
        return (1, 1);
    }
    (1 << (x_eig - low), 1 << (y_eig - low))
}

/// Mode number, bits per pixel, x and y eigen factors (log2 of OS units per
/// pixel). Modes 3, 6 and 7 are text-only.
const MODES: &[(u8, u8, u32, u32)] = &[
    (0, 1, 1, 2),
    (1, 2, 2, 2),
    (2, 4, 3, 2),
    (4, 1, 2, 2),
    (5, 2, 3, 2),
    (8, 2, 1, 2),
    (9, 4, 2, 2),
    (10, 8, 3, 2),
    (11, 2, 1, 2),
    (12, 4, 1, 2),
    (13, 8, 2, 2),
    (14, 4, 1, 2),
    (15, 8, 1, 2),
    (16, 4, 1, 2),
    (17, 4, 1, 2),
    (18, 1, 1, 1),
    (19, 2, 1, 1),
    (20, 4, 1, 1),
    (21, 8, 1, 1),
    (22, 4, 0, 1),
    (23, 1, 1, 1),
    (24, 8, 1, 2),
    (25, 1, 1, 1),
    (26, 2, 1, 1),
    (27, 4, 1, 1),
    (28, 8, 1, 1),
    (29, 1, 1, 1),
    (30, 2, 1, 1),
    (31, 4, 1, 1),
    (33, 1, 1, 2),
    (34, 2, 1, 2),
    (35, 4, 1, 2),
    (36, 8, 1, 2),
    (37, 1, 1, 2),
    (38, 2, 1, 2),
    (39, 4, 1, 2),
    (40, 8, 1, 2),
    (41, 1, 1, 2),
    (42, 2, 1, 2),
    (43, 4, 1, 2),
    (44, 1, 1, 2),
    (45, 2, 1, 2),
    (46, 4, 1, 2),
];

/// The value of the pixel starting at bit `bit` of a row: pixels fill each
/// byte from its least significant bits (the leftmost pixel) up.
fn indexed_pixel(row: &[u8], bit: usize, bpp: usize) -> u8 {
    let byte = row[bit / 8];
    if bpp == 8 {
        byte
    } else {
        byte >> (bit % 8) & ((1 << bpp) - 1)
    }
}

/// Colours for an indexed sprite.
enum Colors {
    Table(Vec<u32>),
    /// A 16-entry palette in a 256-colour sprite (VIDC1): the low 4 bits of
    /// a pixel pick the entry, the high 4 bits override red bit 3, green
    /// bits 2 and 3 and blue bit 3.
    Vidc1(Vec<u32>),
}

impl Colors {
    fn color(&self, index: u8) -> u32 {
        match self {
            Colors::Table(table) => table[usize::from(index)],
            Colors::Vidc1(table) => {
                let base = table[usize::from(index & 0xf)];
                let bit = |n: u8| u32::from(index >> n & 1);
                // VIDC1 has 4-bit guns: the top nibble of each palette byte.
                let gun = |shift: u32| base >> (shift + 4) & 0xf;
                let r = gun(16) & 7 | bit(4) << 3;
                let g = gun(8) & 3 | bit(5) << 2 | bit(6) << 3;
                let b = gun(0) & 7 | bit(7) << 3;
                let byte = |v: u32| v << 4 | v;
                byte(r) << 16 | byte(g) << 8 | byte(b)
            }
        }
    }
}

/// The sprite's own palette if it has one for every colour, else the
/// default one. Each entry is two `&BBGGRR00` words (the two flash colours),
/// shown averaged.
fn sprite_palette(palette: &[u8], bpp: usize) -> Colors {
    let colors = 1usize << bpp;
    let entries: Vec<u32> = palette
        .as_chunks::<8>()
        .0
        .iter()
        .take(256)
        .map(|entry| {
            let channel = |word: usize, byte: usize| u32::from(entry[word * 4 + byte]);
            let average = |byte| (channel(0, byte) + channel(1, byte)) / 2;
            average(1) << 16 | average(2) << 8 | average(3)
        })
        .collect();
    if entries.len() >= colors {
        Colors::Table(entries[..colors].to_vec())
    } else if bpp == 8 && entries.len() == 16 {
        Colors::Vidc1(entries)
    } else {
        Colors::Table(default_palette(bpp))
    }
}

/// The Wimp colours from white (0) to light blue (15), as `0xRRGGBB`.
const WIMP: [u32; 16] = [
    0xffffff, 0xdddddd, 0xbbbbbb, 0x999999, 0x777777, 0x555555, 0x333333, 0x000000, 0x4499ff,
    0xeeee00, 0x00cc00, 0xdd0000, 0xeeeebb, 0x558800, 0xffbb00, 0x00bbff,
];

fn default_palette(bpp: usize) -> Vec<u32> {
    match bpp {
        1 => alloc::vec![WIMP[0], WIMP[7]],
        2 => alloc::vec![WIMP[0], WIMP[2], WIMP[4], WIMP[7]],
        4 => WIMP.to_vec(),
        _ => (0..=255).map(default_256_color).collect(),
    }
}

/// The default 256-colour palette: bits 0-1 are a tint added to all three
/// guns, bits 2-7 are red bit 2, blue bit 2, red bit 3, green bit 2, green
/// bit 3, blue bit 3 of the 4-bit guns.
fn default_256_color(index: u8) -> u32 {
    let bit = |n: u8| u32::from(index >> n & 1);
    let tint = u32::from(index & 3);
    let r = tint | bit(2) << 2 | bit(4) << 3;
    let g = tint | bit(5) << 2 | bit(6) << 3;
    let b = tint | bit(3) << 2 | bit(7) << 3;
    let gun = |v: u32| v << 4 | v;
    gun(r) << 16 | gun(g) << 8 | gun(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_numbers_give_depth_and_pixel_shape() {
        let mode = |m| PixelFormat::from_mode_word(m).unwrap();
        assert_eq!(
            (mode(12).bits_per_pixel(), mode(12).pixel_scale),
            (4, (1, 2))
        );
        assert_eq!(
            (mode(27).bits_per_pixel(), mode(27).pixel_scale),
            (4, (1, 1))
        );
        assert_eq!(
            (mode(13).bits_per_pixel(), mode(13).pixel_scale),
            (8, (1, 1))
        );
        assert_eq!((mode(2).bits_per_pixel(), mode(2).pixel_scale), (4, (2, 1)));
        assert_eq!(mode(0).bits_per_pixel(), 1);
        assert!(mode(28).is_mode_number);
        assert_eq!(PixelFormat::from_mode_word(7), None, "teletext");
        assert_eq!(PixelFormat::from_mode_word(200), None);
    }

    #[test]
    fn risc_os_3_5_mode_words_give_type_and_dpi() {
        // Type 6 (32 bpp), 90 x 90 DPI.
        let word = 6 << 27 | 90 << 14 | 90 << 1 | 1;
        let format = PixelFormat::from_mode_word(word).unwrap();
        assert_eq!((format.kind, format.pixel_scale), (Kind::Tbgr8888, (1, 1)));
        assert!(!format.is_mode_number);
        // Type 5 (16 bpp), 90 DPI across, 45 down: pixels twice as tall.
        let word = 5 << 27 | 45 << 14 | 90 << 1 | 1;
        let format = PixelFormat::from_mode_word(word).unwrap();
        assert_eq!((format.kind, format.pixel_scale), (Kind::Tbgr1555, (1, 2)));
        // Type 3 (4 bpp) with an odd DPI keeps its pixels.
        let word = 3 << 27 | 72 << 14 | 72 << 1 | 1;
        assert_eq!(
            PixelFormat::from_mode_word(word).unwrap().bits_per_pixel(),
            4
        );
        // Zero DPI, unknown types, mode selector pointers.
        assert_eq!(PixelFormat::from_mode_word(6 << 27 | 90 << 1 | 1), None);
        assert_eq!(
            PixelFormat::from_mode_word(9 << 27 | 90 << 14 | 90 << 1 | 1),
            None
        );
        assert_eq!(PixelFormat::from_mode_word(0x8000), None);
    }

    #[test]
    fn pixels_fill_bytes_from_the_low_bits() {
        let row = [0b1110_0100, 0x5a];
        assert_eq!(
            (0..4)
                .map(|x| indexed_pixel(&row, x * 2, 2))
                .collect::<Vec<_>>(),
            [0, 1, 2, 3]
        );
        assert_eq!(indexed_pixel(&row, 4, 4), 0b1110);
        assert_eq!(indexed_pixel(&row, 1, 1), 0);
        assert_eq!(indexed_pixel(&row, 2, 1), 1);
        assert_eq!(indexed_pixel(&row, 8, 8), 0x5a);
    }

    #[test]
    fn deep_colour_pixels_expand_to_8_bits() {
        assert_eq!(rgb555(0x7fff), 0xffffff);
        assert_eq!(rgb555(0x001f), 0xff0000);
        assert_eq!(rgb555(0x03e0), 0x00ff00);
        assert_eq!(rgb555(0x7c00), 0x0000ff);
        assert_eq!(rgb555(0x0010), 0x840000);
    }

    #[test]
    fn default_256_colour_palette_follows_the_bit_layout() {
        assert_eq!(default_256_color(0), 0x000000);
        assert_eq!(default_256_color(255), 0xffffff);
        assert_eq!(default_256_color(3), 0x333333, "tint only");
        assert_eq!(default_256_color(0b0001_0100), 0xcc0000, "red bits 3 and 2");
        assert_eq!(
            default_256_color(0b1000_1000),
            0x0000cc,
            "blue bits 3 and 2"
        );
        assert_eq!(
            default_256_color(0b0110_0000),
            0x00cc00,
            "green bits 3 and 2"
        );
    }

    #[test]
    fn vidc1_palettes_override_the_high_bits() {
        let mut table = alloc::vec![0; 16];
        table[5] = 0x10_20_30;
        let colors = Colors::Vidc1(table);
        assert_eq!(colors.color(5), 0x11_22_33);
        assert_eq!(colors.color(0x15), 0x99_22_33, "bit 4: red bit 3");
        assert_eq!(colors.color(0x65), 0x11_ee_33, "bits 5-6: green bits 2-3");
        assert_eq!(colors.color(0x85), 0x11_22_bb, "bit 7: blue bit 3");
        // The default palette's first 16 entries give the default colours.
        let defaults: Vec<u32> = (0..16).map(|i| default_256_color(i) & 0x70_70_70).collect();
        let colors = Colors::Vidc1(defaults);
        assert!((0..=255).all(|i| colors.color(i) == default_256_color(i)));
    }

    /// A one-sprite file: `mode`, `width_words` x `height`, used bits,
    /// palette entries, then `pixels`.
    fn sprite_file(
        mode: u32,
        width_words: u32,
        height: u32,
        bits: (u32, u32),
        palette: &[u32],
        pixels: &[u8],
    ) -> Vec<u8> {
        let image_at = HEADER_LEN as u32 + palette.len() as u32 * 8;
        let size = image_at + pixels.len() as u32;
        let mut file = Vec::new();
        for w in [1, 16, 16 + size] {
            file.extend_from_slice(&u32::to_le_bytes(w));
        }
        file.extend_from_slice(&size.to_le_bytes());
        file.extend_from_slice(b"test\0\0\0\0\0\0\0\0");
        for w in [
            width_words - 1,
            height - 1,
            bits.0,
            bits.1,
            image_at,
            image_at,
            mode,
        ] {
            file.extend_from_slice(&w.to_le_bytes());
        }
        for &color in palette {
            // &BBGGRR00 as a little-endian word, twice (both flash colours).
            let [_, r, g, b] = color.to_be_bytes();
            for _ in 0..2 {
                file.extend_from_slice(&[0, r, g, b]);
            }
        }
        file.extend_from_slice(pixels);
        file
    }

    #[test]
    fn decodes_a_paletted_sprite_with_wastage() {
        // Mode 27 (4 bpp, square pixels), 3 pixels: first bit 4, last bit 15.
        let palette: Vec<u32> = (0..16).map(|i| i * 0x111111).collect();
        let file = sprite_file(27, 1, 1, (4, 15), &palette, &[0x21, 0x43, 0, 0]);
        let image = decode(&file).unwrap();
        assert_eq!((image.width(), image.height()), (3, 1));
        assert_eq!(
            (image.get(0, 0), image.get(1, 0), image.get(2, 0)),
            (0x222222, 0x333333, 0x444444)
        );
    }

    #[test]
    fn a_mode_number_mask_has_the_image_layout() {
        // The sprite of the test above with a 4 bpp mask after the image:
        // opaque, clear, opaque. The mask offset is at byte 48 of the file.
        let palette: Vec<u32> = (0..16).map(|i| i * 0x111111).collect();
        let mut file = sprite_file(27, 1, 1, (4, 15), &palette, &[0x21, 0x43, 0, 0]);
        let mask_at = (HEADER_LEN + palette.len() * 8 + 4) as u32;
        file[48..52].copy_from_slice(&mask_at.to_le_bytes());
        file.extend_from_slice(&[0xf0, 0xf0, 0, 0]);
        let image = decode(&file).unwrap();
        assert_eq!(image.get_argb(0, 0), 0xff22_2222);
        assert_eq!(image.get_argb(1, 0), crate::image::CLEAR);
        assert_eq!(image.get_argb(2, 0), 0xff44_4444);
        // A mask offset past the end of the file is ignored.
        file[48..52].copy_from_slice(&0x10000u32.to_le_bytes());
        assert!(!decode(&file).unwrap().has_alpha());
    }

    #[test]
    fn rectangular_pixel_modes_are_doubled_and_defaults_apply() {
        // Mode 0 (1 bpp, 1:2 pixels), no palette: Wimp white and black.
        let file = sprite_file(0, 1, 1, (0, 1), &[], &[0b10, 0, 0, 0]);
        let image = decode(&file).unwrap();
        assert_eq!((image.width(), image.height()), (2, 2));
        assert_eq!((image.get(0, 1), image.get(1, 1)), (0xffffff, 0));
    }

    #[test]
    fn rejects_bad_headers() {
        let good = sprite_file(28, 1, 1, (0, 31), &[], &[1, 2, 3, 4]);
        assert!(decode(&good).is_ok());
        let mut bad_name = good.clone();
        bad_name[16] = 1;
        assert!(decode(&bad_name).is_err());
        let mut bad_mode = good.clone();
        bad_mode[12 + 40] = 3;
        assert!(decode(&bad_mode).is_err());
        let mut misaligned = good.clone();
        misaligned[12 + 28] = 30; // last bit 30 in an 8 bpp mode
        assert!(decode(&misaligned).is_err());
        assert!(decode(&good[..good.len() - 1]).is_err());
    }

    #[test]
    fn sprite_larger_than_the_pixel_cap_is_rejected() {
        // Mode 25 (1 bpp): 8192 words x 257 rows is just over 2^26 pixels.
        let pixels = alloc::vec![0u8; 8192 * 4 * 257];
        let file = sprite_file(25, 8192, 257, (0, 31), &[], &pixels);
        assert!(decode(&file).is_err());
    }
}
