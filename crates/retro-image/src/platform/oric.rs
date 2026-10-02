//! Oric HIRES screens and character sets, saved as Oric tape files.
//!
//! Sources:
//! - HIRES screen (40 bytes x 200 rows at 0xA000, serial attributes, bit 7
//!   inverse, ink/paper reset to white on black every row): defence-force
//!   Oric coding part 7, <https://www.defence-force.org/computing/oric/coding/part_7/index.htm>,
//!   and OSDK "Oric graphics in detail", <https://osdk.org/index.php?page=articles&ref=ART9>.
//! - Character set RAM (characters 0-127 at 0xB400 in TEXT mode and at 0x9800
//!   in HIRES mode, 8 bytes each, 6 pixels used): the same sources.
//! - Tape file layout (0x16 sync bytes, 0x24, 9 header bytes with big-endian
//!   inclusive end and start addresses, zero-terminated name of up to 16
//!   characters, body): Defence Force wiki,
//!   <https://wiki.defence-force.org/doku.php?id=oric:hardware:tape_encoding>.
//!   A tape image holds several files; like the ROM loader, bytes between
//!   files are skipped up to the next sync sequence. Tape images in the corpus
//!   start with 3 sync bytes. Some writers store an end address one past the
//!   last byte (and stop there), so only the bytes a decoder needs must be
//!   present.
//! - Character sheet layout (32 per row, 8-pixel cells, white on black):
//!   observed from `recoil2png` output.

use crate::{DecodeError, Format, Image};

pub(super) static FORMATS: &[Format] = &[
    Format::new("Oric", "HIRES screen", &["hir", "hrs"], decode_hires),
    Format::new("Oric", "Character set", &["chs"], decode_charset),
];

const HIRES_START: u16 = 0xa000;
const HIRES_LEN: usize = 8000;
/// Address of character 32 in TEXT mode and in HIRES mode charset RAM.
const CHARSET_STARTS: [u16; 2] = [0xb500, 0x9900];
const CHARSET_LEN: usize = 768;

/// HIRES screen: 200 rows of 40 bytes, 6 pixels per byte, from the first
/// tape file that loads a whole screen at 0xA000.
fn decode_hires(data: &[u8]) -> Result<Image, DecodeError> {
    let file = tape_files(data)?
        .find(|file| file.start == HIRES_START && file.body.len() >= HIRES_LEN)
        .ok_or(DecodeError::Unrecognized)?;
    let mut image = Image::new(240, 200);
    for (y, row) in file.body[..HIRES_LEN].chunks_exact(40).enumerate() {
        let (mut ink, mut paper) = (7, 0);
        for (column, &byte) in row.iter().enumerate() {
            let pixels = if byte & 0x60 == 0 {
                match byte & 0x18 {
                    0x00 => ink = byte & 7,
                    0x10 => paper = byte & 7,
                    _ => {}
                }
                0
            } else {
                byte & 0x3f
            };
            let invert = if byte & 0x80 != 0 { 7 } else { 0 };
            for bit in 0..6 {
                let index = if pixels & (0x20 >> bit) != 0 {
                    ink
                } else {
                    paper
                };
                image.set((column * 6 + bit) as u32, y as u32, color(index ^ invert));
            }
        }
    }
    Ok(image)
}

/// Character set: characters 32-127 drawn as a 32x3 sheet of 8x8 cells, from
/// the first tape file that loads charset RAM from character 0 or 32.
fn decode_charset(data: &[u8]) -> Result<Image, DecodeError> {
    let charset = tape_files(data)?
        .find_map(|file| {
            CHARSET_STARTS.iter().find_map(|&first| {
                let skip = usize::from(first.checked_sub(file.start)?);
                if skip != 0 && skip != 32 * 8 {
                    return None;
                }
                file.body.get(skip..skip + CHARSET_LEN)
            })
        })
        .ok_or(DecodeError::Unrecognized)?;
    let mut image = Image::new(256, 24);
    for (index, glyph) in charset.chunks_exact(8).enumerate() {
        let (left, top) = (index % 32 * 8, index / 32 * 8);
        for (y, &byte) in glyph.iter().enumerate() {
            for bit in 0..8 {
                let set = byte & (0x80 >> bit) != 0;
                let color = if set { 0xffffff } else { 0 };
                image.set((left + bit) as u32, (top + y) as u32, color);
            }
        }
    }
    Ok(image)
}

/// Colour bits: 0 red, 1 green, 2 blue.
fn color(index: u8) -> u32 {
    let channel = |bit: u8| if index & bit != 0 { 0xff } else { 0 };
    channel(1) << 16 | channel(2) << 8 | channel(4)
}

/// One file of an Oric tape image.
struct TapeFile<'a> {
    start: u16,
    /// The bytes present, at most `end - start + 1`.
    body: &'a [u8],
}

const SYNC: [u8; 4] = [0x16, 0x16, 0x16, 0x24];

/// Files of a tape image, which must start with a sync sequence.
fn tape_files(data: &[u8]) -> Result<impl Iterator<Item = TapeFile<'_>>, DecodeError> {
    let syncs = data.iter().take_while(|&&b| b == 0x16).count();
    if syncs < 3 || data.get(syncs) != Some(&0x24) {
        return Err(DecodeError::Unrecognized);
    }
    let mut rest = data;
    Ok(core::iter::from_fn(move || {
        loop {
            let sync = rest.windows(SYNC.len()).position(|w| w == SYNC)?;
            rest = &rest[sync + SYNC.len()..];
            if let Some((file, span)) = parse_file(rest) {
                rest = &rest[span..];
                return Some(file);
            }
            // Not a valid header: a false sync in junk; search on.
        }
    }))
}

/// Parses a file that follows a sync sequence; also returns how many bytes it
/// spans.
fn parse_file(data: &[u8]) -> Option<(TapeFile<'_>, usize)> {
    let header = data.get(..9)?;
    let end = u16::from_be_bytes([header[4], header[5]]);
    let start = u16::from_be_bytes([header[6], header[7]]);
    let len = usize::from(end.checked_sub(start)?) + 1;
    let name_len = data[9..].iter().take(17).position(|&b| b == 0)?;
    let body_start = 9 + name_len + 1;
    let body = &data[body_start..data.len().min(body_start + len)];
    Some((TapeFile { start, body }, body_start + body.len()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use alloc::vec::Vec;

    fn file(start: u16, end: u16, body: &[u8]) -> Vec<u8> {
        let [eh, el] = end.to_be_bytes();
        let [sh, sl] = start.to_be_bytes();
        let mut data = vec![0x16, 0x16, 0x16, 0x24, 0, 0, 0x80, 0, eh, el, sh, sl, 0];
        data.extend_from_slice(b"NAME\0");
        data.extend_from_slice(body);
        data
    }

    #[test]
    fn finds_screen_after_other_files_and_junk() {
        let mut tape = file(0x0501, 0x0503, &[1, 2, 3]);
        tape.extend_from_slice(&[0xff, 0x16, 0x24]); // junk between files
        tape.extend(file(0xa000, 0xbf3f, &[0x40; HIRES_LEN]));
        let starts: Vec<u16> = tape_files(&tape).unwrap().map(|f| f.start).collect();
        assert_eq!(starts, [0x0501, 0xa000]);
        assert!(decode_hires(&tape).is_ok());
    }

    #[test]
    fn rejects_short_screen() {
        let tape = file(0xa000, 0xbf18, &[0x40; 7961]);
        assert!(decode_hires(&tape).is_err());
    }

    #[test]
    fn charset_at_text_and_hires_bases() {
        for start in [0xb400, 0xb500, 0x9800] {
            let tape = file(start, start + 0x3ff, &[0; 1024]);
            assert!(decode_charset(&tape).is_ok(), "{start:x}");
        }
        let tape = file(0xb480, 0xb87f, &[0; 1024]);
        assert!(decode_charset(&tape).is_err());
    }
}
