//! Imploder (File Imploder, `IMP!` and the tags of its relatives), the
//! Amiga executable and data cruncher.
//!
//! The header, the trailer of tables behind the stream, the odd order of the
//! first twelve stream bytes, the code tables and the way match counts,
//! literal runs and distances are read in one intertwined sequence follow
//! Ancient's `IMPDecompressor`
//! (<https://github.com/temisu/ancient>, src/IMPDecompressor.cpp), which is
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
//! Format background: <http://fileformats.archiveteam.org/wiki/Imploder>.
//! Checked on the Sembiance `fileImploder` samples (`IMP!`, `M.H.` and
//! `CHFI` files, and `test_C1.imp`). The checksum behind the tables is not
//! verified, and tags Ancient lists without being sure of them (`RDC9`,
//! `Dupa`, `FLT!`, `PARA`) are not accepted.

use super::lz::{BackwardOutput, MsbBits, PrefixCode};
use crate::bytes::{be16, be32};
use alloc::vec::Vec;

/// Largest unpacked size accepted.
const MAX_RAW_LEN: usize = 1 << 24;
/// Bytes of tables and checksum behind the stream.
const TRAILER_LEN: usize = 0x32;
/// The first stream bytes, which are stored rotated at the stream's end.
const ROTATED: usize = 12;

const TAGS: [&[u8; 4]; 6] = [b"IMP!", b"ATN!", b"EDAM", b"M.H.", b"BDPI", b"CHFI"];

struct Header {
    raw_len: usize,
    /// Offset of the trailer, which is also the length of the stream.
    end: usize,
}

impl Header {
    fn parse(data: &[u8]) -> Option<Self> {
        if !TAGS.iter().any(|t| data.starts_with(&t[..])) {
            return None;
        }
        let raw_len = be32(data, 4)? as usize;
        let end = be32(data, 8)? as usize;
        let fits = end.checked_add(TRAILER_LEN)? <= data.len();
        (end.is_multiple_of(2) && end >= 0xc && fits && raw_len != 0 && raw_len <= MAX_RAW_LEN)
            .then_some(Self { raw_len, end })
    }
}

/// Whether `data` starts with an Imploder header whose sizes fit the file.
pub(crate) fn is_packed(data: &[u8]) -> bool {
    Header::parse(data).is_some()
}

/// The stream, read backward from `end`.
struct Rotated<'a> {
    data: &'a [u8],
    at: usize,
    end: usize,
}

impl Rotated<'_> {
    /// The next byte backward. The first twelve bytes of the stream were
    /// moved behind it, rotated in groups of four.
    fn byte(&mut self) -> Option<u8> {
        self.at = self.at.checked_sub(1)?;
        let source = match self.at {
            i if i >= ROTATED => i,
            i if i < 4 => i + self.end + 8,
            i if i < 8 => i + self.end,
            i => i + self.end - 8,
        };
        self.data.get(source).copied()
    }
}

/// Bits and bytes taken from one stream.
struct Reader<'a> {
    stream: Rotated<'a>,
    bits: MsbBits,
}

impl Reader<'_> {
    fn bits(&mut self, count: u32) -> Option<u32> {
        let stream = &mut self.stream;
        self.bits
            .read(count, || stream.byte().map(|b| (u32::from(b), 8)))
    }
}

/// Unpacks an Imploder file; `None` if it is not one or is damaged.
pub(crate) fn unpack(data: &[u8]) -> Option<Vec<u8>> {
    const LITERAL_LENGTHS: [usize; 4] = [6, 10, 10, 18];
    const LITERAL_BITS: [[u32; 4]; 3] = [[1, 1, 1, 1], [2, 3, 3, 4], [4, 5, 7, 14]];
    let header = Header::parse(data)?;
    let end = header.end;
    let trailer = |at: usize| end + at;
    let distance_values: Vec<[usize; 4]> = (0..2)
        .map(|row| {
            [0, 1, 2, 3].map(|i| be16(data, trailer(18 + (row * 4 + i) * 2)).map_or(0, usize::from))
        })
        .collect();
    let distance_bits: Vec<[u32; 4]> = (0..3)
        .map(|row| {
            [0, 1, 2, 3].map(|i| {
                data.get(trailer(34 + row * 4 + i))
                    .map_or(0, |&b| u32::from(b))
            })
        })
        .collect();

    // Unary codes: up to five ones for the match length, up to two for the
    // two other selectors.
    let lengths = PrefixCode::new(&[
        (1, 0, 0),
        (2, 2, 1),
        (3, 6, 2),
        (4, 14, 3),
        (5, 30, 4),
        (5, 31, 5),
    ])?;
    let selectors = PrefixCode::new(&[(1, 0, 0), (2, 2, 1), (2, 3, 2)])?;

    let mut reader = Reader {
        stream: Rotated { data, at: end, end },
        bits: MsbBits::default(),
    };
    // A stream whose marker byte has no top bit starts one byte lower.
    if data.get(trailer(16))? & 0x80 == 0 {
        reader.stream.at = reader.stream.at.checked_sub(1)?;
    }
    // The anchor bit: the lowest set bit of this byte marks where bits start.
    let half = u32::from(*data.get(trailer(17))?);
    if let Some(anchor) = (0..7).find(|i| half >> i & 1 == 1) {
        reader.bits = MsbBits::with(half >> (anchor + 1), 7 - anchor);
    }

    let mut out = BackwardOutput::new(header.raw_len);
    let mut literals = be32(data, trailer(12))? as usize;
    loop {
        for _ in 0..literals {
            out.put(reader.stream.byte()?)?;
        }
        if out.is_full() {
            return Some(out.finish());
        }
        let first = lengths.decode(|| reader.bits(1))? as usize;
        let selector = first.min(3);
        let mut count = first + 2;
        if count == 6 {
            count += reader.bits(3)? as usize;
        } else if count == 7 {
            count = usize::from(reader.stream.byte()?);
            if count == 0 {
                return None;
            }
        }
        let run = selectors.decode(|| reader.bits(1))? as usize;
        literals = run * 2;
        if literals == 4 {
            literals = LITERAL_LENGTHS[selector];
        }
        literals += reader.bits(LITERAL_BITS[run][selector])? as usize;
        let which = selectors.decode(|| reader.bits(1))? as usize;
        let base = if which == 0 {
            0
        } else {
            distance_values[which - 1][selector]
        };
        let distance = 1 + base + reader.bits(distance_bits[which][selector])? as usize;
        out.copy(distance, count)?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A file with `IMP!` and the given sizes and a zeroed stream and trailer.
    fn file(raw_len: u32, end: u32) -> Vec<u8> {
        let mut out = b"IMP!".to_vec();
        out.extend_from_slice(&raw_len.to_be_bytes());
        out.extend_from_slice(&end.to_be_bytes());
        out.resize(end as usize + TRAILER_LEN, 0);
        out
    }

    #[test]
    fn headers_must_describe_the_file() {
        assert!(is_packed(&file(100, 0x20)));
        assert!(!is_packed(&file(0, 0x20)), "empty");
        assert!(!is_packed(&file(100, 0x21)), "odd end");
        assert!(!is_packed(&file(100, 8)), "end inside the header");
        assert!(!is_packed(&file(MAX_RAW_LEN as u32 + 1, 0x20)));
        let mut truncated = file(100, 0x20);
        truncated.pop();
        assert!(!is_packed(&truncated));
        let mut other = file(100, 0x20);
        other[..4].copy_from_slice(b"IMP?");
        assert!(!is_packed(&other));
    }

    #[test]
    fn noise_in_the_stream_ends_in_none() {
        for fill in [0u8, 0xff, 0x2a] {
            let mut data = file(1000, 0x40);
            data[12..0x40].fill(fill);
            let _ = unpack(&data);
        }
    }
}
