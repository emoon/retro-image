//! Pack-Ice depacker, shared by the Atari ST and Amiga decoders.
//!
//! Handles v1.1-v1.14 (`Ice!` as a footer), v1.x (`Ice!`, `TMM!`, `TSM!`,
//! `SHE!` headers) and v2.x (`ICE!` header).
//!
//! The bit stream layout follows Ancient's `IceDecompressor`
//! (<https://github.com/temisu/ancient>, src/IceDecompressor.cpp), which is
//! distributed under this licence:
//!
//! > Copyright (c) 2017-2026, Teemu Suutari. All rights reserved.
//! >
//! > Redistribution and use in source and binary forms, with or without
//! > modification, are permitted provided that the following conditions
//! > are met:
//! > 1. Redistributions of source code must retain the above copyright
//! >    notice, this list of conditions and the following disclaimer.
//! > 2. Redistributions in binary form must reproduce the above copyright
//! >    notice, this list of conditions and the following disclaimer in the
//! >    documentation and/or other materials provided with the distribution.
//! >
//! > THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
//! > "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
//! > LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
//! > A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
//! > HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
//! > SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
//! > LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
//! > DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
//! > THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
//! > (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
//! > OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
//!
//! Format background: <http://fileformats.archiveteam.org/wiki/Pack-Ice>.

use crate::bytes::be32;
use alloc::vec::Vec;

/// Largest unpacked size accepted.
const MAX_RAW_LEN: usize = 1 << 24;

/// Header tags of the v1.x stream layout.
const V1_TAGS: [&[u8; 4]; 4] = [b"Ice!", b"TMM!", b"TSM!", b"SHE!"];

/// Bit-stream generation, which decides the literal-run code, whether bits
/// come in words or bytes, and whether picture mode exists.
#[derive(Clone, Copy, PartialEq)]
enum Generation {
    /// v1.1-v1.14: `Ice!` footer, 32-bit words, shorter literal-run code.
    Footer,
    /// v1.x header: 32-bit words.
    V1Words,
    /// v1.x header from later releases (and v2.x): bytes.
    Bytes,
}

/// Whether `data` is a Pack-Ice file (header tag, or `Ice!` footer).
pub(crate) fn is_packed(data: &[u8]) -> bool {
    let tag = data.get(..4);
    V1_TAGS.iter().any(|t| tag == Some(&t[..]))
        || tag == Some(b"ICE!")
        || (data.len() >= 12 && data.ends_with(b"Ice!"))
}

/// Unpacks a Pack-Ice file; `None` if it is not one or is damaged.
pub(crate) fn unpack(data: &[u8]) -> Option<Vec<u8>> {
    let header = data.get(..4)?;
    if V1_TAGS.iter().all(|t| header != &t[..]) && header != b"ICE!" {
        // Footer form: stream, raw length, `Ice!`.
        let end = data.len().checked_sub(8)?;
        if !data.ends_with(b"Ice!") {
            return None;
        }
        let raw_len = be32(data, end)? as usize;
        check_raw_len(raw_len)?;
        return depack(&data[..end], raw_len, Generation::Footer, false);
    }
    let version2 = header == b"ICE!";
    let packed_len = be32(data, 4)? as usize;
    let raw_len = be32(data, 8)? as usize;
    if packed_len < 12 || packed_len > data.len() {
        return None;
    }
    check_raw_len(raw_len)?;
    let stream = &data[12..packed_len];
    if version2 {
        return depack(stream, raw_len, Generation::Bytes, true);
    }
    // v1 headers were used by both bit-stream generations.
    depack(stream, raw_len, Generation::V1Words, false)
        .or_else(|| depack(stream, raw_len, Generation::Bytes, false))
}

fn check_raw_len(raw_len: usize) -> Option<()> {
    (raw_len != 0 && raw_len <= MAX_RAW_LEN).then_some(())
}

/// Reads the packed data from its end; literal bytes and bit words come
/// from the same backward stream.
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    bits: u32,
    count: u32,
    bytes: bool,
}

impl Reader<'_> {
    fn byte(&mut self) -> Option<u8> {
        self.pos = self.pos.checked_sub(1)?;
        Some(self.data[self.pos])
    }

    fn word(&mut self) -> Option<u32> {
        self.pos = self.pos.checked_sub(4)?;
        be32(self.data, self.pos)
    }

    fn read(&mut self, mut count: u32) -> Option<u32> {
        let mut value = 0u32;
        while count > 0 {
            if self.count == 0 {
                if self.bytes {
                    self.bits = u32::from(self.byte()?);
                    self.count = 8;
                } else {
                    self.bits = self.word()?;
                    self.count = 32;
                }
            }
            let take = count.min(self.count);
            self.count -= take;
            let chunk = (u64::from(self.bits) >> self.count) & ((1u64 << take) - 1);
            value = ((u64::from(value) << take) | chunk) as u32;
            count -= take;
        }
        Some(value)
    }

    fn available_bits(&self) -> usize {
        self.pos * 8 + self.count as usize
    }

    /// Reads fields of `lengths` bits until one is not all ones; returns
    /// the sum of the all-ones values seen before plus the last value.
    fn cascade(&mut self, lengths: &[u32]) -> Option<u32> {
        let mut base = 0;
        for (i, &length) in lengths.iter().enumerate() {
            let value = self.read(length)?;
            let all_ones = (1 << length) - 1;
            if i == lengths.len() - 1 || value != all_ones {
                return Some(base + value);
            }
            base += all_ones;
        }
        None
    }
}

fn depack(
    stream: &[u8],
    raw_len: usize,
    generation: Generation,
    version2: bool,
) -> Option<Vec<u8>> {
    let bytes = generation == Generation::Bytes;
    let mut reader = Reader {
        data: stream,
        pos: stream.len(),
        bits: 0,
        count: 0,
        bytes,
    };
    // The first word holds the remaining bits above a single anchor bit.
    let anchor = if bytes {
        u32::from(reader.byte()?)
    } else {
        reader.word()?
    };
    if anchor != 0 {
        let used = 31 - anchor.trailing_zeros();
        let bits = if bytes { used.checked_sub(24)? } else { used };
        if bits > 0 {
            reader.bits = anchor >> (32 - used);
            reader.count = bits;
        }
    }

    let mut out = alloc::vec![0u8; raw_len];
    let mut dst = raw_len;
    loop {
        if reader.read(1)? == 1 {
            let count = match generation {
                Generation::Footer => reader.cascade(&[1, 2, 2, 3, 10])?,
                _ => reader.cascade(&[1, 2, 2, 3, 8, 15])?,
            } + 1;
            for _ in 0..count {
                dst = dst.checked_sub(1)?;
                out[dst] = reader.byte()?;
            }
        }
        if dst == 0 {
            break;
        }
        let count_base = reader.cascade(&[1, 1, 1, 1])?;
        let count = match count_base {
            0 => 0,
            1 => 1,
            2 => 2 + reader.read(1)?,
            3 => 4 + reader.read(2)?,
            _ => 8 + reader.read(10)?,
        } as usize
            + 2;
        let distance = if count == 2 {
            let distance = if reader.read(1)? == 1 {
                reader.read(9)? + 0x40
            } else {
                reader.read(6)?
            } as usize;
            distance + count - usize::from(bytes)
        } else {
            let base = match reader.cascade(&[1, 1])? {
                0 => 1,
                1 => 0,
                base => base,
            };
            let distance = match base {
                0 => reader.read(5)?,
                1 => 32 + reader.read(8)?,
                _ => 288 + reader.read(12)?,
            } as usize;
            match (bytes, distance) {
                (true, 0) => 1,
                (true, _) => distance + count - 1,
                (false, _) => distance + count,
            }
        };
        if distance == 0 || count > dst || dst + distance > raw_len {
            return None;
        }
        for _ in 0..count {
            dst -= 1;
            out[dst] = out[dst + distance];
        }
    }

    // Optional "picture mode": the last screen was stored as chunky
    // nibbles and is converted back to interleaved bitplanes.
    if generation != Generation::Footer && reader.available_bits() > 0 && reader.read(1)? == 1 {
        let mut picture_len = 32000;
        if version2 && reader.available_bits() >= 17 && reader.read(1)? == 1 {
            picture_len = reader.read(16)? as usize * 8 + 8;
        }
        let start = raw_len.checked_sub(picture_len)?;
        for group in out[start..].as_chunks_mut::<8>().0 {
            let mut planes = [0u16; 4];
            for j in (0..8).step_by(2) {
                let mut word = u16::from_be_bytes([group[6 - j], group[7 - j]]);
                for k in 0..16 {
                    planes[k & 3] = planes[k & 3] << 1 | word >> 15;
                    word <<= 1;
                }
            }
            for (j, plane) in planes.iter().enumerate() {
                group[j * 2..j * 2 + 2].copy_from_slice(&plane.to_be_bytes());
            }
        }
    }
    (reader.pos == 0).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `raw` as one literal run followed by nothing: the bit stream is one
    /// anchor word holding the run length code, with the literals before it.
    /// `lengths` is the literal-run code of the generation being built.
    fn literal_stream(raw: &[u8], lengths: &[u32]) -> Vec<u8> {
        let mut value = raw.len() - 1;
        let mut bits = alloc::vec![true];
        for (i, &length) in lengths.iter().enumerate() {
            let all_ones = (1 << length) - 1;
            let last = i == lengths.len() - 1 || value < all_ones;
            let field = if last { value } else { all_ones };
            bits.extend((0..length).rev().map(|b| field >> b & 1 == 1));
            if last {
                break;
            }
            value -= all_ones;
        }
        let used = bits.len() as u32;
        let word =
            bits.iter().fold(0u32, |w, &b| w << 1 | u32::from(b)) << (32 - used) | 1 << (31 - used);
        // The first literal read is the last byte before the anchor word.
        let mut stream = raw.to_vec();
        stream.extend_from_slice(&word.to_be_bytes());
        stream
    }

    fn with_header(tag: &[u8; 4], raw: &[u8]) -> Vec<u8> {
        let stream = literal_stream(raw, &[1, 2, 2, 3, 8, 15]);
        let mut out = tag.to_vec();
        out.extend_from_slice(&(12 + stream.len() as u32).to_be_bytes());
        out.extend_from_slice(&(raw.len() as u32).to_be_bytes());
        out.extend_from_slice(&stream);
        out
    }

    #[test]
    fn header_tags_unpack() {
        let raw: Vec<u8> = (0..9).collect();
        for tag in [b"Ice!", b"TMM!", b"TSM!", b"SHE!"] {
            let packed = with_header(tag, &raw);
            assert!(is_packed(&packed));
            assert_eq!(unpack(&packed).as_deref(), Some(&raw[..]), "{tag:?}");
        }
    }

    #[test]
    fn footer_form_uses_the_short_literal_code() {
        // Literal count 40 needs the 10-bit field of the v1.1 code, which
        // the later generations spell with an 8-bit field.
        let raw: Vec<u8> = (0..40).collect();
        let mut packed = literal_stream(&raw, &[1, 2, 2, 3, 10]);
        packed.extend_from_slice(&(raw.len() as u32).to_be_bytes());
        packed.extend_from_slice(b"Ice!");
        assert!(is_packed(&packed));
        assert_eq!(unpack(&packed).as_deref(), Some(&raw[..]));
    }

    #[test]
    fn damaged_files_are_rejected_not_unpacked() {
        let raw: Vec<u8> = (0..9).collect();
        let packed = with_header(b"TMM!", &raw);
        assert_eq!(unpack(&packed[..packed.len() - 1]), None);
        let mut huge = packed.clone();
        huge[8..12].copy_from_slice(&u32::MAX.to_be_bytes());
        assert_eq!(unpack(&huge), None);
        assert_eq!(unpack(b"Ice!"), None);
        assert!(!is_packed(b"FORM\0\0\0\0ILBM"));
        assert_eq!(unpack(b"FORM\0\0\0\0ILBMxxxx"), None);
    }
}
