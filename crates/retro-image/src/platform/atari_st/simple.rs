//! Uncompressed ST screen formats, told apart by size and header.
//!
//! Sources:
//! - NEOchrome, including the 640x400 `0xBABE` virtual canvas:
//!   <https://temlib.org/AtariForumWiki/index.php/NEOchrome_file_format>
//! - Doodle: <https://temlib.org/AtariForumWiki/index.php/Doodle_file_format>
//! - Art Director: <https://temlib.org/AtariForumWiki/index.php/Art_Director_file_format>
//! - GFA Artist ("1000 colours off"):
//!   <https://temlib.org/AtariForumWiki/index.php/GFA_Artist_file_format>
//! - Palette Master: <http://fileformats.archiveteam.org/wiki/Palette_Master>
//! - PaintPro / PlusPaint (`PIC`, single and double height):
//!   <https://temlib.org/AtariForumWiki/index.php?title=PaintPro_ST/PlusPaint_ST>
//! - Dali (`SD0`-`SD2`): <https://temlib.org/AtariForumWiki/index.php/Dali_file_format>
//! - Synthetic Arts: <https://temlib.org/AtariForumWiki/index.php/Synthetic_Arts_file_format>
//! - PaintShop `DA4`: <https://temlib.org/AtariForumWiki/index.php/PaintShop_file_format>
//! - RGB Intermediate: <https://temlib.org/AtariForumWiki/index.php/RGB_Intermediate_file_format>
//! - ColorSTar `BIL` is GFA Artist or DEGAS by size: survey notes in
//!   `docs/formats/atari-st-tt-falcon.md`; checked against `recoil2png` output.
//! - Cyber Paint Cell: <https://temlib.org/AtariForumWiki/index.php/Cyber_Paint_Cell_file_format>,
//!   <http://fileformats.archiveteam.org/wiki/Cyber_Paint_Cell>
//! - DeskPic: <https://temlib.org/AtariForumWiki/index.php/DeskPic_file_format>,
//!   <http://fileformats.archiveteam.org/wiki/DeskPic> (trailing VDI palette)
//! - Sinbad Slideshow: <http://fileformats.archiveteam.org/wiki/Sinbad_Slideshow>
//!   (always 32768 bytes); the screen-then-palette layout is derived from
//!   sample files and `recoil2png` output.
//! - C.O.L.R. Object Editor: <https://temlib.org/AtariForumWiki/index.php/C.O.L.R._Object_Editor_file_format>;
//!   the palette layout is derived from the sample file.
//! - Pablo Paint: <https://temlib.org/AtariForumWiki/index.php/Pablo_Paint_file_format>
//! - Graphics Processor: <http://fileformats.archiveteam.org/wiki/Graphics_Processor>
//!   and survey notes in `docs/formats/atari-st-tt-falcon.md` (raw and RLE
//!   modes); offsets and RLE records derived from sample files.
//! - Atari Image Manager (`IM`, `COL`): no documentation found; derived from
//!   sample files and `recoil2png` output.

use super::common::{
    Resolution, SCREEN_LEN, decode_screen, palette_words, planar_image, st_palette, vdi_palette,
};
use crate::bytes::{be16, be32};
use crate::{Companions, DecodeError, Image};

const NEO_HEADER_LEN: usize = 128;
const CANVAS_FLAG: u16 = 0xbabe;
const DEGAS_LEN: usize = 34 + SCREEN_LEN;

fn ok(image: Option<Image>) -> Result<Image, DecodeError> {
    image.ok_or(DecodeError::Unrecognized)
}

fn words(data: &[u8], offset: usize) -> Result<alloc::vec::Vec<u16>, DecodeError> {
    palette_words(data, offset, 16).ok_or(DecodeError::Unrecognized)
}

/// NEOchrome: flag word, resolution word, 16 palette words, ..., screen at 128.
/// A NEOchrome Master `.RST` file next to a low-resolution picture adds
/// rasters (see `rasters`).
pub(super) fn decode_neo(data: &[u8], companions: &dyn Companions) -> Result<Image, DecodeError> {
    let flag = be16(data, 0).ok_or(DecodeError::Unrecognized)?;
    let words = words(data, 4)?;
    let bitmap = data
        .get(NEO_HEADER_LEN..)
        .ok_or(DecodeError::Unrecognized)?;
    ok(match flag {
        0 if data.len() == NEO_HEADER_LEN + SCREEN_LEN => be16(data, 2)
            .and_then(Resolution::from_index)
            .and_then(|resolution| {
                super::rasters::with_rst(resolution, bitmap, companions)
                    .or_else(|| decode_screen(resolution, bitmap, &words))
            }),
        CANVAS_FLAG if data.len() == NEO_HEADER_LEN + 4 * SCREEN_LEN => {
            planar_image(bitmap, 640, 400, 4, &st_palette(&words), 1)
        }
        _ => None,
    })
}

/// C.O.L.R. Object Editor (`MUR`): a raw low-resolution screen whose
/// palette is in the `.PAL` file next to it, 16 VDI RGB triplets (0-1000)
/// in pen order. Source: <https://temlib.org/AtariForumWiki/index.php/C.O.L.R._Object_Editor_file_format>;
/// the palette layout is derived from the sample file and `recoil2png`
/// output. Without its palette the picture is rejected, as RECOIL does.
pub(super) fn decode_mur(data: &[u8], companions: &dyn Companions) -> Result<Image, DecodeError> {
    let palette = companions
        .get("pal")
        .filter(|pal| pal.len() == MUR_PALETTE_LEN)
        .and_then(|pal| vdi_palette(&pal, 16));
    match palette {
        Some(palette) if data.len() == SCREEN_LEN => {
            ok(planar_image(data, 320, 200, 4, &palette, 1))
        }
        _ => Err(DecodeError::Unrecognized),
    }
}

const MUR_PALETTE_LEN: usize = 16 * 3 * 2;

/// Doodle: a raw 32000-byte high-resolution screen.
pub(super) fn decode_doo(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != SCREEN_LEN {
        return Err(DecodeError::Unrecognized);
    }
    ok(decode_screen(Resolution::High, data, &[]))
}

/// `ART` files: Art Director, GFA Artist, Palette Master or a Doodle-like
/// high-resolution screen, by size.
pub(super) fn decode_art(data: &[u8]) -> Result<Image, DecodeError> {
    match data.len() {
        SCREEN_LEN => decode_doo(data),
        // GFA Artist: palette, then screen.
        32032 => decode_gfa_artist(data),
        // Art Director: screen, 8 palettes, 8 display times, 248 more bytes,
        // of which the 24th (offset 32287) selects the palette shown
        // (derived from sample files and `recoil2png` output).
        32512 => {
            let palette = usize::from(data[SCREEN_LEN + 287]);
            if palette >= 8 {
                return Err(DecodeError::Unrecognized);
            }
            let words = words(data, SCREEN_LEN + palette * 32)?;
            ok(decode_screen(Resolution::Low, data, &words))
        }
        36864 => ok(decode_palette_master(data)),
        // GFA Artist "1000 colours on": planes word, reserved word, screen,
        // normal palette, 69 raster palettes, colour cycling tables.
        34360 => ok(decode_gfa_artist_rasters(data)),
        _ => Err(DecodeError::Unrecognized),
    }
}

/// Palette Master: 32768-byte screen area, then a 16-colour palette and
/// records of a start line and colours 1-15, ended by line 0xFFFF. Colours
/// are always 9-bit ST ones (observed from `recoil2png` output).
fn decode_palette_master(data: &[u8]) -> Option<Image> {
    let all = super::common::words(&data[32768..]);
    let mut palettes: alloc::vec::Vec<(usize, &[u16])> = alloc::vec![(0, &all[..16])];
    for record in all[16..].chunks_exact(16) {
        if record[0] == 0xffff {
            break;
        }
        palettes.push((usize::from(record[0]), record));
    }
    let mut image = Image::new(320, 200);
    for y in 0..200 {
        let (_, palette) = palettes.iter().rev().find(|(line, _)| *line <= y)?;
        let line = &data[y * 160..(y + 1) * 160];
        for x in 0..320 {
            let c = super::common::interleaved_index(line, x, 4);
            // Colour 0 is shared by all palettes.
            let word = if c == 0 { all[0] } else { palette[c] };
            image.set(x, y as u32, super::common::st_rgb(word, false));
        }
    }
    Some(image)
}

/// Line `y` of a GFA Artist "1000 colours" picture uses palette
/// `2 + ceil(y / 3)` of the 70 stored after the screen (the normal one
/// first): derived from the sample file and `recoil2png` output.
fn decode_gfa_artist_rasters(data: &[u8]) -> Option<Image> {
    if be16(data, 0)? != 4 {
        return None;
    }
    let palettes = super::common::words(data.get(32004..32004 + 70 * 32)?);
    let mut image = Image::new(320, 200);
    for y in 0..200usize {
        let palette = &palettes[(2 + y.div_ceil(3)) * 16..][..16];
        let line = &data[4 + y * 160..4 + (y + 1) * 160];
        for x in 0..320 {
            let c = super::common::interleaved_index(line, x, 4);
            image.set(x, y as u32, super::common::st_rgb(palette[c], false));
        }
    }
    Some(image)
}

fn decode_gfa_artist(data: &[u8]) -> Result<Image, DecodeError> {
    let words = words(data, 0)?;
    ok(decode_screen(Resolution::Low, &data[32..], &words))
}

/// ColorSTar `BIL`: GFA Artist layout or a low-resolution DEGAS picture.
pub(super) fn decode_bil(data: &[u8]) -> Result<Image, DecodeError> {
    match data.len() {
        32032 => decode_gfa_artist(data),
        DEGAS_LEN if be16(data, 0) == Some(0) => {
            let words = words(data, 2)?;
            ok(decode_screen(Resolution::Low, &data[34..], &words))
        }
        _ => Err(DecodeError::Unrecognized),
    }
}

/// PaintPro / PlusPaint `PIC`: DEGAS, optionally with a double-height
/// bitmap; also a bare high-resolution screen.
pub(super) fn decode_pic(data: &[u8]) -> Result<Image, DecodeError> {
    let resolution = be16(data, 0).and_then(Resolution::from_index);
    match (data.len(), resolution) {
        (SCREEN_LEN, _) => decode_doo(data),
        (DEGAS_LEN, Some(resolution)) => {
            ok(decode_screen(resolution, &data[34..], &words(data, 2)?))
        }
        (64034, Some(resolution)) => {
            let palette = super::common::screen_palette(resolution, &words(data, 2)?);
            ok(planar_image(
                &data[34..],
                resolution.width(),
                resolution.height() * 2,
                resolution.planes(),
                &palette,
                resolution.y_scale(),
            ))
        }
        _ => Err(DecodeError::Unrecognized),
    }
}

/// Dali `SD0`-`SD2`: long 0, palette, 92 reserved bytes, screen; the
/// resolution comes from the extension.
fn decode_dali(data: &[u8], resolution: Resolution) -> Result<Image, DecodeError> {
    if data.len() != 128 + SCREEN_LEN || data[..4] != [0; 4] {
        return Err(DecodeError::Unrecognized);
    }
    ok(decode_screen(resolution, &data[128..], &words(data, 4)?))
}

pub(super) fn decode_sd0(data: &[u8]) -> Result<Image, DecodeError> {
    decode_dali(data, Resolution::Low)
}

pub(super) fn decode_sd1(data: &[u8]) -> Result<Image, DecodeError> {
    decode_dali(data, Resolution::Medium)
}

pub(super) fn decode_sd2(data: &[u8]) -> Result<Image, DecodeError> {
    decode_dali(data, Resolution::High)
}

/// Synthetic Arts: medium-resolution screen, 3 words, palette.
pub(super) fn decode_srt(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != SCREEN_LEN + 38 {
        return Err(DecodeError::Unrecognized);
    }
    ok(decode_screen(
        Resolution::Medium,
        data,
        &words(data, SCREEN_LEN + 6)?,
    ))
}

/// Cyber Paint Cell: 128-byte header with palette and size, then a
/// low-resolution bitmap of that size.
pub(super) fn decode_cel(data: &[u8]) -> Result<Image, DecodeError> {
    if be16(data, 0) != Some(0xffff) || be16(data, 2) != Some(0) {
        return Err(DecodeError::Unrecognized);
    }
    let width = be16(data, 58).ok_or(DecodeError::Unrecognized)?;
    let height = be16(data, 60).ok_or(DecodeError::Unrecognized)?;
    if !(1..=320).contains(&width) || !(1..=200).contains(&height) {
        return Err(DecodeError::Unrecognized);
    }
    let groups = u32::from(width).div_ceil(16);
    let bitmap = data
        .get(NEO_HEADER_LEN..)
        .ok_or(DecodeError::Unrecognized)?;
    let image = planar_image(
        bitmap,
        groups * 16,
        height.into(),
        4,
        &st_palette(&words(data, 4)?),
        1,
    );
    ok(image.map(|image| super::common::crop(&image, width.into(), height.into())))
}

/// DeskPic: `GF25`, colours, width, height, data size (longs), word-
/// interleaved bitmap, then 256 VDI (0-1000) RGB triplets in pen order.
pub(super) fn decode_gfb(data: &[u8]) -> Result<Image, DecodeError> {
    decode_gfb_inner(data).ok_or(DecodeError::Unrecognized)
}

fn decode_gfb_inner(data: &[u8]) -> Option<Image> {
    if data.get(..4)? != b"GF25" {
        return None;
    }
    let planes = match be32(data, 4)? {
        2 => 1,
        4 => 2,
        16 => 4,
        256 => 8,
        _ => return None,
    };
    let width = be32(data, 8)?;
    let height = be32(data, 12)?;
    let size = be32(data, 16)? as usize;
    if width == 0 || width % 16 != 0 || height == 0 || height > 4096 || width > 4096 {
        return None;
    }
    let bitmap = data.get(20..20usize.checked_add(size)?)?;
    let palette = super::common::vdi_palette(data.get(20 + size..)?, 1 << planes)?;
    planar_image(bitmap, width, height, planes, &palette, 1)
}

/// Pablo Paint (uncompressed only): 36-byte id line, ASCII data size line,
/// resolution byte (0 low, 2 high), compression byte (0), size word,
/// palette, screen.
/// Source: <https://temlib.org/AtariForumWiki/index.php/Pablo_Paint_file_format>
/// (the id line is 36 bytes long in sample files, not 43).
pub(super) fn decode_pablo(data: &[u8]) -> Result<Image, DecodeError> {
    if !data.starts_with(b"PABLO PACKED PICTURE: Groupe CDND \r\n") {
        return Err(DecodeError::Unrecognized);
    }
    let line_end = data[36..]
        .windows(2)
        .position(|w| w == b"\r\n")
        .ok_or(DecodeError::Unrecognized)?;
    let pos = 36 + line_end + 2;
    let resolution = data
        .get(pos)
        .and_then(|&r| Resolution::from_index(r.into()))
        .ok_or(DecodeError::Unrecognized)?;
    if data.get(pos + 1) != Some(&0) {
        // Compression type 29 is undocumented.
        return Err(DecodeError::Unrecognized);
    }
    let words = words(data, pos + 4)?;
    ok(decode_screen(resolution, &data[pos + 36..], &words))
}

/// Graphics Processor: mode word (0-2 raw, 10-12 compressed low, medium
/// or high resolution), palette, more palettes and settings, then at 331
/// the screen, or a data length word and records of a count byte and a
/// unit of one byte per plane: bit 7 set = `count & 0x7f` literal units,
/// else the unit repeated `count` times.
/// Source: survey notes (`docs/formats/atari-st-tt-falcon.md`, raw and RLE
/// modes); the offsets and the RLE records are derived from sample files.
pub(super) fn decode_graphics_processor(data: &[u8]) -> Result<Image, DecodeError> {
    ok(decode_graphics_processor_inner(data))
}

fn decode_graphics_processor_inner(data: &[u8]) -> Option<Image> {
    let mode = be16(data, 0)?;
    let words = palette_words(data, 2, 16)?;
    match mode {
        0..=2 if data.len() == 331 + SCREEN_LEN => {
            decode_screen(Resolution::from_index(mode)?, &data[331..], &words)
        }
        10..=12 => {
            let resolution = Resolution::from_index(mode - 10)?;
            let unit = resolution.planes() as usize;
            let len = usize::from(be16(data, 331)?);
            let packed = data.get(333..333 + len)?;
            let mut bitmap = alloc::vec::Vec::with_capacity(SCREEN_LEN);
            let mut pos = 0;
            while bitmap.len() < SCREEN_LEN {
                let count = usize::from(*packed.get(pos)?);
                pos += 1;
                if count & 0x80 != 0 {
                    let n = (count & 0x7f) * unit;
                    bitmap.extend_from_slice(packed.get(pos..pos + n)?);
                    pos += n;
                } else {
                    let value = packed.get(pos..pos + unit)?;
                    pos += unit;
                    for _ in 0..count {
                        bitmap.extend_from_slice(value);
                    }
                }
            }
            decode_screen(resolution, &bitmap, &words)
        }
        _ => None,
    }
}

/// Side of an Atari Image Manager picture holding `planes` byte planes in
/// `len` bytes: 128 or 256.
fn image_manager_side(len: usize, planes: usize) -> Result<usize, DecodeError> {
    [128, 256]
        .into_iter()
        .find(|side| side * side * planes == len)
        .ok_or(DecodeError::Unrecognized)
}

/// Atari Image Manager `IM`: a square 8-bit grey plane (derived from
/// sample files and `recoil2png` output).
pub(super) fn decode_im(data: &[u8]) -> Result<Image, DecodeError> {
    let side = image_manager_side(data.len(), 1)?;
    let mut image = Image::new(side as u32, side as u32);
    for (i, &v) in data.iter().enumerate() {
        image.set(
            (i % side) as u32,
            (i / side) as u32,
            u32::from(v) * 0x010101,
        );
    }
    Ok(image)
}

/// Atari Image Manager `COL`: four square byte planes, the last three
/// being red, green and blue (derived from sample files and `recoil2png`
/// output).
pub(super) fn decode_aim_col(data: &[u8]) -> Result<Image, DecodeError> {
    let side = image_manager_side(data.len(), 4)?;
    let n = side * side;
    let mut image = Image::new(side as u32, side as u32);
    for i in 0..n {
        let color = u32::from_be_bytes([0, data[n + i], data[2 * n + i], data[3 * n + i]]);
        image.set((i % side) as u32, (i / side) as u32, color);
    }
    Ok(image)
}

/// Sinbad Slideshow: low-resolution screen, palette, padding to 32768.
pub(super) fn decode_ssb(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 32768 {
        return Err(DecodeError::Unrecognized);
    }
    ok(decode_screen(
        Resolution::Low,
        data,
        &words(data, SCREEN_LEN)?,
    ))
}

/// PaintShop `DA4`: raw 640x800 monochrome.
pub(super) fn decode_da4(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 2 * SCREEN_LEN {
        return Err(DecodeError::Unrecognized);
    }
    ok(planar_image(
        data,
        640,
        800,
        1,
        &super::common::MONO_PALETTE,
        1,
    ))
}

/// RGB Intermediate: three low-resolution DEGAS pictures holding the red,
/// green and blue component (0-15) of each pixel.
pub(super) fn decode_rgb(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 3 * DEGAS_LEN {
        return Err(DecodeError::Unrecognized);
    }
    let gun = |i: usize| &data[i * DEGAS_LEN + 34..(i + 1) * DEGAS_LEN];
    let (red, green, blue) = (gun(0), gun(1), gun(2));
    let mut image = Image::new(320, 200);
    for y in 0..200u32 {
        let start = y as usize * 160;
        for x in 0..320 {
            let level = |plane: &[u8]| {
                let line = &plane[start..start + 160];
                super::common::interleaved_index(line, x, 4) as u32 * 0x11
            };
            image.set(x, y, level(red) << 16 | level(green) << 8 | level(blue));
        }
    }
    Ok(image)
}
