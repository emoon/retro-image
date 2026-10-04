//! Interlace Character Editor font sheets with one character set (`.ICE`
//! modes 0x1f-0x25): Super IRG 2, Super 10, Graphics 9 and 11, HIP, CHIP
//! and APAC fonts. The two-set modes are in `ice.rs`.
//!
//! Sources: reverse engineered from the samples `IRG20.ICE`, `SZAP20P.ICE`,
//! `SZAPS9.ICE`, `SZAPS11.ICE`, `HIP20.ICE`, `CHIP20.ICE`, `SZAPAPAC.ICE` and
//! from black-box probing of `recoil2png` with hand-made files (one glyph or
//! header byte changed at a time); see `docs/research/gaps-corpus-misc.md`
//! section 1. Mode names come from the ICE editor threads on AtariAge.
//! GTIA colour behaviour (header byte is COLBK, pixel ORed into hue or
//! luminance, register luminance bit 0 dropped): De Re Atari App. E
//! (<https://www.atariarchives.org/dere/chaptE.php>).
//!
//! Layout: mode byte, mode-dependent header (the mode byte is header byte
//! 0), one 1024-byte set of 128 glyphs. The sheet is nine 256x32 blocks.
//! Block `3i + j` has 64 cells of 16x8; cell `n` blends glyph `n` drawn
//! with colour set `j` and glyph `n + 64` with colour set `i`. Glyphs are 4
//! pixels of 2 bits per row, 4 output pixels wide.

use super::palette::{average, register_rgb, rgb};
use crate::{DecodeError, Image};

const CHARSET: usize = 1024;
const SPREAD: [u8; 4] = [0, 1, 4, 5];
/// Cells per sheet row, and glyph pixels per sheet row.
const CELLS: usize = 16;
const STREAM: usize = CELLS * 4;

/// Which of the pair of glyphs of a cell is drawn.
#[derive(Clone, Copy)]
enum Glyph {
    A,
    B,
}

/// Header length of a mode, or `None` if it isn't a single-set mode.
fn header_len(mode: u8) -> Option<usize> {
    match mode {
        0x1f | 0x23 | 0x24 => Some(8),
        0x20 => Some(14),
        0x21 | 0x22 | 0x25 => Some(3),
        _ => None,
    }
}

/// Mode 0x1f colours, with `h` the header and `set` the colour set 0-2.
fn irg(h: &[u8], set: usize, v: u8) -> u32 {
    register_rgb(match v {
        0 => h[1],
        1 => h[2 + set],
        2 => h[7],
        _ => [h[5], h[7], h[6]][set],
    })
}

/// Mode 0x20 colours: like 0x1f, with colours of its own per glyph.
fn super10(h: &[u8], glyph: Glyph, set: usize, v: u8) -> u32 {
    let w = glyph as usize;
    register_rgb(match v {
        0 => h[1],
        1 => h[2 + 2 * set + w],
        2 => h[12 + w],
        _ => [h[8 + w], h[12 + w], h[10 + w]][set],
    })
}

/// GTIA 9 colour: the background ORed with the pixel's luminance.
fn gtia9(background: u8, set: usize, v: u8) -> u32 {
    rgb(background & 0xfe | (SPREAD[usize::from(v)] * (set as u8 + 1)))
}

/// GTIA 11 colour: the pixel as hue, the background's luminance.
fn gtia11(background: u8, set: usize, v: u8) -> u32 {
    if v == 0 {
        rgb(background & 0xf0)
    } else {
        rgb((background | ((SPREAD[usize::from(v)] * (set as u8 + 1)) << 4)) & 0xfe)
    }
}

/// Colour of pixel value `v` of `glyph` drawn with colour set `set`.
fn color(mode: u8, h: &[u8], glyph: Glyph, set: usize, v: u8) -> u32 {
    match (mode, glyph) {
        (0x1f, _) => irg(h, set, v),
        (0x20, _) => super10(h, glyph, set, v),
        (0x21, Glyph::A) | (0x23, Glyph::A) | (0x25, Glyph::A) => gtia9(h[1], set, v),
        (0x21, Glyph::B) => gtia9(h[2], set, v),
        (0x22, Glyph::A) | (0x24, Glyph::A) => gtia11(h[1], set, v),
        (0x22, Glyph::B) | (0x25, Glyph::B) => gtia11(h[2], set, v),
        (0x23, Glyph::B) => irg(h, set, v),
        (_, Glyph::B) if v == 0 => 0,
        (_, Glyph::B) => irg(h, set, v),
        _ => 0,
    }
}

pub(super) fn decode_ice_single(data: &[u8]) -> Result<Image, DecodeError> {
    let mode = *data.first().ok_or(DecodeError::Unrecognized)?;
    let header_len = header_len(mode).ok_or(DecodeError::Unrecognized)?;
    if data.len() != header_len + CHARSET {
        return Err(DecodeError::Unrecognized);
    }
    let (header, font) = data.split_at(header_len);
    // Pixel `p` of glyph `n` on glyph line `line`.
    let pixel = |n: usize, line: usize, p: usize| (font[n * 8 + line] >> (6 - 2 * p)) & 3;
    // Shifted modes let the glyphs of a sheet row overlap their neighbours.
    let (shift_a, shift_b) = match mode {
        0x23 | 0x24 => (1, -1),
        _ => (0, 0),
    };
    // Value of the pixel of `n_offset + stream position` covering column
    // `x`, 0 where no pixel does.
    let value = |row: usize, line: usize, x: usize, shift: isize, glyph_offset: usize| {
        let k = (x as isize + shift).div_euclid(4);
        match usize::try_from(k) {
            Ok(k) if k < STREAM => pixel(row * CELLS + k / 4 + glyph_offset, line, k % 4),
            _ => 0,
        }
    };
    let mut image = Image::new(256, 288);
    for block in 0..9 {
        let (set_b, set_a) = (block / 3, block % 3);
        for row in 0..4 {
            for line in 0..8 {
                for x in 0..256 {
                    let a = value(row, line, x, shift_a, 0);
                    let b = value(row, line, x, shift_b, 64);
                    let a = color(mode, header, Glyph::A, set_a, a);
                    let b = color(mode, header, Glyph::B, set_b, b);
                    image.set(
                        x as u32,
                        (block * 32 + row * 8 + line) as u32,
                        average([a, b]),
                    );
                }
            }
        }
    }
    Ok(image)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_exact_single_set_sizes() {
        assert!(decode_ice_single(&[0x21; 1027]).is_ok());
        assert!(decode_ice_single(&[0x20; 1038]).is_ok());
        assert!(decode_ice_single(&[0x21; 1038]).is_err());
        assert!(decode_ice_single(&[0x26; 1027]).is_err());
        assert!(decode_ice_single(&[]).is_err());
    }

    #[test]
    fn sheet_is_nine_blocks() {
        let image = decode_ice_single(&[0x25; 1027]).unwrap();
        assert_eq!((image.width(), image.height()), (256, 288));
    }

    #[test]
    fn glyph_pairs_share_a_cell() {
        // Mode 0x21, background 0: glyph 0 line 0 = pixel value 3 (0xc0) in
        // pixel 0; glyph 64 empty. Block 0 uses set 0 for A: luminance 5.
        let mut data = alloc::vec![0x21, 0, 0];
        data.resize(3 + CHARSET, 0);
        data[3] = 0xc0;
        let image = decode_ice_single(&data).unwrap();
        assert_eq!(image.get(0, 0), average([rgb(5), rgb(0)]));
        assert_eq!(image.get(4, 0), 0);
    }
}
