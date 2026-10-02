//! MSX screen pictures: BSAVE VRAM dumps (`SC2`..`SCC`), Graph Saurus pages
//! (`SR5`..`SR8`, `SRS`) and BASIC `COPY` files (`GL5`..`GLS`, `SH5`..`SHC`).
//!
//! Sources:
//! - BSAVE header (`FE`, start, end, exec) and `COPY ... TO "file"` layout
//!   (width and height as LE16, then packed pixels): MSX2 Technical Handbook,
//!   chapter 2 (<https://konamiman.github.io/MSX2-Technical-Handbook/md/Chapter2.html>).
//! - VRAM maps per screen mode: MSX2 Technical Handbook, appendix 5
//!   (<https://konamiman.github.io/MSX2-Technical-Handbook/md/Appendix5.html>).
//! - Graph Saurus header (`FE` raw / `FD` RLE) and RLE scheme: uniskie GSRLE
//!   readme (<https://github.com/uniskie/MSX_MISC_TOOLS/tree/main/GSRLE>).
//! - Observed from `recoil2png` output: picture height follows the amount of
//!   data (up to 212 lines); 512-wide modes are output with doubled lines; a
//!   bitmap dump uses its palette table whenever the table is complete, a
//!   Screen 2-4 dump only when it is not all zeros, otherwise the default
//!   palette applies; Screen 10/11 dumps without a palette are rejected; Graph Saurus
//!   pages ignore the palette table; Screen 3 dumps that stop before the name
//!   table use the BASIC default name table; `SHx` files decode like `GLx`, and
//!   `GLA`/`GLB`/`SHA`/`SHB` hold YAE pixels (synthesized files); sprites are
//!   drawn only for dumps of exactly 0x4000 (Screens 2/3), 0x4000 or more
//!   (Screen 4), exactly 0x8000 (Screens 5/6) or exactly 0xFAA0 bytes (Screens
//!   7-12 and Graph Saurus Screen 8; other Graph Saurus pages never).
//! - Reverse engineered from samples: Sunrise and msx.org Screen 12 dumps use
//!   the extension `S12`; the MSX Photoshop Graphic Kit wrote a Screen 8 `PIC`
//!   with its 7 header bytes zeroed (accepted only with exactly 212 lines).

use alloc::vec;
use alloc::vec::Vec;

use super::vdp::{self, Palette, SpriteTables, Vram};
use crate::{DecodeError, Image};

/// Bitmap screen modes (V9938 Graphic 4 and up, V9958 YJK).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Bitmap {
    /// Screen 5: 256 wide, 4 bpp.
    Graphic4,
    /// Screen 6: 512 wide, 2 bpp.
    Graphic5,
    /// Screen 7: 512 wide, 4 bpp.
    Graphic6,
    /// Screen 8: 256 wide, `GGGRRRBB`.
    Graphic7,
    /// Screens 10 and 11: YJK with YAE palette pixels.
    Yae,
    /// Screen 12: YJK.
    Yjk,
}

impl Bitmap {
    fn bits_per_pixel(self) -> usize {
        match self {
            Self::Graphic5 => 2,
            Self::Graphic4 | Self::Graphic6 => 4,
            Self::Graphic7 | Self::Yae | Self::Yjk => 8,
        }
    }

    fn screen_width(self) -> usize {
        match self {
            Self::Graphic5 | Self::Graphic6 => 512,
            _ => 256,
        }
    }

    /// Wide modes have half-height pixels, so lines are output twice.
    fn doubles_lines(self) -> bool {
        matches!(self, Self::Graphic5 | Self::Graphic6)
    }

    fn default_palette(self) -> Palette {
        match self {
            Self::Graphic5 => vdp::GRAPHIC5_PALETTE,
            _ => vdp::MSX2_PALETTE,
        }
    }

    /// Palette table address and entry count in VRAM.
    fn palette_table(self) -> Option<(usize, usize)> {
        match self {
            Self::Graphic4 => Some((0x7680, 16)),
            Self::Graphic5 => Some((0x7680, 4)),
            Self::Graphic6 | Self::Yae | Self::Yjk => Some((0xfa80, 16)),
            Self::Graphic7 => None,
        }
    }

    fn sprite_tables(self) -> SpriteTables {
        match self {
            Self::Graphic4 | Self::Graphic5 => SpriteTables {
                attributes: 0x7600,
                patterns: 0x7800,
                colours: Some(0x7400),
            },
            _ => SpriteTables {
                attributes: 0xfa00,
                patterns: 0xf000,
                colours: Some(0xf800),
            },
        }
    }
}

/// Picture being built: one `0xRRGGBB` per pixel.
struct Canvas {
    width: usize,
    height: usize,
    pixels: Vec<u32>,
}

impl Canvas {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; width * height],
        }
    }

    fn set(&mut self, x: usize, y: usize, colour: u32) {
        self.pixels[y * self.width + x] = colour;
    }

    fn into_image(self, double_lines: bool) -> Image {
        let factor = if double_lines { 2 } else { 1 };
        let mut image = Image::new(self.width as u32, (self.height * factor) as u32);
        for (i, &colour) in self.pixels.iter().enumerate() {
            let (x, y) = (i % self.width, i / self.width);
            for copy in 0..factor {
                image.set(x as u32, (y * factor + copy) as u32, colour);
            }
        }
        image
    }
}

/// Decodes `width` x `height` pixels packed back to back (no row padding).
fn draw_packed(mode: Bitmap, packed: &[u8], canvas: &mut Canvas, palette: &Palette) {
    let byte = |i: usize| packed.get(i).copied().unwrap_or(0);
    let bpp = mode.bits_per_pixel();
    for y in 0..canvas.height {
        for x in 0..canvas.width {
            let index = y * canvas.width + x;
            let colour = match mode {
                Bitmap::Graphic7 => vdp::graphic7(byte(index)),
                Bitmap::Yae | Bitmap::Yjk => {
                    // Groups of four start at each row's first pixel.
                    let group = y * canvas.width + (x & !3);
                    let bytes = [
                        byte(group),
                        byte(group + 1),
                        byte(group + 2),
                        byte(group + 3),
                    ];
                    vdp::yjk_group(bytes, mode == Bitmap::Yae, palette)[x & 3]
                }
                _ => {
                    let bit = index * bpp;
                    let shift = 8 - bpp - bit % 8;
                    let value = (byte(bit / 8) >> shift) & ((1 << bpp) - 1) as u8;
                    palette[value as usize]
                }
            };
            canvas.set(x, y, colour);
        }
    }
}

/// Overlays the sprites of a bitmap mode, 256 sprite pixels across the screen.
fn draw_bitmap_sprites(mode: Bitmap, vram: &Vram, canvas: &mut Canvas, palette: &Palette) {
    let tables = mode.sprite_tables();
    let scale = canvas.width / 256;
    for y in 0..canvas.height {
        let line = vdp::sprite_line(vram, tables, y as i32);
        for (x, index) in line.iter().enumerate() {
            let Some(index) = *index else { continue };
            let index = index as usize;
            match mode {
                // Graphic 5 shows colour bits 3-2 on even and 1-0 on odd dots.
                Bitmap::Graphic5 => {
                    canvas.set(2 * x, y, palette[index >> 2]);
                    canvas.set(2 * x + 1, y, palette[index & 3]);
                }
                Bitmap::Graphic7 => canvas.set(x, y, vdp::GRAPHIC7_SPRITE_PALETTE[index]),
                _ => {
                    for dot in 0..scale {
                        canvas.set(scale * x + dot, y, palette[index]);
                    }
                }
            }
        }
    }
}

/// Renders a bitmap-mode VRAM image.
fn render_bitmap(mode: Bitmap, vram: &Vram, use_palette_table: bool) -> Result<Image, DecodeError> {
    let width = mode.screen_width();
    let bytes_per_line = width * mode.bits_per_pixel() / 8;
    let height = (vram.loaded() / bytes_per_line).min(212);
    if height == 0 {
        return Err(DecodeError::Unrecognized);
    }
    let palette = mode
        .palette_table()
        .filter(|_| use_palette_table)
        .and_then(|(address, count)| vram.palette(address, count))
        .unwrap_or_else(|| mode.default_palette());
    let mut canvas = Canvas::new(width, height);
    draw_packed(mode, vram.bytes(), &mut canvas, &palette);
    // Graph Saurus pages show no sprites, except Screen 8 ones.
    if use_palette_table || mode == Bitmap::Graphic7 {
        draw_sprites_if_dumped(mode, vram, &mut canvas, &palette);
    }
    Ok(canvas.into_image(mode.doubles_lines()))
}

/// Draws sprites only for dumps of exactly 0x8000 (Screens 5/6) or 0xFAA0
/// bytes (Screens 7-12), as observed from `recoil2png`.
fn draw_sprites_if_dumped(mode: Bitmap, vram: &Vram, canvas: &mut Canvas, palette: &Palette) {
    let full_dump = match mode {
        Bitmap::Graphic4 | Bitmap::Graphic5 => 0x8000,
        _ => 0xfaa0,
    };
    if vram.loaded() == full_dump {
        draw_bitmap_sprites(mode, vram, canvas, palette);
    }
}

/// Body of a BSAVE file (`FE`, start, end, exec) loaded at address 0.
fn bsave_body(data: &[u8]) -> Option<&[u8]> {
    let header = data.get(..7)?;
    let start = u16::from_le_bytes([header[1], header[2]]) as usize;
    let end = u16::from_le_bytes([header[3], header[4]]) as usize;
    if header[0] != 0xfe || start != 0 {
        return None;
    }
    let len = (end + 1).min(data.len() - 7);
    Some(&data[7..7 + len])
}

/// Size of a full 212-line Screen 8 bitmap.
const GRAPHIC7_BITMAP: usize = 212 * 256;

/// Body of a Screen 8 dump whose BSAVE header was zeroed (seen in the MSX
/// Photoshop Graphic Kit sample): accepted only for a complete bitmap.
fn zeroed_header_body(data: &[u8]) -> Option<&[u8]> {
    let (header, body) = data.split_at_checked(7)?;
    (header.iter().all(|&b| b == 0) && body.len() == GRAPHIC7_BITMAP).then_some(body)
}

/// BSAVE dump of a bitmap screen.
pub(super) fn decode_bitmap_dump(mode: Bitmap, data: &[u8]) -> Result<Image, DecodeError> {
    let body = bsave_body(data)
        .or_else(|| {
            (mode == Bitmap::Graphic7)
                .then(|| zeroed_header_body(data))
                .flatten()
        })
        .ok_or(DecodeError::Unrecognized)?;
    let vram = Vram::new(body);
    // YAE pictures are only accepted with their palette.
    if mode == Bitmap::Yae && vram.palette(0xfa80, 16).is_none() {
        return Err(DecodeError::Unrecognized);
    }
    render_bitmap(mode, &vram, true)
}

/// Graph Saurus page: a BSAVE-like header, `FE` for raw data or `FD` for RLE.
pub(super) fn decode_graph_saurus(mode: Bitmap, data: &[u8]) -> Result<Image, DecodeError> {
    let vram = match data.first() {
        Some(0xfe) => Vram::new(bsave_body(data).ok_or(DecodeError::Unrecognized)?),
        Some(0xfd) if data.len() > 7 => Vram::new(&unpack_graph_saurus(&data[7..])),
        _ => return Err(DecodeError::Unrecognized),
    };
    render_bitmap(mode, &vram, false)
}

/// Graph Saurus RLE: a byte of 16 or more is a literal, 1-15 repeats the next
/// byte that many times, and 0 repeats the byte after next (next byte) times.
fn unpack_graph_saurus(packed: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < packed.len() && out.len() < Vram::SIZE {
        let (count, value, step) = match packed[i] {
            literal @ 16.. => (1, literal, 1),
            0 => match packed.get(i + 1..i + 3) {
                Some(&[count, value]) => (if count == 0 { 256 } else { count as usize }, value, 3),
                _ => break,
            },
            count => match packed.get(i + 1) {
                Some(&value) => (count as usize, value, 2),
                None => break,
            },
        };
        out.extend(core::iter::repeat_n(value, count));
        i += step;
    }
    out
}

/// BASIC `COPY` file: width and height (LE16) followed by packed pixels.
pub(super) fn decode_copy(mode: Bitmap, data: &[u8]) -> Result<Image, DecodeError> {
    let header = data.get(..4).ok_or(DecodeError::Unrecognized)?;
    let width = u16::from_le_bytes([header[0], header[1]]) as usize;
    let height = u16::from_le_bytes([header[2], header[3]]) as usize;
    if width == 0 || height == 0 || width > 512 || height > 1024 {
        return Err(DecodeError::Unrecognized);
    }
    let needed = (width * height * mode.bits_per_pixel()).div_ceil(8);
    let pixels = &data[4..];
    if pixels.len() < needed {
        return Err(DecodeError::Unrecognized);
    }
    let mut canvas = Canvas::new(width, height);
    draw_packed(mode, pixels, &mut canvas, &mode.default_palette());
    Ok(canvas.into_image(mode.doubles_lines()))
}

/// Pattern-based screens of the TMS9918 and V9938.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Tiled {
    /// Screen 2: 256x192, 8x1 pixel colour attributes.
    Graphic2,
    /// Screen 3: 64x48 blocks of 4x4 pixels.
    Multicolour,
    /// Screen 4: Screen 2 with V9938 sprite mode 2.
    Graphic3,
}

/// BSAVE dump of a pattern-based screen.
pub(super) fn decode_tiled_dump(mode: Tiled, data: &[u8]) -> Result<Image, DecodeError> {
    let body = bsave_body(data).ok_or(DecodeError::Unrecognized)?;
    let mut vram = Vram::new(body);
    let (palette_table, minimum) = match mode {
        Tiled::Multicolour => (0x2020, 0x600),
        _ => (0x1b80, 0x3800),
    };
    if vram.loaded() < minimum {
        return Err(DecodeError::Unrecognized);
    }
    let palette = vram
        .palette(palette_table, 16)
        .filter(|_| vram.is_valid_palette(palette_table))
        .unwrap_or(match mode {
            Tiled::Graphic3 => vdp::MSX2_PALETTE,
            _ => vdp::MSX1_PALETTE,
        });
    if mode == Tiled::Multicolour && vram.loaded() <= 0x800 {
        // BASIC's name table: each pattern covers 4 character rows.
        for i in 0..0x300 {
            vram.set(0x800 + i, (i / 128 * 32 + i % 32) as u8);
        }
    }
    let mut canvas = Canvas::new(256, 192);
    for y in 0..192 {
        for x in 0..256 {
            let cell = y / 8 * 32 + x / 8;
            let index = match mode {
                Tiled::Multicolour => {
                    let name = vram.get(0x800 + cell) as usize;
                    let byte = vram.get(name * 8 + ((y / 8) & 3) * 2 + ((y / 4) & 1));
                    if x & 4 == 0 { byte >> 4 } else { byte & 15 }
                }
                _ => {
                    let pattern = (y / 64) * 256 + vram.get(0x1800 + cell) as usize;
                    let bits = vram.get(pattern * 8 + (y & 7));
                    let colours = vram.get(0x2000 + pattern * 8 + (y & 7));
                    if bits & (0x80 >> (x & 7)) != 0 {
                        colours >> 4
                    } else {
                        colours & 15
                    }
                }
            };
            canvas.set(x, y, palette[index as usize]);
        }
    }
    // Sprites are drawn for 16 KiB dumps (Screen 2/3) or at least 16 KiB
    // (Screen 4), as observed from `recoil2png`.
    let with_sprites = match mode {
        Tiled::Graphic3 => vram.loaded() >= 0x4000,
        _ => vram.loaded() == 0x4000,
    };
    if with_sprites {
        let tables = match mode {
            Tiled::Graphic3 => SpriteTables {
                attributes: 0x1e00,
                patterns: 0x3800,
                colours: Some(0x1c00),
            },
            _ => SpriteTables {
                attributes: 0x1b00,
                patterns: 0x3800,
                colours: None,
            },
        };
        for y in 0..192 {
            let line = vdp::sprite_line(&vram, tables, y as i32);
            for (x, index) in line.iter().enumerate() {
                if let Some(index) = *index {
                    canvas.set(x, y, palette[index as usize]);
                }
            }
        }
    }
    Ok(canvas.into_image(false))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_saurus_rle() {
        let packed = [0x20, 3, 5, 0, 2, 7, 0, 0];
        assert_eq!(unpack_graph_saurus(&packed), [0x20, 5, 5, 5, 7, 7]);
    }

    #[test]
    fn bsave_body_honours_end_address() {
        let data = [0xfe, 0, 0, 1, 0, 0, 0, 9, 8, 7];
        assert_eq!(bsave_body(&data), Some(&[9u8, 8][..]));
        assert_eq!(bsave_body(&[0xfe, 1, 0, 1, 0, 0, 0, 9]), None);
        assert_eq!(bsave_body(&[0xfe]), None);
    }

    #[test]
    fn screen5_height_follows_data() {
        let mut data = vec![0xfe, 0, 0, 0xff, 0xff, 0, 0];
        data.extend(core::iter::repeat_n(0x12, 128 * 3 + 5));
        let image = decode_bitmap_dump(Bitmap::Graphic4, &data).unwrap();
        assert_eq!((image.width(), image.height()), (256, 3));
        assert_eq!(&image.rgb()[..6], &[0, 0, 0, 0x24, 0xdb, 0x24]);
    }

    #[test]
    fn screen8_accepts_zeroed_header_only_when_complete() {
        let mut data = vec![0u8; 7 + GRAPHIC7_BITMAP];
        data[7] = 0xff;
        let image = decode_bitmap_dump(Bitmap::Graphic7, &data).unwrap();
        assert_eq!((image.width(), image.height()), (256, 212));
        assert!(decode_bitmap_dump(Bitmap::Graphic7, &data[..data.len() - 1]).is_err());
        assert!(decode_bitmap_dump(Bitmap::Graphic4, &data).is_err());
        data[3] = 1;
        assert!(decode_bitmap_dump(Bitmap::Graphic7, &data).is_err());
    }

    #[test]
    fn copy_file_doubles_wide_mode_lines() {
        let data = [2, 0, 1, 0, 0x1b];
        let image = decode_copy(Bitmap::Graphic5, &data).unwrap();
        assert_eq!((image.width(), image.height()), (2, 2));
        assert_eq!(
            decode_copy(Bitmap::Graphic5, &data[..4]),
            Err(DecodeError::Unrecognized)
        );
    }
}
