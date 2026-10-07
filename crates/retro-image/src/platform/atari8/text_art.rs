//! Character-based Atari 8-bit pictures: the Ascii-Art Editor `.ART`
//! and Graph `.ALL`.
//!
//! Sources: Just Solve "Ascii-Art Editor"
//! (<http://fileformats.archiveteam.org/wiki/Ascii-Art_Editor>) and "Graph"
//! (<http://fileformats.archiveteam.org/wiki/Graph>), the RECOIL formats list
//! (<https://recoil.sourceforge.net/formats.html>; up to 64x24 characters
//! mono, 160x192 5 colors) and De Re Atari ch. 2-3 for the ANTIC mode 2 and 4
//! character rules (<https://www.atariarchives.org/dere/chapt02.php>). The
//! file layouts were reverse engineered from the corpus samples (DEBIL1, SIANO,
//! PRODIGY.ART; OBCY.ALL) and probed with hand-made files fed to `recoil2png`
//! (black box).
//!
//! ART: ATASCII text, every line ended by `9B`, at most 24 lines of 64
//! characters, shown in the ROM font. The picture is as wide as the longest
//! line (at least 1) and shorter lines are padded with spaces.
//!
//! ALL: 24 bytes naming the font of each character row, `n` fonts of 1024 bytes
//! (`n` is the file length minus 989, over 1024), 40x24 screen codes, then the
//! colors COLOR0-3 and COLOR4 (background). It is shown in ANTIC mode 4: pixel
//! values 1-3 are playfield 0-2 and 3 in a glyph whose code has bit 7 set is
//! playfield 3. A row naming a font that is not in the file is an error.

use super::gtia;
use super::palette::register_rgb;
use super::text::{mode2, screen_code};
use crate::{DecodeError, Image};
use alloc::vec::Vec;

/// Ascii-Art Editor limits.
const MAX_COLUMNS: usize = 64;
const MAX_ROWS: usize = 24;

/// Ascii-Art Editor picture.
pub(super) fn decode_ascii_art(data: &[u8]) -> Result<Image, DecodeError> {
    let text = data.strip_suffix(&[0x9b]).ok_or(DecodeError::Invalid)?;
    let lines: Vec<&[u8]> = text.split(|&c| c == 0x9b).collect();
    let width = lines
        .iter()
        .map(|line| line.len())
        .max()
        .unwrap_or(0)
        .max(1);
    if lines.len() > MAX_ROWS || width > MAX_COLUMNS {
        return Err(DecodeError::Invalid);
    }
    let mut codes = alloc::vec![0; width * lines.len()];
    for (row, line) in lines.iter().enumerate() {
        for (column, &c) in line.iter().enumerate() {
            codes[row * width + column] = screen_code(c);
        }
    }
    mode2(&codes, width)
}

const ALL_HEADER: usize = 24;
const FONT: usize = 1024;
const SCREEN: usize = 40 * 24;
const COLORS: usize = 5;

/// Graph picture: ANTIC mode 4 with one font per row.
pub(super) fn decode_all(data: &[u8]) -> Result<Image, DecodeError> {
    let fixed = ALL_HEADER + SCREEN + COLORS;
    let fonts_len = data
        .len()
        .checked_sub(fixed)
        .filter(|len| *len > 0 && len % FONT == 0)
        .ok_or(DecodeError::Invalid)?;
    let (rows, rest) = data.split_at(ALL_HEADER);
    let (fonts, rest) = rest.split_at(fonts_len);
    let (screen, colors) = rest.split_at(SCREEN);
    if rows.iter().any(|&f| usize::from(f) >= fonts_len / FONT) {
        return Err(DecodeError::Invalid);
    }
    let register = |i: usize| register_rgb(colors[i]);
    // Value 0 is the background (COLOR4); 1-3 are COLOR0-2; COLOR3 is the
    // inverse variant of 3.
    let palette = [
        register(4),
        register(0),
        register(1),
        register(2),
        register(3),
    ];
    let mut image = Image::new(320, 192)?;
    for (i, &code) in screen.iter().enumerate() {
        let (column, row) = ((i % 40) as u32, (i / 40) as u32);
        let font = &fonts[usize::from(rows[i / 40]) * FONT..][..FONT];
        let glyph = &font[usize::from(code & 0x7f) * 8..][..8];
        for (line, &bits) in glyph.iter().enumerate() {
            for pixel in 0..4u32 {
                let slot = gtia::antic4_register((bits >> (6 - 2 * pixel)) & 3, code & 0x80 != 0);
                for half in 0..2 {
                    let x = column * 8 + pixel * 2 + half;
                    image.set(x, row * 8 + line as u32, palette[slot]);
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
    fn ascii_art_pads_short_lines() {
        let image = decode_ascii_art(&[b'A', b'B', 0x9b, b'C', 0x9b]).unwrap();
        assert_eq!((image.width(), image.height()), (16, 16));
        assert!(decode_ascii_art(b"A").is_err());
        assert!(decode_ascii_art(&[]).is_err());
        assert_eq!(decode_ascii_art(&[0x9b]).unwrap().width(), 8);
        let mut wide = alloc::vec![b'a'; 65];
        wide.push(0x9b);
        assert!(decode_ascii_art(&wide).is_err());
        assert!(decode_ascii_art(&[0x9b; 25]).is_err());
        assert!(decode_ascii_art(&[0x9b; 24]).is_ok());
    }

    #[test]
    fn all_needs_a_font_per_row() {
        let mut data = alloc::vec![0u8; 989 + 1024];
        assert!(decode_all(&data).is_ok());
        data[5] = 1;
        assert!(decode_all(&data).is_err());
        data.resize(989 + 2048, 0);
        assert!(decode_all(&data).is_ok());
        data.push(0);
        assert!(decode_all(&data).is_err());
        assert!(decode_all(&[0; 989]).is_err());
    }
}
