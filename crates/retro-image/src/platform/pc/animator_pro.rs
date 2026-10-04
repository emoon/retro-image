//! Autodesk Animator Pro `PIC` pictures and cursors (`.PIC`, `.CEL`, `.CUR`),
//! the `0x9500` format that replaced the original Animator PIC of `animator.rs`.
//!
//! Sources:
//! - Animator Pro source, BSD 3-clause (<https://github.com/AnimatorPro/Animator-Pro-C>,
//!   Wayback copy via <https://archive.org/details/animator-pro-source>):
//!   `src/PJ/inc/picfile.h` (64-byte header, chunk types), `src/PJ/pjhigh/pichead.c`
//!   and `picload.c` (chunk walk, default palette when no `CMAP` chunk) and
//!   `src/PJ/flic386p/libsrc/util/initcmap.c` (the default palette below).
//! - Reverse engineered from the 12 `.CUR` samples in
//!   `corpus/extra/animator-pro-pic`: each is the 64-byte header followed by
//!   one raw pixel chunk, and the total size field equals the file length.
//!
//! Layout: file size (u32), `0x9500` (u16), width, height, x, y (i16), a user
//! id (u32), depth (8), padding to 64 bytes, then chunks of
//! `size (u32, includes the header), type (u16)`: type 0 is a palette (an
//! 8-byte header with a version word, then 256 RGB triples), type 1 is the
//! raw 8-bit pixels. Files without a palette chunk use the default palette.
//! Palette chunks and 1-bit pixel chunks (type 2) have no sample here: the
//! palette read follows `picload.c` and is untested, type 2 is rejected.
//!
//! The default palette is copied from `initcmap.c`, whose notice is:
//!
//! ```text
//! BSD 3-Clause License
//!
//! Copyright (c) 1989-1994, Jim Kent
//! All rights reserved.
//!
//! Redistribution and use in source and binary forms, with or without
//! modification, are permitted provided that the following conditions are met:
//!
//! 1. Redistributions of source code must retain the above copyright notice, this
//!    list of conditions and the following disclaimer.
//!
//! 2. Redistributions in binary form must reproduce the above copyright notice,
//!    this list of conditions and the following disclaimer in the documentation
//!    and/or other materials provided with the distribution.
//!
//! 3. Neither the name of the copyright holder nor the names of its
//!    contributors may be used to endorse or promote products derived from
//!    this software without specific prior written permission.
//!
//! THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
//! AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
//! IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
//! DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
//! FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
//! DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
//! SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
//! CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
//! OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
//! OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
//! ```

use alloc::vec::Vec;

use crate::bytes::{le16, le32};
use crate::image::check_size;
use crate::{DecodeError, Image};

const FAIL: DecodeError = DecodeError::Unrecognized;
const MAGIC: u16 = 0x9500;
const HEADER_LEN: usize = 64;
const DEPTH_AT: usize = 18;
const CHUNK_HEADER_LEN: usize = 6;
const PALETTE_CHUNK: u16 = 0;
const PIXEL_CHUNK: u16 = 1;
/// Size of the palette chunk header (`Fat_chunk`): size, type, version.
const PALETTE_HEADER_LEN: usize = 8;

pub(super) fn decode_pic(data: &[u8]) -> Result<Image, DecodeError> {
    if le16(data, 4) != Some(MAGIC)
        || le32(data, 0) != u32::try_from(data.len()).ok()
        || data.get(DEPTH_AT) != Some(&8)
    {
        return Err(FAIL);
    }
    let width = usize::from(le16(data, 6).ok_or(FAIL)?);
    let height = usize::from(le16(data, 8).ok_or(FAIL)?);
    check_size(width, height)?;
    let mut palette = default_palette();
    let mut pos = HEADER_LEN;
    while pos < data.len() {
        let size = le32(data, pos).ok_or(FAIL)? as usize;
        let kind = le16(data, pos + 4).ok_or(FAIL)?;
        let body = data
            .get(pos + CHUNK_HEADER_LEN..pos.checked_add(size).ok_or(FAIL)?)
            .ok_or(FAIL)?;
        match kind {
            PALETTE_CHUNK => {
                let rgb = body
                    .get(PALETTE_HEADER_LEN - CHUNK_HEADER_LEN..)
                    .ok_or(FAIL)?;
                for (entry, c) in palette.iter_mut().zip(rgb.as_chunks::<3>().0) {
                    *entry = u32::from(c[0]) << 16 | u32::from(c[1]) << 8 | u32::from(c[2]);
                }
            }
            PIXEL_CHUNK => {
                let pixels = body.get(..width * height).ok_or(FAIL)?;
                return Image::from_indexed(width as u32, height as u32, pixels, &palette);
            }
            _ => return Err(FAIL),
        }
        pos += size.max(CHUNK_HEADER_LEN);
    }
    Err(FAIL)
}

fn default_palette() -> Vec<u32> {
    DEFAULT_PALETTE
        .as_chunks::<3>()
        .0
        .iter()
        .map(|c| u32::from(c[0]) << 16 | u32::from(c[1]) << 8 | u32::from(c[2]))
        .collect()
}

/// Animator 1.0 start-up palette, 8-bit values (the brightest are 252).
#[rustfmt::skip]
const DEFAULT_PALETTE: [u8; 768] = [
    0, 0, 0, 8, 8, 8, 16, 16, 16, 24, 24, 24,
    32, 32, 32, 40, 40, 40, 48, 48, 48, 56, 56, 56,
    64, 64, 64, 72, 72, 72, 80, 80, 80, 88, 88, 88,
    96, 96, 96, 104, 104, 104, 112, 112, 112, 120, 120, 120,
    132, 132, 132, 140, 140, 140, 148, 148, 148, 156, 156, 156,
    164, 164, 164, 172, 172, 172, 180, 180, 180, 188, 188, 188,
    196, 196, 196, 204, 204, 204, 212, 212, 212, 220, 220, 220,
    228, 228, 228, 236, 236, 236, 244, 244, 244, 252, 252, 252,
    252, 204, 204, 252, 252, 204, 204, 252, 204, 204, 252, 252,
    204, 204, 252, 252, 204, 252, 252, 156, 156, 252, 204, 156,
    252, 252, 156, 204, 252, 156, 156, 252, 156, 156, 252, 204,
    156, 252, 252, 156, 204, 252, 156, 156, 252, 204, 156, 252,
    252, 156, 252, 252, 156, 204, 252, 108, 108, 252, 156, 108,
    252, 204, 108, 252, 252, 108, 204, 252, 108, 156, 252, 108,
    108, 252, 108, 108, 252, 156, 108, 252, 204, 108, 252, 252,
    108, 204, 252, 108, 156, 252, 108, 108, 252, 156, 108, 252,
    204, 108, 252, 252, 108, 252, 252, 108, 204, 252, 108, 156,
    252, 60, 60, 252, 108, 60, 252, 156, 60, 252, 204, 60,
    252, 252, 60, 204, 252, 60, 156, 252, 60, 108, 252, 60,
    60, 252, 60, 60, 252, 108, 60, 252, 156, 60, 252, 204,
    60, 252, 252, 60, 204, 252, 60, 156, 252, 60, 108, 252,
    60, 60, 252, 108, 60, 252, 156, 60, 252, 204, 60, 252,
    252, 60, 252, 252, 60, 204, 252, 60, 156, 252, 60, 108,
    252, 12, 60, 252, 12, 12, 252, 60, 12, 252, 108, 12,
    252, 156, 12, 252, 204, 12, 252, 252, 12, 204, 252, 12,
    156, 252, 12, 108, 252, 12, 60, 252, 12, 12, 252, 12,
    12, 252, 60, 12, 252, 108, 12, 252, 156, 12, 252, 204,
    12, 252, 252, 12, 204, 252, 12, 156, 252, 12, 108, 252,
    12, 60, 252, 12, 12, 252, 60, 12, 252, 108, 12, 252,
    156, 12, 252, 204, 12, 252, 252, 12, 252, 252, 12, 204,
    252, 12, 156, 252, 12, 108, 204, 12, 60, 204, 12, 12,
    204, 60, 12, 204, 108, 12, 204, 156, 12, 204, 204, 12,
    156, 204, 12, 108, 204, 12, 60, 204, 12, 12, 204, 12,
    12, 204, 60, 12, 204, 108, 12, 204, 156, 12, 204, 204,
    12, 156, 204, 12, 108, 204, 12, 60, 204, 12, 12, 204,
    60, 12, 204, 108, 12, 204, 156, 12, 204, 204, 12, 204,
    204, 12, 156, 204, 12, 108, 156, 12, 60, 156, 12, 12,
    156, 60, 12, 156, 108, 12, 156, 156, 12, 108, 156, 12,
    60, 156, 12, 12, 156, 12, 12, 156, 60, 12, 156, 108,
    12, 156, 156, 12, 108, 156, 12, 60, 156, 12, 12, 156,
    60, 12, 156, 108, 12, 156, 156, 12, 156, 156, 12, 108,
    108, 12, 60, 108, 12, 12, 108, 60, 12, 108, 108, 12,
    60, 108, 12, 12, 108, 12, 12, 108, 60, 12, 108, 108,
    12, 60, 108, 12, 12, 108, 60, 12, 108, 108, 12, 108,
    60, 12, 12, 60, 60, 12, 12, 60, 12, 12, 60, 60,
    12, 12, 60, 60, 12, 60, 108, 60, 60, 108, 108, 60,
    60, 108, 60, 60, 108, 108, 60, 60, 108, 108, 60, 108,
    156, 60, 60, 156, 108, 60, 156, 156, 60, 108, 156, 60,
    60, 156, 60, 60, 156, 108, 60, 156, 156, 60, 108, 156,
    60, 60, 156, 108, 60, 156, 156, 60, 156, 156, 60, 108,
    204, 60, 60, 204, 108, 60, 204, 156, 60, 204, 204, 60,
    156, 204, 60, 108, 204, 60, 60, 204, 60, 60, 204, 108,
    60, 204, 156, 60, 204, 204, 60, 156, 204, 60, 108, 204,
    60, 60, 204, 108, 60, 204, 156, 60, 204, 204, 60, 204,
    204, 60, 156, 204, 60, 108, 204, 108, 108, 204, 156, 108,
    204, 204, 108, 156, 204, 108, 108, 204, 108, 108, 204, 156,
    108, 204, 204, 108, 156, 204, 108, 108, 204, 156, 108, 204,
    204, 108, 204, 204, 108, 156, 204, 156, 156, 204, 204, 156,
    156, 204, 156, 156, 204, 204, 156, 156, 204, 204, 156, 204,
    156, 108, 108, 156, 156, 108, 108, 156, 108, 108, 156, 156,
    108, 108, 156, 156, 108, 156, 12, 12, 12, 60, 60, 60,
    108, 108, 108, 156, 156, 156, 204, 204, 204, 252, 252, 252,
    252, 88, 12, 156, 28, 20, 144, 144, 252, 0, 0, 0,
    88, 88, 88, 152, 152, 152, 208, 208, 208, 252, 0, 0,
];
