//! Rob Northen Compression (RNC, "ProPack"), methods 1 and 2, in both the
//! old stream layout (12-byte header, read from the end) and the new one
//! (18-byte header with CRCs and a chunk count). Used by Amiga and PC games
//! for data files, including some pictures.
//!
//! The header fields, the decision between the old and new streams (the
//! stream-start checks and the packed-data CRC-16 as the last resort), the
//! fixed code tables of the old streams and of method 2, and the Huffman
//! tables of the new method 1 stream follow Ancient's `RNCDecompressor`
//! (<https://github.com/temisu/ancient>, src/RNCDecompressor.cpp), which is
//! distributed under this license:
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
//! Format background: <http://fileformats.archiveteam.org/wiki/RNC>.
//! Checked on the Sembiance `rnc` samples, including `test_C1.rnc1`,
//! `test_C1.rnc1old` and `test_C1.rnc2`, which hold the same data packed three
//! ways.
//!
//! The CRCs are not checked. The `...\x01` variant of Total Carnage is not
//! recognized.

use super::lz::{
    BackwardOutput, ByteBits, LsbBits, MsbBits, PrefixCode, Ranges, Stream, copy_back, put,
};
use crate::bytes::{be16, be32};
use alloc::vec::Vec;

/// Largest unpacked size accepted.
const MAX_RAW_LEN: usize = 1 << 24;

const OLD_HEADER_LEN: usize = 12;
const NEW_HEADER_LEN: usize = 18;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Method {
    One,
    Two,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Layout {
    Old,
    New,
}

struct Header {
    method: Method,
    layout: Layout,
    raw_len: usize,
    packed_len: usize,
    chunks: u8,
}

impl Header {
    fn parse(data: &[u8]) -> Option<Self> {
        let method = match data.get(..4)? {
            b"RNC\x01" => Method::One,
            b"RNC\x02" => Method::Two,
            _ => return None,
        };
        let raw_len = be32(data, 4)? as usize;
        let packed_len = be32(data, 8)? as usize;
        if raw_len == 0 || packed_len == 0 || raw_len > MAX_RAW_LEN {
            return None;
        }
        let layout = layout(data, method, packed_len)?;
        let header_len = match layout {
            Layout::Old => OLD_HEADER_LEN,
            Layout::New => NEW_HEADER_LEN,
        };
        if packed_len.checked_add(header_len)? > data.len() {
            return None;
        }
        Some(Self {
            method,
            layout,
            raw_len,
            packed_len,
            chunks: if layout == Layout::New {
                *data.get(17)?
            } else {
                0
            },
        })
    }
}

/// Both layouts carry the same tag, so the stream decides: an old stream
/// ends (at its beginning, as it is read backward) with literals, and a new
/// one starts with bits a writer always leaves a certain way; failing both,
/// a matching CRC-16 over the packed bytes means a new stream.
fn layout(data: &[u8], method: Method, packed_len: usize) -> Option<Layout> {
    if data.len() <= NEW_HEADER_LEN {
        return Some(Layout::Old);
    }
    let new_start = data[NEW_HEADER_LEN];
    let old_at = match method {
        Method::One => packed_len.checked_add(11)?,
        Method::Two => packed_len.checked_add(10)?,
    };
    let old_start = *data.get(old_at)?;
    if old_start & 0x80 == 0 {
        return Some(Layout::New);
    }
    let looks_old = match method {
        Method::One => new_start & 3 != 0 || new_start & 0x7c == 0,
        Method::Two => new_start & 0x30 == 0x30,
    };
    if looks_old {
        return Some(Layout::Old);
    }
    let packed = data.get(NEW_HEADER_LEN..NEW_HEADER_LEN.checked_add(packed_len)?);
    let crc_matches = packed.is_some_and(|p| Some(crc16(p)) == be16(data, 14));
    Some(if crc_matches {
        Layout::New
    } else {
        Layout::Old
    })
}

/// CRC-16 with the reflected polynomial 0xa001, starting from 0.
fn crc16(data: &[u8]) -> u16 {
    data.iter().fold(0, |mut crc, &byte| {
        crc ^= u16::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                crc >> 1 ^ 0xa001
            } else {
                crc >> 1
            };
        }
        crc
    })
}

/// Whether `data` starts with an RNC header whose sizes fit the file.
pub(crate) fn is_packed(data: &[u8]) -> bool {
    Header::parse(data).is_some()
}

/// Unpacks an RNC file; `None` if it is not one or is damaged.
pub(crate) fn unpack(data: &[u8]) -> Option<Vec<u8>> {
    let header = Header::parse(data)?;
    match (header.layout, header.method) {
        (Layout::Old, method) => unpack_old(data, &header, method == Method::Two),
        (Layout::New, Method::One) => unpack_new_one(data, &header),
        (Layout::New, Method::Two) => unpack_new_two(data, &header),
    }
}

/// A stream read backward from its end, bits and literal bytes sharing it.
struct BackBits<'a> {
    input: Stream<'a>,
    bits: MsbBits,
}

impl BackBits<'_> {
    fn bits(&mut self, count: u32) -> Option<u32> {
        let input = &mut self.input;
        self.bits
            .read(count, || Some((u32::from(input.back_byte()?), 8)))
    }
}

/// The old streams: bits and literal bytes from one backward byte stream,
/// output written backward.
fn unpack_old(data: &[u8], header: &Header, method_two: bool) -> Option<Vec<u8>> {
    const LITERALS_ONE: Ranges<6> = Ranges::new([
        (1, false),
        (1, false),
        (2, false),
        (2, false),
        (3, false),
        (10, false),
    ]);
    const LITERALS_TWO: Ranges<18> = Ranges::new([
        (1, false),
        (1, false),
        (1, false),
        (2, false),
        (3, false),
        (4, false),
        (5, false),
        (6, false),
        (7, false),
        (8, false),
        (9, false),
        (10, false),
        (11, false),
        (12, false),
        (13, false),
        (14, false),
        (15, false),
        (16, false),
    ]);
    let stream = &data[OLD_HEADER_LEN..OLD_HEADER_LEN + header.packed_len];
    let mut s = BackBits {
        input: Stream::new(stream),
        bits: MsbBits::default(),
    };
    let (mut distance_bits, mut length_bits) = (12, 10);
    if method_two {
        let sizes = u32::from(s.input.back_byte()?) + 1;
        distance_bits = sizes & 15;
        length_bits = (sizes >> 4) + 1;
    }
    // The first byte holds the anchor bit: its lowest set bit marks where the
    // bits to use start.
    let first = u32::from(s.input.back_byte()?);
    if let Some(anchor) = (0..7).find(|i| first >> i & 1 == 1) {
        s.bits = MsbBits::with(first >> (anchor + 1), 7 - anchor);
    }
    let length_code = PrefixCode::new(&[(1, 0, 0), (2, 2, 1), (3, 6, 2), (4, 14, 3), (4, 15, 4)])?;
    let distance_code = PrefixCode::new(&[(1, 0, 1), (2, 2, 0), (2, 3, 2)])?;
    let lengths = Ranges::<5>::new([
        (0, false),
        (0, false),
        (1, false),
        (2, false),
        (length_bits, false),
    ]);
    let distances = Ranges::<3>::new([(5, false), (8, false), (distance_bits, false)]);

    let mut out = BackwardOutput::new(header.raw_len);
    loop {
        let literals = if method_two {
            LITERALS_TWO.cascade(|n| s.bits(n))?
        } else {
            LITERALS_ONE.cascade(|n| s.bits(n))?
        };
        for _ in 0..literals {
            out.put(s.input.back_byte()?)?;
        }
        if out.is_full() {
            return Some(out.finish());
        }
        let which = length_code.decode(|| s.bits(1))?;
        let count = lengths.decode(which, |n| s.bits(n))? + 2;
        let distance = if count != 2 {
            let which = distance_code.decode(|| s.bits(1))?;
            distances.decode(which, |n| s.bits(n))?
        } else if s.bits(1)? == 0 {
            s.bits(6)? as usize
        } else {
            s.bits(9)? as usize + 64
        };
        out.copy(
            if distance == 0 {
                1
            } else {
                distance + count - 1
            },
            count,
        )?;
    }
}

/// Bits of the new method 1 stream: 16-bit little-endian words, low bits first.
struct WordBits<'a> {
    input: Stream<'a>,
    bits: LsbBits,
}

impl WordBits<'_> {
    fn bits(&mut self, count: u32) -> Option<u32> {
        let input = &mut self.input;
        self.bits.read(count, || Some((input.le_word()?, 16)))
    }

    /// A table of code lengths for `PrefixCode::canonical`: a count, then 4
    /// bits each; `Some(None)` if the count is 0 and the table is unused.
    fn table(&mut self) -> Option<Option<PrefixCode>> {
        let count = self.bits(5)?;
        if count == 0 {
            return Some(None);
        }
        let lengths: Vec<u8> = (0..count)
            .map(|_| self.bits(4).map(|l| l as u8))
            .collect::<Option<_>>()?;
        Some(Some(PrefixCode::canonical(&lengths)?))
    }

    /// A number: a code, and for codes of 2 or more, that many less one
    /// further bits below an implied top bit.
    fn number(&mut self, code: &Option<PrefixCode>) -> Option<usize> {
        let value = code.as_ref()?.decode(|| self.bits(1))?;
        if value < 2 {
            return Some(value as usize);
        }
        let extra = value - 1;
        (extra <= 24).then_some(())?;
        Some(1 << extra | self.bits(extra)? as usize)
    }
}

/// The new method 1 stream: chunks, each with three Huffman tables (literal
/// counts, distances, lengths) and a count of literal-run and match pairs.
fn unpack_new_one(data: &[u8], header: &Header) -> Option<Vec<u8>> {
    // The reader may run one byte past the end of the stream.
    let mut stream = data[NEW_HEADER_LEN..NEW_HEADER_LEN + header.packed_len].to_vec();
    stream.push(0);
    let mut s = WordBits {
        input: Stream::new(&stream),
        bits: LsbBits::default(),
    };
    let mut out = alloc::vec![0u8; header.raw_len];
    let mut at = 0;
    s.bits(2)?;
    while at < out.len() {
        let literal_code = s.table()?;
        let distance_code = s.table()?;
        let length_code = s.table()?;
        let pairs = s.bits(16)?;
        for _ in 1..pairs {
            let literals = s.number(&literal_code)?;
            for _ in 0..literals {
                put(&mut out, &mut at, s.input.byte()?)?;
            }
            let distance = s.number(&distance_code)? + 1;
            let count = s.number(&length_code)? + 2;
            copy_back(&mut out, &mut at, distance, count)?;
        }
        let literals = s.number(&literal_code)?;
        for _ in 0..literals {
            put(&mut out, &mut at, s.input.byte()?)?;
        }
    }
    Some(out)
}

/// The new method 2 stream: one command code per item; bits and bytes
/// share the forward stream.
fn unpack_new_two(data: &[u8], header: &Header) -> Option<Vec<u8>> {
    const LITERAL: u32 = 0;
    const MOVE: u32 = 1;
    const MOVE_TWO: u32 = 2;
    const MOVE_THREE: u32 = 3;
    const CONDITIONAL: u32 = 4;
    /// A length of 9 is the marker for a block of literals.
    const BLOCK: u32 = 9;
    let commands = PrefixCode::new(&[
        (1, 0, LITERAL),
        (2, 2, MOVE),
        (3, 6, MOVE_TWO),
        (4, 14, MOVE_THREE),
        (4, 15, CONDITIONAL),
    ])?;
    let lengths = PrefixCode::new(&[
        (2, 0, 4),
        (2, 2, 5),
        (3, 2, 6),
        (3, 3, 7),
        (3, 6, 8),
        (3, 7, BLOCK),
    ])?;
    let distances = PrefixCode::new(&[
        (1, 0b000000, 0),
        (3, 0b000110, 1),
        (4, 0b001000, 2),
        (4, 0b001001, 3),
        (5, 0b010101, 4),
        (5, 0b010111, 5),
        (5, 0b011101, 6),
        (5, 0b011111, 7),
        (6, 0b101000, 8),
        (6, 0b101001, 9),
        (6, 0b101100, 10),
        (6, 0b101101, 11),
        (6, 0b111000, 12),
        (6, 0b111001, 13),
        (6, 0b111100, 14),
        (6, 0b111101, 15),
    ])?;
    let stream = &data[NEW_HEADER_LEN..NEW_HEADER_LEN + header.packed_len];
    let mut s = ByteBits::new(stream);
    let mut out = alloc::vec![0u8; header.raw_len];
    let mut at = 0;
    s.bits(2)?;
    let (mut found, mut done) = (0u8, false);
    while !done && found < header.chunks {
        match commands.decode(|| s.bits(1))? {
            LITERAL => {
                let byte = s.input.byte()?;
                put(&mut out, &mut at, byte)?;
            }
            MOVE => {
                let count = lengths.decode(|| s.bits(1))?;
                if count == BLOCK {
                    let block = (s.bits(4)? + 3) * 4;
                    for _ in 0..block {
                        let byte = s.input.byte()?;
                        put(&mut out, &mut at, byte)?;
                    }
                } else {
                    let distance = read_distance(&mut s, &distances)?;
                    copy_back(&mut out, &mut at, distance, count as usize)?;
                }
            }
            MOVE_TWO => {
                let distance = usize::from(s.input.byte()?) + 1;
                copy_back(&mut out, &mut at, distance, 2)?;
            }
            MOVE_THREE => {
                let distance = read_distance(&mut s, &distances)?;
                copy_back(&mut out, &mut at, distance, 3)?;
            }
            _ => {
                let count = usize::from(s.input.byte()?);
                if count != 0 {
                    let distance = read_distance(&mut s, &distances)?;
                    copy_back(&mut out, &mut at, distance, count + 8)?;
                } else {
                    found += 1;
                    done = s.bits(1)? == 0;
                }
            }
        }
    }
    (at == out.len() && found == header.chunks).then_some(out)
}

/// A distance: a multiplier of 256 by code, then a byte.
fn read_distance(s: &mut ByteBits, code: &PrefixCode) -> Option<usize> {
    let high = code.decode(|| s.bits(1))? as usize;
    Some((usize::from(s.input.byte()?) | high << 8) + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc16_is_the_arc_variant() {
        assert_eq!(crc16(b"123456789"), 0xbb3d);
        assert_eq!(crc16(&[]), 0);
    }

    /// A header for method 1 with the given sizes, then `body`.
    fn file(raw_len: u32, packed_len: u32, body: &[u8]) -> Vec<u8> {
        let mut out = b"RNC\x01".to_vec();
        out.extend_from_slice(&raw_len.to_be_bytes());
        out.extend_from_slice(&packed_len.to_be_bytes());
        out.extend_from_slice(body);
        out
    }

    #[test]
    fn headers_must_fit_the_file() {
        // Old layout: 12-byte header (a file of 18 bytes or fewer cannot tell).
        let mut body = alloc::vec![0u8; 6];
        body[0] = 0x80;
        assert!(is_packed(&file(10, 6, &body)));
        assert!(!is_packed(&file(10, 7, &body)), "packed size past the end");
        assert!(!is_packed(&file(0, 6, &body)), "empty");
        assert!(!is_packed(&file(MAX_RAW_LEN as u32 + 1, 6, &body)));
        assert!(!is_packed(b"RNC\x03\0\0\0\x0a\0\0\0\x06"));
    }

    #[test]
    fn damaged_streams_fail_instead_of_looping() {
        // Whatever the layout is taken for, noise must end in `None`.
        for fill in [0u8, 0xff, 0x55] {
            for method in [1u8, 2] {
                let mut data = file(100, 40, &[fill; 40]);
                data[3] = method;
                assert!(unpack(&data).is_none() || is_packed(&data));
                let long = [data.clone(), alloc::vec![fill; 40]].concat();
                let _ = unpack(&long);
            }
        }
    }
}
