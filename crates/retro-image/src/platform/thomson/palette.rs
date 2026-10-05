//! Thomson colors: palette values and how they look on screen.
//!
//! Sources:
//! - Palette values are 12 bits, `0BGR` with 4 bits per channel (the
//!   operand of BASIC `PALETTE`), and the 16 power-on entries (black, red,
//!   green, yellow, blue, magenta, cyan, white, gray, then the pastel
//!   shades and orange) that the TO8/TO9+/MO6 set up and the TO7/70 has in
//!   ROM: MAME `src/mame/thomson/thomson_m.cpp` (`thom_pal_init`),
//!   <https://github.com/mamedev/mame/blob/master/src/mame/thomson/thomson_m.cpp>.
//! - Screen levels: the EF9369 palette chip of the TO8 and later applies a
//!   built-in gamma of 2.8, `level = 255 * (v / 15) ^ (1 / 2.8)` truncated,
//!   as MAME does (`thom_configure_palette`, same file); MAME uses the same
//!   curve for the TO7/70's fixed palette.
//!
//! The palette values and the gamma curve come from MAME's
//! `src/mame/thomson/thomson_m.cpp`, used under its license:
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

/// The 16 palette entries at power-on, as 12-bit `0BGR` values.
pub(super) const DEFAULT_PALETTE: [u16; 16] = [
    0x000, 0x00f, 0x0f0, 0x0ff, 0xf00, 0xf0f, 0xff0, 0xfff, //
    0x777, 0x33a, 0x3a3, 0x3aa, 0xa33, 0xa3a, 0xee7, 0x07b,
];

/// Screen level of each 4-bit channel value: `255 * (v / 15) ^ (1 / 2.8)`.
const LEVELS: [u8; 16] = [
    0, 96, 124, 143, 159, 172, 183, 194, 203, 212, 220, 228, 235, 242, 248, 255,
];

/// A 12-bit `0BGR` palette value as `0xRRGGBB`.
pub(super) fn rgb(value: u16) -> u32 {
    let level = |shift: u16| u32::from(LEVELS[usize::from(value >> shift & 15)]);
    level(0) << 16 | level(4) << 8 | level(8)
}

/// All 16 entries of a palette as `0xRRGGBB`.
pub(super) fn to_rgb(palette: &[u16; 16]) -> [u32; 16] {
    palette.map(rgb)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channels_are_bgr_with_gamma() {
        assert_eq!(rgb(0x000), 0x000000);
        assert_eq!(rgb(0x00f), 0xff0000);
        assert_eq!(rgb(0x0f0), 0x00ff00);
        assert_eq!(rgb(0xf00), 0x0000ff);
        // Orange: R 11, G 7, B 0; mid levels are lifted by the gamma.
        assert_eq!(rgb(0x07b), 0xe4c200);
        // Bits above the 12 color bits are ignored.
        assert_eq!(rgb(0x1fff), 0xffffff);
    }

    #[test]
    fn default_palette_starts_black_red_green_yellow() {
        let rgb = to_rgb(&DEFAULT_PALETTE);
        assert_eq!(rgb[..4], [0x000000, 0xff0000, 0x00ff00, 0xffff00]);
        assert_eq!(rgb[7], 0xffffff);
    }
}
