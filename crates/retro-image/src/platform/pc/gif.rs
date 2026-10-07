//! CompuServe GIF87a and GIF89a (and Fractint FRA files, which are GIFs).
//!
//! Sources:
//! - GIF89a specification, W3C: <https://www.w3.org/Graphics/GIF/spec-gif89a.txt>
//!   (logical screen descriptor, color tables, image descriptor, interlace
//!   passes, variable-width LZW with clear and end codes, extension blocks).
//! - Deark `gif.c` (<https://github.com/jsummers/deark>, MIT license) for how
//!   real files deviate, and as the oracle for the sample files.
//! - Fractint FRA: <http://fileformats.archiveteam.org/wiki/FRA_(Fractint)>
//!   (parameters follow the trailer or sit in an application extension).
//!
//! A MacBinary wrapper (`crate::macbinary`) is removed first.
//!
//! Only the first image is decoded. Its transparent color index (from a
//! graphic control extension before the image) is transparent. When the first
//! image is smaller than the logical screen it is placed on a screen filled
//! with the background color (transparent when the background index is also
//! the transparent index), and cut off where it leaves the screen; without a
//! usable screen the frame is shown alone.
//! Missing trailers and truncated LZW data are tolerated: the pixels decoded
//! so far are kept.
//!
//! Verification: no RECOIL oracle for this format; output was compared pixel
//! for pixel with Deark's PNG output on the sample files.

// Parts of this file follow Deark's modules/gif.c
// (Deark, https://github.com/jsummers/deark):
//
// Copyright (C) 2016 Jason Summers
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

use alloc::vec;
use alloc::vec::Vec;

use crate::bytes::le16;
use crate::image::{CLEAR, check_size};
use crate::macbinary::data_fork_or_self;
use crate::{DecodeError, Image};

const FAIL: DecodeError = DecodeError::Invalid;
const MAX_CODES: usize = 4096;

pub(super) fn decode_gif(data: &[u8]) -> Result<Image, DecodeError> {
    let data = data_fork_or_self(data);
    if !is_gif(data) {
        return Err(FAIL);
    }
    let screen_w = usize::from(le16(data, 6).ok_or(FAIL)?);
    let screen_h = usize::from(le16(data, 8).ok_or(FAIL)?);
    let flags = *data.get(10).ok_or(FAIL)?;
    let background = usize::from(*data.get(11).ok_or(FAIL)?);
    let mut at = 13;
    let global = if flags & 0x80 != 0 {
        let (table, next) = read_palette(data, at, flags)?;
        at = next;
        Some(table)
    } else {
        None
    };

    // Skip extensions up to the first image descriptor, noting the
    // transparent color of a graphic control extension (`21 F9 04`, flags,
    // delay, color index): bit 0 of the flags says the index is used.
    let mut transparent = None;
    loop {
        match *data.get(at).ok_or(FAIL)? {
            0x21 => {
                if data.get(at + 1..at + 3) == Some(&[0xf9, 4])
                    && data.get(at + 3).is_some_and(|f| f & 1 != 0)
                {
                    transparent = data.get(at + 6).copied().map(usize::from);
                }
                at = skip_sub_blocks(data, at + 2)?;
            }
            0x2c => break,
            _ => return Err(FAIL),
        }
    }
    let (left, top) = (
        usize::from(le16(data, at + 1).ok_or(FAIL)?),
        usize::from(le16(data, at + 3).ok_or(FAIL)?),
    );
    let (width, height) = (
        usize::from(le16(data, at + 5).ok_or(FAIL)?),
        usize::from(le16(data, at + 7).ok_or(FAIL)?),
    );
    let frame_flags = *data.get(at + 9).ok_or(FAIL)?;
    at += 10;
    check_size(width, height)?;
    let palette = if frame_flags & 0x80 != 0 {
        let (table, next) = read_palette(data, at, frame_flags)?;
        at = next;
        table
    } else {
        global.clone().ok_or(FAIL)?
    };

    let min_code_size = *data.get(at).ok_or(FAIL)?;
    let stream = collect_sub_blocks(data, at + 1);
    let mut indices = lzw_decode(&stream, min_code_size, width * height)?;
    if frame_flags & 0x40 != 0 {
        indices = deinterlace(&indices, width, height);
    }

    let mut colors: Vec<u32> = pad(&palette).iter().map(|c| 0xff00_0000 | c).collect();
    if let Some(clear) = transparent {
        colors[clear] = CLEAR;
    }
    let frame = Image::from_indexed_argb(width as u32, height as u32, &indices, &colors)?;
    let screen_ok = screen_w != 0 && screen_h != 0 && check_size(screen_w, screen_h).is_ok();
    // The frame must overlap the screen; the part outside it is cut off.
    if !screen_ok
        || left >= screen_w
        || top >= screen_h
        || (left, top, width, height) == (0, 0, screen_w, screen_h)
    {
        return Ok(frame);
    }
    // The background index is only meaningful with a global table.
    let fill = match global.as_ref().and_then(|table| table.get(background)) {
        _ if transparent == Some(background) => CLEAR,
        Some(color) => 0xff00_0000 | color,
        None => 0xff00_0000,
    };
    let pixels = core::iter::repeat_n(fill, screen_w * screen_h);
    let mut canvas = Image::from_argb(screen_w as u32, screen_h as u32, pixels)?;
    canvas.paste(&frame, left, top, width, height);
    Ok(canvas)
}

/// `GIF87a` or `GIF89a` followed by a screen descriptor; the content check
/// behind `.signature()`.
fn is_gif(data: &[u8]) -> bool {
    (data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a")) && data.len() >= 13
}

/// Reads the color table at `at` sized by the low bits of `flags`; returns
/// the colors and the offset after the table.
fn read_palette(data: &[u8], at: usize, flags: u8) -> Result<(Vec<u32>, usize), DecodeError> {
    let len = 3usize << ((flags & 7) + 1);
    let bytes = data.get(at..at + len).ok_or(FAIL)?;
    let table = bytes
        .as_chunks::<3>()
        .0
        .iter()
        .map(|c| u32::from(c[0]) << 16 | u32::from(c[1]) << 8 | u32::from(c[2]))
        .collect();
    Ok((table, at + len))
}

/// The table padded with black to 256 entries, so a stray index is not fatal.
fn pad(palette: &[u32]) -> Vec<u32> {
    let mut table = vec![0; 256];
    table[..palette.len()].copy_from_slice(palette);
    table
}

/// Offset after the data sub-blocks starting at `at`.
fn skip_sub_blocks(data: &[u8], mut at: usize) -> Result<usize, DecodeError> {
    loop {
        let len = usize::from(*data.get(at).ok_or(FAIL)?);
        at += 1 + len;
        if len == 0 {
            return Ok(at);
        }
    }
}

/// The concatenated payload of the sub-blocks at `at`; stops at the
/// terminator or the end of the data.
fn collect_sub_blocks(data: &[u8], mut at: usize) -> Vec<u8> {
    let mut out = Vec::new();
    while let Some(&len) = data.get(at) {
        if len == 0 {
            break;
        }
        let end = (at + 1 + usize::from(len)).min(data.len());
        out.extend_from_slice(&data[at + 1..end]);
        at = end;
    }
    out
}

/// Decodes GIF-flavoured LZW (LSB-first codes, 2^n clear and 2^n+1 end
/// codes, width growing to 12 bits) into at most `count` indices. Short
/// data leaves the rest 0.
fn lzw_decode(stream: &[u8], min_code_size: u8, count: usize) -> Result<Vec<u8>, DecodeError> {
    if !(1..=8).contains(&min_code_size) {
        return Err(FAIL);
    }
    let clear = 1usize << min_code_size;
    let end = clear + 1;
    let mut prefix = [0u16; MAX_CODES];
    let mut suffix = [0u8; MAX_CODES];
    let mut length = [0u16; MAX_CODES];
    for i in 0..clear {
        suffix[i] = i as u8;
        length[i] = 1;
    }
    let mut out = vec![0u8; count];
    let mut written = 0;
    let mut next = end + 1;
    let mut size = u32::from(min_code_size) + 1;
    let mut previous: Option<usize> = None;
    let (mut bits, mut have) = (0u32, 0u32);
    let mut bytes = stream.iter();

    'codes: while written < count {
        while have < size {
            let Some(&b) = bytes.next() else {
                break 'codes;
            };
            bits |= u32::from(b) << have;
            have += 8;
        }
        let code = (bits & ((1 << size) - 1)) as usize;
        bits >>= size;
        have -= size;

        if code == clear {
            next = end + 1;
            size = u32::from(min_code_size) + 1;
            previous = None;
            continue;
        }
        if code == end {
            break;
        }
        // The first code after a clear must be a literal.
        let Some(prev) = previous else {
            if code >= clear {
                return Err(FAIL);
            }
            out[written] = code as u8;
            written += 1;
            previous = Some(code);
            continue;
        };
        // A code equal to `next` is the not-yet-defined KwKwK case.
        if code > next || (code == next && next >= MAX_CODES) {
            return Err(FAIL);
        }
        let known = code < next;
        let entry = if known { code } else { prev };
        let first = first_byte(&prefix, &suffix, length[entry], entry);

        if next < MAX_CODES {
            prefix[next] = prev as u16;
            suffix[next] = first;
            length[next] = length[prev] + 1;
            next += 1;
            if next == 1 << size && size < 12 {
                size += 1;
            }
        }
        let code = if known { code } else { next - 1 };
        let run = usize::from(length[code]).min(count - written);
        // Fill the string back to front.
        let mut cursor = code;
        let total = usize::from(length[code]);
        for pos in (0..total).rev() {
            if pos < run {
                out[written + pos] = suffix[cursor];
            }
            cursor = usize::from(prefix[cursor]);
        }
        written += run;
        previous = Some(code);
    }
    if written == 0 {
        return Err(FAIL);
    }
    Ok(out)
}

/// The first byte of the string for `code`.
fn first_byte(prefix: &[u16], suffix: &[u8], length: u16, mut code: usize) -> u8 {
    for _ in 1..length {
        code = usize::from(prefix[code]);
    }
    suffix[code]
}

/// Rows stored in four interlace passes (every 8th from 0, every 8th from
/// 4, every 4th from 2, every 2nd from 1) back to display order.
fn deinterlace(indices: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut out = vec![0u8; indices.len()];
    let passes = [(0, 8), (4, 8), (2, 4), (1, 2)];
    let mut stored = indices.chunks_exact(width);
    for (first, step) in passes {
        for y in (first..height).step_by(step) {
            if let Some(row) = stored.next() {
                out[y * width..(y + 1) * width].copy_from_slice(row);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 2x2 GIF with a 4-color global table; `lzw` is the image data.
    fn gif(lzw: &[u8], frame_flags: u8) -> Vec<u8> {
        let mut g = b"GIF89a".to_vec();
        g.extend_from_slice(&[2, 0, 2, 0, 0x81, 0, 0]);
        g.extend_from_slice(&[0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255]);
        g.extend_from_slice(&[0x21, 0xf9, 4, 1, 0, 0, 0, 0]); // transparency
        g.extend_from_slice(&[0x2c, 0, 0, 0, 0, 2, 0, 2, 0, frame_flags, 2]);
        g.push(lzw.len() as u8);
        g.extend_from_slice(lzw);
        g.extend_from_slice(&[0, 0x3b]);
        g
    }

    /// Packs (code, width) pairs LSB first.
    fn pack(codes: &[(u32, u32)]) -> Vec<u8> {
        let (mut out, mut bits, mut have) = (Vec::new(), 0u32, 0);
        for &(code, width) in codes {
            bits |= code << have;
            have += width;
            while have >= 8 {
                out.push(bits as u8);
                bits >>= 8;
                have -= 8;
            }
        }
        if have > 0 {
            out.push(bits as u8);
        }
        out
    }

    /// Clear, pixels 1 2 3 0, end; the width grows to 4 bits after the
    /// table reaches 8 entries.
    fn literals() -> Vec<u8> {
        pack(&[(4, 3), (1, 3), (2, 3), (3, 3), (0, 4), (5, 4)])
    }

    #[test]
    fn decodes_literals_and_the_transparent_index_is_clear() {
        let image = decode_gif(&gif(&literals(), 0)).unwrap();
        assert_eq!((image.width(), image.height()), (2, 2));
        assert_eq!(image.rgb(), &[255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0]);
        // Index 0 is the transparent one, so only the last pixel is transparent.
        assert_eq!(image.get_argb(0, 0), 0xffff_0000);
        assert_eq!(image.get_argb(1, 1), CLEAR);
    }

    #[test]
    fn interlace_reorders_rows() {
        // Two rows: pass one stores row 0, pass two (start 4) none, pass
        // three none, pass four row 1; so file order equals display order
        // for a 2-row image.
        let plain = decode_gif(&gif(&literals(), 0)).unwrap();
        let interlaced = decode_gif(&gif(&literals(), 0x40)).unwrap();
        assert_eq!(plain, interlaced);
        let rows = deinterlace(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9], 1, 10);
        assert_eq!(rows, [0, 5, 3, 6, 2, 7, 4, 8, 1, 9]);
    }

    #[test]
    fn repeated_strings_use_the_kwkwk_code() {
        // Clear, 1, then code 6 (the next free entry, "11"), end: 1 1 1.
        let codes = pack(&[(4, 3), (1, 3), (6, 3), (5, 3)]);
        let image = decode_gif(&gif(&codes, 0)).unwrap();
        assert_eq!(&image.rgb()[..9], &[255, 0, 0, 255, 0, 0, 255, 0, 0]);
    }

    #[test]
    fn rejects_bad_data() {
        let good = gif(&literals(), 0);
        assert!(decode_gif(&good[..20]).is_err());
        assert!(decode_gif(b"GIF89a").is_err());
        let mut no_table = good.clone();
        no_table[10] = 0;
        assert!(decode_gif(&no_table).is_err());
        assert!(lzw_decode(&[0xff; 8], 9, 4).is_err());
    }
}
