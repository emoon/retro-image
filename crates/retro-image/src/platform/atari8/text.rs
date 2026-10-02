//! Text-mode screens that store only screen codes and are shown with the
//! OS ROM font ([`ROM_FONT`]): Mad Studio GR0, AN2, GR1, GR2, AN4, AN5;
//! GR0-style dumps (ASC, SCR, SGE); Dir Logo Maker (DLM).
//!
//! Sources:
//! - Mad Studio file formats PDF (MIT-licensed project),
//!   <https://raw.githubusercontent.com/Gury8/Mad-Studio/master/docs/mad-studio-file-formats.pdf>:
//!   GR0 (960 screen bytes), AN2 (max X, max Y, screen), GR1 (480 bytes +
//!   COLOR4, COLOR0-3), GR2 (240 bytes + 5 colours), AN4/AN5 (max X, max Y,
//!   COLOR4, COLOR0-3, screen).
//! - Character modes: De Re Atari ch. 2 (ANTIC modes 2, 4-7) and Mapping
//!   the Atari (bits 6-7 of a mode 6/7 code select the colour register;
//!   bit 7 of a mode 4/5 code selects playfield 3 for pixel value 3).
//! - DLM: Just Solve "Dir Logo Maker" (256 bytes, 11x16 characters): the
//!   file is 16 DOS 2 directory entries whose 11 name bytes are ATASCII.
//! - Observed from `recoil2png` output: the accepted sizes, the default
//!   colours (mode 2: background 0x00, foreground luminance 0x0E; modes
//!   6/7 without colours: the OS power-up colours), no scaling for AN2,
//!   AN4 and AN5 (ANTIC 5 lines doubled), and 16-pixel-wide characters in
//!   GR1/GR2.

use super::palette::register_rgb;
use super::rom_font::ROM_FONT;
use super::screen::OS_COLORS;
use crate::{DecodeError, Image};

/// OS power-up value of COLOR3 (playfield 3).
const OS_PF3: u8 = 0x46;

/// The glyph of screen code `code` (bit 7, inverse video, ignored).
fn glyph(code: u8) -> &'static [u8] {
    let start = usize::from(code & 0x7f) * 8;
    &ROM_FONT[start..start + 8]
}

/// ANTIC mode 2 (Graphics 0) characters, 8x8 pixels, white on black;
/// bit 7 of a code shows the glyph inverted.
fn mode2(codes: &[u8], columns: usize) -> Image {
    let rows = codes.len() / columns;
    let (background, foreground) = (register_rgb(0x00), register_rgb(0x0e));
    let mut image = Image::new(columns as u32 * 8, rows as u32 * 8);
    for (i, &code) in codes.iter().enumerate() {
        let (x0, y0) = ((i % columns) as u32 * 8, (i / columns) as u32 * 8);
        let inverse = if code & 0x80 != 0 { 0xff } else { 0 };
        for (row, &bits) in glyph(code).iter().enumerate() {
            let bits = bits ^ inverse;
            for column in 0..8 {
                let set = bits & (0x80 >> column) != 0;
                let color = if set { foreground } else { background };
                image.set(x0 + column, y0 + row as u32, color);
            }
        }
    }
    image
}

/// A Graphics 0 screen of 40-character lines (GR0, ASC, SCR, SGE): 24
/// lines (960 bytes) as RECOIL reads them, or up to the 30 lines ANTIC can
/// show in 240 scanlines, as in the 28-line screens of ATASCII art
/// competitions.
pub(super) fn decode_gr0(data: &[u8]) -> Result<Image, DecodeError> {
    if !data.len().is_multiple_of(40) || !(24..=30).contains(&(data.len() / 40)) {
        return Err(DecodeError::Unrecognized);
    }
    Ok(mode2(data, 40))
}

/// Splits Mad Studio's max X and max Y header (`header` bytes in all) from
/// the screen, which must hold exactly the (max X + 1) x (max Y + 1) codes.
fn sized(
    data: &[u8],
    header: usize,
    max_rows: usize,
) -> Result<(&[u8], &[u8], usize), DecodeError> {
    let [max_x, max_y, ..] = *data else {
        return Err(DecodeError::Unrecognized);
    };
    let (columns, rows) = (usize::from(max_x) + 1, usize::from(max_y) + 1);
    let (head, codes) = data
        .split_at_checked(header)
        .ok_or(DecodeError::Unrecognized)?;
    if columns > 40 || rows > max_rows || codes.len() != columns * rows {
        return Err(DecodeError::Unrecognized);
    }
    Ok((head, codes, columns))
}

/// Mad Studio ANTIC 2: max X, max Y, then the screen codes.
pub(super) fn decode_an2(data: &[u8]) -> Result<Image, DecodeError> {
    let (_, codes, columns) = sized(data, 2, 24)?;
    Ok(mode2(codes, columns))
}

/// Dir Logo Maker: 16 directory entries of 16 bytes; bytes 5-15 of each
/// are an 11-character line of ATASCII.
pub(super) fn decode_dlm(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 256 {
        return Err(DecodeError::Unrecognized);
    }
    let mut codes = [0u8; 16 * 11];
    for (line, entry) in codes.chunks_exact_mut(11).zip(data.chunks_exact(16)) {
        for (code, &atascii) in line.iter_mut().zip(&entry[5..]) {
            *code = screen_code(atascii);
        }
    }
    Ok(mode2(&codes, 11))
}

/// The screen code showing ATASCII character `c`.
fn screen_code(c: u8) -> u8 {
    let inverse = c & 0x80;
    inverse
        | match c & 0x7f {
            c @ 0x00..=0x1f => c + 0x40,
            c @ 0x20..=0x5f => c - 0x20,
            c => c,
        }
}

/// ANTIC modes 6 and 7 (Graphics 1 and 2): 64 glyphs drawn 16 pixels wide
/// and `line_height` lines tall (8 or 16); bits 6-7 of a code select
/// playfield 0-3 for set pixels, others show the background.
/// `colors` are COLOR4 (background), COLOR0-3.
fn mode6(codes: &[u8], line_height: u32, colors: [u8; 5]) -> Image {
    let rows = codes.len() / 20;
    let mut image = Image::new(20 * 8, rows as u32 * 8);
    for (i, &code) in codes.iter().enumerate() {
        let (x0, y0) = ((i % 20) as u32 * 8, (i / 20) as u32 * 8);
        let foreground = register_rgb(colors[1 + usize::from(code >> 6)]);
        let background = register_rgb(colors[0]);
        for (row, &bits) in glyph(code & 0x3f).iter().enumerate() {
            for column in 0..8 {
                let set = bits & (0x80 >> column) != 0;
                let color = if set { foreground } else { background };
                image.set(x0 + column, y0 + row as u32, color);
            }
        }
    }
    image.scaled(2, line_height / 8)
}

/// Mad Studio Graphics 1 (480 codes) or Graphics 2 (240 codes), optionally
/// followed by COLOR4 and COLOR0-3.
fn decode_mode6(data: &[u8], len: usize, line_height: u32) -> Result<Image, DecodeError> {
    let [background, pf0, pf1, pf2] = OS_COLORS;
    let colors = match data.get(len..) {
        Some([]) => [background, pf0, pf1, pf2, OS_PF3],
        Some(&[c4, c0, c1, c2, c3]) => [c4, c0, c1, c2, c3],
        _ => return Err(DecodeError::Unrecognized),
    };
    Ok(mode6(&data[..len], line_height, colors))
}

pub(super) fn decode_gr1(data: &[u8]) -> Result<Image, DecodeError> {
    decode_mode6(data, 480, 8)
}

pub(super) fn decode_gr2(data: &[u8]) -> Result<Image, DecodeError> {
    decode_mode6(data, 240, 16)
}

/// ANTIC modes 4 and 5: glyphs of 4 two-bit pixels (2 pixels wide) per
/// row, `line_height` lines tall (8 or 16). Pixel values 1-3 show
/// playfield 0-2, or playfield 3 for value 3 when bit 7 of the code is set.
/// `colors` are COLOR4 (background), COLOR0-3.
fn mode4(codes: &[u8], columns: usize, line_height: u32, colors: [u8; 5]) -> Image {
    let rows = codes.len() / columns;
    let mut image = Image::new(columns as u32 * 4, rows as u32 * 8);
    for (i, &code) in codes.iter().enumerate() {
        let (x0, y0) = ((i % columns) as u32 * 4, (i / columns) as u32 * 8);
        for (row, &bits) in glyph(code).iter().enumerate() {
            for column in 0..4 {
                let value = (bits >> (6 - 2 * column)) & 3;
                let register = match value {
                    3 if code & 0x80 != 0 => 4,
                    _ => usize::from(value),
                };
                image.set(x0 + column, y0 + row as u32, register_rgb(colors[register]));
            }
        }
    }
    image.scaled(2, line_height / 8)
}

/// Mad Studio ANTIC 4 / 5: max X, max Y, COLOR4, COLOR0-3, then the codes.
fn decode_mode4(data: &[u8], max_rows: usize, line_height: u32) -> Result<Image, DecodeError> {
    let (head, codes, columns) = sized(data, 7, max_rows)?;
    let colors = [head[2], head[3], head[4], head[5], head[6]];
    Ok(mode4(codes, columns, line_height, colors))
}

pub(super) fn decode_an4(data: &[u8]) -> Result<Image, DecodeError> {
    decode_mode4(data, 24, 8)
}

pub(super) fn decode_an5(data: &[u8]) -> Result<Image, DecodeError> {
    decode_mode4(data, 12, 16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inverse_video_flips_the_glyph() {
        let mut data = [0u8; 960];
        data[1] = 0x80;
        let image = decode_gr0(&data).unwrap();
        assert_eq!(image.get(0, 0), 0);
        assert_eq!(image.get(8, 0), register_rgb(0x0e));
    }

    #[test]
    fn gr0_takes_24_to_30_lines() {
        assert_eq!(decode_gr0(&[0; 1120]).unwrap().height(), 224);
        assert!(decode_gr0(&[0; 920]).is_err());
        assert!(decode_gr0(&[0; 1240]).is_err());
        assert!(decode_gr0(&[0; 961]).is_err());
    }

    #[test]
    fn atascii_maps_to_screen_codes() {
        assert_eq!(screen_code(b' '), 0);
        assert_eq!(screen_code(b'A'), 0x21);
        assert_eq!(screen_code(0x01), 0x41);
        assert_eq!(screen_code(b'a'), b'a');
        assert_eq!(screen_code(0xa0), 0x80);
    }

    #[test]
    fn an2_size_must_match_header() {
        assert_eq!(decode_an2(&[2, 1, 0, 0, 0, 0, 0, 0]).unwrap().width(), 24);
        assert!(decode_an2(&[2, 1, 0, 0, 0, 0, 0]).is_err());
        assert!(decode_an2(&[40, 0]).is_err());
    }
}
