//! Grafix `GRX`, uncompressed and compressed.
//!
//! The survey found no documentation; everything here was reverse
//! engineered from sample files and by feeding hand-modified copies of
//! them to `recoil2png` as a black box (flipping single bits of the packed
//! data and watching which output pixels change):
//!
//! - `GRXP`, version, program name, then at 28 a compression word (0 or 1),
//!   width, height and colour count, 256 VDI RGB triplets in pen order.
//! - At 1572 a word, the unpacked size (long) and the lengths of two packed
//!   streams (longs); data starts at 1586.
//! - Unpacked data is word-interleaved planes with rows padded to 16 pixels.
//! - Compressed files hold two LZW streams, unpacking to the first and the
//!   second half of the data. Codes are read least significant bit first
//!   and start 9 bits wide; 0-255 are literal bytes, 256 widens the codes
//!   by one bit, 257 clears the dictionary (back to 9 bits) and new entries
//!   start at 258.

use super::common::{MAX_PIXELS, crop, planar_image, vdi_palette};
use crate::bytes::{be16, be32};
use crate::{DecodeError, Image};
use alloc::vec::Vec;

const PALETTE: usize = 36;
const SIZES: usize = PALETTE + 256 * 6 + 2;
const DATA: usize = SIZES + 12;

pub(super) fn decode_grx(data: &[u8]) -> Result<Image, DecodeError> {
    decode(data).ok_or(DecodeError::Unrecognized)
}

fn decode(data: &[u8]) -> Option<Image> {
    if data.get(..4)? != b"GRXP" {
        return None;
    }
    let width = u32::from(be16(data, 30)?);
    let height = u32::from(be16(data, 32)?);
    let colors = usize::from(be16(data, 34)?);
    let planes = match colors {
        2 => 1,
        4 => 2,
        16 => 4,
        256 => 8,
        _ => return None,
    };
    if width == 0 || height == 0 || width as usize * height as usize > MAX_PIXELS {
        return None;
    }
    let palette = vdi_palette(data.get(PALETTE..)?, colors)?;
    let padded = width.div_ceil(16) * 16;
    let len = (padded / 8 * planes) as usize * height as usize;
    let unpacked;
    let bitmap = match be16(data, 28)? {
        0 => data.get(DATA..)?,
        1 => {
            unpacked = unpack(data, len)?;
            &unpacked
        }
        _ => return None,
    };
    let image = planar_image(bitmap, padded, height, planes, &palette, 1)?;
    Some(if padded == width {
        image
    } else {
        crop(&image, width, height)
    })
}

/// Unpacks the two LZW streams of a compressed file into `len` bytes.
fn unpack(data: &[u8], len: usize) -> Option<Vec<u8>> {
    let size = be32(data, SIZES)? as usize;
    if size != len {
        return None;
    }
    let first_len = be32(data, SIZES + 4)? as usize;
    let second_len = be32(data, SIZES + 8)? as usize;
    let middle = DATA.checked_add(first_len)?;
    let first = data.get(DATA..middle)?;
    let second = data.get(middle..middle.checked_add(second_len)?)?;
    let mut out = Vec::with_capacity(len);
    lzw(first, len / 2, &mut out)?;
    lzw(second, len - len / 2, &mut out)?;
    Some(out)
}

const WIDEN: u16 = 256;
const CLEAR: u16 = 257;
const FIRST_ENTRY: u16 = 258;
const MAX_BITS: u32 = 12;

/// Appends `count` bytes unpacked from `src` to `out`.
fn lzw(src: &[u8], count: usize, out: &mut Vec<u8>) -> Option<()> {
    let end = out.len() + count;
    let mut bits = BitReader::new(src);
    // Dictionary entry `code` is entry `prefix[code]` followed by
    // `suffix[code]`; entries below 256 are single bytes.
    let mut prefix = [0u16; 1 << MAX_BITS];
    let mut suffix = [0u8; 1 << MAX_BITS];
    let mut width = 9;
    let mut next = FIRST_ENTRY;
    let mut previous: Option<u16> = None;
    let mut stack = Vec::new();
    while out.len() < end {
        let code = bits.read(width)?;
        match code {
            WIDEN => {
                width += 1;
                if width > MAX_BITS {
                    return None;
                }
                continue;
            }
            CLEAR => {
                width = 9;
                next = FIRST_ENTRY;
                previous = None;
                continue;
            }
            _ => {}
        }
        // The string's first byte, needed for the new dictionary entry.
        let mut walk = match (code < next, previous) {
            (true, _) => code,
            (false, Some(previous)) if code == next => {
                // The entry being defined: previous string + its first byte.
                stack.push(first_byte(&prefix, previous));
                previous
            }
            _ => return None,
        };
        while walk >= FIRST_ENTRY {
            stack.push(suffix[usize::from(walk)]);
            walk = prefix[usize::from(walk)];
        }
        let first = walk as u8;
        stack.push(first);
        while let Some(byte) = stack.pop() {
            if out.len() < end {
                out.push(byte);
            }
        }
        if let Some(previous) = previous
            && usize::from(next) < prefix.len()
        {
            prefix[usize::from(next)] = previous;
            suffix[usize::from(next)] = first;
            next += 1;
        }
        previous = Some(code);
    }
    Some(())
}

fn first_byte(prefix: &[u16], mut code: u16) -> u8 {
    while code >= FIRST_ENTRY {
        code = prefix[usize::from(code)];
    }
    code as u8
}

/// Reads codes least significant bit first.
struct BitReader<'a> {
    src: &'a [u8],
    pos: usize,
    buffer: u32,
    count: u32,
}

impl<'a> BitReader<'a> {
    fn new(src: &'a [u8]) -> Self {
        Self {
            src,
            pos: 0,
            buffer: 0,
            count: 0,
        }
    }

    fn read(&mut self, width: u32) -> Option<u16> {
        while self.count < width {
            self.buffer |= u32::from(*self.src.get(self.pos)?) << self.count;
            self.pos += 1;
            self.count += 8;
        }
        let value = self.buffer & ((1 << width) - 1);
        self.buffer >>= width;
        self.count -= width;
        Some(value as u16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Packs 9-bit codes least significant bit first.
    fn pack(codes: &[u16]) -> Vec<u8> {
        let mut out = Vec::new();
        let (mut buffer, mut count) = (0u32, 0);
        for &code in codes {
            buffer |= u32::from(code) << count;
            count += 9;
            while count >= 8 {
                out.push(buffer as u8);
                buffer >>= 8;
                count -= 8;
            }
        }
        if count > 0 {
            out.push(buffer as u8);
        }
        out
    }

    #[test]
    fn unpacks_literals_and_runs() {
        // 'a', then 258 = "aa" (not yet defined), 259 = "aaa", then 'b'.
        let src = pack(&[0x61, 258, 259, 0x62]);
        let mut out = Vec::new();
        lzw(&src, 7, &mut out).unwrap();
        assert_eq!(out, b"aaaaaab");
    }

    #[test]
    fn clear_restarts_the_dictionary() {
        let src = pack(&[0x61, 0x62, CLEAR, 0x63, 258]);
        let mut out = Vec::new();
        lzw(&src, 5, &mut out).unwrap();
        assert_eq!(out, b"abccc");
    }

    #[test]
    fn rejects_undefined_codes() {
        let src = pack(&[0x61, 300]);
        assert_eq!(lzw(&src, 4, &mut Vec::new()), None);
    }
}
