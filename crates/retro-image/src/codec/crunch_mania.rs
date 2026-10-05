//! Crunch-Mania (`CrM!`, `Crm!`, `CrM2`, `Crm2`), the Amiga cruncher with a
//! standard LZ mode, an LZH mode with Huffman tables per block, and the
//! "sampled" variants that delta-code the data first.
//!
//! The header (tag, raw and packed sizes, the bits stored behind the stream),
//! both bit-stream layouts, their code tables and the delta pass follow
//! Ancient's `CRMDecompressor` and `DLTADecode`
//! (<https://github.com/temisu/ancient>, src/), which are distributed under
//! this license:
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
//! Format background: <http://fileformats.archiveteam.org/wiki/Crunch-Mania>.
//! Checked on the Sembiance `crunchMania` samples, including the four
//! `test_C1` files that hold the same data in each mode. The scene-specific
//! tags Ancient maps to these modes (`DCS!`, `Iron`, `MSS!` and others) are
//! not accepted.

use super::lz::{BackwardOutput, LsbBits, PrefixCode, Ranges, Stream};
use crate::bytes::{be16, be32};
use alloc::vec::Vec;

/// Largest unpacked size accepted.
const MAX_RAW_LEN: usize = 1 << 24;
const HEADER_LEN: usize = 14;
/// The bits behind the stream: 32 bits of content and a 16-bit shift.
const TAIL_LEN: usize = 6;

#[derive(Clone, Copy)]
struct Header {
    /// LZH mode (`CrM2`, `Crm2`) instead of the standard mode.
    lzh: bool,
    /// Delta coded first (`Crm!`, `Crm2`).
    sampled: bool,
    raw_len: usize,
    packed_len: usize,
}

impl Header {
    fn parse(data: &[u8]) -> Option<Self> {
        let (lzh, sampled) = match data.get(..4)? {
            b"CrM!" => (false, false),
            b"Crm!" => (false, true),
            b"CrM2" => (true, false),
            b"Crm2" => (true, true),
            _ => return None,
        };
        let raw_len = be32(data, 6)? as usize;
        let packed_len = be32(data, 10)? as usize;
        let fits = packed_len.checked_add(HEADER_LEN)? <= data.len();
        (fits && packed_len >= TAIL_LEN && raw_len != 0 && raw_len <= MAX_RAW_LEN).then_some(Self {
            lzh,
            sampled,
            raw_len,
            packed_len,
        })
    }
}

/// Whether `data` starts with a Crunch-Mania header whose sizes fit the file.
pub(crate) fn is_packed(data: &[u8]) -> bool {
    Header::parse(data).is_some()
}

/// The stream read backward, bits taken from the low end of its bytes.
struct Reader<'a> {
    input: Stream<'a>,
    bits: LsbBits,
}

impl Reader<'_> {
    fn bits(&mut self, count: u32) -> Option<u32> {
        let input = &mut self.input;
        self.bits
            .read(count, || Some((u32::from(input.back_byte()?), 8)))
    }
}

/// Unpacks a Crunch-Mania file; `None` if it is not one or is damaged.
pub(crate) fn unpack(data: &[u8]) -> Option<Vec<u8>> {
    let header = Header::parse(data)?;
    let tail_at = HEADER_LEN + header.packed_len - TAIL_LEN;
    // Some bits at the end belong to the stream's start: the shift says how
    // many of the 16 low bits are not used, and those before it are.
    let content = be32(data, tail_at)?;
    let shift = u32::from(be16(data, tail_at + 4)?);
    if shift > 16 {
        return None;
    }
    let mut reader = Reader {
        input: Stream::new(&data[HEADER_LEN..tail_at]),
        bits: LsbBits::with(content >> (16 - shift), shift + 16),
    };
    let mut out = BackwardOutput::new(header.raw_len);
    if header.lzh {
        unpack_lzh(&mut reader, &mut out)?;
    } else {
        unpack_standard(&mut reader, &mut out)?;
    }
    if !out.is_full() {
        return None;
    }
    let mut bytes = out.finish();
    if header.sampled {
        let mut sum = 0u8;
        for byte in &mut bytes {
            sum = sum.wrapping_add(*byte);
            *byte = sum;
        }
    }
    Some(bytes)
}

/// A Huffman table of `value_bits`-bit values: a depth, then per depth the
/// number of codes of that length, then the values, codes assigned in order.
fn read_table(reader: &mut Reader, value_bits: u32) -> Option<PrefixCode> {
    let max_depth = reader.bits(4)?;
    if max_depth == 0 {
        return None;
    }
    let counts: Vec<u32> = (0..max_depth)
        .map(|i| reader.bits((i + 1).min(value_bits)))
        .collect::<Option<_>>()?;
    let mut code = PrefixCode::new(&[])?;
    let mut next = 0u32;
    for depth in 1..=max_depth {
        for _ in 0..counts[depth as usize - 1] {
            let value = reader.bits(value_bits)?;
            code.insert(depth, next >> (max_depth - depth), value)?;
            next += 1 << (max_depth - depth);
        }
    }
    Some(code)
}

/// LZH mode: blocks of items, each block with its own length and distance
/// tables; a length code with bit 8 set is a literal.
fn unpack_lzh(reader: &mut Reader, out: &mut BackwardOutput) -> Option<()> {
    loop {
        let lengths = read_table(reader, 9)?;
        let distances = read_table(reader, 4)?;
        let items = reader.bits(16)? + 1;
        for _ in 0..items {
            let code = lengths.decode(|| reader.bits(1))?;
            if code & 0x100 != 0 {
                out.put(code as u8)?;
                continue;
            }
            let distance_bits = distances.decode(|| reader.bits(1))?;
            let distance = if distance_bits == 0 {
                reader.bits(1)? as usize + 1
            } else {
                (reader.bits(distance_bits)? | 1 << distance_bits) as usize + 1
            };
            out.copy(distance, code as usize + 3)?;
        }
        if reader.bits(1)? == 0 {
            return Some(());
        }
    }
}

/// Standard mode: a flag bit per item, a literal byte or a match whose
/// length (23 stands for a run of literals) and distance have prefix codes.
fn unpack_standard(reader: &mut Reader, out: &mut BackwardOutput) -> Option<()> {
    const LENGTHS: Ranges<4> = Ranges::new([(1, false), (2, false), (4, false), (8, false)]);
    const DISTANCES: Ranges<3> = Ranges::new([(5, false), (9, false), (14, false)]);
    let length_code = PrefixCode::new(&[(1, 0, 0), (2, 2, 1), (3, 6, 2), (3, 7, 3)])?;
    let distance_code = PrefixCode::new(&[(1, 0, 1), (2, 2, 0), (2, 3, 2)])?;
    while !out.is_full() {
        if reader.bits(1)? == 1 {
            out.put(reader.bits(8)? as u8)?;
            continue;
        }
        let which = length_code.decode(|| reader.bits(1))?;
        let mut count = LENGTHS.decode(which, |n| reader.bits(n))? + 2;
        if count == 23 {
            let run = if reader.bits(1)? == 1 {
                reader.bits(5)?
            } else {
                reader.bits(14)?
            } as usize
                + 15;
            for _ in 0..run {
                out.put(reader.bits(8)? as u8)?;
            }
        } else {
            if count > 23 {
                count -= 1;
            }
            let which = distance_code.decode(|| reader.bits(1))?;
            let distance = DISTANCES.decode(which, |n| reader.bits(n))?;
            out.copy(distance, count)?;
        }
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A header for `tag` with the given sizes and `body` as the stream and
    /// tail (the tail holds a zero shift).
    fn file(tag: &[u8; 4], raw_len: u32, body: &[u8]) -> Vec<u8> {
        let mut out = tag.to_vec();
        out.extend_from_slice(&[0, 0]);
        out.extend_from_slice(&raw_len.to_be_bytes());
        out.extend_from_slice(&(body.len() as u32).to_be_bytes());
        out.extend_from_slice(body);
        out
    }

    #[test]
    fn headers_must_describe_the_file() {
        let body = [0u8; 8];
        for tag in [b"CrM!", b"Crm!", b"CrM2", b"Crm2"] {
            assert!(is_packed(&file(tag, 10, &body)));
        }
        assert!(!is_packed(&file(b"CrM?", 10, &body)));
        assert!(!is_packed(&file(b"CrM!", 0, &body)), "empty");
        assert!(
            !is_packed(&file(b"CrM!", 10, &body[..5])),
            "no room for the tail"
        );
        let mut short = file(b"CrM!", 10, &body);
        short.pop();
        assert!(!is_packed(&short));
    }

    #[test]
    fn standard_mode_literals_and_the_sampled_pass() {
        // Bits come from the low end: a 1 flag, then 8 bits of a literal.
        // The tail gives 16 + shift bits: with shift 2, the top 18 bits of the
        // 32-bit content (the `>> 14`).
        let literal = |byte: u8| 1 | u32::from(byte) << 1;
        let bits = literal(0x10) | literal(0x05) << 9;
        let mut body = (bits << 14).to_be_bytes().to_vec();
        body.extend_from_slice(&2u16.to_be_bytes());
        // Output is written from the end: the first literal is the last byte.
        let standard = file(b"CrM!", 2, &body);
        assert_eq!(unpack(&standard).as_deref(), Some(&[0x05, 0x10][..]));
        // Crm!: a running sum over the output.
        let sampled = file(b"Crm!", 2, &body);
        assert_eq!(unpack(&sampled).as_deref(), Some(&[0x05, 0x15][..]));
        // A stream that ends before the output is full fails.
        assert_eq!(unpack(&file(b"CrM!", 3, &body)), None);
    }

    #[test]
    fn noise_ends_in_none() {
        for tag in [b"CrM!", b"Crm!", b"CrM2", b"Crm2"] {
            for fill in [0u8, 0xff, 0x6d] {
                let _ = unpack(&file(tag, 500, &[fill; 40]));
            }
        }
    }
}
