//! DreamGrafix pictures (ProDOS PNT/$8005): a 256-colour or 3200-colour
//! Super Hi-Res screen compressed with a 12-bit LZW variant, followed by a
//! 17-byte footer.
//!
//! Sources:
//! - Footer, the two unpacked layouts and the colour modes: CiderPress II
//!   Super Hi-Res notes, "PNT/$8005 and PIC/$8003: DreamGrafix Image"
//!   (<https://ciderpress2.com/formatdoc/SuperHiRes-notes.html>).
//! - LZW: read from CiderPress II `FileConv/Gfx/SuperHiRes_DreamGrafix.cs`
//!   (<https://github.com/fadden/CiderPress2>), which is based on code by
//!   DreamGrafix co-author Jason Andersen. Codes are packed least
//!   significant bit first and start at 9 bits; 256 clears the table, 257
//!   ends the data, 258 is the first free code. The width grows by one bit
//!   when the next free code reaches 2^width, up to 12 bits, after which no
//!   more codes are added. The code after a clear is a literal byte.
//! - The pictures in the 3200-colour layout go through the Brooks renderer
//!   (`super_hires::render_3200`); the 256-colour layout is the 32 KB
//!   screen dump.
//! - Checked against the 10 DreamGrafix files in
//!   `corpus/extra/next-amiga-pc/apple2gs-dreamgrafix`, by eye; every
//!   file must unpack to exactly the size its colour mode calls for.
//!
//! The licence of the CiderPress II source:
//!
//! ```text
//! Copyright 2023 faddenSoft
//!
//! Licensed under the Apache License, Version 2.0 (the "License");
//! you may not use this file except in compliance with the License.
//! You may obtain a copy of the License at
//!
//!     http://www.apache.org/licenses/LICENSE-2.0
//!
//! Unless required by applicable law or agreed to in writing, software
//! distributed under the License is distributed on an "AS IS" BASIS,
//! WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//! See the License for the specific language governing permissions and
//! limitations under the License.
//! ```

use alloc::vec::Vec;

use super::super_hires;
use crate::bytes::le16;
use crate::{DecodeError, Image};

const FOOTER_LEN: usize = 17;
/// Length byte and name.
const SIGNATURE: &[u8; 11] = b"\x0aDreamWorld";
const PIXELS_LEN: usize = 32000;
const SCREEN_LEN: usize = 0x8000;
const COLORS_3200_LEN: usize = 200 * 32;

/// Unpacked sizes: pixels, SCBs and palettes plus 512 spare bytes (256
/// colours), or pixels, 200 palettes and 512 spare bytes (3200 colours).
const SIZE_256: usize = PIXELS_LEN + 256 + 512 + 512;
const SIZE_3200: usize = PIXELS_LEN + COLORS_3200_LEN + 512;

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let footer_at = data.len().checked_sub(FOOTER_LEN).ok_or(fail)?;
    let (packed, footer) = data.split_at(footer_at);
    if &footer[6..] != SIGNATURE || le16(footer, 2) != Some(200) || le16(footer, 4) != Some(320) {
        return Err(fail);
    }
    match le16(footer, 0).ok_or(fail)? {
        0 => {
            let screen = unpack(packed, SIZE_256).ok_or(fail)?;
            super_hires::decode_screen(&screen[..SCREEN_LEN])
        }
        1 => {
            let screen = unpack(packed, SIZE_3200).ok_or(fail)?;
            super_hires::render_3200(
                &screen[..PIXELS_LEN],
                &screen[PIXELS_LEN..][..COLORS_3200_LEN],
            )
        }
        _ => Err(fail),
    }
}

const CLEAR: u16 = 256;
const END: u16 = 257;
const FIRST_FREE: u16 = 258;
const MIN_BITS: u32 = 9;
const MAX_BITS: u32 = 12;
const TABLE_LEN: usize = 1 << MAX_BITS;

/// Unpacks LZW data that must produce exactly `len` bytes by its end code.
fn unpack(packed: &[u8], len: usize) -> Option<Vec<u8>> {
    // Every code above 255 is an earlier code plus one byte.
    let mut prefix = [0u16; TABLE_LEN];
    let mut suffix = [0u8; TABLE_LEN];
    let mut out = Vec::with_capacity(len);
    let mut bit = 0usize;
    let mut bits = MIN_BITS;
    let mut free = FIRST_FREE;
    // The previous code and the first byte of its string.
    let mut previous: Option<(u16, u8)> = None;
    // Strings come out back to front: a string is at most one byte per code.
    let mut stack = Vec::with_capacity(TABLE_LEN);
    loop {
        let code = read_code(packed, bit, bits)?;
        bit += bits as usize;
        match code {
            END => break,
            CLEAR => {
                bits = MIN_BITS;
                free = FIRST_FREE;
                let literal = read_code(packed, bit, bits)?;
                bit += bits as usize;
                let byte = u8::try_from(literal).ok()?;
                out.push(byte);
                previous = Some((literal, byte));
                continue;
            }
            _ => {}
        }
        let (old, first) = previous?;
        // A code not in the table yet is the previous string plus its own
        // first byte.
        let known = code < free;
        let mut at = if known { code } else { old };
        if !known {
            if code != free {
                return None;
            }
            stack.push(first);
        }
        while at >= CLEAR {
            if at >= free {
                return None;
            }
            stack.push(suffix[usize::from(at)]);
            at = prefix[usize::from(at)];
        }
        let first = u8::try_from(at).ok()?;
        out.push(first);
        out.extend(stack.drain(..).rev());
        if out.len() > len {
            return None;
        }
        if usize::from(free) < TABLE_LEN {
            prefix[usize::from(free)] = old;
            suffix[usize::from(free)] = first;
            free += 1;
            if u32::from(free) == 1 << bits && bits < MAX_BITS {
                bits += 1;
            }
        }
        previous = Some((code, first));
    }
    (out.len() == len).then_some(out)
}

/// The `bits`-bit code at bit offset `at`, least significant bit first, or
/// `None` if it runs past the end of `packed`.
fn read_code(packed: &[u8], at: usize, bits: u32) -> Option<u16> {
    if at + bits as usize > packed.len() * 8 {
        return None;
    }
    let word = (0..3).fold(0u32, |word, i| {
        word | u32::from(packed.get(at / 8 + i).copied().unwrap_or(0)) << (8 * i)
    });
    u16::try_from(word >> (at % 8) & ((1 << bits) - 1)).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Packs codes LSB first at the given widths.
    fn pack(codes: &[(u16, u32)]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut bit = 0usize;
        for &(code, bits) in codes {
            for i in 0..bits {
                if bit / 8 == out.len() {
                    out.push(0);
                }
                out[bit / 8] |= ((code >> i) as u8 & 1) << (bit % 8);
                bit += 1;
            }
        }
        out
    }

    #[test]
    fn unpacks_literals_table_entries_and_the_kwkwk_case() {
        // Literals a and b add 258 = "ab"; reading 258 adds 259 = "ba";
        // 260 is the next free code, so it is the previous string "ab" plus
        // its own first byte: "aba".
        let packed = pack(&[
            (CLEAR, 9),
            (u16::from(b'a'), 9),
            (u16::from(b'b'), 9),
            (258, 9),
            (260, 9),
            (END, 9),
        ]);
        assert_eq!(unpack(&packed, 7).as_deref(), Some(&b"abababa"[..]));
    }

    #[test]
    fn rejects_wrong_length_and_missing_clear() {
        let packed = pack(&[(CLEAR, 9), (1, 9), (END, 9)]);
        assert_eq!(unpack(&packed, 1).as_deref(), Some(&[1u8][..]));
        assert!(unpack(&packed, 2).is_none());
        let packed = pack(&[(1, 9), (END, 9)]);
        assert!(unpack(&packed, 1).is_none());
    }
}
