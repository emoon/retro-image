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

pub(super) fn put(out: &mut [u8], at: &mut usize, byte: u8) -> Option<()> {
    *out.get_mut(*at)? = byte;
    *at += 1;
    Some(())
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
