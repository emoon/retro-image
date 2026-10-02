//! Shared Atari ST building blocks: palette words, screen modes,
//! word-interleaved bitplanes and the PackBits run-length scheme.
//!
//! Sources:
//! - Palette word layouts (ST `.....RRR .GGG.BBB`, STE extra LSB above the
//!   three MSBs): Atari ST/STe/MSTe/TT/F030 Hardware Register Listing,
//!   <https://temlib.org/AtariForumWiki/index.php/Atari_ST/STe/MSTe/TT/F030_Hardware_Register_Listing>,
//!   and <http://fileformats.archiveteam.org/wiki/Atari_ST_color_palette>.
//! - Word-interleaved bitplanes: Atari Compendium chapter 5,
//!   <http://cd.textfiles.com/ataricompendium/BOOK/HTML/CHAP5.HTM>.
//! - PackBits: <https://temlib.org/AtariForumWiki/index.php/PackBits_Compression_Algorithm>.
//! - Observed from `recoil2png` output: 3-bit components are scaled by bit
//!   replication (7 -> 0xff); a palette that uses any STE bit is decoded as
//!   12-bit STE (4-bit components times 0x11); medium resolution is shown
//!   with every line doubled.

use alloc::vec::Vec;

use crate::Image;

/// Big-endian 16-bit word at `offset`, if in range.
pub(super) fn be16(data: &[u8], offset: usize) -> Option<u16> {
    let bytes = data.get(offset..offset.checked_add(2)?)?;
    Some(u16::from_be_bytes([bytes[0], bytes[1]]))
}

/// Big-endian 32-bit word at `offset`, if in range.
pub(super) fn be32(data: &[u8], offset: usize) -> Option<u32> {
    let bytes = data.get(offset..offset.checked_add(4)?)?;
    Some(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

/// Replicates a 3-bit value to 8 bits.
pub(super) fn scale3(v: u16) -> u32 {
    let v = u32::from(v & 7);
    (v << 5) | (v << 2) | (v >> 1)
}

/// One component of an ST/STE palette word, already shifted down to its
/// lowest nibble.
fn component(nibble: u16, ste: bool) -> u32 {
    if ste {
        let v = ((nibble & 7) << 1) | ((nibble >> 3) & 1);
        u32::from(v) * 0x11
    } else {
        scale3(nibble)
    }
}

/// Converts an ST (`ste == false`) or STE palette word to `0xRRGGBB`.
pub(super) fn st_rgb(word: u16, ste: bool) -> u32 {
    component(word >> 8, ste) << 16 | component(word >> 4, ste) << 8 | component(word, ste)
}

/// Whether a palette uses the STE's extra component bits.
pub(super) fn uses_ste_bits(words: impl IntoIterator<Item = u16>) -> bool {
    words.into_iter().any(|w| w & 0x888 != 0)
}

/// Reads `count` big-endian palette words starting at `offset`.
pub(super) fn palette_words(data: &[u8], offset: usize, count: usize) -> Option<Vec<u16>> {
    (0..count).map(|i| be16(data, offset + i * 2)).collect()
}

/// Converts palette words to colors, deciding ST vs. STE for the whole set.
pub(super) fn st_palette(words: &[u16]) -> Vec<u32> {
    let ste = uses_ste_bits(words.iter().copied());
    words.iter().map(|&w| st_rgb(w, ste)).collect()
}

/// The three ST Shifter modes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Resolution {
    /// 320x200, 4 planes.
    Low,
    /// 640x200, 2 planes.
    Medium,
    /// 640x400, 1 plane.
    High,
}

impl Resolution {
    pub(super) fn from_index(index: u16) -> Option<Self> {
        match index {
            0 => Some(Self::Low),
            1 => Some(Self::Medium),
            2 => Some(Self::High),
            _ => None,
        }
    }

    pub(super) fn width(self) -> u32 {
        match self {
            Self::Low => 320,
            Self::Medium | Self::High => 640,
        }
    }

    pub(super) fn height(self) -> u32 {
        match self {
            Self::Low | Self::Medium => 200,
            Self::High => 400,
        }
    }

    pub(super) fn planes(self) -> u32 {
        match self {
            Self::Low => 4,
            Self::Medium => 2,
            Self::High => 1,
        }
    }

    /// Number of palette entries the mode displays.
    pub(super) fn colors(self) -> usize {
        1 << self.planes()
    }

    /// Lines are doubled on output so pixels keep their aspect ratio.
    pub(super) fn y_scale(self) -> u32 {
        match self {
            Self::Medium => 2,
            Self::Low | Self::High => 1,
        }
    }
}

/// Size of one ST screen in bytes.
pub(super) const SCREEN_LEN: usize = 32000;

/// Colors of a monochrome picture: set bits are black on white, whatever
/// the palette says (observed from `recoil2png` output).
pub(super) const MONO_PALETTE: [u32; 2] = [0xffffff, 0x000000];

/// Builds the palette for an ST screen from 16 palette words.
pub(super) fn screen_palette(resolution: Resolution, words: &[u16]) -> Vec<u32> {
    match resolution {
        Resolution::High => MONO_PALETTE.to_vec(),
        _ => st_palette(&words[..resolution.colors().min(words.len())]),
    }
}

/// Decodes a 32000-byte ST screen in `resolution` with 16 palette words.
pub(super) fn decode_screen(resolution: Resolution, bitmap: &[u8], words: &[u16]) -> Option<Image> {
    let palette = screen_palette(resolution, words);
    planar_image(
        bitmap,
        resolution.width(),
        resolution.height(),
        resolution.planes(),
        &palette,
        resolution.y_scale(),
    )
}

/// Palette index of pixel `x` on a word-interleaved line starting at `line`.
pub(super) fn interleaved_index(line: &[u8], x: u32, planes: u32) -> usize {
    let group = (x / 16 * planes * 2) as usize;
    let bit = 15 - (x % 16);
    let mut index = 0;
    for plane in 0..planes as usize {
        let offset = group + plane * 2;
        let word = u16::from_be_bytes([line[offset], line[offset + 1]]);
        index |= usize::from((word >> bit) & 1) << plane;
    }
    index
}

/// Renders word-interleaved bitplanes. `width` must be a multiple of 16.
/// Each source line is repeated `y_scale` times. Returns `None` when the
/// bitmap is too short or an index is outside `palette`.
pub(super) fn planar_image(
    bitmap: &[u8],
    width: u32,
    height: u32,
    planes: u32,
    palette: &[u32],
    y_scale: u32,
) -> Option<Image> {
    let stride = (width / 16 * planes * 2) as usize;
    if !width.is_multiple_of(16) || bitmap.len() < stride * height as usize {
        return None;
    }
    let mut image = Image::new(width, height * y_scale);
    for y in 0..height {
        let line = &bitmap[y as usize * stride..][..stride];
        for x in 0..width {
            let color = *palette.get(interleaved_index(line, x, planes))?;
            for dy in 0..y_scale {
                image.set(x, y * y_scale + dy, color);
            }
        }
    }
    Some(image)
}

/// Upper bound on a picture's area, so corrupt headers can't make a
/// decoder allocate gigabytes.
pub(super) const MAX_PIXELS: usize = 1 << 24;

/// Big-endian words of `data` (a trailing odd byte is ignored).
pub(super) fn words(data: &[u8]) -> Vec<u16> {
    data.chunks_exact(2)
        .map(|w| u16::from_be_bytes([w[0], w[1]]))
        .collect()
}

/// Colour of pixel `i` (row-major) of `image`.
fn pixel(image: &Image, i: usize) -> u32 {
    let p = &image.rgb()[i * 3..i * 3 + 3];
    u32::from_be_bytes([0, p[0], p[1], p[2]])
}

/// Per-component average of two colours, rounding down: how two screens
/// shown on alternate frames are rendered (observed from `recoil2png`
/// output).
pub(super) fn mix(a: u32, b: u32) -> u32 {
    let channel = |shift: u32| (((a >> shift & 0xff) + (b >> shift & 0xff)) / 2) << shift;
    channel(16) | channel(8) | channel(0)
}

/// Mixes two images of the same size, see [`mix`].
pub(super) fn mix_images(a: &Image, b: &Image) -> Image {
    let mut image = Image::new(a.width(), a.height());
    for i in 0..(a.width() * a.height()) as usize {
        let (x, y) = (i as u32 % a.width(), i as u32 / a.width());
        image.set(x, y, mix(pixel(a, i), pixel(b, i)));
    }
    image
}

/// Doubles every pixel horizontally.
pub(super) fn double_width(image: &Image) -> Image {
    let mut wide = Image::new(image.width() * 2, image.height());
    for i in 0..(image.width() * image.height()) as usize {
        let (x, y) = (i as u32 % image.width(), i as u32 / image.width());
        wide.set(x * 2, y, pixel(image, i));
        wide.set(x * 2 + 1, y, pixel(image, i));
    }
    wide
}

/// Copies the top-left `width` x `height` pixels of `image`.
pub(super) fn crop(image: &Image, width: u32, height: u32) -> Image {
    let mut out = Image::new(width, height);
    for y in 0..height.min(image.height()) {
        for x in 0..width.min(image.width()) {
            out.set(x, y, pixel(image, (y * image.width() + x) as usize));
        }
    }
    out
}

/// VDI intensity (0-1000) to 8 bits, truncating (observed from
/// `recoil2png` output).
pub(super) fn vdi_level(v: u16) -> u32 {
    u32::from(v.min(1000)) * 255 / 1000
}

/// VDI pen used for hardware palette index `index` of a `colors`-entry
/// palette: pen 1 (black) is the last register and pens 2-15 are
/// scrambled (the usual GEM VDI colour mapping; the 256-colour variant,
/// where register 15 shows pen 255, is observed from `recoil2png` output).
pub(super) fn vdi_pen(index: usize, colors: usize) -> usize {
    const PENS_16: [usize; 16] = [0, 2, 3, 6, 4, 7, 5, 8, 9, 10, 11, 14, 12, 15, 13, 1];
    match colors {
        2 => index,
        4 => [0, 2, 3, 1][index & 3],
        16 => PENS_16[index & 15],
        _ if index == colors - 1 => 1,
        _ if index == 15 => colors - 1,
        _ if index < 16 => PENS_16[index],
        _ => index,
    }
}

/// Reads `colors` VDI RGB triplets (pen order) and returns them in
/// hardware index order.
pub(super) fn vdi_palette(data: &[u8], colors: usize) -> Option<Vec<u32>> {
    (0..colors)
        .map(|index| {
            let pen = vdi_pen(index, colors);
            let c = |k: usize| be16(data, (pen * 3 + k) * 2).map(vdi_level);
            Some(c(0)? << 16 | c(1)? << 8 | c(2)?)
        })
        .collect()
}

/// Renders a 1-bit bitmap of `row_len`-byte lines, set bits black.
pub(super) fn mono_image(bitmap: &[u8], width: u32, height: u32, row_len: usize) -> Option<Image> {
    if bitmap.len() < row_len * height as usize || row_len * 8 < width as usize {
        return None;
    }
    let mut image = Image::new(width, height);
    for y in 0..height {
        let line = &bitmap[y as usize * row_len..];
        for x in 0..width {
            let bit = line[x as usize / 8] >> (7 - x % 8) & 1;
            image.set(x, y, MONO_PALETTE[usize::from(bit)]);
        }
    }
    Some(image)
}

/// Reorders bitplanes stored line by line, each line holding one complete
/// row per plane (as in IFF bodies), into word-interleaved screen order.
pub(super) fn line_planes_to_interleaved(
    data: &[u8],
    width: u32,
    height: u32,
    planes: u32,
) -> Option<Vec<u8>> {
    let row = (width / 8) as usize;
    let planes = planes as usize;
    let stride = row * planes;
    if stride == 0 {
        return None;
    }
    let data = data.get(..stride * height as usize)?;
    let mut out = alloc::vec![0; data.len()];
    for (src, dst) in data.chunks_exact(stride).zip(out.chunks_exact_mut(stride)) {
        for plane in 0..planes {
            for word in 0..row / 2 {
                let from = plane * row + word * 2;
                let to = (word * planes + plane) * 2;
                dst[to..to + 2].copy_from_slice(&src[from..from + 2]);
            }
        }
    }
    Some(out)
}

/// Whole planes stored one after another become word-interleaved data.
pub(super) fn separate_planes_to_interleaved(data: &[u8], planes: usize) -> Vec<u8> {
    let plane_len = data.len() / planes.max(1);
    let mut out = alloc::vec![0; plane_len * planes];
    for plane in 0..planes {
        for word in 0..plane_len / 2 {
            let from = plane * plane_len + word * 2;
            let to = (word * planes + plane) * 2;
            out[to..to + 2].copy_from_slice(&data[from..from + 2]);
        }
    }
    out
}

/// Unpacks PackBits data until `out_len` bytes are produced. Returns the
/// unpacked bytes and the number of input bytes consumed.
pub(super) fn unpack_bits(data: &[u8], out_len: usize) -> Option<(Vec<u8>, usize)> {
    // Each input byte yields at most 64 output bytes.
    if out_len / 64 > data.len() {
        return None;
    }
    let mut out = Vec::with_capacity(out_len);
    let mut pos = 0;
    while out.len() < out_len {
        let control = *data.get(pos)? as i8;
        pos += 1;
        match control {
            0..=127 => {
                let count = control as usize + 1;
                out.extend_from_slice(data.get(pos..pos + count)?);
                pos += count;
            }
            -128 => {}
            _ => {
                let value = *data.get(pos)?;
                pos += 1;
                let count = (1 - isize::from(control)) as usize;
                out.extend(core::iter::repeat_n(value, count));
            }
        }
    }
    out.truncate(out_len);
    Some((out, pos))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn st_and_ste_palette_words() {
        assert_eq!(st_rgb(0x0777, false), 0xffffff);
        assert_eq!(st_rgb(0x0123, false), 0x24496d);
        assert_eq!(st_rgb(0x0047, true), 0x0088ee);
        assert_eq!(st_rgb(0x0fff, true), 0xffffff);
        assert_eq!(st_rgb(0x0800, true), 0x110000);
    }

    #[test]
    fn interleaved_planes_lsb_first() {
        // plane 0 = 0x8000, plane 1 = 0x0000, plane 2 = 0x8000, plane 3 = 0x0001
        let line = [0x80, 0, 0, 0, 0x80, 0, 0, 1];
        assert_eq!(interleaved_index(&line, 0, 4), 0b0101);
        assert_eq!(interleaved_index(&line, 15, 4), 0b1000);
    }

    #[test]
    fn packbits_literal_and_run() {
        let data = [2, 1, 2, 3, 0xfe, 9, 0x80];
        assert_eq!(
            unpack_bits(&data, 6),
            Some((alloc::vec![1, 2, 3, 9, 9, 9], 6))
        );
        assert_eq!(unpack_bits(&data[..3], 3), None);
    }
}
