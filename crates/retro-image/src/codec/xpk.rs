//! XPK (`XPKF`) container and its RLEN, FAST, MASH and NUKE sub-packers,
//! shared by the Amiga decoders (SVG, IFF-RGFX, YAFA).
//!
//! xpkmaster.library wrapped a file in a stream of chunks, each packed by a
//! sub-packer library named in the header. Pictures packed this way carry
//! no other mark of it: the stream replaces the bitmap bytes.
//!
//! The stream header (36 bytes, 38 plus an extra block with flag bit 2),
//! the chunk header (type, checksum, packed and unpacked sizes; 8 bytes, or
//! 12 with flag bit 0), the 4-byte chunk padding, and the four sub-packer
//! bit streams follow Ancient's `XPKMain`, `RLENDecompressor`,
//! `FASTDecompressor`, `MASHDecompressor` and `NUKEDecompressor`
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
//! Which packers files use, and the header values, were read off the
//! samples: SVG and IFF-RGFX pictures (Sembiance's `image/sgx`,
//! `image/rgfx`) and YAFA animations (`video/iffYAFA`), packed with RLEN,
//! MASH, NUKE and FAST. Password-protected streams and the other
//! sub-packers are not supported.

use super::lz::{ByteBits, LsbBits, MsbBits, Ranges, Stream, copy_back, put};
use crate::bytes::{be16, be32};
use alloc::vec::Vec;

/// Bytes before the first chunk without an extra header block.
const HEADER_LEN: usize = 36;
/// Chunk type: stored as is.
const CHUNK_RAW: u8 = 0;
/// Chunk type: packed by the stream's sub-packer.
const CHUNK_PACKED: u8 = 1;

/// The sub-packers we can undo, as named in the stream header.
type Unpacker = fn(packed: &[u8], out: &mut [u8]) -> Option<()>;

fn unpacker(id: &[u8]) -> Option<Unpacker> {
    Some(match id {
        b"RLEN" => rlen,
        b"FAST" => fast,
        b"MASH" => mash,
        b"NUKE" => nuke,
        _ => return None,
    })
}

/// The fields of the stream header the walk needs.
struct Header {
    unpacker: Unpacker,
    /// End of the stream: the data after it is not ours.
    end: usize,
    raw_len: usize,
    long_chunk_headers: bool,
    first_chunk: usize,
}

impl Header {
    fn parse(data: &[u8]) -> Option<Self> {
        if !data.starts_with(b"XPKF") {
            return None;
        }
        let end = (be32(data, 4)? as usize).checked_add(8)?;
        let raw_len = be32(data, 12)? as usize;
        let flags = *data.get(32)?;
        let first_chunk = if flags & 4 != 0 {
            38 + usize::from(be16(data, 36)?)
        } else {
            HEADER_LEN
        };
        // A password-protected stream cannot be read, and an empty one is no picture.
        if flags & 2 != 0 || raw_len == 0 || end > data.len() || first_chunk >= end {
            return None;
        }
        Some(Self {
            unpacker: unpacker(data.get(8..12)?)?,
            end,
            raw_len,
            long_chunk_headers: flags & 1 != 0,
            first_chunk,
        })
    }
}

/// Whether `data` starts with an XPK stream we can unpack.
pub(crate) fn is_packed(data: &[u8]) -> bool {
    Header::parse(data).is_some()
}

/// The most a chunk can expand: MASH reaches about 11,000 times on a run of
/// one byte, the other packers less. A chunk claiming more is damaged or
/// meant to force a large allocation.
const MAX_EXPANSION: usize = 1 << 14;

fn expands_plausibly(packed: usize, raw: usize) -> bool {
    raw <= packed.saturating_mul(MAX_EXPANSION)
}

/// Unpacks an XPK stream whose declared unpacked size is at most `limit`
/// bytes (the caller knows how much it can use). `None` if it is not a
/// stream we support, is damaged, or declares more than `limit`.
pub(crate) fn unpack(data: &[u8], limit: usize) -> Option<Vec<u8>> {
    let header = Header::parse(data)?;
    if header.raw_len > limit {
        return None;
    }
    let chunk_header_len = if header.long_chunk_headers { 12 } else { 8 };
    let mut out = Vec::new();
    let mut at = header.first_chunk;
    while out.len() < header.raw_len {
        let kind = *data.get(at)?;
        let (packed_len, raw_len) = if header.long_chunk_headers {
            (be32(data, at + 4)? as usize, be32(data, at + 8)? as usize)
        } else {
            (
                usize::from(be16(data, at + 4)?),
                usize::from(be16(data, at + 6)?),
            )
        };
        let start = at.checked_add(chunk_header_len)?;
        let body = data.get(start..start.checked_add(packed_len)?.min(header.end))?;
        if body.len() != packed_len || raw_len > header.raw_len - out.len() {
            return None;
        }
        let filled = out.len();
        match kind {
            CHUNK_RAW if raw_len == packed_len => out.extend_from_slice(body),
            CHUNK_PACKED if expands_plausibly(packed_len, raw_len) => {
                out.resize(filled + raw_len, 0);
                (header.unpacker)(body, &mut out[filled..])?;
            }
            _ => return None,
        }
        // Chunk data is padded to a multiple of 4 bytes.
        at = start.checked_add(packed_len.checked_add(3)? & !3)?;
    }
    Some(out)
}

/// Run-length coding: a count below 128 of literal bytes, otherwise `256 -
/// count` repeats of the next byte.
fn rlen(packed: &[u8], out: &mut [u8]) -> Option<()> {
    let mut input = Stream::new(packed);
    let mut at = 0;
    while at < out.len() {
        let count = input.byte()?;
        if count == 0 {
            return None;
        }
        if count < 128 {
            for _ in 0..count {
                put(out, &mut at, input.byte()?)?;
            }
        } else {
            let byte = input.byte()?;
            for _ in 0..256 - usize::from(count) {
                put(out, &mut at, byte)?;
            }
        }
    }
    Some(())
}

/// Literal bytes from the front, the copy flags and 16-bit match words from
/// the back: a flag of 0 is a literal, 1 a match of `18 - (low nibble)` bytes
/// at the distance in the upper 12 bits.
fn fast(packed: &[u8], out: &mut [u8]) -> Option<()> {
    let mut input = Stream::new(packed);
    let mut flags = MsbBits::default();
    let mut at = 0;
    while at < out.len() {
        let flag = flags.read(1, || Some((input.back_word()?, 16)))?;
        if flag == 0 {
            let byte = input.byte()?;
            put(out, &mut at, byte)?;
        } else {
            let word = input.back_word()? as usize;
            let count = (18 - (word & 15)).min(out.len() - at);
            copy_back(out, &mut at, word >> 4, count)?;
        }
    }
    Some(())
}

/// LZRW-style: bytes are read from the front for both literals and bit
/// refills; a unary code gives the literal run, then comes a match.
fn mash(packed: &[u8], out: &mut [u8]) -> Option<()> {
    const DISTANCES: Ranges<8> = Ranges::new([
        (5, false),
        (7, false),
        (9, false),
        (10, false),
        (11, false),
        (12, false),
        (13, false),
        (14, false),
    ]);
    let mut stream = ByteBits::new(packed);
    let mut at = 0;
    while at < out.len() {
        // Literal run: 0 to 5 bytes by a unary code, or a longer run.
        let mut run = 0;
        while run < 6 && stream.bits(1)? == 1 {
            run += 1;
        }
        let mut run = run as usize;
        if run == 6 {
            let width = stream.ones(1, 17)?;
            run = stream.bits(width)? as usize + (1 << width) + 4;
        }
        for _ in 0..run {
            let byte = stream.input.byte()?;
            put(out, &mut at, byte)?;
        }

        let (count, distance) = if stream.bits(1)? == 1 {
            let width = stream.ones(1, 16)?;
            let count = stream.bits(width)? as usize + (1 << width) + 2;
            let selector = stream.bits(3)?;
            (count, DISTANCES.decode(selector, |n| stream.bits(n))?)
        } else if stream.bits(1)? == 1 {
            let selector = stream.bits(3)?;
            (3, DISTANCES.decode(selector, |n| stream.bits(n))?)
        } else {
            (2, stream.bits(9)? as usize)
        };
        // Streams end with a match of distance 0 after the last literal.
        if distance == 0 && at == out.len() {
            break;
        }
        let count = count.min(out.len() - at);
        copy_back(out, &mut at, distance, count)?;
    }
    Some(())
}

/// NUKE's four bit readers, which take words from the same front cursor as
/// they run empty, and its literal bytes read from the back.
struct NukeBits<'a> {
    input: Stream<'a>,
    one: MsbBits,
    two: MsbBits,
    four: LsbBits,
    any: MsbBits,
}

impl NukeBits<'_> {
    fn bit(&mut self) -> Option<u32> {
        let input = &mut self.input;
        self.one.read(1, || Some((input.word()?, 16)))
    }

    fn bits2(&mut self) -> Option<u32> {
        let input = &mut self.input;
        self.two.read(2, || Some((input.word()?, 16)))
    }

    fn bits4(&mut self) -> Option<u32> {
        let input = &mut self.input;
        self.four.read(4, || Some((input.long()?, 32)))
    }

    fn bits(&mut self, count: u32) -> Option<u32> {
        let input = &mut self.input;
        self.any.read(count, || Some((input.word()?, 16)))
    }
}

/// LZ77 with separate streams: literal bytes read backward from the end,
/// and four bit readers (1, 2, 4 and variable width) taking words from the
/// front.
fn nuke(packed: &[u8], out: &mut [u8]) -> Option<()> {
    const DISTANCES: Ranges<16> = Ranges::new([
        (4, false),
        (6, false),
        (8, false),
        (9, false),
        (4, true),
        (7, false),
        (9, false),
        (11, false),
        (13, false),
        (14, false),
        (5, true),
        (7, false),
        (9, false),
        (11, false),
        (13, false),
        (14, false),
    ]);
    let mut stream = NukeBits {
        input: Stream::new(packed),
        one: MsbBits::default(),
        two: MsbBits::default(),
        four: LsbBits::default(),
        any: MsbBits::default(),
    };
    let mut at = 0;
    loop {
        if stream.bit()? == 0 {
            let mut run = 0;
            if stream.bit()? == 1 {
                run = 1;
            } else {
                loop {
                    let step = stream.bits2()?;
                    run += if step == 0 { 3 } else { 5 - step };
                    if step != 0 {
                        break;
                    }
                }
            }
            for _ in 0..run {
                let byte = stream.input.back_byte()?;
                put(out, &mut at, byte)?;
            }
        }
        if at == out.len() {
            return Some(());
        }
        let index = stream.bits4()?;
        let distance = DISTANCES.decode(index, |n| stream.bits(n))?;
        let count = match index {
            0..4 => 2,
            4..10 => 3,
            _ => match stream.bits2()? {
                0 => {
                    let mut count = 6;
                    loop {
                        let step = stream.bits4()?;
                        count += if step == 0 { 15 } else { 16 - step };
                        if step != 0 {
                            break;
                        }
                    }
                    count
                }
                short => 7 - short,
            },
        };
        copy_back(out, &mut at, distance, count as usize)?;
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A stream of one chunk list; each chunk is `(type, packed, raw_len)`.
    pub(crate) fn stream(packer: &[u8; 4], raw_len: u32, chunks: &[(u8, &[u8], u16)]) -> Vec<u8> {
        let mut body = Vec::new();
        for &(kind, data, raw) in chunks {
            body.extend_from_slice(&[kind, 0, 0, 0]);
            body.extend_from_slice(&(data.len() as u16).to_be_bytes());
            body.extend_from_slice(&raw.to_be_bytes());
            body.extend_from_slice(data);
            body.resize(body.len().next_multiple_of(4), 0);
        }
        let mut file = b"XPKF".to_vec();
        file.extend_from_slice(&[0; 4]);
        file.extend_from_slice(packer);
        file.extend_from_slice(&raw_len.to_be_bytes());
        file.resize(HEADER_LEN, 0);
        file.extend_from_slice(&body);
        let packed_len = (file.len() - 8) as u32;
        file[4..8].copy_from_slice(&packed_len.to_be_bytes());
        file
    }

    #[test]
    fn walks_raw_and_packed_chunks() {
        // "abc" raw, then four repeats of 'z' by RLEN (count 252 = 4 repeats).
        let file = stream(
            b"RLEN",
            7,
            &[(CHUNK_RAW, b"abc", 3), (CHUNK_PACKED, &[252, b'z'], 4)],
        );
        assert!(is_packed(&file));
        assert_eq!(unpack(&file, 7).as_deref(), Some(&b"abczzzz"[..]));
    }

    #[test]
    fn rlen_literals_and_repeats() {
        let mut out = [0; 5];
        assert_eq!(rlen(&[2, 1, 2, 253, 9], &mut out), Some(()));
        assert_eq!(out, [1, 2, 9, 9, 9]);
        // Count 0 is an error, and so is output that overruns.
        assert_eq!(rlen(&[0, 1], &mut out), None);
        assert_eq!(rlen(&[2, 1, 2, 250, 9], &mut out), None);
    }

    #[test]
    fn fast_copies_matches_from_the_back_streams() {
        // Flags 0, 0, 1 in the last word; the match word: distance 2, 4 bytes.
        let packed = [b'a', b'b', 0x00, 0x2e, 0x20, 0x00];
        let mut out = [0; 6];
        assert_eq!(fast(&packed, &mut out), Some(()));
        assert_eq!(&out, b"ababab");
        // A distance reaching before the output is rejected.
        let packed = [b'a', b'b', 0x00, 0x5e, 0x20, 0x00];
        assert_eq!(fast(&packed, &mut out), None);
    }

    #[test]
    fn mash_decodes_a_literal_run_and_a_match() {
        // The first byte holds the bits: run code 110 (two literals), 0 and 1
        // (a 3-byte match), selector 000. The literals follow it, then a
        // second bit byte starts with the 5-bit distance 00010.
        let packed = [0b1100_1000, b'a', b'b', 0b0001_0000];
        let mut out = [0; 5];
        assert_eq!(mash(&packed, &mut out), Some(()));
        assert_eq!(&out, b"ababa");
    }

    #[test]
    fn nuke_decodes_a_literal_and_a_match() {
        // Bits come from words fetched in the order they are needed: the
        // 1-bit reader's (0 = literals follow, 1 = one literal, then 1 = no
        // more literals: 0b011 followed by zeros), the 4-bit reader's long
        // (distance code 0, a 2-byte match) and the variable reader's word
        // (distance 1 in its top nibble). The literal byte is read backward
        // from the end.
        let packed = [0x60, 0x00, 0, 0, 0, 0, 0x10, 0x00, b'a'];
        let mut out = [0; 3];
        assert_eq!(nuke(&packed, &mut out), Some(()));
        assert_eq!(&out, b"aaa");
        // Asking for more than the stream makes fails.
        let mut longer = [0; 5];
        assert_eq!(nuke(&packed, &mut longer), None);
    }

    #[test]
    fn nuke_literal_run_only() {
        // One literal flag-less run is not enough to fill the output, so
        // it must fail rather than loop or panic.
        let mut out = [0; 4];
        assert_eq!(nuke(&[0; 16], &mut out), None);
    }

    /// A stream with long chunk headers: one packed chunk of `raw_len`
    /// bytes whose body is `body`, declaring `raw_len` for the whole.
    fn long_stream(packer: &[u8; 4], raw_len: u32, body: &[u8]) -> Vec<u8> {
        let mut file = b"XPKF".to_vec();
        file.extend_from_slice(&[0; 4]);
        file.extend_from_slice(packer);
        file.extend_from_slice(&raw_len.to_be_bytes());
        file.resize(32, 0);
        file.extend_from_slice(&[1, 0, 0, 0]); // flags: long chunk headers
        file.push(CHUNK_PACKED);
        file.extend_from_slice(&[0, 0, 0]);
        file.extend_from_slice(&(body.len() as u32).to_be_bytes());
        file.extend_from_slice(&raw_len.to_be_bytes());
        file.extend_from_slice(body);
        file.resize(file.len().next_multiple_of(4), 0);
        let packed_len = (file.len() - 8) as u32;
        file[4..8].copy_from_slice(&packed_len.to_be_bytes());
        file
    }

    #[test]
    fn chunks_cannot_claim_more_than_a_packer_can_produce() {
        // Two bytes of RLEN cannot make 1 GiB; this used to be allocated and
        // zero-filled before the packer ran.
        let bomb = long_stream(b"RLEN", 0x4000_0000, &[0, 0]);
        assert_eq!(unpack(&bomb, usize::MAX), None);
        assert!(expands_plausibly(2, 2 << 14));
        assert!(!expands_plausibly(2, (2 << 14) + 1));
        assert!(!expands_plausibly(0, 1));
        // A chunk of real RLEN data still unpacks (127 repeats of one byte).
        let fine = long_stream(b"RLEN", 128, &[129, 7, 255, 7]);
        assert_eq!(unpack(&fine, 128).map(|v| v.len()), Some(128));
    }

    #[test]
    fn rejects_bad_streams() {
        let file = stream(b"RLEN", 3, &[(CHUNK_RAW, b"abc", 3)]);
        assert_eq!(unpack(&file, 2), None, "over the caller's limit");
        assert_eq!(unpack(&file[..40], 3), None, "truncated");
        let mut password = file.clone();
        password[32] = 2;
        assert!(!is_packed(&password));
        let mut unknown = file.clone();
        unknown[8..12].copy_from_slice(b"XXXX");
        assert!(!is_packed(&unknown));
        // A chunk claiming more output than the stream declares.
        let big = stream(b"RLEN", 3, &[(CHUNK_RAW, b"abcd", 4)]);
        assert_eq!(unpack(&big, 3), None);
        // A raw chunk whose sizes disagree.
        let skewed = stream(b"RLEN", 3, &[(CHUNK_RAW, b"abc", 2)]);
        assert_eq!(unpack(&skewed, 3), None);
    }
}
