//! Apple IIGS Super Hi-Res pictures: screen dump ($C1/0000), Brooks
//! 3200-colour ($C1/0002), compressed 3200-colour (`.3201`), Apple Preferred
//! Format ($C0/0002) and Paintworks ($C0/0000).
//!
//! Sources:
//! - Screen memory (160-byte lines, SCBs at $7D00, sixteen 16-colour palettes
//!   at $7E00, colour word `0RGB` little-endian; 320 and 640 modes, fill
//!   mode, 640-mode palette per pixel column): CiderPress II Super Hi-Res
//!   notes (<https://ciderpress2.com/formatdoc/SuperHiRes-notes.html>) and
//!   the Apple IIGS Hardware Reference
//!   (<https://archive.org/details/Apple_IIgs_Hardware_Reference>).
//! - Brooks: File Type Note $C1/0002
//!   (<https://mirrors.apple2.org.za/ftp.gno.org/doc/apple/filetypes/ftn.c1.0002>):
//!   200 palettes after the pixels, colour 15 stored first.
//! - `.3201`: CiderPress II notes (high-ASCII "APP", 200 palettes, PackBytes).
//! - APF: File Type Note $C0/0002
//!   (<https://mirrors.apple2.org.za/ftp.gno.org/doc/apple/filetypes/ftn.c0.0002>):
//!   MAIN and MULTIPAL blocks.
//! - Packed screen: File Type Note $C0/0001
//!   (<https://mirrors.apple2.org.za/ftp.gno.org/doc/apple/filetypes/ftn.c0.0001>):
//!   the 32 KB screen dump passed through PackBytes.
//! - Paintworks: File Type Note $C0/0000
//!   (<https://mirrors.apple2.org.za/ftp.gno.org/doc/apple/filetypes/ftn.c0.0000>).
//! - Paintworks animation (`ANI`): CiderPress II's file format notes
//!   (<https://github.com/fadden/CiderPress2>, `FileConv/Gfx/PaintworksAnim-notes.md`,
//!   from Antoine Vignau's reverse engineering): a 32 KB screen dump, a
//!   length field equal to the file length minus `$8008`, then the frames.
//!   Only the first frame is shown. No sample file was found, so this is
//!   unverified apart from a unit test.
//! - Output size (640-mode pictures at 640 pixels with doubled lines,
//!   320-mode lines doubled horizontally next to them): observed from
//!   `recoil2png` output.

use alloc::vec::Vec;

use super::pack_bytes;
use crate::bytes::{le16, le32};
use crate::{DecodeError, Image};

const LINE_LEN: usize = 160;
const SCREEN_LEN: usize = 32000;
const MODE_640: u8 = 0x80;
const FILL: u8 = 0x20;

type Palette = [u32; 16];

struct Line<'a> {
    pixels: &'a [u8],
    scb: u8,
    palette: Palette,
}

/// 16 colour words, optionally stored colour 15 first.
fn read_palette(words: &[u8], reversed: bool) -> Palette {
    let mut palette = [0; 16];
    for (i, word) in words.as_chunks::<2>().0.iter().take(16).enumerate() {
        let w = u32::from(u16::from_le_bytes([word[0], word[1]]));
        let color = ((w & 0xf00) << 8 | (w & 0xf0) << 4 | (w & 0xf)) * 0x11;
        palette[if reversed { 15 - i } else { i }] = color;
    }
    palette
}

/// Draws lines; `width` is the pixel count of a 320-mode line.
fn render(lines: &[Line], width: usize) -> Result<Image, DecodeError> {
    if lines.is_empty() || width == 0 {
        return Err(DecodeError::Unrecognized);
    }
    let hires = lines.iter().any(|l| l.scb & MODE_640 != 0);
    let scale = if hires { 2 } else { 1 };
    let out_width = width * scale;
    let mut image = Image::new(out_width as u32, (lines.len() * scale) as u32);
    let mut row = Vec::with_capacity(out_width);
    for (y, line) in lines.iter().enumerate() {
        row.clear();
        if line.scb & MODE_640 != 0 {
            for x in 0..out_width {
                let byte = line.pixels.get(x / 4).copied().unwrap_or(0);
                let j = x % 4;
                let value = usize::from(byte >> (6 - 2 * j) & 3);
                row.push(line.palette[[8, 12, 0, 4][j] + value]);
            }
        } else {
            let mut previous = line.palette[0];
            for x in 0..width {
                let byte = line.pixels.get(x / 2).copied().unwrap_or(0);
                let value = usize::from(if x % 2 == 0 { byte >> 4 } else { byte & 15 });
                let color = if value == 0 && line.scb & FILL != 0 {
                    previous
                } else {
                    line.palette[value]
                };
                previous = color;
                for _ in 0..scale {
                    row.push(color);
                }
            }
        }
        for dy in 0..scale {
            for (x, &color) in row.iter().enumerate() {
                image.set(x as u32, (y * scale + dy) as u32, color);
            }
        }
    }
    Ok(image)
}

/// $C1/0000: a 32 KB dump of screen memory.
pub(super) fn decode_screen(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 0x8000 {
        return Err(DecodeError::Unrecognized);
    }
    let lines: Vec<Line> = (0..200)
        .map(|y| {
            let scb = data[0x7d00 + y];
            let palette_at = 0x7e00 + usize::from(scb & 15) * 32;
            Line {
                pixels: &data[y * LINE_LEN..(y + 1) * LINE_LEN],
                scb,
                palette: read_palette(&data[palette_at..palette_at + 32], false),
            }
        })
        .collect();
    render(&lines, 320)
}

/// A screen dump under an extension shared with other platforms (`.SCR`).
/// Besides the exact size, it must not carry an AMSDOS header (a CPC file)
/// and every colour in the palettes the SCBs select must be a valid `0RGB`
/// word: the high nibble of its high byte is zero. Random data, other
/// machines' screens and the padded CPC overscan files fail that.
/// Observed from the corpus; the rule is not in the File Type Note.
pub(super) fn decode_checked_screen(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 0x8000 || crate::platform::amstrad_cpc::has_amsdos_header(data) {
        return Err(DecodeError::Unrecognized);
    }
    let valid = |palette: usize| {
        data[0x7e00 + palette * 32..][..32]
            .iter()
            .skip(1)
            .step_by(2)
            .all(|&b| b < 16)
    };
    if !data[0x7d00..0x7dc8]
        .iter()
        .all(|&scb| valid(usize::from(scb & 15)))
    {
        return Err(DecodeError::Unrecognized);
    }
    decode_screen(data)
}

/// $C0/0001: a screen dump packed with PackBytes, which must unpack to
/// exactly 32 KB using all of the data.
pub(super) fn decode_packed_screen(data: &[u8]) -> Result<Image, DecodeError> {
    // Asking for one byte more makes the unpacker stop only at the end of
    // the data, so an exact result means nothing is left over.
    let screen = pack_bytes::unpack(data, 0x8001)
        .filter(|s| s.len() == 0x8000)
        .ok_or(DecodeError::Unrecognized)?;
    decode_screen(&screen)
}

/// 320-mode pixels with one reversed palette per line.
pub(super) fn render_3200(pixels: &[u8], palettes: &[u8]) -> Result<Image, DecodeError> {
    let lines: Vec<Line> = pixels
        .as_chunks::<LINE_LEN>()
        .0
        .iter()
        .zip(palettes.as_chunks::<32>().0)
        .map(|(pixels, palette)| Line {
            pixels,
            scb: 0,
            palette: read_palette(palette, true),
        })
        .collect();
    render(&lines, 320)
}

/// $C1/0002 (Brooks): pixels, then 200 palettes.
pub(super) fn decode_brooks(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != SCREEN_LEN + 200 * 32 {
        return Err(DecodeError::Unrecognized);
    }
    let (pixels, palettes) = data.split_at(SCREEN_LEN);
    render_3200(pixels, palettes)
}

/// `.3201`: "APP" in high ASCII, 200 palettes, PackBytes pixels.
pub(super) fn decode_3201(data: &[u8]) -> Result<Image, DecodeError> {
    const PIXELS_AT: usize = 4 + 200 * 32;
    if data.len() <= PIXELS_AT || data[..4] != [0xc1, 0xd0, 0xd0, 0] {
        return Err(DecodeError::Unrecognized);
    }
    let pixels = pack_bytes::unpack(&data[PIXELS_AT..], SCREEN_LEN)
        .filter(|p| p.len() == SCREEN_LEN)
        .ok_or(DecodeError::Unrecognized)?;
    render_3200(&pixels, &data[4..PIXELS_AT])
}

/// $C0/0000 (Paintworks): palette, background, patterns, then 200 or 396
/// PackBytes lines in 320 mode.
pub(super) fn decode_paintworks(data: &[u8]) -> Result<Image, DecodeError> {
    const PIXELS_AT: usize = 0x222;
    // Colour words are `0RGB`: the high nibble of each high byte is zero.
    if data.len() <= PIXELS_AT || data[..32].iter().skip(1).step_by(2).any(|&b| b > 15) {
        return Err(DecodeError::Unrecognized);
    }
    let pixels =
        pack_bytes::unpack(&data[PIXELS_AT..], 396 * LINE_LEN).ok_or(DecodeError::Unrecognized)?;
    let height = match pixels.len() / LINE_LEN {
        396.. => 396,
        200.. => 200,
        _ => return Err(DecodeError::Unrecognized),
    };
    let palette = read_palette(&data[..32], false);
    let lines: Vec<Line> = pixels
        .as_chunks::<LINE_LEN>()
        .0
        .iter()
        .take(height)
        .map(|pixels| Line {
            pixels,
            scb: 0,
            palette,
        })
        .collect();
    render(&lines, 320)
}

/// Paintworks animation (ProDOS `ANI`): the first frame is a $C1/0000 screen
/// dump, followed by a length (the rest of the file after 8 header bytes), a
/// frame delay, a flags word and the changes that make up the other frames.
/// Only the length ties the file to the format, so it must match exactly.
pub(super) fn decode_animation(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let after = data
        .len()
        .checked_sub(0x8008)
        .filter(|&n| n > 0)
        .ok_or(fail)?;
    if le32(data, 0x8000).map(|length| length as usize) != Some(after) {
        return Err(fail);
    }
    decode_screen(&data[..0x8000])
}

/// $C0/0002 (Apple Preferred Format): a list of named blocks.
pub(super) fn decode_apf(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let mut main = None;
    let mut multipal = None;
    let mut rest = data;
    while rest.len() >= 5 {
        let len = u32::from_le_bytes([rest[0], rest[1], rest[2], rest[3]]) as usize;
        let name_len = usize::from(rest[4]);
        if len < 5 + name_len || len > rest.len() {
            break;
        }
        let name = &rest[5..5 + name_len];
        let body = &rest[5 + name_len..len];
        match name {
            b"MAIN" => main = Some(body),
            b"MULTIPAL" => multipal = Some(body),
            _ => {}
        }
        rest = &rest[len..];
    }
    let main = main.ok_or(fail)?;
    let field = |at: usize| le16(main, at).ok_or(fail);
    let master_640 = field(0)? as u8 & MODE_640;
    let width = usize::from(field(2)?);
    let tables = usize::from(field(4)?);
    let tables_at = 6;
    let lines_at = tables_at + tables * 32;
    let line_count = usize::from(field(lines_at)?);
    // Far beyond any IIGS screen; keeps corrupt headers from making huge images.
    if width > 2048 || line_count > 4096 {
        return Err(fail);
    }
    let directory = main
        .get(lines_at + 2..lines_at + 2 + line_count * 4)
        .ok_or(fail)?;
    let mut packed = &main[lines_at + 2 + line_count * 4..];
    let multipal = match multipal {
        Some(block) => {
            let count = usize::from(le16(block, 0).ok_or(fail)?);
            let palettes = block.get(2..2 + count * 32).ok_or(fail)?;
            Some(palettes)
        }
        None => None,
    };

    let mut unpacked = Vec::with_capacity(line_count);
    for entry in directory.as_chunks::<4>().0 {
        let packed_len = usize::from(u16::from_le_bytes([entry[0], entry[1]]));
        // The 320/640 choice follows MasterMode. With MULTIPAL the per-line
        // mode is ignored: some such files hold garbage there (observed from
        // `recoil2png` output).
        let scb = match multipal {
            Some(_) => master_640,
            None => master_640 | (entry[2] & !MODE_640),
        };
        let line_len = if scb & MODE_640 != 0 {
            width.div_ceil(4)
        } else {
            width.div_ceil(2)
        };
        let bytes = packed.get(..packed_len).ok_or(fail)?;
        packed = &packed[packed_len..];
        let pixels = pack_bytes::unpack(bytes, line_len).ok_or(fail)?;
        unpacked.push((pixels, scb));
    }
    let table = |index: usize| {
        main.get(tables_at + index * 32..tables_at + index * 32 + 32)
            .map(|words| read_palette(words, false))
            .unwrap_or([0; 16])
    };
    let lines: Vec<Line> = unpacked
        .iter()
        .enumerate()
        .map(|(y, (pixels, scb))| Line {
            pixels,
            scb: *scb,
            palette: match multipal {
                Some(palettes) => palettes
                    .get(y * 32..y * 32 + 32)
                    .map(|words| read_palette(words, false))
                    .unwrap_or([0; 16]),
                None => table(usize::from(scb & 15)),
            },
        })
        .collect();
    // A 640-mode width counts 640-mode pixels; `render` takes 320-mode ones.
    let any_640 = lines.iter().any(|l| l.scb & MODE_640 != 0);
    render(&lines, if any_640 { width / 2 } else { width })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packed_screen_must_unpack_to_exactly_32k() {
        // 128 runs of one byte repeated 64 x 4 times: 32 KB.
        let packed: Vec<u8> = (0..128).flat_map(|_| [0xff, 0]).collect();
        let image = decode_packed_screen(&packed).unwrap();
        assert_eq!((image.width(), image.height()), (320, 200));
        assert!(decode_packed_screen(&packed[2..]).is_err());
        let mut longer = packed.clone();
        longer.extend_from_slice(&[0x40, 0]);
        assert!(decode_packed_screen(&longer).is_err());
    }

    #[test]
    fn animation_shows_its_first_frame_and_needs_a_matching_length() {
        let mut data = alloc::vec![0u8; 0x8000 + 8 + 8];
        data[0x8000..0x8004].copy_from_slice(&8u32.to_le_bytes());
        // Colour 0 of palette 0 is red.
        data[0x7e01] = 0x0f;
        let image = decode_animation(&data).unwrap();
        assert_eq!(image, decode_screen(&data[..0x8000]).unwrap());
        assert_eq!(image.get(0, 0), 0xff0000);
        data[0x8000] = 9;
        assert!(decode_animation(&data).is_err());
        // A plain screen dump has no trailer.
        assert!(decode_animation(&data[..0x8000]).is_err());
    }

    #[test]
    fn checked_screen_needs_valid_palettes_for_the_used_scbs() {
        let mut data = alloc::vec![0u8; 0x8000];
        assert!(decode_checked_screen(&data).is_ok());
        // Palette 0 is selected by every SCB; a high byte of 0x40 is no 0RGB word.
        data[0x7e01] = 0x40;
        assert!(decode_checked_screen(&data).is_err());
        // A palette no SCB selects is not checked.
        data[0x7e01] = 0;
        data[0x7e00 + 5 * 32 + 1] = 0x40;
        assert!(decode_checked_screen(&data).is_ok());
    }
}
