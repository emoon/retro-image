//! Pieces shared by the LZ-style depackers: two-ended byte streams, bit
//! readers, extra-bit range tables and match copying.
//!
//! The mechanisms (a forward and a backward cursor that must not cross,
//! bit readers that refill from words of a given width and take bits from
//! the most or least significant end, range tables of extra bits with
//! optional restarts, overlapping match copies) follow Ancient's
//! `InputStream`, `OutputStream` and `VariableLengthCodeDecoder`
//! (<https://github.com/temisu/ancient>, src/), which are distributed under
//! this licence:
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

use alloc::vec::Vec;

/// Two ends of one buffer: a forward cursor and a backward cursor that must
/// not cross.
pub(super) struct Stream<'a> {
    data: &'a [u8],
    front: usize,
    back: usize,
}

impl<'a> Stream<'a> {
    pub(super) fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            front: 0,
            back: data.len(),
        }
    }

    pub(super) fn byte(&mut self) -> Option<u8> {
        let byte = *self
            .data
            .get(self.front)
            .filter(|_| self.front < self.back)?;
        self.front += 1;
        Some(byte)
    }

    pub(super) fn word(&mut self) -> Option<u32> {
        Some(u32::from(self.byte()?) << 8 | u32::from(self.byte()?))
    }

    /// A 16-bit word with its low byte first.
    pub(super) fn le_word(&mut self) -> Option<u32> {
        let low = u32::from(self.byte()?);
        Some(u32::from(self.byte()?) << 8 | low)
    }

    pub(super) fn long(&mut self) -> Option<u32> {
        Some(self.word()? << 16 | self.word()?)
    }

    /// A byte from the back; the bytes of a big-endian word are met in
    /// reverse.
    pub(super) fn back_byte(&mut self) -> Option<u8> {
        if self.back <= self.front {
            return None;
        }
        self.back -= 1;
        Some(self.data[self.back])
    }

    pub(super) fn back_word(&mut self) -> Option<u32> {
        let low = u32::from(self.back_byte()?);
        Some(u32::from(self.back_byte()?) << 8 | low)
    }
}

/// Bits taken from the most significant end of words fetched on demand.
#[derive(Default)]
pub(super) struct MsbBits {
    content: u32,
    left: u32,
}

impl MsbBits {
    /// A reader that starts with the low `left` bits of `content` still unread.
    pub(super) fn with(content: u32, left: u32) -> Self {
        Self { content, left }
    }

    /// `count` (at most 24) bits; `fetch` supplies a word and its width.
    pub(super) fn read(
        &mut self,
        count: u32,
        mut fetch: impl FnMut() -> Option<(u32, u32)>,
    ) -> Option<u32> {
        let (mut count, mut value) = (count, 0u32);
        while count > 0 {
            if self.left == 0 {
                (self.content, self.left) = fetch()?;
            }
            let take = count.min(self.left);
            self.left -= take;
            value = value << take | (self.content >> self.left) & ((1 << take) - 1);
            count -= take;
        }
        Some(value)
    }
}

/// Bits taken from the least significant end of words fetched on demand.
#[derive(Default)]
pub(super) struct LsbBits {
    content: u32,
    left: u32,
}

impl LsbBits {
    /// A reader that starts with the low `left` bits of `content` still unread.
    pub(super) fn with(content: u32, left: u32) -> Self {
        Self { content, left }
    }

    /// `count` (at most 24) bits; `fetch` supplies a word and its width.
    pub(super) fn read(
        &mut self,
        count: u32,
        mut fetch: impl FnMut() -> Option<(u32, u32)>,
    ) -> Option<u32> {
        let (mut count, mut value, mut shift) = (count, 0u32, 0);
        while count > 0 {
            if self.left == 0 {
                (self.content, self.left) = fetch()?;
            }
            let take = count.min(self.left);
            value |= (self.content & ((1 << take) - 1)) << shift;
            self.content = self.content.checked_shr(take).unwrap_or(0);
            self.left -= take;
            count -= take;
            shift += take;
        }
        Some(value)
    }
}

/// Appends `count` bytes copied from `distance` bytes back, one at a time
/// so that overlapping copies repeat. `None` if that reaches before the start
/// or past the end.
pub(super) fn copy_back(
    out: &mut [u8],
    at: &mut usize,
    distance: usize,
    count: usize,
) -> Option<()> {
    if distance == 0 || distance > *at || count > out.len() - *at {
        return None;
    }
    for _ in 0..count {
        out[*at] = out[*at - distance];
        *at += 1;
    }
    Some(())
}

/// An output filled from its last byte to its first, as the depackers that
/// read their input backward write it.
pub(super) struct BackwardOutput {
    out: Vec<u8>,
    at: usize,
}

impl BackwardOutput {
    /// An output of `len` bytes, still to be written.
    pub(super) fn new(len: usize) -> Self {
        Self {
            out: alloc::vec![0; len],
            at: len,
        }
    }

    pub(super) fn is_full(&self) -> bool {
        self.at == 0
    }

    pub(super) fn put(&mut self, byte: u8) -> Option<()> {
        self.at = self.at.checked_sub(1)?;
        self.out[self.at] = byte;
        Some(())
    }

    /// `count` bytes copied from `distance` bytes behind the write position
    /// (that is, later in the output), one at a time so that overlaps repeat.
    pub(super) fn copy(&mut self, distance: usize, count: usize) -> Option<()> {
        if distance == 0 || count > self.at || self.at + distance > self.out.len() {
            return None;
        }
        for _ in 0..count {
            self.at -= 1;
            self.out[self.at] = self.out[self.at + distance];
        }
        Some(())
    }

    pub(super) fn finish(self) -> Vec<u8> {
        self.out
    }
}

pub(super) fn put(out: &mut [u8], at: &mut usize, byte: u8) -> Option<()> {
    *out.get_mut(*at)? = byte;
    *at += 1;
    Some(())
}

/// A prefix code, walked one bit at a time with the first bit read being the
/// most significant of a code.
pub(super) struct PrefixCode {
    nodes: Vec<Node>,
}

#[derive(Default)]
struct Node {
    children: [usize; 2],
    value: Option<u32>,
}

impl PrefixCode {
    /// A code from `(length, code, value)` entries: the low `length` bits
    /// of `code`, first bit highest. `None` if one code begins another.
    pub(super) fn new(entries: &[(u32, u32, u32)]) -> Option<Self> {
        let mut code = Self {
            nodes: alloc::vec![Node::default()],
        };
        for &(length, bits, value) in entries {
            code.insert(length, bits, value)?;
        }
        Some(code)
    }

    /// The code that assigns equal-length codes in order of value and the
    /// shorter codes first (as Deflate does), for values `0..lengths.len()`
    /// with the given code lengths; 0 leaves a value out. `None` if there
    /// are no codes or the lengths oversubscribe the code space.
    pub(super) fn canonical(lengths: &[u8]) -> Option<Self> {
        let max = u32::from(*lengths.iter().max()?);
        if max == 0 || max > 31 {
            return None;
        }
        let mut code = Self::new(&[])?;
        let mut next = 0u32;
        for depth in 1..=max {
            for (value, _) in lengths
                .iter()
                .enumerate()
                .filter(|&(_, &l)| u32::from(l) == depth)
            {
                code.insert(depth, next >> (max - depth), value as u32)?;
                next += 1 << (max - depth);
            }
        }
        Some(code)
    }

    /// Adds a code like those of [`PrefixCode::new`]; `None` on a conflict.
    pub(super) fn insert(&mut self, length: u32, bits: u32, value: u32) -> Option<()> {
        let mut at = 0;
        for shift in (0..length).rev() {
            if self.nodes[at].value.is_some() {
                return None;
            }
            let bit = (bits >> shift & 1) as usize;
            at = match self.nodes[at].children[bit] {
                0 => {
                    self.nodes.push(Node::default());
                    let child = self.nodes.len() - 1;
                    self.nodes[at].children[bit] = child;
                    child
                }
                child => child,
            };
        }
        let node = &mut self.nodes[at];
        if node.value.is_some() || node.children != [0, 0] {
            return None;
        }
        node.value = Some(value);
        Some(())
    }

    /// The value of the code whose bits `bit` supplies, `None` if they run
    /// out or follow no code.
    pub(super) fn decode(&self, mut bit: impl FnMut() -> Option<u32>) -> Option<u32> {
        let mut at = 0;
        loop {
            if let Some(value) = self.nodes[at].value {
                return Some(value);
            }
            at = self.nodes[at].children[usize::from(bit()? & 1 == 1)];
            if at == 0 {
                return None;
            }
        }
    }
}

/// A forward stream whose bit reads refill from single bytes, so bits and
/// whole bytes share one cursor.
pub(super) struct ByteBits<'a> {
    pub(super) input: Stream<'a>,
    bits: MsbBits,
}

impl<'a> ByteBits<'a> {
    pub(super) fn new(data: &'a [u8]) -> Self {
        Self {
            input: Stream::new(data),
            bits: MsbBits::default(),
        }
    }

    pub(super) fn bits(&mut self, count: u32) -> Option<u32> {
        let input = &mut self.input;
        self.bits
            .read(count, || Some((u32::from(input.byte()?), 8)))
    }

    /// Counts the ones that follow in a run ended by a zero, starting from
    /// `count`; `limit` ones in all are an error.
    pub(super) fn ones(&mut self, mut count: u32, limit: u32) -> Option<u32> {
        while self.bits(1)? == 1 {
            count += 1;
            if count >= limit {
                return None;
            }
        }
        Some(count)
    }
}

/// A table of extra-bit counts: entry `i` adds `bits[i]` bits to the base
/// where the previous entries' ranges end. `reset` entries restart the base
/// at 0 and say how wide the range they begin is.
pub(super) struct Ranges<const N: usize> {
    bits: [u32; N],
    base: [u32; N],
}

impl<const N: usize> Ranges<N> {
    /// `entries` are `(bits, restart)`: `restart` starts the base over.
    pub(super) const fn new(entries: [(u32, bool); N]) -> Self {
        let mut bits = [0; N];
        let mut base = [0; N];
        let mut next = 0;
        let mut i = 0;
        while i < N {
            let (width, restart) = entries[i];
            if restart {
                next = 0;
            }
            bits[i] = width;
            base[i] = next;
            next += 1 << width;
            i += 1;
        }
        Self { bits, base }
    }

    /// Reads the entries' fields in turn until one is below its largest
    /// value (a run of maximal fields means "more"); the value is that
    /// entry's base, minus its index, plus the field.
    pub(super) fn cascade(&self, mut read: impl FnMut(u32) -> Option<u32>) -> Option<usize> {
        for i in 0..N {
            let width = self.bits[i];
            if width == 0 {
                return None;
            }
            let field = read(width)?;
            if i == N - 1 || field != (1 << width) - 1 {
                return Some((self.base[i] + field) as usize - i);
            }
        }
        None
    }

    /// The value for entry `index`, reading its extra bits with `read`.
    pub(super) fn decode(
        &self,
        index: u32,
        read: impl FnOnce(u32) -> Option<u32>,
    ) -> Option<usize> {
        let i = index as usize;
        Some((self.base.get(i)? + read(*self.bits.get(i)?)?) as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_bits(bits: &str) -> impl FnMut() -> Option<u32> + '_ {
        let mut chars = bits.chars();
        move || chars.next().map(|c| u32::from(c == '1'))
    }

    #[test]
    fn prefix_codes_decode_and_reject_conflicts() {
        let code = PrefixCode::new(&[(1, 0, 7), (2, 2, 8), (2, 3, 9)]).unwrap();
        assert_eq!(code.decode(read_bits("0")), Some(7));
        assert_eq!(code.decode(read_bits("10")), Some(8));
        assert_eq!(code.decode(read_bits("11")), Some(9));
        assert_eq!(code.decode(read_bits("1")), None, "bits run out");
        // A code that begins another, either way round, and a duplicate.
        assert!(PrefixCode::new(&[(1, 0, 1), (2, 1, 2)]).is_none());
        assert!(PrefixCode::new(&[(2, 1, 2), (1, 0, 1)]).is_none());
        assert!(PrefixCode::new(&[(2, 1, 1), (2, 1, 2)]).is_none());
    }

    #[test]
    fn canonical_codes_order_by_length_then_value() {
        // Lengths 2, 1, 3, 3: value 1 gets "0", value 0 "10", values 2 and 3
        // "110" and "111".
        let code = PrefixCode::canonical(&[2, 1, 3, 3]).unwrap();
        assert_eq!(code.decode(read_bits("0")), Some(1));
        assert_eq!(code.decode(read_bits("10")), Some(0));
        assert_eq!(code.decode(read_bits("110")), Some(2));
        assert_eq!(code.decode(read_bits("111")), Some(3));
        assert!(PrefixCode::canonical(&[]).is_none());
        assert!(PrefixCode::canonical(&[0, 0]).is_none());
        assert!(
            PrefixCode::canonical(&[1, 1, 1]).is_none(),
            "oversubscribed"
        );
    }

    #[test]
    fn cascades_add_fields_until_one_is_not_all_ones() {
        // Widths 1, 2, 3: values 0 and 1 by the first field alone, then
        // 2-4 after a full first field, and so on.
        const TABLE: Ranges<3> = Ranges::new([(1, false), (2, false), (3, false)]);
        let mut fields = [1u32, 3, 5].into_iter();
        assert_eq!(TABLE.cascade(|_| fields.next()), Some(6 + 5 - 2));
        let mut fields = [0u32].into_iter();
        assert_eq!(TABLE.cascade(|_| fields.next()), Some(0));
        let mut fields = [1u32, 2].into_iter();
        assert_eq!(TABLE.cascade(|_| fields.next()), Some(2 + 2 - 1));
    }

    #[test]
    fn backward_output_fills_from_the_end_and_repeats_overlaps() {
        let mut out = BackwardOutput::new(5);
        out.put(b'e').unwrap();
        out.put(b'd').unwrap();
        // Copy 3 bytes from 2 behind the write position: e, d, e.
        out.copy(2, 3).unwrap();
        assert!(out.is_full());
        assert_eq!(out.finish(), [b'e', b'd', b'e', b'd', b'e']);
        let mut out = BackwardOutput::new(4);
        assert!(out.copy(1, 1).is_none(), "nothing behind the first byte");
        out.put(1).unwrap();
        assert!(out.copy(0, 1).is_none());
        assert!(out.copy(1, 4).is_none(), "more than is left");
    }
}
