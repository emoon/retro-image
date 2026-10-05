//! BTX0 texture archives (`.nsbtx`) and the `TEX0` textures of model files
//! (`.nsbmd`): the DS 3D textures, drawn as a contact sheet.
//!
//! Sources: NitroPaint (BSD 2-Clause, notice below),
//! `NitroPaint/object/NitroTexArc.c` (`TexarcReadNsbtx`, `readDictionary`)
//! and `NitroPaint/texture.c` (`TxiSample*`, `TxiBlend`), and GBATEK, the
//! pages "BTX0 (.NSBTX Texture Data)" and "DS 3D Texture Attributes" and
//! "Formats" (<https://problemkaputt.de/gbatek.htm>, no licence, facts only).
//! No sample file was available, so this is unverified: it is tested only on
//! files built from those layouts.
//!
//! A G3D file has the header of a G2D file (see `nitro.rs`) but, after it, a
//! list of 32-bit offsets of its chunks; `BTX0` has `TEX0` and `BMD0` has
//! `MDL0` and `TEX0`. The `TEX0` chunk holds a dictionary of textures, a
//! dictionary of palettes, and the data. A dictionary is a count byte at 1,
//! the offset at 6 of its entries, then a 16-bit entry size and a 16-bit
//! size of the list, the entries, and after them the 16-byte names. A texture
//! entry is the `TEXIMAGE_PARAM` register value (the data offset divided by 8
//! in bits 0-15, the sizes `8 << n` in bits 20-22 and 23-25, the format in
//! 26-28, and color 0 transparent in bit 29) and 4 bytes of the stored size,
//! which is not used. A palette entry is the offset divided by 8 and a
//! flag. The chunk's header gives the dictionary offsets (texture 0xE,
//! palette 0x34) and the data offsets (texture 0x14, compressed texture
//! 0x24, compressed texture attributes 0x28, palette 0x38), all from the
//! start of the chunk.
//!
//! The formats are 1 A3I5, 2 four colors, 3 sixteen colors, 4 256 colors, 5
//! compressed 4 x 4, 6 A5I3 and 7 direct 15-bit color with an alpha bit, from
//! GBATEK. Pixels are row by row, the low bits of a byte first. A texture
//! that is partly transparent is composited onto the shared fill. In the
//! compressed format each 4 x 4 block has 32 bits of 2-bit pixels (the
//! first row in the low byte) and 16 bits of attributes (the palette offset
//! in bits 0-13, in 4-byte steps, and the mode in 14-15); the mode says
//! whether the fourth pixel is transparent and whether the third and fourth
//! are mixed from the first two. Mixed colors are computed on the 5-bit
//! channels, which may differ slightly from the console's.
//!
//! Palettes are named separately and the file does not say which belongs to
//! which texture. The match is a guess in this order: the same name, the
//! name with `_pl` or `_p` added, a palette whose name begins or ends the
//! texture's name (the longest, at least 3 letters), the palette with the
//! texture's own number, the first palette. The first two are what Mario Kart
//! DS does. A texture of a file with no palette uses grays.
//!
//! The textures are drawn at their size, left to right in rows 1024 pixels
//! wide with 4 pixels between them, on a mid-gray background, in the order of
//! the file. Textures that would take the sheet past 2048 pixels in height are
//! left out.

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

use alloc::vec::Vec;

use super::bgr555;
use crate::bytes::{le16, le32};
use crate::image::{check_size, over_fill};
use crate::{DecodeError, Image};

const BYTE_ORDER_MARK: u16 = 0xfeff;
const SHEET_BACKGROUND: u32 = 0x80_8080;
/// Widest and tallest sheet, and the space between textures.
const SHEET_WIDTH: usize = 1024;
const SHEET_HEIGHT: usize = 2048;
const GAP: usize = 4;
const NAME_LEN: usize = 16;

/// Pixel format numbers of `TEXIMAGE_PARAM`.
const A3I5: u32 = 1;
const PALETTE_4: u32 = 2;
const PALETTE_16: u32 = 3;
const PALETTE_256: u32 = 4;
const COMPRESSED: u32 = 5;
const A5I3: u32 = 6;
const DIRECT: u32 = 7;

/// The `TEX0` chunk of a `BTX0` or `BMD0` file.
fn tex0_chunk(data: &[u8]) -> Option<&[u8]> {
    if !(data.starts_with(b"BTX0") || data.starts_with(b"BMD0"))
        || le16(data, 4)? != BYTE_ORDER_MARK
    {
        return None;
    }
    let size = le32(data, 8)? as usize;
    let header = usize::from(le16(data, 0xc)?);
    let count = usize::from(le16(data, 0xe)?);
    if header < 0x10 || size > data.len() || count == 0 || count > 16 {
        return None;
    }
    let data = &data[..size];
    for i in 0..count {
        let at = le32(data, header + i * 4)? as usize;
        if data.get(at..at.checked_add(4)?)? == b"TEX0" {
            let len = le32(data, at + 4)? as usize;
            return data.get(at..at.checked_add(len)?);
        }
    }
    None
}

/// A dictionary: `count` entries of `unit` bytes, and their 16-byte names.
struct Dict<'a> {
    count: usize,
    entries: &'a [u8],
    names: &'a [u8],
}

impl<'a> Dict<'a> {
    fn parse(chunk: &'a [u8], at: usize, unit: usize) -> Option<Self> {
        let count = usize::from(*chunk.get(at.checked_add(1)?)?);
        let list = at.checked_add(usize::from(le16(chunk, at.checked_add(6)?)?))?;
        if usize::from(le16(chunk, list)?) != unit {
            return None;
        }
        let entries_at = list.checked_add(4)?;
        let names_at = entries_at.checked_add(count * unit)?;
        Some(Self {
            count,
            entries: chunk.get(entries_at..names_at)?,
            names: chunk.get(names_at..names_at.checked_add(count * NAME_LEN)?)?,
        })
    }

    fn entry(&self, i: usize, unit: usize) -> &'a [u8] {
        &self.entries[i * unit..][..unit]
    }

    /// The name of entry `i`, without the zero padding.
    fn name(&self, i: usize) -> &'a [u8] {
        let name = &self.names[i * NAME_LEN..][..NAME_LEN];
        let end = name.iter().position(|&b| b == 0).unwrap_or(NAME_LEN);
        &name[..end]
    }
}

/// A texture of the dictionary, from its `TEXIMAGE_PARAM`.
struct Texture {
    format: u32,
    width: usize,
    height: usize,
    /// Color 0 of a palette is transparent.
    color0_transparent: bool,
    /// Byte offset of the data in its data block.
    offset: usize,
}

impl Texture {
    fn new(param: u32) -> Self {
        Self {
            format: param >> 26 & 7,
            width: 8 << (param >> 20 & 7),
            height: 8 << (param >> 23 & 7),
            color0_transparent: param >> 29 & 1 != 0,
            offset: (param & 0xffff) as usize * 8,
        }
    }

    /// Bytes of the data of the base level.
    fn data_len(&self) -> usize {
        let pixels = self.width * self.height;
        match self.format {
            PALETTE_4 | COMPRESSED => pixels / 4,
            PALETTE_16 => pixels / 2,
            A3I5 | PALETTE_256 | A5I3 => pixels,
            _ => pixels * 2,
        }
    }
}

/// The data blocks of a `TEX0` chunk.
struct Blocks<'a> {
    chunk: &'a [u8],
    texture: usize,
    compressed: usize,
    attributes: usize,
    palette: usize,
}

impl Blocks<'_> {
    fn bytes(&self, block: usize, offset: usize, len: usize) -> Option<&[u8]> {
        let start = block.checked_add(offset)?;
        self.chunk.get(start..start.checked_add(len)?)
    }
}

/// The colors of one palette, from its first color to the end of the palette
/// data, or none: a gray ramp.
struct Colors<'a> {
    data: Option<&'a [u8]>,
}

impl Colors<'_> {
    /// Color number `i` of a texture of `bits` bits a pixel, as red, green,
    /// blue and alpha; black if the palette ends before it.
    fn get(&self, i: usize, bits: u32) -> [u8; 4] {
        let rgb = match self.data {
            Some(data) => bgr555(le16(data, i * 2).unwrap_or(0)),
            None => {
                let gray = (i as u32 * 255 / ((1 << bits) - 1)).min(255);
                gray * 0x01_0101
            }
        };
        let [_, r, g, b] = rgb.to_be_bytes();
        [r, g, b, 255]
    }
}

/// The colors mixed `weight` eighths of the way from `a` to `b`.
fn mix(a: [u8; 4], b: [u8; 4], weight: u32) -> [u8; 4] {
    let channel =
        |i: usize| ((u32::from(a[i]) * (8 - weight) + u32::from(b[i]) * weight + 4) / 8) as u8;
    [channel(0), channel(1), channel(2), 255]
}

/// The pixels of `texture` as red, green, blue and alpha, row by row.
fn render(texture: &Texture, blocks: &Blocks, colors: &Colors) -> Option<Vec<[u8; 4]>> {
    let (width, height) = (texture.width, texture.height);
    let len = texture.data_len();
    let color0 = |index: usize, color: [u8; 4]| {
        if index == 0 && texture.color0_transparent {
            [color[0], color[1], color[2], 0]
        } else {
            color
        }
    };
    let alpha5 = |a: usize| ((a * 510 + 31) / 62) as u8;
    let mut pixels = Vec::with_capacity(width * height);
    match texture.format {
        COMPRESSED => {
            let texels = blocks.bytes(blocks.compressed, texture.offset, len)?;
            let attributes = blocks.bytes(blocks.attributes, texture.offset / 2, len / 2)?;
            pixels.resize(width * height, [0; 4]);
            let blocks_per_row = width / 4;
            for (n, texels) in texels.as_chunks::<4>().0.iter().enumerate() {
                let attribute = le16(attributes, n * 2)?;
                let at = usize::from(attribute & 0x3fff) * 2;
                let color = |i: usize| colors.get(at + i, 8);
                let (c0, c1) = (color(0), color(1));
                let palette = match attribute >> 14 {
                    0 => [c0, c1, color(2), [0; 4]],
                    1 => [c0, c1, mix(c0, c1, 4), [0; 4]],
                    2 => [c0, c1, color(2), color(3)],
                    _ => [c0, c1, mix(c0, c1, 3), mix(c0, c1, 5)],
                };
                let word = u32::from_le_bytes(*texels);
                let (left, top) = (n % blocks_per_row * 4, n / blocks_per_row * 4);
                for k in 0..16 {
                    let value = (word >> (2 * k) & 3) as usize;
                    pixels[(top + k / 4) * width + left + k % 4] = palette[value];
                }
            }
        }
        DIRECT => {
            let data = blocks.bytes(blocks.texture, texture.offset, len)?;
            for word in data.as_chunks::<2>().0 {
                let word = u16::from_le_bytes(*word);
                let [_, r, g, b] = bgr555(word).to_be_bytes();
                pixels.push([r, g, b, if word & 0x8000 != 0 { 255 } else { 0 }]);
            }
        }
        _ => {
            let data = blocks.bytes(blocks.texture, texture.offset, len)?;
            for i in 0..width * height {
                let pixel = match texture.format {
                    A3I5 => {
                        let d = usize::from(data[i]);
                        let a3 = d >> 5;
                        let mut color = colors.get(d & 31, 5);
                        color[3] = alpha5(a3 << 2 | a3 >> 1);
                        color
                    }
                    A5I3 => {
                        let d = usize::from(data[i]);
                        let mut color = colors.get(d & 7, 3);
                        color[3] = alpha5(d >> 3);
                        color
                    }
                    PALETTE_4 => {
                        let index = usize::from(data[i / 4] >> (i % 4 * 2) & 3);
                        color0(index, colors.get(index, 2))
                    }
                    PALETTE_16 => {
                        let index = usize::from(data[i / 2] >> (i % 2 * 4) & 15);
                        color0(index, colors.get(index, 4))
                    }
                    _ => {
                        let index = usize::from(data[i]);
                        color0(index, colors.get(index, 8))
                    }
                };
                pixels.push(pixel);
            }
        }
    }
    Some(pixels)
}

/// The first palette whose name `wanted` accepts.
fn find_palette(palettes: &Dict, wanted: impl Fn(&[u8]) -> bool) -> Option<usize> {
    (0..palettes.count).find(|&p| wanted(palettes.name(p)))
}

/// The palette that probably goes with texture number `index` named `name`.
fn match_palette(palettes: &Dict, name: &[u8], index: usize) -> Option<usize> {
    let with_suffix =
        |suffix: &[u8]| find_palette(palettes, |pal| pal.strip_prefix(name) == Some(suffix));
    let longest_affix = (0..palettes.count)
        .filter(|&p| {
            let pal = palettes.name(p);
            pal.len() >= 3 && (name.starts_with(pal) || name.ends_with(pal))
        })
        .max_by_key(|&p| palettes.name(p).len());
    let same_number = (index < palettes.count).then_some(index);
    with_suffix(b"")
        .or_else(|| with_suffix(b"_pl"))
        .or_else(|| with_suffix(b"_p"))
        .or(longest_affix)
        .or(same_number)
        .or((palettes.count > 0).then_some(0))
}

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let chunk = tex0_chunk(data).ok_or(fail)?;
    let at = |offset: usize| le32(chunk, offset).map(|v| v as usize).ok_or(fail);
    let textures = Dict::parse(chunk, usize::from(le16(chunk, 0xe).ok_or(fail)?), 8).ok_or(fail)?;
    let palettes = Dict::parse(chunk, at(0x34)?, 4).ok_or(fail)?;
    let blocks = Blocks {
        chunk,
        texture: at(0x14)?,
        compressed: at(0x24)?,
        attributes: at(0x28)?,
        palette: at(0x38)?,
    };
    // Lay the textures out in rows, skipping those that cannot be read.
    let mut placed = Vec::new();
    let (mut x, mut y, mut row_height, mut sheet_width) = (0, 0, 0, 0);
    for i in 0..textures.count {
        let entry = textures.entry(i, 8);
        let texture = Texture::new(le32(entry, 0).ok_or(fail)?);
        if texture.format == 0 {
            continue;
        }
        let palette = match_palette(&palettes, textures.name(i), i).map(|p| {
            let offset = usize::from(le16(palettes.entry(p, 4), 0).unwrap_or(0)) * 8;
            blocks.palette.saturating_add(offset)
        });
        let colors = Colors {
            data: palette.and_then(|start| chunk.get(start..)),
        };
        let Some(pixels) = render(&texture, &blocks, &colors) else {
            continue;
        };
        let new_row = x > 0 && x + texture.width > SHEET_WIDTH;
        let (left, top) = if new_row {
            (0, y + row_height + GAP)
        } else {
            (x, y)
        };
        if top + texture.height > SHEET_HEIGHT {
            break;
        }
        if new_row {
            row_height = 0;
        }
        placed.push((left, top, texture.width, pixels));
        (x, y) = (left + texture.width + GAP, top);
        row_height = row_height.max(texture.height);
        sheet_width = sheet_width.max(x - GAP);
    }
    if placed.is_empty() {
        return Err(fail);
    }
    let sheet_height = y + row_height;
    check_size(sheet_width, sheet_height)?;
    let mut colors = alloc::vec![SHEET_BACKGROUND; sheet_width * sheet_height];
    for (left, top, width, pixels) in &placed {
        for (n, &pixel) in pixels.iter().enumerate() {
            colors[(top + n / width) * sheet_width + left + n % width] = over_fill(pixel);
        }
    }
    Ok(Image::from_colors(
        sheet_width as u32,
        sheet_height as u32,
        colors.into_iter(),
    ))
}

#[cfg(test)]
mod tests {
    use super::super::testing::{Tex, btx0};
    use super::*;
    use crate::image::TRANSPARENT_FILL;

    /// `TEXIMAGE_PARAM` of a texture of `8 << s` by `8 << t` pixels.
    fn param(format: u32, s: u32, t: u32, color0_transparent: bool) -> u32 {
        format << 26 | u32::from(color0_transparent) << 29 | s << 20 | t << 23
    }

    /// Sixteen colors: white, red, then black.
    fn colors16() -> Vec<u16> {
        let mut colors = alloc::vec![0, 0x7fff, 0x001f];
        colors.resize(16, 0);
        colors
    }

    #[test]
    fn textures_are_laid_out_in_a_row_with_their_palettes() {
        let mut sixteen = [0u8; 32];
        sixteen[0] = 0x10; // pixel 0 = 0, pixel 1 = 1 (low nibble first)
        sixteen[1] = 0x02; // pixel 2 = 2, pixel 3 = 0
        let mut direct = [0u8; 128];
        direct[0..2].copy_from_slice(&0x83e0u16.to_le_bytes()); // green, opaque
        direct[2..4].copy_from_slice(&0x03e0u16.to_le_bytes()); // alpha bit clear
        let file = btx0(
            &[
                Tex {
                    name: "wall",
                    param: param(PALETTE_16, 0, 0, true),
                    data: &sixteen,
                    attributes: &[],
                },
                Tex {
                    name: "sky",
                    param: param(DIRECT, 0, 0, false),
                    data: &direct,
                    attributes: &[],
                },
            ],
            &[("wall", &colors16())],
        );
        let image = decode(&file).unwrap();
        assert_eq!((image.width(), image.height()), (20, 8));
        // Color 0 of the first texture is transparent, color 1 is white.
        assert_eq!(image.get(0, 0), TRANSPARENT_FILL);
        assert_eq!(image.get(1, 0), 0xff_ffff);
        assert_eq!(image.get(2, 0), 0xff_0000);
        // Four pixels of gap, then the direct texture.
        assert_eq!(image.get(9, 4), SHEET_BACKGROUND);
        assert_eq!(image.get(12, 0), 0x00_ff00);
        assert_eq!(image.get(13, 0), TRANSPARENT_FILL);
    }

    #[test]
    fn translucent_formats_scale_their_alpha() {
        let mut a3i5 = [0u8; 64];
        a3i5[0] = 0xe1; // alpha 7 of 7, color 1
        a3i5[1] = 0x21; // alpha 1 of 7, color 1
        let mut a5i3 = [0u8; 64];
        a5i3[0] = 0xf9; // alpha 31 of 31, color 1
        a5i3[1] = 0x09; // alpha 1 of 31
        let mut white = alloc::vec![0, 0x7fff];
        white.resize(32, 0x7fff);
        let file = btx0(
            &[
                Tex {
                    name: "a",
                    param: param(A3I5, 0, 0, false),
                    data: &a3i5,
                    attributes: &[],
                },
                Tex {
                    name: "b",
                    param: param(A5I3, 0, 0, false),
                    data: &a5i3,
                    attributes: &[],
                },
            ],
            &[("a", &white), ("b", &white)],
        );
        let image = decode(&file).unwrap();
        assert_eq!(image.get(0, 0), 0xff_ffff);
        // Alpha 33 of 255 over the grey 0xc0 gives (255 * 33 + 192 * 222 + 127) / 255.
        let low = |alpha: u32| (255 * alpha + 192 * (255 - alpha) + 127) / 255;
        assert_eq!(image.get(1, 0), low(33) * 0x01_0101);
        assert_eq!(image.get(12, 0), 0xff_ffff);
        assert_eq!(image.get(13, 0), low(8) * 0x01_0101);
    }

    #[test]
    fn compressed_blocks_pick_colors_by_mode() {
        // 8 x 8: four blocks in rows. Block 0 uses mode 1 (the third color is
        // the mean, the fourth transparent) with palette colors 0 and 1;
        // block 1 mode 3 with colors 4 and 5.
        let mut texels = [0u8; 16];
        texels[0..4].copy_from_slice(&[0xe4, 0, 0, 0]); // row 0: 0 1 2 3
        texels[4..8].copy_from_slice(&[0xe4, 0, 0, 0]);
        let mut attributes = [0u8; 8];
        attributes[0..2].copy_from_slice(&(1u16 << 14).to_le_bytes());
        attributes[2..4].copy_from_slice(&(3u16 << 14 | 2).to_le_bytes());
        let mut colors = alloc::vec![0u16; 8];
        colors[1] = 0x7fff; // black, white
        colors[5] = 0x7fff; // colors 4 (black) and 5 (white)
        let file = btx0(
            &[Tex {
                name: "c",
                param: param(COMPRESSED, 0, 0, false),
                data: &texels,
                attributes: &attributes,
            }],
            &[("c", &colors)],
        );
        let image = decode(&file).unwrap();
        assert_eq!((image.width(), image.height()), (8, 8));
        assert_eq!(image.get(0, 0), 0);
        assert_eq!(image.get(1, 0), 0xff_ffff);
        assert_eq!(image.get(2, 0), 128 * 0x01_0101);
        assert_eq!(image.get(3, 0), TRANSPARENT_FILL);
        // Mode 3: three eighths and five eighths of the way to white.
        assert_eq!(image.get(6, 0), 96 * 0x01_0101);
        assert_eq!(image.get(7, 0), 159 * 0x01_0101);
    }

    #[test]
    fn palettes_are_matched_by_name_then_by_position() {
        // Two 4-color textures. "sky" has the palette "sky_pl"; "bg_two"
        // ends in the palette name "two". Pixel 0 = color 1 of the palette.
        let data = [0x01u8; 16];
        let red = [0, 0x001f, 0, 0];
        let blue = [0, 0x7c00, 0, 0];
        let green = [0, 0x03e0, 0, 0];
        let textures = |names: [&'static str; 3]| -> Vec<u8> {
            let tex = |name| Tex {
                name,
                param: param(PALETTE_4, 0, 0, false),
                data: &data,
                attributes: &[],
            };
            btx0(
                &[tex(names[0]), tex(names[1]), tex(names[2])],
                &[("other", &green), ("two", &blue), ("sky_pl", &red)],
            )
        };
        let image = decode(&textures(["sky", "bg_two", "zzz"])).unwrap();
        assert_eq!(image.get(0, 0), 0xff_0000); // sky: sky_pl
        assert_eq!(image.get(12, 0), 0x00_00ff); // bg_two: ends with "two"
        // The third texture has no matching name: the palette with its number (2).
        assert_eq!(image.get(24, 0), 0xff_0000);
    }

    #[test]
    fn rows_wrap_at_1024_pixels_and_the_sheet_stops_at_2048() {
        // 512 x 8 textures: the second would end at 1028, so it starts a row.
        let wide = alloc::vec![0u8; 512 * 8];
        let tex = |name| Tex {
            name,
            param: param(PALETTE_256, 6, 0, false),
            data: &wide,
            attributes: &[],
        };
        let file = btx0(&[tex("a"), tex("b"), tex("c")], &[("a", &[0])]);
        let image = decode(&file).unwrap();
        assert_eq!((image.width(), image.height()), (512, 8 + 4 + 8 + 4 + 8));
        // Four 1024 x 512 textures: the fourth would end past row 2048.
        let tall = alloc::vec![0u8; 1024 * 512];
        let tex = |name| Tex {
            name,
            param: param(PALETTE_256, 7, 6, false),
            data: &tall,
            attributes: &[],
        };
        let file = btx0(&[tex("a"), tex("b"), tex("c"), tex("d")], &[("a", &[0])]);
        let image = decode(&file).unwrap();
        assert_eq!((image.width(), image.height()), (1024, 3 * 512 + 2 * 4));
    }

    #[test]
    fn the_container_must_be_sound() {
        let data = [0u8; 32];
        let file = btx0(
            &[Tex {
                name: "a",
                param: param(PALETTE_16, 0, 0, false),
                data: &data,
                attributes: &[],
            }],
            &[("a", &colors16())],
        );
        assert!(decode(&file).is_ok());
        for at in [0usize, 5, 0x10] {
            let mut bad = file.clone();
            bad[at] ^= 0xff;
            assert!(decode(&bad).is_err(), "byte {at:#x}");
        }
        assert!(decode(&file[..file.len() - 1]).is_err());
        assert!(decode(&file[..0x30]).is_err());
        // A texture whose data lies past the end of the file is skipped, and
        // with nothing left the file is rejected.
        let mut bad = file.clone();
        bad[0x14 + 0x14] = 0xff; // data block offset
        assert!(decode(&bad).is_err());
    }
}
