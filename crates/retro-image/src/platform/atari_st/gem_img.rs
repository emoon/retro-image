//! GEM Bit Image (`IMG`) and its colour dialects (`XIMG`, `STTT`).
//!
//! Sources:
//! - <https://temlib.org/AtariForumWiki/index.php/GEM_Bit_Image_file_format>
//! - "Understanding color IMGs" (Dr. Bob, 1992):
//!   <https://temlib.org/AtariForumWiki/index.php/IMG_file>
//! - Atari Compendium appendix C:
//!   <http://cd.textfiles.com/ataricompendium/BOOK/HTML/APPENDC.HTM>
//! - Observed from `recoil2png` output: VDI intensities (0-1000) are
//!   scaled as `v * 255 / 1000`, truncated; monochrome set bits are black.

use alloc::vec::Vec;

use super::common::{be16, st_rgb};
use crate::{DecodeError, Image};

pub(super) fn decode_img(data: &[u8]) -> Result<Image, DecodeError> {
    decode(data).ok_or(DecodeError::Unrecognized)
}

/// Upper bound on the picture area, so corrupt headers can't make us
/// allocate gigabytes.
const MAX_PIXELS: usize = 1 << 24;

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
        || !(1..=8).contains(&header.planes)
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
    let palette = palette(data, &h)?;
    let row_len = h.width.div_ceil(8);
    // Version 3 files store each plane whole, one after another; the
    // others store all plane rows of a line together.
    let plane_major = h.version == 3;
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
    let (sx, sy) = pixel_scale(&h);
    let mut image = Image::new((h.width * sx) as u32, (h.height * sy) as u32);
    for y in 0..h.height {
        for x in 0..h.width {
            let mut index = 0;
            for plane in 0..h.planes {
                let row = if plane_major {
                    plane * h.height + y
                } else {
                    y * h.planes + plane
                };
                let byte = bitmap[row * row_len + x / 8];
                index |= usize::from(byte >> (7 - x % 8) & 1) << plane;
            }
            let color = *palette.get(index)?;
            for dy in 0..sy {
                for dx in 0..sx {
                    image.set((x * sx + dx) as u32, (y * sy + dy) as u32, color);
                }
            }
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

/// VDI intensity (0-1000) to 8 bits.
fn vdi_level(v: u16) -> u32 {
    u32::from(v.min(1000)) * 255 / 1000
}

fn palette(data: &[u8], h: &Header) -> Option<Vec<u32>> {
    let colors = 1usize << h.planes;
    let extra = &data[16..h.header_len];
    if h.planes == 1 {
        // Longer monochrome headers belong to other, unsupported dialects.
        return extra.is_empty().then(|| alloc::vec![0xffffff, 0x000000]);
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
        // Palettes shorter than the plane count allows are not supported.
        if usize::from(be16(extra, 4)?) < colors {
            return None;
        }
        let words: Vec<u16> = (0..colors)
            .map(|i| be16(extra, 6 + i * 2))
            .collect::<Option<_>>()?;
        let ste = sttt_uses_ste(&words);
        return Some(words.iter().map(|&w| st_rgb(w, ste)).collect());
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
                let pattern = data.get(pos + 1..pos + 1 + pattern_len)?;
                pos += 1 + pattern_len;
                for _ in 0..n {
                    out.extend_from_slice(pattern);
                }
            }
            0x80 => {
                let n = usize::from(byte(pos)?);
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
