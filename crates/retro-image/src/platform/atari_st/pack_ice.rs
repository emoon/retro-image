//! Pack-Ice depacker (versions 1.x-2.x with an `Ice!`/`ICE!` header).
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

/// Whether `data` starts with a Pack-Ice header.
pub(super) fn is_packed(data: &[u8]) -> bool {
    matches!(data.get(..4), Some(b"Ice!" | b"ICE!"))
}

/// Unpacks a Pack-Ice file with an `Ice!` or `ICE!` header.
pub(super) fn unpack(data: &[u8]) -> Option<Vec<u8>> {
    let version2 = match data.get(..4)? {
        b"Ice!" => false,
        b"ICE!" => true,
        _ => return None,
    };
    let packed_len = be32(data, 4)? as usize;
    let raw_len = be32(data, 8)? as usize;
    if packed_len < 12 || packed_len > data.len() || raw_len == 0 || raw_len > MAX_RAW_LEN {
        return None;
    }
    let stream = &data[12..packed_len];
    if version2 {
        return depack(stream, raw_len, true, true);
    }
    // `Ice!` was used by both bit-stream generations.
    depack(stream, raw_len, false, false).or_else(|| depack(stream, raw_len, true, false))
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

fn depack(stream: &[u8], raw_len: usize, bytes: bool, version2: bool) -> Option<Vec<u8>> {
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
            let count = reader.cascade(&[1, 2, 2, 3, 8, 15])? + 1;
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
    if reader.available_bits() > 0 && reader.read(1)? == 1 {
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
