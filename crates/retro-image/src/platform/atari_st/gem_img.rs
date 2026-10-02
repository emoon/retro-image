//! GEM Bit Image (`IMG`) and its colour dialects (`XIMG`, `STTT`, `TIMG`).
//!
//! Sources:
//! - <https://temlib.org/AtariForumWiki/index.php/GEM_Bit_Image_file_format>
//! - "Understanding color IMGs" (Dr. Bob, 1992):
//!   <https://temlib.org/AtariForumWiki/index.php/IMG_file>
//! - Atari Compendium appendix C:
//!   <http://cd.textfiles.com/ataricompendium/BOOK/HTML/APPENDC.HTM>
//! - Observed from `recoil2png` output: VDI intensities (0-1000) are
//!   scaled as `v * 255 / 1000`, truncated; monochrome set bits are black;
//!   16, 24 and 32-"plane" XIMG pictures hold chunky RGB565, RGB and xRGB
//!   pixels; TIMG pictures are true bitplanes, see [`timg_color`].
//! - Reverse engineered from sample files and black-box tests with
//!   `recoil2png` (hand-modified copies): FSNAP's line-above copies and
//!   256-byte literals, 8-plane grey levels and the 24-bit BGR dialect.

use alloc::vec::Vec;

use super::common::{MAX_PIXELS, st_rgb, vdi_level};
use crate::bytes::be16;
use crate::image::planar_pixels;
use crate::{DecodeError, Image};

pub(super) fn decode_img(data: &[u8]) -> Result<Image, DecodeError> {
    decode(data).ok_or(DecodeError::Unrecognized)
}

struct Header {
    version: usize,
    header_len: usize,
    planes: usize,
    pattern_len: usize,
    pixel_width: usize,
    pixel_height: usize,
    width: usize,
    height: usize,
}

fn header(data: &[u8]) -> Option<Header> {
    let word = |i: usize| be16(data, i * 2).map(usize::from);
    let header = Header {
        version: word(0)?,
        header_len: word(1)? * 2,
        planes: word(2)?,
        pattern_len: word(3)?,
        pixel_width: word(4)?,
        pixel_height: word(5)?,
        width: word(6)?,
        height: word(7)?,
    };
    if header.version > 3
        || header.header_len < 16
        || header.header_len > data.len()
        || !(1..=8).contains(&header.pattern_len)
        || !matches!(header.planes, 1..=8 | 15 | 16 | 24 | 32)
        || header.width == 0
        || header.height == 0
        || header.width * header.height > MAX_PIXELS
    {
        return None;
    }
    Some(header)
}

fn decode(data: &[u8]) -> Option<Image> {
    let h = header(data)?;
    if h.header_len == 18 && be16(data, 16)? == 3 {
        return bgr_literals(&data[h.header_len..], &h);
    }
    let palette = palette(data, &h)?;
    let timg = timg_bits(data, &h);
    if h.planes > 8 && timg.is_none() {
        return true_color(&data[h.header_len..], &h);
    }
    let row_len = h.width.div_ceil(8);
    // STTT files store each plane whole, one after another (derived from
    // sample files); the others store all plane rows of a line together.
    let plane_major = data.get(16..20) == Some(b"STTT");
    let bitmap = if plane_major {
        unpack(
            &data[h.header_len..],
            h.pattern_len,
            row_len,
            h.height * h.planes,
        )?
    } else {
        unpack(
            &data[h.header_len..],
            h.pattern_len,
            row_len * h.planes,
            h.height,
        )?
    };
    let (sx, sy) = if timg.is_some() {
        (1, 1)
    } else {
        pixel_scale(&h)
    };
    let values = planar_pixels(&bitmap, h.width, h.height, row_len, h.planes, |plane, y| {
        let row = if plane_major {
            plane * h.height + y
        } else {
            y * h.planes + plane
        };
        row * row_len
    });
    let mut image = Image::new(h.width as u32, h.height as u32);
    for (y, row) in values.chunks_exact(h.width).enumerate() {
        for (x, &value) in row.iter().enumerate() {
            let index = value as usize;
            let color = match timg {
                Some(bits) => timg_color(index, bits),
                None => *palette.get(index)?,
            };
            image.set(x as u32, y as u32, color);
        }
    }
    Some(if (sx, sy) == (1, 1) {
        image
    } else {
        image.scaled(sx as u32, sy as u32)
    })
}

/// TIMG: `TIMG`, a word (3) and the red, green and blue bit counts.
fn timg_bits(data: &[u8], h: &Header) -> Option<[u32; 3]> {
    let extra = &data[16..h.header_len];
    if extra.len() != 12 || &extra[..4] != b"TIMG" {
        return None;
    }
    let bits = [be16(extra, 6)?, be16(extra, 8)?, be16(extra, 10)?].map(u32::from);
    let total: u32 = bits.iter().sum();
    (bits.iter().all(|&b| (1..=8).contains(&b)) && total as usize == h.planes).then_some(bits)
}

/// TIMG planes hold red, then green, then blue, least significant bit
/// first (derived from sample files); components are scaled by bit
/// replication.
fn timg_color(index: usize, bits: [u32; 3]) -> u32 {
    let mut shift = 0;
    let mut color = 0;
    for (i, &count) in bits.iter().enumerate() {
        let v = (index >> shift) as u32 & ((1 << count) - 1);
        shift += count;
        let mut level = 0;
        let mut pos = 8i32 - count as i32;
        while pos > -(count as i32) {
            level |= if pos >= 0 { v << pos } else { v >> -pos };
            pos -= count as i32;
        }
        color |= (level & 0xff) << (16 - 8 * i);
    }
    color
}

/// True colour lines are chunky xRGB1555 or RGB565 words, RGB or xRGB
/// pixels; they are never scaled for pixel aspect.
fn true_color(data: &[u8], h: &Header) -> Option<Image> {
    let bytes = h.planes.div_ceil(8);
    let line_len = h.width * bytes;
    let bitmap = unpack(data, h.pattern_len, line_len, h.height)?;
    let mut image = Image::new(h.width as u32, h.height as u32);
    for (y, line) in bitmap.chunks_exact(line_len).enumerate() {
        for (x, p) in line.chunks_exact(bytes).enumerate() {
            let color = match bytes {
                2 => super::falcon::rgb565(u16::from_be_bytes([p[0], p[1]])),
                _ => u32::from_be_bytes([0, p[bytes - 3], p[bytes - 2], p[bytes - 1]]),
            };
            image.set(x as u32, y as u32, color);
        }
    }
    Some(image)
}

/// A true-colour dialect flagged by a ninth header word of 3 (the planes
/// and pattern words are ignored): lines of chunky blue, green, red pixels
/// stored only as `0x80, n` records followed by `n` pixels. Derived from a
/// sample file and black-box tests with `recoil2png`, which rejects
/// pattern, solid and repeat records in it.
fn bgr_literals(data: &[u8], h: &Header) -> Option<Image> {
    let total = h.width * h.height;
    let mut image = Image::new(h.width as u32, h.height as u32);
    let (mut pos, mut pixel) = (0, 0);
    while pixel < total {
        if *data.get(pos)? != 0x80 {
            return None;
        }
        let n = usize::from(*data.get(pos + 1)?);
        let run = data.get(pos + 2..pos + 2 + n * 3)?;
        pos += 2 + n * 3;
        for p in run.as_chunks::<3>().0 {
            if pixel < total {
                let color = u32::from_be_bytes([0, p[2], p[1], p[0]]);
                image.set((pixel % h.width) as u32, (pixel / h.width) as u32, color);
            }
            pixel += 1;
        }
    }
    Some(image)
}

/// Pixels clearly taller or wider than square are doubled on output
/// (observed from `recoil2png` output).
fn pixel_scale(h: &Header) -> (usize, usize) {
    if h.pixel_height * 2 > h.pixel_width * 3 {
        (1, 2)
    } else if h.pixel_width * 2 > h.pixel_height * 3 {
        (2, 1)
    } else {
        (1, 1)
    }
}

fn palette(data: &[u8], h: &Header) -> Option<Vec<u32>> {
    let extra = &data[16..h.header_len];
    if h.planes > 8 {
        // True colour: no palette, at most an empty XIMG one.
        let timg = extra.len() == 12 && &extra[..4] == b"TIMG";
        return (extra.is_empty() || extra == b"XIMG\0\0" || timg).then(Vec::new);
    }
    let colors = 1usize << h.planes;
    if h.planes == 1 {
        // Longer monochrome headers belong to other, unsupported dialects.
        return extra.is_empty().then(|| alloc::vec![0xffffff, 0x000000]);
    }
    // One extra word (seen as 0 or 1) still means no palette.
    if extra.len() <= 2 && h.planes <= 4 {
        return Some(super::common::default_vdi_palette(colors));
    }
    // 256 colours without a palette are inverted grey levels. Without an
    // `XIMG` header the planes count from the most significant bit (black-box
    // tests with `recoil2png`, moving the samples' rasters between headers).
    if h.planes == 8 && (extra.len() <= 2 || extra == b"XIMG\0\0") {
        let msb_first = extra.len() <= 2;
        return Some(
            (0..=255u8)
                .map(|i| {
                    let level = 255 - u32::from(if msb_first { i.reverse_bits() } else { i });
                    level * 0x010101
                })
                .collect(),
        );
    }
    if extra.len() >= 6 + colors * 6 && &extra[..4] == b"XIMG" && be16(extra, 4)? == 0 {
        return (0..colors)
            .map(|i| {
                let c = |k| be16(extra, 6 + (i * 3 + k) * 2).map(vdi_level);
                Some(c(0)? << 16 | c(1)? << 8 | c(2)?)
            })
            .collect();
    }
    if extra.get(..4)? == b"STTT" {
        // Entries beyond a shorter palette are black (observed from
        // `recoil2png` output).
        let count = usize::from(be16(extra, 4)?).min(colors);
        let words: Vec<u16> = (0..count)
            .map(|i| be16(extra, 6 + i * 2))
            .collect::<Option<_>>()?;
        let ste = sttt_uses_ste(&words);
        let mut palette: Vec<u32> = words.iter().map(|&w| st_rgb(w, ste)).collect();
        palette.resize(colors, 0);
        return Some(palette);
    }
    None
}

/// STTT palettes are treated as STE only when some component has the STE
/// bit set and is neither the darkest nor the brightest 3-bit level
/// (observed from `recoil2png` output).
fn sttt_uses_ste(words: &[u16]) -> bool {
    words.iter().any(|&w| {
        (0..3).any(|i| {
            let nibble = w >> (i * 4) & 0xf;
            nibble & 8 != 0 && !matches!(nibble & 7, 0 | 7)
        })
    })
}

/// Decodes the run-length data into `lines` lines of `line_len` bytes.
/// Runs may cross line ends; a scanline-repeat record is honoured when it
/// starts a line.
fn unpack(data: &[u8], pattern_len: usize, line_len: usize, lines: usize) -> Option<Vec<u8>> {
    let total = line_len * lines;
    let mut out = Vec::with_capacity(total);
    let mut pos = 0;
    let byte = |pos: usize| data.get(pos).copied();
    // Bytes of `out` that hold complete lines (including repeats).
    let mut done = 0;
    let mut repeat = 1;
    while done < total {
        if out.len() == done && data.get(pos..pos + 3) == Some(&[0, 0, 0xff]) {
            repeat = usize::from(byte(pos + 3)?).max(1);
            pos += 4;
            continue;
        }
        if out.len() >= done + line_len {
            let excess = out.split_off(done + line_len);
            for _ in 1..repeat {
                out.extend_from_within(done..done + line_len);
            }
            done = out.len().min(total);
            out.extend_from_slice(&excess);
            repeat = 1;
            continue;
        }
        let x = byte(pos)?;
        pos += 1;
        match x {
            0 => {
                let n = usize::from(byte(pos)?);
                if n == 0 {
                    // Snapshot tools (FSNAP) write `0, 0, n` inside lines:
                    // `n + 1` bytes copied from the line above (derived from
                    // sample files and `recoil2png` output).
                    let count = usize::from(byte(pos + 1)?) + 1;
                    pos += 2;
                    let start = out.len().checked_sub(line_len)?;
                    for i in start..start + count {
                        out.push(out[i]);
                    }
                    continue;
                }
                let pattern = data.get(pos + 1..pos + 1 + pattern_len)?;
                pos += 1 + pattern_len;
                for _ in 0..n {
                    out.extend_from_slice(pattern);
                }
            }
            0x80 => {
                // A zero count means 256 (FSNAP; derived from sample files).
                let n = match byte(pos)? {
                    0 => 256,
                    n => usize::from(n),
                };
                out.extend_from_slice(data.get(pos + 1..pos + 1 + n)?);
                pos += 1 + n;
            }
            _ => {
                let fill = if x & 0x80 != 0 { 0xff } else { 0 };
                out.extend(core::iter::repeat_n(fill, usize::from(x & 0x7f)));
            }
        }
    }
    out.truncate(total);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_literals_patterns_and_line_repeat() {
        // Line repeat x2, then solid 0xff run of 1, literal 1 byte, pattern 1x2 bytes.
        let data = [0, 0, 0xff, 2, 0x81, 0x80, 1, 0x12, 0, 1, 0xab, 0xcd];
        let out = unpack(&data, 2, 4, 2).unwrap();
        assert_eq!(out, [0xff, 0x12, 0xab, 0xcd, 0xff, 0x12, 0xab, 0xcd]);
    }

    #[test]
    fn fsnap_copies_from_the_line_above_and_long_literals() {
        // Line 1: literal 1 2 3; line 2: copy 2 bytes from above, solid 0.
        let data = [0x80, 3, 1, 2, 3, 0, 0, 1, 0x01];
        assert_eq!(unpack(&data, 1, 3, 2).unwrap(), [1, 2, 3, 1, 2, 0]);
        let mut long = alloc::vec![0x80, 0];
        long.extend(1..=255u8);
        long.push(0);
        assert_eq!(unpack(&long, 1, 256, 1).unwrap().len(), 256);
        // A copy needs a line above.
        assert_eq!(unpack(&[0, 0, 1], 1, 3, 1), None);
    }

    #[test]
    fn runs_may_cross_line_ends() {
        let out = unpack(&[0x03, 0x83], 2, 2, 3).unwrap();
        assert_eq!(out, [0, 0, 0, 0xff, 0xff, 0xff]);
    }

    #[test]
    fn vdi_levels_truncate() {
        assert_eq!(vdi_level(1000), 255);
        assert_eq!(vdi_level(501), 127);
        assert_eq!(vdi_level(2000), 255);
    }

    #[test]
    fn rejects_truncated_data() {
        let header = [0, 1, 0, 8, 0, 1, 0, 2, 0, 85, 0, 85, 0, 16, 0, 2];
        assert_eq!(decode_img(&header), Err(DecodeError::Unrecognized));
    }
}
