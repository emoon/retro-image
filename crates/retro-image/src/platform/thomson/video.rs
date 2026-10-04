//! Thomson screen modes: how the two video RAM banks (RAMA, RAMB) become
//! pixels.
//!
//! Sources:
//! - Pixel layout of each mode: Prehisto, "Les fichiers graphiques
//!   Thomson" (ContacThoms bulletin article, Collection Thomson),
//!   <http://web.archive.org/web/20251005163132/http://collection.thomson.free.fr/code/articles/prehisto_bulletin/page.php?XI=0&XJ=13>:
//!   40 columns (RAMA "forme" bits, RAMB colour byte per 8 pixels), bitmap 4
//!   (RAMA bit = colour 2, RAMB bit = colour 1), bitmap 16 (two pixels per
//!   byte, high nibble left), 80 columns (set bit = colour 1).
//! - The 40-column colour byte of the TO7/70 and TO8 (foreground in bits
//!   6-3, background in bits 7 and 2-0, the pastel bit inverted), and the
//!   order of the banks across the line in bitmap 16 and 80 columns (a RAMA
//!   byte, then the RAMB byte at the same address): MAME
//!   `src/mame/thomson/to_video.cpp` (`to770`, `bitmap16`, `mode80`
//!   scanline functions),
//!   <https://github.com/mamedev/mame/blob/master/src/mame/thomson/to_video.cpp>.
//! - Bitmap 16 pixels are twice as wide as they are tall, and 80-column
//!   pixels half as wide: shown 2x1 and 1x2 to keep the 4:3 screen.
//!
//! The colour-byte decoding and bank order follow MAME's
//! `src/mame/thomson/to_video.cpp`, used under its licence:
//!
//! ```text
//! license:BSD-3-Clause
//! copyright-holders:Antoine Mine
//!
//! Redistribution and use in source and binary forms, with or without
//! modification, are permitted provided that the following conditions are
//! met:
//!
//! 1. Redistributions of source code must retain the above copyright
//!    notice, this list of conditions and the following disclaimer.
//!
//! 2. Redistributions in binary form must reproduce the above copyright
//!    notice, this list of conditions and the following disclaimer in the
//!    documentation and/or other materials provided with the distribution.
//!
//! 3. Neither the name of the copyright holder nor the names of its
//!    contributors may be used to endorse or promote products derived from
//!    this software without specific prior written permission.
//!
//! THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS
//! IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
//! TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A
//! PARTICULAR PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL THE COPYRIGHT
//! HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
//! SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED
//! TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
//! PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF
//! LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
//! NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
//! SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
//! ```

use crate::{DecodeError, Image};

/// Screen bytes stored column by column, `lines` bytes per column, as in
/// MAP files. Columns are 8 pixels wide in 40 columns and bitmap 4 (where
/// each bank has its own columns), and alternate RAMA, RAMB across the
/// line in bitmap 16 and 80 columns.
#[derive(Debug, Clone, Copy)]
pub(super) struct Columns<'a> {
    pub(super) bytes: &'a [u8],
    pub(super) lines: usize,
}

impl Columns<'_> {
    fn count(&self) -> usize {
        self.bytes.len() / self.lines
    }

    fn at(&self, x: usize, y: usize) -> u8 {
        self.bytes[x * self.lines + y]
    }
}

/// The foreground and background palette indices of a TO7/70 40-column
/// colour byte: bits 6-3 and bits 7, 2-0, with the pastel bit (the top
/// bit of each index) stored inverted.
pub(super) fn attribute_colors(byte: u8) -> (usize, usize) {
    let foreground = (byte >> 3 & 15) ^ 8;
    let background = (byte & 7 | byte >> 4 & 8) ^ 8;
    (usize::from(foreground), usize::from(background))
}

/// 40 columns: RAMA holds the "forme" bits, RAMB the colour bytes.
pub(super) fn columns40(forme: Columns, couleur: Columns, palette: &[u32; 16]) -> Image {
    draw_bits(forme, palette, |x, y, bit| {
        let (foreground, background) = attribute_colors(couleur.at(x, y));
        if bit { foreground } else { background }
    })
}

/// Bitmap 4: a pixel's colour is `2 * RAMA bit + RAMB bit`.
pub(super) fn bitmap4(rama: Columns, ramb: Columns, palette: &[u32; 16]) -> Image {
    let mut image = Image::new((rama.count() * 8) as u32, rama.lines as u32);
    for x in 0..rama.count() {
        for y in 0..rama.lines {
            let (a, b) = (rama.at(x, y), ramb.at(x, y));
            for bit in 0..8 {
                let index = usize::from(a >> (7 - bit) & 1) << 1 | usize::from(b >> (7 - bit) & 1);
                image.set((x * 8 + bit) as u32, y as u32, palette[index]);
            }
        }
    }
    image
}

/// Bitmap 16: two 4-bit pixels per byte, high nibble left, shown 2x1.
pub(super) fn bitmap16(bytes: Columns, palette: &[u32; 16]) -> Result<Image, DecodeError> {
    let mut image = Image::new((bytes.count() * 2) as u32, bytes.lines as u32);
    for x in 0..bytes.count() {
        for y in 0..bytes.lines {
            let byte = bytes.at(x, y);
            image.set((x * 2) as u32, y as u32, palette[usize::from(byte >> 4)]);
            image.set(
                (x * 2 + 1) as u32,
                y as u32,
                palette[usize::from(byte & 15)],
            );
        }
    }
    image.scaled(2, 1)
}

/// 80 columns: set bits in colour 1, clear bits in colour 0, shown 1x2.
pub(super) fn columns80(bytes: Columns, palette: &[u32; 16]) -> Result<Image, DecodeError> {
    draw_bits(bytes, palette, |_, _, bit| usize::from(bit)).scaled(1, 2)
}

/// 8 pixels per byte, MSB left; `color(x, y, bit)` picks the palette index
/// of a pixel in byte column `x`.
fn draw_bits(
    bytes: Columns,
    palette: &[u32; 16],
    color: impl Fn(usize, usize, bool) -> usize,
) -> Image {
    let mut image = Image::new((bytes.count() * 8) as u32, bytes.lines as u32);
    for x in 0..bytes.count() {
        for y in 0..bytes.lines {
            let byte = bytes.at(x, y);
            for bit in 0..8 {
                let index = color(x, y, byte & (0x80 >> bit) != 0);
                image.set((x * 8 + bit) as u32, y as u32, palette[index]);
            }
        }
    }
    image
}

#[cfg(test)]
mod tests {
    use super::*;

    const PALETTE: [u32; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

    #[test]
    fn attribute_pastel_bits_are_inverted() {
        // 0xC0: both pastel bits set (saturated), black on black.
        assert_eq!(attribute_colors(0xc0), (0, 0));
        // 0xC8: red foreground on black.
        assert_eq!(attribute_colors(0xc8), (1, 0));
        // Pastel bits clear: foreground 7 + 8 = 15, background 0 + 8 = 8.
        assert_eq!(attribute_colors(0x38), (15, 8));
        // Background in bits 2-0 and 7.
        assert_eq!(attribute_colors(0x87), (8, 7));
    }

    #[test]
    fn columns40_uses_forme_bits_and_colour_byte() {
        let forme = [0b1000_0001];
        let couleur = [0xc8 | 2]; // red on green
        let image = columns40(
            Columns {
                bytes: &forme,
                lines: 1,
            },
            Columns {
                bytes: &couleur,
                lines: 1,
            },
            &PALETTE,
        );
        assert_eq!((image.width(), image.height()), (8, 1));
        let row: [u32; 8] = core::array::from_fn(|x| image.get(x as u32, 0));
        assert_eq!(row, [1, 2, 2, 2, 2, 2, 2, 1]);
    }

    #[test]
    fn bitmap4_rama_is_the_high_bit() {
        let image = bitmap4(
            Columns {
                bytes: &[0b1100_0000],
                lines: 1,
            },
            Columns {
                bytes: &[0b1010_0000],
                lines: 1,
            },
            &PALETTE,
        );
        let row: [u32; 4] = core::array::from_fn(|x| image.get(x as u32, 0));
        assert_eq!(row, [3, 2, 1, 0]);
    }

    #[test]
    fn bitmap16_pixels_are_doubled_high_nibble_first() {
        // Two columns (RAMA, RAMB), two lines each.
        let image = bitmap16(
            Columns {
                bytes: &[0x12, 0x34, 0x56, 0x78],
                lines: 2,
            },
            &PALETTE,
        )
        .unwrap();
        assert_eq!((image.width(), image.height()), (8, 2));
        let row: [u32; 8] = core::array::from_fn(|x| image.get(x as u32, 1));
        assert_eq!(row, [3, 3, 4, 4, 7, 7, 8, 8]);
    }

    #[test]
    fn columns80_doubles_lines() {
        let image = columns80(
            Columns {
                bytes: &[0x80],
                lines: 1,
            },
            &PALETTE,
        )
        .unwrap();
        assert_eq!((image.width(), image.height()), (8, 2));
        assert_eq!((image.get(0, 1), image.get(1, 1)), (1, 0));
    }
}
