//! 8x8 character sets: FNT (one set of 128 characters) and FN2 (two sets).
//!
//! Sources:
//! - FNT: Graph2Font manual (<https://g2f.atari8.info/instrukcja_eng.html>)
//!   and De Re Atari ch. 3 (<https://www.atariarchives.org/dere/chapt03.php>):
//!   1024-byte charset, 8 bytes per character, top row first, bit 7 leftmost.
//! - FN2: Atari FontMaker page
//!   (<http://matosimi.websupport.sk/atari/atari-fontmaker/>; "dual font",
//!   2 x 1024 bytes); docs only. Drawing the two fonts as halves of 8x16
//!   characters: observed from `recoil2png` output.
//! - SIF: Super-IRG Font Editor doc SIFE.TXT by Bill Kendrick
//!   (<http://ftp.pigwa.net/stuff/collections/atari_forever/Tools%20-%20atr/Super%20IRG%20Font%20Editor/SIFE.TXT>):
//!   two 1024-byte ANTIC mode 4 charsets flipped every frame.
//! - ACS: Just Solve "AtariTools-800"
//!   (<http://fileformats.archiveteam.org/wiki/AtariTools-800>, 4 colours).
//!   Size 1028, the colour bytes and the 16-character rows: observed from
//!   `recoil2png` output.
//! - JGP: Just Solve "Jet Graphics Planner"
//!   (<http://fileformats.archiveteam.org/wiki/Jet_Graphics_Planner>;
//!   exactly 2054 bytes, 4 colours).
//!   The binary-load header (any 2048-byte segment), the two charsets
//!   stacked as 8x16 characters
//!   and the grey colours: observed from `recoil2png` output.
//! - NLQ: Just Solve "Daisy-Dot font"
//!   (<http://fileformats.archiveteam.org/wiki/Daisy-Dot_font>) and the
//!   Daisy-Dot II reader of
//!   monobit (MIT, <https://github.com/robhagemans/monobit>): signature,
//!   per character a width and two passes of column bytes. The 20x16 cell
//!   sheet: observed from `recoil2png` output. Daisy-Dot III (`3` 0x9B,
//!   variable widths, optional 32-dot characters, 3-byte trailer) from the
//!   same monobit reader; RECOIL rejects it, so its sheet (32x16 or 32x32
//!   cells) is our own extension of the Daisy-Dot II one.
//! - SXS (1024-byte font of 16x16 characters as a DOS binary-load file), ODF (OD Font Editor,
//!   8x10 characters), F80 (The Last Word, 4x8 characters, two per 8-byte
//!   group; manual: <https://atari8.co.uk/wp-content/uploads/2015/03/The-Last-Word-2.1.pdf>):
//!   layouts reverse engineered from samples and checked against
//!   `recoil2png` output.
//! - Accepted sizes (FNT 1024-1026 bytes), the sheet layout of 32
//!   characters per row and the colours (SIF: 0x00, 0x4C, 0xCC, 0x8C, the
//!   two charsets mixed): observed from `recoil2png` output.

use super::antic::{Bitmap, fill};
use super::palette::register_rgb;
use super::screen::GREY_COLORS;
use crate::{DecodeError, Image};
use alloc::vec::Vec;

const CHARS_PER_ROW: usize = 32;

/// Raw 128-character font; RECOIL also accepts 1 or 2 trailing bytes.
pub(super) fn decode_fnt(data: &[u8]) -> Result<Image, DecodeError> {
    match data.len() {
        1024..=1026 => Ok(sheet(&[&data[..1024]])),
        _ => Err(DecodeError::Unrecognized),
    }
}

/// Two 128-character fonts shown as 8x16 characters: the first font gives
/// the top half, the second the bottom half.
pub(super) fn decode_fn2(data: &[u8]) -> Result<Image, DecodeError> {
    match data.len() {
        2048 => Ok(sheet(&[&data[..1024], &data[1024..]])),
        _ => Err(DecodeError::Unrecognized),
    }
}

/// Super-IRG font: two ANTIC mode 4 charsets shown on alternate frames.
pub(super) fn decode_sif(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 2048 {
        return Err(DecodeError::Unrecognized);
    }
    const COLORS: [u8; 4] = [0x00, 0x4c, 0xcc, 0x8c];
    let charset = |font: &[u8]| {
        let mut image = Image::new(CHARS_PER_ROW as u32 * 8, 32);
        for (index, glyph) in font.as_chunks::<8>().0.iter().enumerate() {
            let x = (index % CHARS_PER_ROW) as u32 * 8;
            let y = (index / CHARS_PER_ROW) as u32 * 8;
            draw_multicolor_glyph(&mut image, x, y, glyph, COLORS);
        }
        image
    };
    Ok(Image::blend(&[
        &charset(&data[..1024]),
        &charset(&data[1024..]),
    ]))
}

/// AtariTools-800 font: background and playfield 0-2, then an ANTIC mode 4
/// charset drawn 16 characters to a row.
pub(super) fn decode_acs(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 1028 {
        return Err(DecodeError::Unrecognized);
    }
    let (colors, charset) = data.split_at(4);
    let colors = [colors[0], colors[1], colors[2], colors[3]];
    let mut image = Image::new(128, 64);
    for (index, glyph) in charset.as_chunks::<8>().0.iter().enumerate() {
        let (x, y) = ((index % 16) as u32 * 8, (index / 16) as u32 * 8);
        draw_multicolor_glyph(&mut image, x, y, glyph, colors);
    }
    Ok(image)
}

/// Jet Graphics Planner: a DOS binary-load header for one 2048-byte
/// segment (at any address), then two ANTIC mode 4 charsets shown as 8x16
/// characters (first charset on top), in greys.
pub(super) fn decode_jgp(data: &[u8]) -> Result<Image, DecodeError> {
    let charsets = binary_load(data, 2048).ok_or(DecodeError::Unrecognized)?;
    let mut image = Image::new(CHARS_PER_ROW as u32 * 8, 64);
    for (part, charset) in charsets.as_chunks::<1024>().0.iter().enumerate() {
        for (index, glyph) in charset.as_chunks::<8>().0.iter().enumerate() {
            let x = (index % CHARS_PER_ROW) as u32 * 8;
            let y = (index / CHARS_PER_ROW) as u32 * 16 + part as u32 * 8;
            draw_multicolor_glyph(&mut image, x, y, glyph, GREY_COLORS);
        }
    }
    Ok(image)
}

/// The contents of a DOS binary-load file holding one segment of exactly
/// `len` bytes: `FF FF`, start and end address (little-endian), data.
fn binary_load(data: &[u8], len: usize) -> Option<&[u8]> {
    let [
        0xff,
        0xff,
        start_low,
        start_high,
        end_low,
        end_high,
        ref segment @ ..,
    ] = *data
    else {
        return None;
    };
    let start = u16::from_le_bytes([start_low, start_high]);
    let end = u16::from_le_bytes([end_low, end_high]);
    let last = u16::try_from(len.checked_sub(1)?).ok()?;
    (start.checked_add(last) == Some(end) && segment.len() == len).then_some(segment)
}

/// SXS: a 1024-byte font saved as a DOS binary-load file, holding 32
/// characters of 16x16 pixels; each is 4 consecutive glyphs (top left, top
/// right, bottom left, bottom right). Drawn 16 to a row.
pub(super) fn decode_sxs(data: &[u8]) -> Result<Image, DecodeError> {
    let font = binary_load(data, 1024).ok_or(DecodeError::Unrecognized)?;
    let (background, foreground) = (register_rgb(0x00), register_rgb(0x0e));
    let mut image = Image::new(256, 32);
    for (index, glyph) in font.as_chunks::<8>().0.iter().enumerate() {
        let (big, quarter) = (index / 4, index % 4);
        let x = (big % 16 * 16 + quarter % 2 * 8) as u32;
        let y = (big / 16 * 16 + quarter / 2 * 8) as u32;
        draw_glyph(&mut image, x, y, glyph, |set| {
            if set { foreground } else { background }
        });
    }
    Ok(image)
}

/// OD Font Editor: 128 characters of 8x10 pixels, 10 bytes each, drawn 32
/// to a row.
pub(super) fn decode_odf(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 1280 {
        return Err(DecodeError::Unrecognized);
    }
    let (background, foreground) = (register_rgb(0x00), register_rgb(0x0e));
    let mut image = Image::new(CHARS_PER_ROW as u32 * 8, 40);
    for (index, glyph) in data.as_chunks::<10>().0.iter().enumerate() {
        let x = (index % CHARS_PER_ROW) as u32 * 8;
        let y = (index / CHARS_PER_ROW) as u32 * 10;
        draw_glyph(&mut image, x, y, glyph, |set| {
            if set { foreground } else { background }
        });
    }
    Ok(image)
}

/// The Last Word 80-column font: 128 characters of 4x8 pixels; each
/// 8-byte group holds two characters, the even one in the high nibbles.
/// Drawn 32 to a row.
pub(super) fn decode_f80(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 512 {
        return Err(DecodeError::Unrecognized);
    }
    let (background, foreground) = (register_rgb(0x00), register_rgb(0x0e));
    let mut image = Image::new(CHARS_PER_ROW as u32 * 4, 32);
    for (pair, rows) in data.as_chunks::<8>().0.iter().enumerate() {
        for half in 0..2 {
            let index = 2 * pair + half;
            let x0 = (index % CHARS_PER_ROW) as u32 * 4;
            let y0 = (index / CHARS_PER_ROW) as u32 * 8;
            for (row, &bits) in rows.iter().enumerate() {
                let nibble = if half == 0 { bits >> 4 } else { bits & 0x0f };
                for column in 0..4 {
                    let set = nibble & (8 >> column) != 0;
                    let color = if set { foreground } else { background };
                    image.set(x0 + column, y0 + row as u32, color);
                }
            }
        }
    }
    Ok(image)
}

/// Characters stored in Daisy-Dot fonts: 32-124 except 96 and 123.
fn daisy_dot_codes() -> impl Iterator<Item = u32> {
    (32..125).filter(|&code| code != 96 && code != 123)
}

/// Daisy-Dot NLQ printer font, version II or III.
pub(super) fn decode_nlq(data: &[u8]) -> Result<Image, DecodeError> {
    if let Some(glyphs) = data.strip_prefix(b"DAISY-DOT NLQ FONT\x9b") {
        decode_daisy_dot2(glyphs)
    } else if let Some(glyphs) = data.strip_prefix(b"3\x9b") {
        decode_daisy_dot3(glyphs)
    } else {
        Err(DecodeError::Unrecognized)
    }
}

/// Daisy-Dot II: for each character a width, the even rows' column bytes,
/// the odd rows' column bytes and a 0x9B separator. Characters are 16 dots
/// tall and drawn in 20x16 cells, 16 to a row, by character code from 32.
fn decode_daisy_dot2(mut glyphs: &[u8]) -> Result<Image, DecodeError> {
    const CELL_WIDTH: u32 = 20;
    let mut image = Image::new(16 * CELL_WIDTH, 96);
    for code in daisy_dot_codes() {
        let (&width, rest) = glyphs.split_first().ok_or(DecodeError::Unrecognized)?;
        let width = usize::from(width);
        let (glyph, rest) = rest
            .split_at_checked(2 * width)
            .ok_or(DecodeError::Unrecognized)?;
        let (&0x9b, rest) = rest.split_first().ok_or(DecodeError::Unrecognized)? else {
            return Err(DecodeError::Unrecognized);
        };
        if width as u32 > CELL_WIDTH {
            return Err(DecodeError::Unrecognized);
        }
        glyphs = rest;
        let (x, y) = ((code - 32) % 16 * CELL_WIDTH, (code - 32) / 16 * 16);
        draw_dot_passes(&mut image, x, y, glyph);
    }
    if glyphs.is_empty() {
        Ok(image)
    } else {
        Err(DecodeError::Unrecognized)
    }
}

/// Daisy-Dot III: no space glyph; for each other character a byte of
/// width (1-32) plus 64 if the character is 32 dots tall, then one or two
/// 16-dot bands of passes like Daisy-Dot II, without separators; then the
/// height, underline row and space width. Drawn like Daisy-Dot II in 32-dot
/// wide cells, 16 or (if any character is tall) 32 dots high.
fn decode_daisy_dot3(data: &[u8]) -> Result<Image, DecodeError> {
    const CELL_WIDTH: u32 = 32;
    let mut glyphs = Vec::new();
    let mut rest = data;
    for code in daisy_dot_codes().skip(1) {
        let (&size, after) = rest.split_first().ok_or(DecodeError::Unrecognized)?;
        let (bands, width) = (usize::from(size >> 6) + 1, usize::from(size & 0x3f));
        if bands > 2 || !(1..=CELL_WIDTH as usize).contains(&width) {
            return Err(DecodeError::Unrecognized);
        }
        let (glyph, after) = after
            .split_at_checked(2 * width * bands)
            .ok_or(DecodeError::Unrecognized)?;
        glyphs.push((code, width, glyph));
        rest = after;
    }
    if rest.len() != 3 {
        return Err(DecodeError::Unrecognized);
    }
    let tall = glyphs
        .iter()
        .any(|&(_, width, glyph)| glyph.len() > 2 * width);
    let cell_height = if tall { 32 } else { 16 };
    let mut image = Image::new(16 * CELL_WIDTH, 6 * cell_height);
    for (code, width, glyph) in glyphs {
        let (x, y) = (
            (code - 32) % 16 * CELL_WIDTH,
            (code - 32) / 16 * cell_height,
        );
        for (band, passes) in glyph.chunks_exact(2 * width).enumerate() {
            draw_dot_passes(&mut image, x, y + 16 * band as u32, passes);
        }
    }
    Ok(image)
}

/// Draws a 16-dot band of a Daisy-Dot glyph at (`x`, `y`): `passes` holds
/// one column byte per column for the even rows, then as many for the odd
/// rows; bit 7 is the top. Set dots are white on black.
fn draw_dot_passes(image: &mut Image, x: u32, y: u32, passes: &[u8]) {
    let (background, foreground) = (register_rgb(0x00), register_rgb(0x0e));
    let (even, odd) = passes.split_at(passes.len() / 2);
    for (column, (&even, &odd)) in even.iter().zip(odd).enumerate() {
        for bit in 0..8 {
            for (row, bits) in [(2 * bit, even), (2 * bit + 1, odd)] {
                let set = bits & (0x80 >> bit) != 0;
                let color = if set { foreground } else { background };
                image.set(x + column as u32, y + row, color);
            }
        }
    }
}

/// Draws an ANTIC mode 4 glyph (4x8 pixels of 2 bits, each 2 pixels wide)
/// with its top-left corner at (`x`, `y`); `colors` are background and
/// playfield 0-2.
pub(super) fn draw_multicolor_glyph(
    image: &mut Image,
    x: u32,
    y: u32,
    glyph: &[u8],
    colors: [u8; 4],
) {
    let bitmap = Bitmap {
        data: glyph,
        bytes_per_line: 1,
        lines: 8,
        bits: 2,
    };
    for row in 0..8 {
        for column in 0..4 {
            let color = register_rgb(colors[usize::from(bitmap.pixel(column, row))]);
            fill(image, x + 2 * column as u32, y + row as u32, 2, 1, color);
        }
    }
}

/// Draws 128 characters 32 to a row, white on black. Each character stacks
/// its glyph from every font in `fonts`.
fn sheet(fonts: &[&[u8]]) -> Image {
    let char_height = 8 * fonts.len() as u32;
    let rows = 128 / CHARS_PER_ROW as u32;
    let mut image = Image::new(CHARS_PER_ROW as u32 * 8, rows * char_height);
    let (background, foreground) = (register_rgb(0x00), register_rgb(0x0e));
    for (part, font) in fonts.iter().enumerate() {
        for (index, glyph) in font.as_chunks::<8>().0.iter().enumerate() {
            let x = (index % CHARS_PER_ROW) as u32 * 8;
            let y = (index / CHARS_PER_ROW) as u32 * char_height + part as u32 * 8;
            draw_glyph(&mut image, x, y, glyph, |set| {
                if set { foreground } else { background }
            });
        }
    }
    image
}

/// Draws an 8x8 1-bit glyph with its top-left corner at (`x`, `y`).
pub(super) fn draw_glyph(
    image: &mut Image,
    x: u32,
    y: u32,
    glyph: &[u8],
    color: impl Fn(bool) -> u32,
) {
    for (row, &bits) in glyph.iter().enumerate() {
        for column in 0..8 {
            let set = bits & (0x80 >> column) != 0;
            image.set(x + column, y + row as u32, color(set));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lays_out_32_chars_per_row() {
        let mut data = [0u8; 1024];
        data[33 * 8] = 0x80; // character 33: row 1, column 1
        let image = decode_fnt(&data).unwrap();
        assert_eq!((image.width(), image.height()), (256, 32));
        assert_eq!(image.get(8, 8), 0xeeeeee);
    }

    #[test]
    fn daisy_dot3_draws_tall_glyphs_in_two_bands() {
        let mut data = alloc::vec![b'3', 0x9b];
        // '!' is 1 dot wide and 32 tall: top band rows 0 and 1, bottom row 1.
        data.extend_from_slice(&[0x41, 0x80, 0x80, 0x00, 0x80]);
        for _ in daisy_dot_codes().skip(2) {
            data.extend_from_slice(&[0x01, 0x00, 0x00]);
        }
        data.extend_from_slice(&[31, 24, 8]);
        let image = decode_nlq(&data).unwrap();
        assert_eq!((image.width(), image.height()), (512, 192));
        let lit = |y| image.get(32, y) == 0xeeeeee;
        assert_eq!(
            [lit(0), lit(1), lit(2), lit(16), lit(17)],
            [true, true, false, false, true]
        );
        assert!(decode_nlq(&data[..data.len() - 1]).is_err());
    }

    #[test]
    fn binary_load_needs_matching_addresses() {
        let mut data = alloc::vec![0xff, 0xff, 0x00, 0xb4, 0xff, 0xb7];
        data.resize(6 + 1024, 0);
        assert!(binary_load(&data, 1024).is_some());
        data[4] = 0xfe;
        assert!(binary_load(&data, 1024).is_none());
        assert!(binary_load(&data, 1023).is_none());
    }

    #[test]
    fn f80_packs_two_characters_per_group() {
        let mut data = [0u8; 512];
        data[0] = 0x81; // char 0 row 0: leftmost dot; char 1 row 0: rightmost
        let image = decode_f80(&data).unwrap();
        let lit = |x| image.get(x, 0) == 0xeeeeee;
        assert_eq!([lit(0), lit(3), lit(4), lit(7)], [true, false, false, true]);
    }

    #[test]
    fn rejects_wrong_sizes() {
        assert!(decode_fnt(&[0; 1027]).is_err());
        assert!(decode_fn2(&[0; 1024]).is_err());
    }
}
