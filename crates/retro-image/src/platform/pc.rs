//! IBM PC.
//!
//! Sources:
//! - Microsoft Paint (MSP): Encyclopedia of Graphics File Formats,
//!   <https://www.fileformat.info/format/mspaint/egff.htm> (32-byte header,
//!   "DanM" uncompressed version 1, "LinS" version 2 with a per-line size
//!   map and RLE). Set bit = white: observed from `recoil2png` output.
//! - Award BIOS logo (EPA): Deark `awbm.c` (<https://github.com/jsummers/deark>,
//!   MIT licence): version 1 is a grid of 8x14 character cells (width and
//!   height in cells, one attribute byte per cell, then the cell bitmaps);
//!   version 2 ("AWBM") is a 4-bit planar or 8-bit chunky bitmap followed by
//!   "RGB " and a 6-bit VGA palette (RGB order, as observed from
//!   `recoil2png` output).
//! - Handy Scanner HS2: Deark `misc2.c` (MIT licence): headerless 1-bit
//!   bitmap, 105 bytes (840 pixels) per row.
//! - PCX, Targa, Dr. Halo, BMP and GIF: see `pc/pcx.rs`, `pc/tga.rs`, `pc/halo.rs`,
//!   `pc/bmp.rs`, `pc/gif.rs`, `pc/colorix.rs`
//!   (survey: `docs/research/gaps-pc-japan.md`).
//! - PCPaint/PICtor, Animator PIC/CEL, FLI/FLC and Dr. Halo PIC: see
//!   `pc/pcpaint.rs`, `pc/animator.rs`, `pc/flic.rs`, `pc/flh.rs`, `pc/halo_pic.rs`.
//! - Windows icons and cursors: see `pc/ico.rs`.
//! - CGA palette and the 6-bit to 8-bit palette scaling: observed from
//!   `recoil2png` output.

mod animator;
mod animator_pro;
mod bmp;
mod cga;
mod colorix;
mod dcx;
mod dgi;
mod flf;
mod flh;
mod flic;
mod gif;
mod gifexe;
mod gws_exepic;
mod halo;
mod halo_pic;
mod hp_icn;
mod ico;
mod image72;
mod kips;
mod optiks;
mod os2_icon;
mod pcpaint;
mod pcx;
mod pcx2com;
mod pixit;
mod tga;

use alloc::vec;
use alloc::vec::Vec;

use crate::bytes::le16;
use crate::image::{check_size, planar_pixels};
use crate::{BitOrder, DecodeError, Format, Image};

pub(super) static FORMATS: &[Format] = &[
    // Version 2 first: its "AWBM" header would also pass as version 1 cells.
    Format::new("PC", "Award BIOS logo version 2", &["epa"], decode_awbm).signature(),
    Format::new("PC", "Award BIOS logo", &["epa"], decode_epa_cells),
    Format::new("PC", "Handy Scanner 2000 POSTERING", &["hs2"], decode_hs2),
    Format::new("PC", "Microsoft Paint version 1 or 2", &["msp"], decode_msp).signature(),
    Format::new("PC", "ZSoft PC Paintbrush", &["pcx"], pcx::decode_pcx).signature(),
    Format::new("PC", "ZSoft DCX multi-page PCX", &["dcx"], dcx::decode_dcx).signature(),
    Format::new("PC", "Digi-Pic", &["dgi"], dgi::decode_dgi).signature(),
    Format::new("PC", "HP 100LX/200LX icon", &["icn"], hp_icn::decode_icn).signature(),
    Format::new("PC", "IBM KIPS bitmap", &["kps"], kips::decode_kps),
    Format::new("PC", "Windows and OS/2 bitmap", &["bmp"], bmp::decode_bmp).signature(),
    Format::new(
        "PC",
        "OS/2 icon and pointer",
        &["ico", "ptr"],
        os2_icon::decode_os2_icon,
    )
    .signature(),
    Format::new(
        "PC",
        "Windows DIB without file header",
        &["dib"],
        bmp::decode_dib,
    ),
    Format::new(
        "PC",
        "Windows icon and cursor",
        &["ico", "cur"],
        ico::decode_ico,
    )
    .signature(),
    Format::new("PC", "CompuServe GIF", &["gif", "fra"], gif::decode_gif).signature(),
    Format::new(
        "PC",
        "GIFEXE self-displaying GIF",
        &["exe"],
        gifexe::decode_gifexe,
    )
    .signature(),
    Format::new(
        "PC",
        "Graphic Workshop self-displaying picture",
        &["exe"],
        gws_exepic::decode_gws_exepic,
    )
    .signature(),
    Format::new(
        "PC",
        "OPTIKS self-displaying picture",
        &["com"],
        optiks::decode_optiks,
    )
    .signature(),
    Format::new(
        "PC",
        "PCX2COM self-displaying picture",
        &["com"],
        pcx2com::decode_pcx2com,
    )
    .signature(),
    Format::new(
        "PC",
        "PIXIT self-displaying picture",
        &["com", "exe", "pix"],
        pixit::decode_pixit,
    )
    .signature(),
    Format::new(
        "PC",
        "ColoRIX VGA Paint",
        &["rix", "sci", "scx", "scr"],
        colorix::decode_rix,
    )
    .signature(),
    Format::new("PC", "ColoRIX EGA", &["scr"], colorix::decode_ega_scr),
    Format::new("PC", "PCPaint and PICtor", &["pic"], pcpaint::decode_pic).signature(),
    Format::new("PC", "PCPaint clip", &["clp"], pcpaint::decode_clp),
    Format::new(
        "PC",
        "Autodesk Animator picture and cel",
        &["pic", "cel"],
        animator::decode_cel,
    )
    .signature(),
    Format::new(
        "PC",
        "Autodesk Animator Pro picture and cursor",
        &["pic", "cel", "cur"],
        animator_pro::decode_pic,
    )
    .signature(),
    Format::new(
        "PC",
        "Autodesk Animator FLI and FLC",
        &["fli", "flc", "flh"],
        flic::decode_flic,
    )
    .signature(),
    Format::new("PC", "Dr. Halo PIC", &["pic"], halo_pic::decode_pic).signature(),
    Format::new("PC", "Truevision Targa", &["tga"], tga::decode_tga),
    Format::with_companions("PC", "Dr. Halo", &["cut"], halo::decode_cut),
    Format::new("PC", "Turbo Rascal Syntax Error", &["flf"], flf::decode_flf).signature(),
    Format::new("PC", "Image 72 font", &["fnt"], image72::decode),
];

/// The 16 colours of the IBM CGA/EGA text palette, by attribute value.
/// Also used by the text-mode art formats.
pub(super) const CGA_PALETTE: [u32; 16] = [
    0x000000, 0x0000aa, 0x00aa00, 0x00aaaa, 0xaa0000, 0xaa00aa, 0xaa5500, 0xaaaaaa, 0x555555,
    0x5555ff, 0x55ff55, 0x55ffff, 0xff5555, 0xff55ff, 0xffff55, 0xffffff,
];

/// A CGA 4-colour set: black, then three `CGA_PALETTE` entries.
pub(super) const fn cga_set(colours: [usize; 3]) -> [u32; 4] {
    [
        CGA_PALETTE[0],
        CGA_PALETTE[colours[0]],
        CGA_PALETTE[colours[1]],
        CGA_PALETTE[colours[2]],
    ]
}

const MSP_HEADER_LEN: usize = 32;

fn decode_msp(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let header = data.get(..MSP_HEADER_LEN).ok_or(fail)?;
    let word = |at| le16(header, at).map(usize::from).ok_or(fail);
    let (width, height) = (word(4)?, word(6)?);
    check_size(width, height)?;
    let row_len = width.div_ceil(8);
    let bitmap = match &header[..4] {
        b"DanM" => data
            .get(MSP_HEADER_LEN..MSP_HEADER_LEN + row_len * height)
            .ok_or(fail)?
            .to_vec(),
        b"LinS" => {
            let map_end = MSP_HEADER_LEN + height * 2;
            let map = data.get(MSP_HEADER_LEN..map_end).ok_or(fail)?;
            // A 3-byte run gives at most 255 bytes; this keeps corrupt
            // sizes from allocating huge bitmaps.
            if row_len * height > data.len().saturating_mul(85) {
                return Err(fail);
            }
            let mut bitmap = vec![0u8; row_len * height];
            let mut pos = map_end;
            for (y, size) in map
                .as_chunks::<2>()
                .0
                .iter()
                .map(|w| usize::from(u16::from_le_bytes([w[0], w[1]])))
                .enumerate()
            {
                let line = data.get(pos..pos + size).ok_or(fail)?;
                pos += size;
                unpack_msp_line(line, &mut bitmap[y * row_len..(y + 1) * row_len]);
            }
            bitmap
        }
        _ => return Err(fail),
    };
    mono(&bitmap, width, height, row_len)
}

/// A 1-bit bitmap, most significant bit leftmost, set bit white.
fn mono(bitmap: &[u8], width: usize, height: usize, row_len: usize) -> Result<Image, DecodeError> {
    let colors = [0, 0xffffff];
    Image::from_bits(
        width as u32,
        height as u32,
        bitmap,
        row_len,
        BitOrder::MsbFirst,
        colors,
    )
}

/// `00 count value` is a run; any other byte `n` is followed by `n` literals.
/// Output beyond the line is dropped.
fn unpack_msp_line(src: &[u8], out: &mut [u8]) {
    let mut pos = 0;
    let mut x = 0;
    let mut put = |value: u8| {
        if let Some(slot) = out.get_mut(x) {
            *slot = value;
        }
        x += 1;
    };
    while pos < src.len() {
        let kind = src[pos];
        if kind == 0 {
            let (Some(&count), Some(&value)) = (src.get(pos + 1), src.get(pos + 2)) else {
                return;
            };
            (0..count).for_each(|_| put(value));
            pos += 3;
        } else {
            let literal = &src[pos + 1..src.len().min(pos + 1 + usize::from(kind))];
            literal.iter().for_each(|&b| put(b));
            pos += 1 + usize::from(kind);
        }
    }
}

/// Version 1: attribute bytes (background in the high nibble), then the
/// 14-byte bitmaps of the cells, row by row.
fn decode_epa_cells(data: &[u8]) -> Result<Image, DecodeError> {
    const CELL_HEIGHT: usize = 14;
    let fail = DecodeError::Unrecognized;
    let (columns, rows) = match data {
        [b'A', b'W', b'B', b'M', ..] => return Err(fail),
        [c, r, ..] => (usize::from(*c), usize::from(*r)),
        _ => return Err(fail),
    };
    let cells = columns * rows;
    let bitmaps = 2 + cells;
    if cells == 0 || data.len() < bitmaps + cells * CELL_HEIGHT {
        return Err(fail);
    }
    let (width, height) = (columns * 8, rows * CELL_HEIGHT);
    let indices: Vec<u8> = (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .map(|(x, y)| {
            let cell = y / CELL_HEIGHT * columns + x / 8;
            let attribute = data[2 + cell];
            let bits = data[bitmaps + cell * CELL_HEIGHT + y % CELL_HEIGHT];
            if bits >> (7 - x % 8) & 1 != 0 {
                attribute & 15
            } else {
                attribute >> 4
            }
        })
        .collect();
    Image::from_indexed(width as u32, height as u32, &indices, &CGA_PALETTE)
}

/// Version 2: width, height, bitmap, then "RGB " and the palette.
fn decode_awbm(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let header = data.get(..8).ok_or(fail)?;
    let word = |at| le16(header, at).map(usize::from).ok_or(fail);
    let (width, height) = (word(4)?, word(6)?);
    if &header[..4] != b"AWBM" {
        return Err(fail);
    }
    check_size(width, height)?;
    let palette_at = |bitmap_len: usize, colors: usize| {
        let at = 8 + bitmap_len;
        (data.get(at..at + 4) == Some(b"RGB ") && data.len() >= at + 4 + colors * 3)
            .then_some(at + 4)
    };
    let planar_row = width.div_ceil(8);
    let (colors, palette_start, chunky) = if let Some(at) = palette_at(width * height, 256) {
        (256, at, true)
    } else if let Some(at) = palette_at(planar_row * 4 * height, 16) {
        (16, at, false)
    } else {
        return Err(fail);
    };
    let palette: Vec<u32> = data[palette_start..palette_start + colors * 3]
        .as_chunks::<3>()
        .0
        .iter()
        .map(|c| vga_rgb([c[0], c[1], c[2]]))
        .collect();
    let bitmap = &data[8..];
    let indices: Vec<u8> = if chunky {
        bitmap[..width * height].to_vec()
    } else {
        planar_pixels(bitmap, width, height, planar_row, 4, |plane, y| {
            (y * 4 + plane) * planar_row
        })
        .into_iter()
        .map(|v| v as u8)
        .collect()
    };
    Image::from_indexed(width as u32, height as u32, &indices, &palette)
}

/// One of the 64 EGA colours: bits 0-2 are blue, green, red at 2/3 intensity
/// and bits 3-5 the same at 1/3.
pub(super) fn ega_64(index: u8) -> u32 {
    let level = |high: u8, low: u8| {
        u32::from((index >> high & 1) * 0xaa) + u32::from((index >> low & 1) * 0x55)
    };
    level(2, 5) << 16 | level(1, 4) << 8 | level(0, 3)
}

/// A VGA DAC entry (red, green, blue of 0-63; higher bits ignored) as
/// `0xRRGGBB`, each value scaled `v * 4 + v / 16`. Also used by the
/// text-mode art formats.
pub(super) fn vga_rgb(rgb: [u8; 3]) -> u32 {
    let scale = |v: u8| u32::from((v & 63) << 2 | (v & 63) >> 4);
    scale(rgb[0]) << 16 | scale(rgb[1]) << 8 | scale(rgb[2])
}

/// A VGA DAC value (0-63, larger values saturate) scaled to 8 bits as
/// `round(v * 255 / 63)`. Unlike [`vga_rgb`] this is the rounding Deark uses,
/// which differs from `v * 4 + v / 16` for some values.
pub(super) fn dac_rounded(v: u8) -> u32 {
    (u32::from(v.min(63)) * 510 + 63) / 126
}

fn decode_hs2(data: &[u8]) -> Result<Image, DecodeError> {
    const ROW_LEN: usize = 105;
    if data.is_empty() || !data.len().is_multiple_of(ROW_LEN) {
        return Err(DecodeError::Unrecognized);
    }
    mono(data, ROW_LEN * 8, data.len() / ROW_LEN, ROW_LEN)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn awbm_larger_than_the_pixel_cap_is_rejected() {
        // 16-colour planar: 65535 x 1025 pixels in 32 MiB of planes.
        let (width, height) = (65535usize, 1025usize);
        let bitmap_len = width.div_ceil(8) * 4 * height;
        let mut data = vec![0u8; 8 + bitmap_len + 4 + 16 * 3];
        data[..4].copy_from_slice(b"AWBM");
        data[4..6].copy_from_slice(&(width as u16).to_le_bytes());
        data[6..8].copy_from_slice(&(height as u16).to_le_bytes());
        data[8 + bitmap_len..][..4].copy_from_slice(b"RGB ");
        assert!(decode_awbm(&data).is_err());
    }

    #[test]
    fn msp_larger_than_the_pixel_cap_is_rejected() {
        // 65535 x 8192 pixels: the bitmap passes the 85x-input guard, the
        // picture would be 1.5 GiB of RGB.
        let mut data = vec![0u8; 800_000];
        data[..4].copy_from_slice(b"LinS");
        data[4..6].copy_from_slice(&65535u16.to_le_bytes());
        data[6..8].copy_from_slice(&8192u16.to_le_bytes());
        assert!(decode_msp(&data).is_err());
    }
}
