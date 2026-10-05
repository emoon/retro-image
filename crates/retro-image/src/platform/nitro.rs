//! Nintendo DS NitroSDK resources: palettes (`.nclr`), character data
//! (`.ncgr`, `.ncbr`) and screens (`.nscr`), and the 3D textures of `.nsbtx`
//! and `.nsbmd` files (`texture.rs`).
//!
//! Sources:
//! - NitroPaint by Garhoogin, BSD 2-Clause (notice below),
//!   <https://github.com/Garhoogin/NitroPaint>, commit `dec4dc4`:
//!   `NitroPaint/object/nns.c` (the G2D file header and sections),
//!   `NitroPalette.c` (`PalReadNclr`), `NitroCharacter.c` (`ChrReadNcgr`,
//!   `ChrWriteNcgr`), `NitroScreen.c` (`ScrReadNscr`, `ScriReadScreenData`).
//! - GBATEK, "DS Files - Video Palette/Character/Screen" (NCLR, NCGR, NSCR),
//!   <https://problemkaputt.de/gbatek.htm> (no licence, facts only).
//!
//! GBATEK and NitroPaint disagree in two places, and NitroPaint decides. In
//! an NCGR, GBATEK reads the first two 16-bit fields of the character
//! section as a size in KB, "always 0x20"; NitroPaint reads them as the
//! height and the width of the picture in tiles, which for the usual
//! 256 x 256 pixel picture is the same two bytes (`0x20`), and its own writer
//! stores them that way. In an NSCR, GBATEK lists the width with 4 bytes and
//! the height at the next 2, overlapping; NitroPaint reads two 16-bit values,
//! width and height in pixels.
//!
//! No sample of these formats could be found (no free ones are published),
//! so the decoders are unverified. They follow the layouts NitroPaint reads
//! and are tested only on files built from those layouts; see
//! `docs/research/gaps-nintendo.md` C1.
//!
//! All the files share a header: a 4-byte magic (`RLCN`, `RGCN`, `RCSN`), the
//! byte order mark `0xFEFF`, a version, the file size, the header size
//! (0x10), and the number of sections; each section has a 4-byte magic (read
//! backwards: `TTLP`, `RAHC`, `NRCS`), its size including this 8-byte
//! header, and its data. That header is strict enough for content
//! detection, so the three formats have signatures.
//!
//! Files that this does not cover are rejected: the older G2D layout whose
//! section sizes leave out the section headers, palettes with a `PCMP`
//! compression table (read as if uncompressed they would put colors in the
//! wrong palettes), and files wrapped in BIOS compression.
//!
//! Colors are BGR555 (`v << 3 | v >> 2` widens a channel, as elsewhere in
//! this crate; NitroPaint uses a rounding table that differs by one level).
//! A character file alone is a sheet in grays; with the `.nclr` of the same
//! name it is drawn in the first palette, and with the `.nscr` of the same
//! name it is drawn as that screen. Color 0 is drawn as color 0 of palette 0,
//! which is what the DS shows behind a transparent pixel of a background
//! (NitroPaint's default too).

// Parts of this file follow NitroPaint (https://github.com/Garhoogin/NitroPaint):
//
// BSD 2-Clause License
//
// Copyright (c) 2020, Garhoogin
// All rights reserved.
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are met:
//
// 1. Redistributions of source code must retain the above copyright notice, this
//    list of conditions and the following disclaimer.
//
// 2. Redistributions in binary form must reproduce the above copyright notice,
//    this list of conditions and the following disclaimer in the documentation
//    and/or other materials provided with the distribution.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
// AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
// DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
// FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
// DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
// SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
// CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
// OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
// OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

mod character;
mod screen;
#[cfg(test)]
mod testing;
mod texture;

use alloc::vec::Vec;

use crate::bytes::{le16, le32};
use crate::image::check_size;
use crate::{Companions, DecodeError, Format, Image};
use character::Character;
use screen::Screen;

pub(super) static FORMATS: &[Format] = &[
    Format::new("Nintendo DS", "Nitro palette", &["nclr"], decode_palette).signature(),
    Format::with_companions(
        "Nintendo DS",
        "Nitro character data",
        &["ncgr", "ncbr"],
        decode_character,
    )
    .signature(),
    Format::with_companions("Nintendo DS", "Nitro screen", &["nscr"], decode_screen).signature(),
    Format::new(
        "Nintendo DS",
        "Nitro 3D textures",
        &["nsbtx", "btx0", "nsbmd", "bmd0"],
        texture::decode,
    )
    .signature(),
];

/// Byte order mark of the little-endian files, as a 16-bit value.
const BYTE_ORDER_MARK: u16 = 0xfeff;
/// Smallest header size, and the size of the 8-byte header of a section.
const HEADER_LEN: usize = 0x10;
const SECTION_HEADER_LEN: usize = 8;

/// The data of the section called `section` (as it is stored, the 4 letters
/// backwards) in the file `data` of type `magic`, if the file's header is
/// sound and its sections lie inside the file.
fn section<'a>(data: &'a [u8], magic: &[u8; 4], section: &[u8; 4]) -> Option<&'a [u8]> {
    if data.get(..4)? != magic || le16(data, 4)? != BYTE_ORDER_MARK {
        return None;
    }
    let file_size = le32(data, 8)? as usize;
    let header_size = usize::from(le16(data, 0xc)?);
    let count = le16(data, 0xe)?;
    if header_size < HEADER_LEN || file_size < header_size || file_size > data.len() || count == 0 {
        return None;
    }
    let data = &data[..file_size];
    let mut found = None;
    let mut offset = header_size;
    for _ in 0..count {
        let name = data.get(offset..offset + 4)?;
        let size = le32(data, offset + 4)? as usize;
        let end = offset.checked_add(size)?;
        if size < SECTION_HEADER_LEN || end > data.len() {
            return None;
        }
        if name == section && found.is_none() {
            found = Some(&data[offset + SECTION_HEADER_LEN..end]);
        }
        offset = end;
    }
    found
}

/// A palette (`PLTT` section of an NCLR).
struct Palette {
    colors: Vec<u32>,
}

impl Palette {
    /// Section body: bit depth (3 = 4 bpp, 4 = 8 bpp) at 0, the extended
    /// palette flag at 4, the size of the colors at 8 and their offset in
    /// the body at 0xC, then the 16-bit colors.
    fn parse(data: &[u8]) -> Option<Self> {
        let body = section(data, b"RLCN", b"TTLP")?;
        if !matches!(le32(body, 0)?, 3 | 4) {
            return None;
        }
        let size = le32(body, 8)? as usize;
        let offset = le32(body, 0xc)? as usize;
        let bytes = body.get(offset..offset.checked_add(size)?)?;
        let colors: Vec<u32> = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&word| bgr555(u16::from_le_bytes(word)))
            .collect();
        (!colors.is_empty()).then_some(Self { colors })
    }
}

/// A DS color: red in bits 0-4, green in 5-9, blue in 10-14.
fn bgr555(word: u16) -> u32 {
    let widen = |bits: u16| u32::from(bits << 3 | bits >> 2);
    widen(word & 31) << 16 | widen(word >> 5 & 31) << 8 | widen(word >> 10 & 31)
}

/// The color of pixel value `index` in palette number `number` of a character
/// set of `bits` bits a pixel: from `palette` when there is one, else a gray
/// ramp. A palette without that color gives black. Value 0 is color 0 of
/// palette 0 in every palette.
fn pixel_color(palette: Option<&Palette>, bits: usize, number: usize, index: u8) -> u32 {
    match palette {
        Some(palette) => {
            let number = if index == 0 { 0 } else { number };
            let at = number << bits | usize::from(index);
            palette.colors.get(at).copied().unwrap_or(0)
        }
        None => {
            let gray = if bits == 4 {
                u32::from(index) * 17
            } else {
                u32::from(index)
            };
            gray * 0x01_0101
        }
    }
}

/// The palette of the `.nclr` companion, if there is a sound one.
fn companion_palette(companions: &dyn Companions) -> Option<Palette> {
    Palette::parse(&companions.get("nclr")?)
}

/// The colors of palette 0 for pixel values of `bits` bits.
fn first_palette(palette: Option<&Palette>, bits: usize) -> Vec<u32> {
    (0..1 << bits)
        .map(|index| pixel_color(palette, bits, 0, index as u8))
        .collect()
}

/// The palette as swatches, 16 to a row, 16 pixels square.
fn decode_palette(data: &[u8]) -> Result<Image, DecodeError> {
    const PER_ROW: usize = 16;
    const SIZE: usize = 16;
    let palette = Palette::parse(data).ok_or(DecodeError::Unrecognized)?;
    let rows = palette.colors.len().div_ceil(PER_ROW);
    let (width, height) = (PER_ROW * SIZE, rows * SIZE);
    check_size(width, height)?;
    let colors = (0..width * height).map(|i| {
        let (x, y) = (i % width, i / width);
        palette
            .colors
            .get(y / SIZE * PER_ROW + x / SIZE)
            .copied()
            .unwrap_or(0)
    });
    Ok(Image::from_colors(width as u32, height as u32, colors))
}

/// The character set `data`, drawn as the screen of its `.nscr` companion if
/// there is a sound one, else as a sheet of tiles.
fn decode_character(data: &[u8], companions: &dyn Companions) -> Result<Image, DecodeError> {
    let character = Character::parse(data).ok_or(DecodeError::Unrecognized)?;
    let palette = companion_palette(companions);
    let screen = companions.get("nscr");
    if let Some(screen) = screen.as_deref().and_then(Screen::parse)
        && let Ok(image) = screen.draw(&character, palette.as_ref())
    {
        return Ok(image);
    }
    character.sheet(palette.as_ref())
}

/// The screen `data`, drawn with the `.ncgr` (or `.ncbr`) and `.nclr`
/// companions; it needs the character data.
fn decode_screen(data: &[u8], companions: &dyn Companions) -> Result<Image, DecodeError> {
    let screen = Screen::parse(data).ok_or(DecodeError::Unrecognized)?;
    let characters = companions
        .get("ncgr")
        .or_else(|| companions.get("ncbr"))
        .ok_or(DecodeError::Unrecognized)?;
    let character = Character::parse(&characters).ok_or(DecodeError::Unrecognized)?;
    screen.draw(&character, companion_palette(companions).as_ref())
}

#[cfg(test)]
mod tests {
    use super::testing::{ncgr, nclr, nscr};
    use super::*;
    use crate::NoCompanions;

    #[test]
    fn a_palette_is_drawn_as_swatches() {
        // Colors: red, green, blue (BGR555), then 13 black.
        let mut words = alloc::vec![0x001f, 0x03e0, 0x7c00];
        words.resize(16, 0);
        let image = decode_palette(&nclr(3, &words)).unwrap();
        assert_eq!((image.width(), image.height()), (256, 16));
        assert_eq!(image.get(0, 0), 0xff0000);
        assert_eq!(image.get(16 + 15, 15), 0x00ff00);
        assert_eq!(image.get(32, 8), 0x0000ff);
        assert_eq!(image.get(48, 0), 0);
        let big = decode_palette(&nclr(4, &[0; 256])).unwrap();
        assert_eq!((big.width(), big.height()), (256, 256));
    }

    #[test]
    fn the_header_must_be_sound() {
        let good = nclr(3, &[0; 16]);
        assert!(decode_palette(&good).is_ok());
        // Not the magic, a wrong byte order mark, a size beyond the file.
        let mut bad = good.clone();
        bad[0] = b'X';
        assert!(decode_palette(&bad).is_err());
        let mut bad = good.clone();
        bad[5] = 0xff;
        assert!(decode_palette(&bad).is_err());
        let mut bad = good.clone();
        bad[8] += 1;
        assert!(decode_palette(&bad).is_err());
        // A section running past the file, a depth that is not 4 or 8 bits.
        let mut bad = good.clone();
        bad[0x14] = 0xff;
        assert!(decode_palette(&bad).is_err());
        assert!(decode_palette(&nclr(5, &[0; 16])).is_err());
        assert!(decode_palette(&good[..0x20]).is_err());
        // Trailing data after the file size is ignored.
        let mut padded = good;
        padded.extend_from_slice(&[0; 7]);
        assert!(decode_palette(&padded).is_ok());
    }

    #[test]
    fn a_character_set_takes_its_palette_and_screen_from_companions() {
        // One 4 bpp tile with pixel value 1, a screen of 8 x 8 pixels that
        // uses it, and a palette whose color 1 is green.
        let tile: Vec<u8> = alloc::vec![0x11; 32];
        let characters = ncgr(3, (1, 1), 0, 0, &tile);
        let mut words = alloc::vec![0x7fff, 0x03e0];
        words.resize(16, 0);
        let palette = nclr(3, &words);
        let screen = nscr(8, 8, 0, &[0x00, 0x00]);
        let image = decode_character(&characters, &NoCompanions).unwrap();
        assert_eq!((image.width(), image.height()), (8, 8));
        assert_eq!(image.get(0, 0), 0x11_1111);
        struct Files<'a>(&'a [u8], &'a [u8]);
        impl Companions for Files<'_> {
            fn get(&self, extension: &str) -> Option<Vec<u8>> {
                match extension {
                    "nclr" => Some(self.0.to_vec()),
                    "nscr" => Some(self.1.to_vec()),
                    _ => None,
                }
            }
            fn get_named(&self, _name: &str) -> Option<Vec<u8>> {
                None
            }
        }
        let with = decode_character(&characters, &Files(&palette, &screen)).unwrap();
        assert_eq!((with.width(), with.height()), (8, 8));
        assert_eq!(with.get(3, 3), 0x00ff00);
        // The screen alone needs the character data.
        assert!(decode_screen(&screen, &NoCompanions).is_err());
    }
}
