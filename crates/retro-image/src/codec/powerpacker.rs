//! PowerPacker (`PP20`) depacker, shared by the Amiga decoders.
//!
//! Format background:
//! <http://fileformats.archiveteam.org/wiki/PowerPacker>. The bit stream
//! layout (offset-width table in the header, 24-bit raw length and start
//! shift in the footer, backward LSB-first words) follows Ancient's
//! `PPDecompressor` (<https://github.com/temisu/ancient>,
//! src/PPDecompressor.cpp), which is distributed under this licence:
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
//! Encrypted files (`PX20`) are not supported.

use crate::bytes::be32;
use alloc::vec::Vec;

/// The densest code is a match run of 7 bytes per 3 bits (about 19 bytes per
/// packed byte), so a raw length beyond this per packed byte is a lie.
const MAX_EXPANSION: usize = 20;

/// Offset widths of the four match classes, selected by the header.
const MODES: [[u8; 4]; 5] = [
    [9, 9, 9, 9],
    [9, 10, 10, 10],
    [9, 10, 11, 11],
    [9, 10, 12, 12],
    [9, 10, 12, 13],
];

/// Header bytes before the packed stream: tag and offset-width table.
const HEADER_LEN: usize = 8;
/// Footer bytes after the stream: raw length (24 bits) and start shift.
const FOOTER_LEN: usize = 4;

/// Whether `data` starts with a plain (unencrypted) PowerPacker header.
pub(crate) fn is_packed(data: &[u8]) -> bool {
    data.len() >= 16 && data.starts_with(b"PP20") && widths(data).is_some()
}

fn widths(data: &[u8]) -> Option<[u8; 4]> {
    let table = data.get(4..8)?;
    MODES.iter().find(|m| *m == table).copied()
}

/// Backward reader: 32-bit big-endian words from the end, bits taken from
/// the least significant end; the first bit of a multi-bit value is its
/// most significant.
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    word: u32,
    left: u32,
}

impl Reader<'_> {
    fn bit(&mut self) -> Option<u32> {
        if self.left == 0 {
            self.pos = self.pos.checked_sub(4)?;
            self.word = be32(self.data, self.pos)?;
            self.left = 32;
        }
        let bit = self.word & 1;
        self.word >>= 1;
        self.left -= 1;
        Some(bit)
    }

    fn read(&mut self, count: u32) -> Option<u32> {
        let mut value = 0;
        for _ in 0..count {
            value = value << 1 | self.bit()?;
        }
        Some(value)
    }

    /// Sums fields of `width` bits until one is below its maximum.
    fn run(&mut self, width: u32, start: usize) -> Option<usize> {
        let mut total = start;
        loop {
            let step = self.read(width)?;
            total += step as usize;
            if step < (1 << width) - 1 {
                return Some(total);
            }
        }
    }
}

/// Unpacks a `PP20` file; `None` if it is not one or is damaged.
pub(crate) fn unpack(data: &[u8]) -> Option<Vec<u8>> {
    if !is_packed(data) {
        return None;
    }
    let widths = widths(data)?;
    let footer = be32(data, data.len() - FOOTER_LEN)?;
    let raw_len = (footer >> 8) as usize;
    let shift = footer & 0xff;
    let stream = &data[HEADER_LEN..data.len() - FOOTER_LEN];
    if raw_len == 0
        || raw_len > data.len() * MAX_EXPANSION
        || shift >= 32
        || !stream.len().is_multiple_of(4)
    {
        return None;
    }
    let mut reader = Reader {
        data: stream,
        pos: stream.len(),
        word: 0,
        left: 0,
    };
    reader.read(shift)?;

    let mut out = alloc::vec![0u8; raw_len];
    let mut dst = raw_len;
    loop {
        if reader.read(1)? == 0 {
            let count = reader.run(2, 1)?;
            if count > dst {
                return None;
            }
            for _ in 0..count {
                dst -= 1;
                out[dst] = reader.read(8)? as u8;
            }
        }
        if dst == 0 {
            break;
        }
        let class = reader.read(2)? as usize;
        let (count, distance) = if class == 3 {
            let width = if reader.read(1)? == 1 {
                u32::from(widths[3])
            } else {
                7
            };
            let distance = reader.read(width)? as usize + 1;
            (reader.run(3, 5)?, distance)
        } else {
            let distance = reader.read(u32::from(widths[class]))? as usize + 1;
            (class + 2, distance)
        };
        if count > dst || dst + distance > raw_len {
            return None;
        }
        for _ in 0..count {
            dst -= 1;
            out[dst] = out[dst + distance];
        }
    }
    Some(out)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Packs `raw` as literal runs only: enough to exercise the framing.
    pub(crate) fn literal_pp20(raw: &[u8], table: [u8; 4]) -> Vec<u8> {
        // Bits are consumed LSB-first, newest word last; build the bit
        // list in reading order, then fill words from the end.
        let mut bits: Vec<bool> = Vec::new();
        let push = |bits: &mut Vec<bool>, value: usize, width: u32| {
            bits.extend((0..width).rev().map(|b| value >> b & 1 == 1));
        };
        // One literal run: 2-bit steps of 3 while more than 3 remain.
        push(&mut bits, 0, 1);
        let mut extra = raw.len() - 1;
        while extra >= 3 {
            push(&mut bits, 3, 2);
            extra -= 3;
        }
        push(&mut bits, extra, 2);
        for &byte in raw.iter().rev() {
            push(&mut bits, usize::from(byte), 8);
        }
        let shift = (32 - bits.len() % 32) % 32;
        let mut padded = alloc::vec![false; shift];
        padded.extend(bits);
        // Words are read from the end of the stream, so the first word read
        // is stored last.
        let mut words: Vec<u32> = padded
            .chunks(32)
            .map(|w| {
                w.iter()
                    .enumerate()
                    .fold(0u32, |acc, (i, &b)| acc | u32::from(b) << i)
            })
            .collect();
        words.reverse();
        let mut out = b"PP20".to_vec();
        out.extend_from_slice(&table);
        for w in words {
            out.extend_from_slice(&w.to_be_bytes());
        }
        out.extend_from_slice(&((raw.len() as u32) << 8 | shift as u32).to_be_bytes());
        out
    }

    #[test]
    fn unpacks_literal_runs() {
        let raw: Vec<u8> = (0..23).map(|i| i * 7 + 1).collect();
        let packed = literal_pp20(&raw, [9, 9, 9, 9]);
        assert!(is_packed(&packed));
        assert_eq!(unpack(&packed).as_deref(), Some(&raw[..]));
    }

    #[test]
    fn rejects_bad_tables_and_oversize_lengths() {
        let raw = [1u8, 2, 3, 4];
        let mut packed = literal_pp20(&raw, [9, 9, 9, 9]);
        packed[4] = 8;
        assert!(!is_packed(&packed));
        let mut packed = literal_pp20(&raw, [9, 9, 9, 9]);
        let n = packed.len();
        packed[n - 4..n - 1].copy_from_slice(&[0xff, 0xff, 0xff]);
        assert_eq!(unpack(&packed), None);
        assert!(unpack(b"PX20\x09\x09\x09\x09\0\0\0\0\0\0\0\0").is_none());
    }

    #[test]
    fn truncated_stream_fails() {
        let raw: Vec<u8> = (0..40).collect();
        let packed = literal_pp20(&raw, [9, 10, 11, 11]);
        let mut cut = packed.clone();
        cut.drain(8..12);
        assert_eq!(unpack(&cut), None);
    }
}
