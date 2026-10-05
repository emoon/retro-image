//! Cels: a `CCB ` chunk, one `PDAT` chunk of source data and optionally a
//! `PLUT` chunk, as `.cel` files and (the first frame of) `.anim` files hold.
//!
//! Sources:
//! - "3DO File Format" in the 3DO Portfolio 2.5 documentation
//!   (`ppgfldr/smmfldr/cdmfldr/08CDM001.html`, mirrored at
//!   <https://3dodev.com/documentation/file_formats/media/container/3do> and in
//!   <https://github.com/trapexit/3do-devkit> `docs/3dosdk/`; prose only, its
//!   license is not stated): the chunks and the `CCB `, `PLUT` and `ANIM`
//!   layouts.
//! - The Graphics Programmer's Guide of the same documentation (`5gpgc.html`
//!   on the CCB flags, `5gpgd.html` on packed and unpacked source data,
//!   `5gpge.html` on the preamble words and `3gpgc.html` on the pixel decoder,
//!   under `ppgfldr/ggsfldr/gpgfldr/`; prose only).
//! - The `CCBPRE` rule, the unpacked row stride and the 16-bit coded cels were
//!   found by checking the documents against the samples of
//!   `trapexit/3do-devkit` (data only, none of its source was read).
//!
//! The `CCB ` chunk is the cel control block as 18 big-endian words after its
//! header: version, flags, three pointers, position, six size and perspective
//! words, `PIXC`, the two preamble words `PRE0` and `PRE1`, width and height.
//! The flags used here are `PACKED` (bit 9), `BGND` (bit 5), `CCBPRE` (bit 22,
//! the preamble is in the chunk, not at the start of the data) and the four
//! `PLUTA` bits (0 to 3). `PRE0`
//! gives bits per pixel (bits 0 to 2: 1, 2, 4, 6, 8, 16 for 1 to 6), `UNCODED`
//! (bit 4) and the row count minus one (bits 6 to 15); `PRE1` of unpacked
//! cels the pixels per row minus one (bits 0 to 10), `LRFORM` (bit 11) and the
//! row stride in words minus two (`WOFFSET`: bits 24 to 31 up to 6 bits per
//! pixel, bits 16 to 25 for 8 and 16). A packed cel takes its width from
//! the `CCB ` (up to 2048 pixels).
//!
//! Unpacked rows start on word boundaries. A packed row starts with an offset
//! (8 bits for pixels of 6 bits or less in the first byte, else 10 bits in the
//! first two bytes) giving the row's length in words minus two, then packets
//! read most significant bit first: a 2-bit type (literal `01`, repeat
//! `11`, transparent `10`, end of row `00`) and a 6-bit count minus one, then
//! the pixels.
//!
//! A coded pixel's color is entry `pixel & 31` of the `PLUT` (a 16-bit coded
//! pixel keeps other values in its upper bits), where pixels of fewer than 5
//! bits take their missing high bits from `PLUTA`; the 16-bit entries and
//! uncoded 16-bit pixels keep their color in bits 0 to 14.

use super::{Chunk, chunks};
use crate::bytes::{be16, be32};
use crate::image::{check_size, over_fill_argb, xrgb1555};
use crate::{DecodeError, Image};

/// Widest cel: the cel engine counts the pixels of a row in 11 bits. Packed
/// cels get their width from the `CCB ` chunk, so it needs this limit; a row
/// is cheap to describe but costs this many pixels to expand.
const MAX_WIDTH: usize = 2048;

const PACKED: u32 = 1 << 9;
const BGND: u32 = 1 << 5;
const CCBPRE: u32 = 1 << 22;

/// What the decoder needs from the cel control block.
struct Ccb {
    flags: u32,
    pre0: u32,
    pre1: u32,
    width: usize,
}

impl Ccb {
    fn parse(body: &[u8]) -> Option<Self> {
        Some(Self {
            flags: be32(body, 4)?,
            pre0: be32(body, 56)?,
            pre1: be32(body, 60)?,
            width: be32(body, 64)? as usize,
        })
    }
}

/// How the pixels of the cel are stored.
struct Layout {
    width: usize,
    height: usize,
    bits: u32,
    coded: bool,
    packed: bool,
    /// Bytes from one unpacked row to the next.
    stride: usize,
}

impl Layout {
    /// The fewest bytes of source data a cel of this size can have: the last
    /// row of unpacked cels, or an offset in front of each packed row (which
    /// start at least two words apart).
    fn least_source_len(&self) -> usize {
        let before_last_row = self.height - 1;
        if self.packed {
            before_last_row * 8 + 1
        } else {
            before_last_row * self.stride + (self.width * self.bits as usize).div_ceil(8)
        }
    }
}

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let mut all = chunks(data).skip_while(|chunk| &chunk.tag != b"CCB ");
    let ccb = Ccb::parse(all.next().ok_or(fail)?.body).ok_or(fail)?;
    // The picture's own chunks lie before the next cel control block.
    let own: alloc::vec::Vec<Chunk> = all.take_while(|chunk| &chunk.tag != b"CCB ").collect();
    let source = own
        .iter()
        .find(|chunk| &chunk.tag == b"PDAT")
        .ok_or(fail)?
        .body;
    let plut = own
        .iter()
        .find(|chunk| &chunk.tag == b"PLUT")
        .map(|c| c.body);

    let packed = ccb.flags & PACKED != 0;
    // Without `CCBPRE` the preamble leads the data: one word, or two if unpacked.
    let (pre0, pre1, data_at) = if ccb.flags & CCBPRE != 0 {
        (ccb.pre0, ccb.pre1, 0)
    } else if packed {
        (be32(source, 0).ok_or(fail)?, 0, 4)
    } else {
        (
            be32(source, 0).ok_or(fail)?,
            be32(source, 4).ok_or(fail)?,
            8,
        )
    };
    let layout = layout(&ccb, pre0, pre1).ok_or(fail)?;
    check_size(layout.width, layout.height)?;
    let source = source.get(data_at..).ok_or(fail)?;
    // Header-sized buffers only once the data can fill them.
    if source.len() < layout.least_source_len() {
        return Err(fail);
    }

    let colors = palette(plut);
    let bgnd = ccb.flags & BGND != 0;
    let pluta = ccb.flags & 15;
    let color = |pixel: u32| -> u32 {
        let argb15 = if layout.coded {
            let missing = !((1u32 << layout.bits.min(5)) - 1) & 0x1e;
            let index = (pixel | ((pluta << 1) & missing)) & 31;
            colors[index as usize]
        } else {
            (pixel & 0x7fff) as u16
        };
        argb(argb15, bgnd)
    };

    let mut pixels = alloc::vec![0u32; layout.width * layout.height];
    if layout.packed {
        unpack_rows(source, &layout, &color, &mut pixels).ok_or(fail)?;
    } else {
        read_rows(source, &layout, &color, &mut pixels).ok_or(fail)?;
    }
    Ok(Image::from_colors(
        layout.width as u32,
        layout.height as u32,
        pixels.into_iter().map(over_fill_argb),
    ))
}

/// The cel's geometry and pixel format from its preamble words, `None` for
/// what is not read (see the module comment).
fn layout(ccb: &Ccb, pre0: u32, pre1: u32) -> Option<Layout> {
    let bits = match pre0 & 7 {
        1 => 1,
        2 => 2,
        3 => 4,
        4 => 6,
        5 => 8,
        6 => 16,
        _ => return None,
    };
    let coded = pre0 & 0x10 == 0;
    // Uncoded pixels are 16-bit colors here (8-bit ones need unfolding).
    if !coded && bits != 16 {
        return None;
    }
    let packed = ccb.flags & PACKED != 0;
    let height = (pre0 >> 6 & 0x3ff) as usize + 1;
    let (width, stride) = if packed {
        if ccb.width > MAX_WIDTH {
            return None;
        }
        (ccb.width, 0)
    } else {
        if pre1 & 0x800 != 0 {
            return None; // left/right memory format
        }
        let word_offset = if bits >= 8 {
            pre1 >> 16 & 0x3ff
        } else {
            pre1 >> 24
        };
        let width = (pre1 & 0x7ff) as usize + 1;
        let stride = (word_offset as usize + 2) * 4;
        if stride < (width * bits as usize).div_ceil(32) * 4 {
            return None;
        }
        (width, stride)
    };
    Some(Layout {
        width,
        height,
        bits,
        coded,
        packed,
        stride,
    })
}

/// The 32 entries of a `PLUT` chunk (a count, then 16-bit colors), or a ramp
/// of grays when the cel has none.
fn palette(plut: Option<&[u8]>) -> [u16; 32] {
    let mut colors: [u16; 32] =
        core::array::from_fn(|i| (i as u16) << 10 | (i as u16) << 5 | i as u16);
    if let Some(body) = plut {
        let count = be32(body, 0).unwrap_or(0) as usize;
        for (i, color) in colors.iter_mut().enumerate().take(count) {
            *color = be16(body, 4 + i * 2).unwrap_or(0);
        }
    }
    colors
}

/// `0xAARRGGBB` from a 15-bit color: transparent when it is zero and `bgnd`
/// is not set.
fn argb(color: u16, bgnd: bool) -> u32 {
    let alpha = if color & 0x7fff == 0 && !bgnd {
        0
    } else {
        0xff
    };
    alpha << 24 | xrgb1555(color)
}

/// Reads the bits of `data` most significant bit first.
struct Bits<'a> {
    data: &'a [u8],
    at: usize,
}

impl Bits<'_> {
    /// The next `count` (at most 16) bits, `None` past the end.
    fn read(&mut self, count: u32) -> Option<u32> {
        let end = self.at.checked_add(count as usize)?;
        if end > self.data.len().saturating_mul(8) {
            return None;
        }
        let mut value = 0;
        for bit in self.at..end {
            value = value << 1 | u32::from(self.data[bit / 8] >> (7 - bit % 8) & 1);
        }
        self.at = end;
        Some(value)
    }
}

/// Unpacked source data: rows of `layout.stride` bytes.
fn read_rows(
    source: &[u8],
    layout: &Layout,
    color: &dyn Fn(u32) -> u32,
    pixels: &mut [u32],
) -> Option<()> {
    for (y, row) in pixels.chunks_mut(layout.width).enumerate() {
        let mut bits = Bits {
            data: source.get(y * layout.stride..)?,
            at: 0,
        };
        for pixel in row {
            *pixel = color(bits.read(layout.bits)?);
        }
    }
    Some(())
}

/// Packed source data: each row is an offset to the next one and packets.
fn unpack_rows(
    source: &[u8],
    layout: &Layout,
    color: &dyn Fn(u32) -> u32,
    pixels: &mut [u32],
) -> Option<()> {
    let offset_bytes = if layout.bits >= 8 { 2 } else { 1 };
    let mut row_at = 0usize;
    for row in pixels.chunks_mut(layout.width) {
        let start = source.get(row_at..)?;
        let offset = if layout.bits >= 8 {
            usize::from(be16(start, 0)? & 0x3ff)
        } else {
            usize::from(*start.first()?)
        };
        let next = row_at.checked_add((offset + 2) * 4)?;
        let mut bits = Bits {
            data: source.get(..next.min(source.len()))?,
            at: row_at.checked_add(offset_bytes)?.checked_mul(8)?,
        };
        let mut x = 0;
        while x < layout.width {
            let Some(kind) = bits.read(2) else { break };
            if kind == 0 {
                break;
            }
            let count = bits.read(6)? as usize + 1;
            let end = (x + count).min(layout.width);
            match kind {
                // Literal pixels.
                1 => {
                    for pixel in &mut row[x..end] {
                        *pixel = color(bits.read(layout.bits)?);
                    }
                    for _ in end..x + count {
                        bits.read(layout.bits)?;
                    }
                }
                // One repeated pixel.
                3 => row[x..end].fill(color(bits.read(layout.bits)?)),
                // Transparent pixels: the buffer is already clear.
                _ => {}
            }
            x += count;
        }
        row_at = next;
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    /// A `CCB ` chunk with the given flags, preamble words and size.
    fn ccb(flags: u32, pre0: u32, pre1: u32, width: u32, height: u32) -> Vec<u8> {
        let mut words = alloc::vec![0u32; 18];
        words[1] = flags;
        words[14] = pre0;
        words[15] = pre1;
        words[16] = width;
        words[17] = height;
        let mut chunk = b"CCB ".to_vec();
        chunk.extend_from_slice(&80u32.to_be_bytes());
        for word in words {
            chunk.extend_from_slice(&word.to_be_bytes());
        }
        chunk
    }

    fn pdat(body: &[u8]) -> Vec<u8> {
        let mut chunk = b"PDAT".to_vec();
        chunk.extend_from_slice(&(8 + body.len() as u32).to_be_bytes());
        chunk.extend_from_slice(body);
        chunk.resize(chunk.len().next_multiple_of(4), 0);
        chunk
    }

    fn plut(colors: &[u16]) -> Vec<u8> {
        let mut chunk = b"PLUT".to_vec();
        chunk.extend_from_slice(&(12 + colors.len() as u32 * 2).to_be_bytes());
        chunk.extend_from_slice(&(colors.len() as u32).to_be_bytes());
        for color in colors {
            chunk.extend_from_slice(&color.to_be_bytes());
        }
        chunk.resize(chunk.len().next_multiple_of(4), 0);
        chunk
    }

    #[test]
    fn unpacked_16_bit_cel_with_the_preamble_in_the_ccb() {
        // 2x1, 16 bits uncoded: BPP 6, UNCODED, one row; stride 2 words.
        let pre0 = 6 | 0x10;
        let pre1 = 1;
        let mut file = ccb(CCBPRE, pre0, pre1, 2, 1);
        file.extend(pdat(&[0x7c, 0x00, 0x00, 0x00, 0, 0, 0, 0]));
        let image = decode(&file).unwrap();
        assert_eq!((image.width(), image.height()), (2, 1));
        assert_eq!(image.get(0, 0), 0xff0000);
        assert_eq!(
            image.get(1, 0),
            crate::image::TRANSPARENT_FILL,
            "zero is clear"
        );
    }

    #[test]
    fn preamble_leads_the_data_without_ccbpre_and_bgnd_keeps_black() {
        // Unpacked 4-bit coded 2x1: PRE0 BPP 3 (4 bits); PRE1 width - 1 = 1.
        let mut file = ccb(BGND, 0, 0, 2, 1);
        file.extend(plut(&[0x0000, 0x7fff]));
        let mut data = 3u32.to_be_bytes().to_vec();
        data.extend_from_slice(&1u32.to_be_bytes());
        data.extend_from_slice(&[0x01, 0, 0, 0]);
        file.extend(pdat(&data));
        let image = decode(&file).unwrap();
        assert_eq!((image.get(0, 0), image.get(1, 0)), (0x000000, 0xffffff));
    }

    #[test]
    fn packed_rows_mix_literal_repeat_and_transparent_packets() {
        // 6 bits per pixel coded, 5 wide, 2 rows. Row offset byte first
        // (8 bits), then packets: literal x2 (01 000001, pixels 1 and 2),
        // transparent x1 (10 000000), repeat x2 of pixel 1 (11 000001, 1).
        let mut row = Vec::new();
        let mut bits = Vec::new();
        for (value, count) in [
            (0b01u32, 2),
            (0b000001, 6),
            (1, 6),
            (2, 6),
            (0b10, 2),
            (0, 6),
            (0b11, 2),
            (1, 6),
            (1, 6),
        ] {
            for i in (0..count).rev() {
                bits.push((value >> i & 1) as u8);
            }
        }
        row.push(2); // offset: 4 words minus 2
        let mut byte = 0;
        for (i, bit) in bits.iter().enumerate() {
            byte = byte << 1 | bit;
            if i % 8 == 7 {
                row.push(byte);
                byte = 0;
            }
        }
        if bits.len() % 8 != 0 {
            row.push(byte << (8 - bits.len() % 8));
        }
        row.resize(16, 0);
        let source = [row.clone(), row].concat();
        let pre0 = 4 | 1 << 6; // BPP 6 bits, 2 rows
        let mut file = ccb(PACKED | CCBPRE, pre0, 0, 5, 2);
        file.extend(plut(&[0x0000, 0x7c00, 0x03e0]));
        file.extend(pdat(&source));
        let image = decode(&file).unwrap();
        let row_colors: Vec<u32> = (0..5).map(|x| image.get(x, 1)).collect();
        let (red, green, clear) = (0xff0000, 0x00ff00, crate::image::TRANSPARENT_FILL);
        assert_eq!(row_colors, [red, green, clear, red, red]);
    }

    #[test]
    fn packed_widths_beyond_the_cel_engine_are_refused() {
        // 6-bit coded, one row of 4096 pixels: an offset byte of 0, then
        // seven zero bytes (an end-of-row packet). The engine counts pixels in
        // 11 bits, so 4096 is no cel, and 65536 wide rows must not be
        // expanded from a few bytes.
        for width in [2049, 4096, 65536] {
            let mut file = ccb(PACKED | CCBPRE, 4, 0, width, 1);
            file.extend(pdat(&[0; 8]));
            assert!(decode(&file).is_err(), "width {width}");
        }
        let mut file = ccb(PACKED | CCBPRE, 4, 0, 2048, 1);
        file.extend(pdat(&[0; 8]));
        assert_eq!(decode(&file).unwrap().width(), 2048);
    }

    #[test]
    fn declared_sizes_need_the_data_before_anything_is_allocated() {
        // 16-bit unpacked, 2048x1024, with 8 bytes of data.
        let mut file = ccb(CCBPRE, 6 | 0x10 | 1023 << 6, 2047 | 1022 << 16, 2048, 1024);
        file.extend(pdat(&[0; 8]));
        assert!(decode(&file).is_err());
        // Packed, 1024 rows, with data for two.
        let mut file = ccb(PACKED | CCBPRE, 4 | 1023 << 6, 0, 2048, 1024);
        file.extend(pdat(&[0; 16]));
        assert!(decode(&file).is_err());
    }

    #[test]
    fn rejects_unsupported_layouts_and_truncated_data() {
        let mut file = ccb(CCBPRE, 6 | 0x10, 1, 2, 1);
        file.extend(pdat(&[0; 8]));
        assert!(decode(&file[..file.len() - 5]).is_err());
        let mut lr = ccb(CCBPRE, 6 | 0x10, 1 | 0x800, 2, 1);
        lr.extend(pdat(&[0; 8]));
        assert!(decode(&lr).is_err());
        let mut eight_uncoded = ccb(CCBPRE, 5 | 0x10, 1, 2, 1);
        eight_uncoded.extend(pdat(&[0; 8]));
        assert!(decode(&eight_uncoded).is_err());
        assert!(decode(&ccb(CCBPRE, 6 | 0x10, 1, 2, 1)).is_err());
    }
}
