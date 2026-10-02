//! Maki-chan Graphics: MAG (`MAKI02`) and MKI (`MAKI01A`/`MAKI01B`), shared by
//! the MSX, NEC PC-80/88/98 and Sharp X68000 platforms.
//!
//! Sources:
//! - Header layouts, flag A / flag B / colour stream decompression, the 15 copy
//!   positions, MKI mask and XOR filter, MSX model flags and YJK:
//!   Kirinn Bunnylin, "Maki-chan Graphics" (<https://mooncore.eu/bunny/txt/makichan.htm>).
//! - YJK conversion: grauw, "The YJK screen modes" (<https://map.grauw.nl/articles/yjk/>).
//! - Observed from `recoil2png` output (sample files and synthesized headers):
//!   - palette precision per machine code: MSX 3 bits; PC-98/PC-88 (codes 0x00,
//!     0x88) 4 bits in 200-line mode, otherwise R5 G6 B5; X68000 (0x68) 5 bits
//!     per channel plus an intensity bit taken from green bit 2; PC-80 (0x80) and
//!     Mac (0x99) 8 bits; 0x62 and 0x70 4 bits (8 bits in 256-colour mode); other
//!     codes 3 bits;
//!   - which images are stretched: 200-line mode (codes 0x00, 0x88 and unknown
//!     codes), always for 0x80, never for 0x62/0x68/0x70/0x99; MSX doubles
//!     height of non-interlaced 512-wide modes and width of interlaced 256-wide
//!     modes; MSX model flag high nibbles 3 and 7+, and bit 3 set, are rejected;
//!   - output starts at the 4-byte aligned left edge and ends at the right edge;
//!   - YJK groups cut by the right edge show their Y as grey;
//!   - MKI: names `X68K` and `MSX2` select X68000 and 3-bit palettes, others 4 bits.

use alloc::vec;
use alloc::vec::Vec;

use super::Machine;
use super::precision::Precision;
use crate::{DecodeError, Image};

/// Largest picture accepted, in output pixels.
const MAX_PIXELS: usize = 1 << 24;

fn le16(data: &[u8], offset: usize) -> Option<usize> {
    let b = data.get(offset..offset + 2)?;
    Some(u16::from_le_bytes([b[0], b[1]]) as usize)
}

fn le32(data: &[u8], offset: usize) -> Option<usize> {
    let b = data.get(offset..offset + 4)?;
    Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
}

fn read_palette(grb: &[u8], precision: Precision) -> Vec<u32> {
    grb.chunks_exact(3)
        .map(|c| precision.rgb(c[1], c[0], c[2]))
        .collect()
}

/// Rows of `0xRRGGBB`, stretched on output.
struct Picture {
    width: usize,
    height: usize,
    pixels: Vec<u32>,
    double_width: bool,
    double_height: bool,
}

impl Picture {
    fn into_image(self) -> Image {
        let (sx, sy) = (
            1 + self.double_width as usize,
            1 + self.double_height as usize,
        );
        let mut image = Image::new((self.width * sx) as u32, (self.height * sy) as u32);
        for y in 0..self.height * sy {
            for x in 0..self.width * sx {
                let colour = self.pixels[(y / sy) * self.width + x / sx];
                image.set(x as u32, y as u32, colour);
            }
        }
        image
    }
}

/// Parsed MAG header; offsets are absolute.
struct MagHeader {
    name: [u8; 4],
    machine: u8,
    flags: u8,
    mode: u8,
    left: usize,
    top: usize,
    right: usize,
    bottom: usize,
    flag_a: usize,
    flag_b: usize,
    colours: usize,
    palette: usize,
}

impl MagHeader {
    fn parse(data: &[u8]) -> Option<Self> {
        if !data.starts_with(b"MAKI02  ") {
            return None;
        }
        let name = data.get(8..12)?.try_into().ok()?;
        let comment_end = 12 + data.get(12..)?.iter().position(|&b| b == 0x1a)?;
        let h = comment_end + data[comment_end..].iter().position(|&b| b == 0)?;
        let header = data.get(h..h + 32)?;
        let at = |offset: usize| h.checked_add(le32(header, offset)?);
        let parsed = Self {
            name,
            machine: header[1],
            flags: header[2],
            mode: header[3],
            left: le16(header, 4)?,
            top: le16(header, 6)?,
            right: le16(header, 8)?,
            bottom: le16(header, 10)?,
            flag_a: at(12)?,
            flag_b: at(16)?,
            colours: at(24)?,
            palette: h + 32,
        };
        (parsed.right >= parsed.left && parsed.bottom >= parsed.top).then_some(parsed)
    }

    fn bits_per_pixel(&self) -> usize {
        if self.mode & 0x80 != 0 { 8 } else { 4 }
    }

    fn machine(&self) -> Machine {
        match (self.machine, &self.name) {
            (0x03, _) => Machine::Msx,
            (0x68, _) | (_, b"X68K") | (_, b"XPST") => Machine::X68000,
            (0x80, _) | (_, b"PC80") => Machine::Pc80,
            (_, b"PCVA") => Machine::Pc88Va,
            (0x88, _) | (_, b"PC88") | (_, b"88SR") => Machine::Pc88,
            _ => Machine::Pc98,
        }
    }
}

/// Decompressed MAG bitmap: `byte_width` bytes per row.
struct Unpacked {
    bytes: Vec<u8>,
    byte_width: usize,
    height: usize,
    /// Output width in source pixels, from the aligned left edge to the right edge.
    width: usize,
}

/// The 15 copy sources: (16-bit units left, rows up).
const COPY_FROM: [(usize, usize); 15] = [
    (1, 0),
    (2, 0),
    (4, 0),
    (0, 1),
    (1, 1),
    (0, 2),
    (1, 2),
    (2, 2),
    (0, 4),
    (1, 4),
    (2, 4),
    (0, 8),
    (1, 8),
    (2, 8),
    (0, 16),
];

fn unpack_mag(data: &[u8], header: &MagHeader) -> Option<Unpacked> {
    let pixels_per_byte = 8 / header.bits_per_pixel();
    let left = (header.left / pixels_per_byte) & !3;
    let right = (header.right / pixels_per_byte + 4) & !3;
    let byte_width = right - left;
    let height = header.bottom - header.top + 1;
    let width = header.right - left * pixels_per_byte + 1;
    if byte_width * pixels_per_byte * height > MAX_PIXELS {
        return None;
    }
    let mut bytes = vec![0u8; byte_width * height];
    let mut action = vec![0u8; byte_width / 4];
    let byte = |i: usize| data.get(i).copied().unwrap_or(0);
    let (mut flag_bit, mut flag_b, mut colour) = (0usize, header.flag_b, header.colours);
    let mut pos = 0;
    let mut slot = 0;
    while pos < bytes.len() {
        let flag_byte = data.get(header.flag_a.checked_add(flag_bit / 8)?)?;
        if flag_byte & (0x80 >> (flag_bit % 8)) != 0 {
            action[slot] ^= byte(flag_b);
            flag_b += 1;
        }
        flag_bit += 1;
        let nibbles = action[slot];
        slot = (slot + 1) % action.len();
        for nibble in [nibbles >> 4, nibbles & 15] {
            if pos >= bytes.len() {
                break;
            }
            let value = match nibble {
                0 => {
                    colour += 2;
                    [byte(colour - 2), byte(colour - 1)]
                }
                n => {
                    let (units, rows) = COPY_FROM[n as usize - 1];
                    match pos.checked_sub(units * 2 + rows * byte_width) {
                        Some(src) => [bytes[src], bytes[src + 1]],
                        None => [0, 0],
                    }
                }
            };
            bytes[pos..pos + 2].copy_from_slice(&value);
            pos += 2;
        }
    }
    Some(Unpacked {
        bytes,
        byte_width,
        height,
        width,
    })
}

/// Indexed pixels at `bits` per pixel, MSB first.
fn indexed(unpacked: &Unpacked, bits: usize, width: usize, palette: &[u32]) -> Vec<u32> {
    let per_byte = 8 / bits;
    let mask = ((1u32 << bits) - 1) as u8;
    let mut pixels = Vec::with_capacity(width * unpacked.height);
    for row in unpacked.bytes.chunks_exact(unpacked.byte_width) {
        for x in 0..width {
            let shift = 8 - bits * (x % per_byte + 1);
            let index = (row[x / per_byte] >> shift) & mask;
            pixels.push(palette.get(index as usize).copied().unwrap_or(0));
        }
    }
    pixels
}

fn yjk_pixels(unpacked: &Unpacked, width: usize, yae: bool, palette: &[u32]) -> Vec<u32> {
    let mut pixels = Vec::with_capacity(width * unpacked.height);
    let mut pal16 = [0; 16];
    for (dst, src) in pal16.iter_mut().zip(palette) {
        *dst = *src;
    }
    for row in unpacked.bytes.chunks_exact(unpacked.byte_width) {
        for x in (0..width).step_by(4) {
            let group = [row[x], row[x + 1], row[x + 2], row[x + 3]];
            let colours = crate::platform::msx::yjk_group(group, yae, &pal16);
            for (i, &colour) in colours.iter().enumerate().take(width - x) {
                let b = group[i];
                pixels.push(if x + 4 > width && !(yae && b & 8 != 0) {
                    let y = crate::platform::msx::level5(b >> 3);
                    y << 16 | y << 8 | y
                } else {
                    colour
                });
            }
        }
    }
    pixels
}

fn msx_picture(header: &MagHeader, unpacked: &Unpacked, palette: &[u32]) -> Option<Picture> {
    let screen = header.flags >> 4;
    let interlaced = header.flags & 0x0c == 0;
    if header.flags & 8 != 0 {
        return None;
    }
    let bpp = header.bits_per_pixel();
    let (width, pixels) = match screen {
        // Screens 10/11 (YAE) and 12 (YJK): one byte per pixel.
        2 | 4 => {
            let width = unpacked.width * bpp / 8;
            (width, yjk_pixels(unpacked, width, screen == 2, palette))
        }
        // Screen 6: 2-bit pixels whatever the stored depth.
        6 => {
            let width = unpacked.width * bpp / 2;
            (width, indexed(unpacked, 2, width, palette))
        }
        0 | 1 | 5 => (
            unpacked.width,
            indexed(unpacked, bpp, unpacked.width, palette),
        ),
        _ => return None,
    };
    let wide_screen = matches!(screen, 0 | 6);
    Some(Picture {
        width,
        height: unpacked.height,
        pixels,
        double_width: !wide_screen && interlaced,
        double_height: wide_screen && !interlaced,
    })
}

/// Decodes a MAG picture if it was saved on `machine`.
pub(in crate::platform) fn decode_mag(data: &[u8], machine: Machine) -> Result<Image, DecodeError> {
    let header = MagHeader::parse(data).ok_or(DecodeError::Unrecognized)?;
    if header.machine() != machine {
        return Err(DecodeError::Unrecognized);
    }
    let colours = if header.bits_per_pixel() == 8 {
        256
    } else {
        16
    };
    let grb = data
        .get(header.palette..header.palette + colours * 3)
        .ok_or(DecodeError::Unrecognized)?;
    let unpacked = unpack_mag(data, &header).ok_or(DecodeError::Unrecognized)?;
    let mode_200_lines = header.mode & 1 != 0;
    let picture = if header.machine == 0x03 {
        let palette = read_palette(grb, Precision::Bits(3));
        msx_picture(&header, &unpacked, &palette).ok_or(DecodeError::Unrecognized)?
    } else {
        let (precision, double_height) = match header.machine {
            0x00 | 0x88 if mode_200_lines => (Precision::Bits(4), true),
            0x00 | 0x88 => (Precision::Rgb565, false),
            0x68 => (Precision::X68000, false),
            0x80 => (Precision::Bits(8), true),
            0x99 => (Precision::Bits(8), false),
            0x62 | 0x70 if header.mode & 0x80 != 0 => (Precision::Bits(8), false),
            0x62 | 0x70 => (Precision::Bits(4), false),
            _ => (Precision::Bits(3), mode_200_lines),
        };
        let palette = read_palette(grb, precision);
        let bpp = header.bits_per_pixel();
        Picture {
            width: unpacked.width,
            height: unpacked.height,
            pixels: indexed(&unpacked, bpp, unpacked.width, &palette),
            double_width: false,
            double_height,
        }
    };
    if picture.width == 0 {
        return Err(DecodeError::Unrecognized);
    }
    Ok(picture.into_image())
}

/// Decodes a 640x400 MKI picture if it was saved on `machine`.
pub(in crate::platform) fn decode_mki(data: &[u8], machine: Machine) -> Result<Image, DecodeError> {
    let xor_rows = match data.get(..8) {
        Some(b"MAKI01A ") => 2,
        Some(b"MAKI01B ") => 4,
        _ => return Err(DecodeError::Unrecognized),
    };
    const FLAG_A: usize = 96;
    const FLAG_B: usize = FLAG_A + 1000;
    if data.len() < FLAG_B {
        return Err(DecodeError::Unrecognized);
    }
    let (precision, detected) = match &data[8..12] {
        b"X68K" => (Precision::X68000, Machine::X68000),
        b"MSX2" => (Precision::Bits(3), Machine::Msx),
        _ => (Precision::Bits(4), Machine::Pc98),
    };
    if detected != machine {
        return Err(DecodeError::Unrecognized);
    }
    let palette = read_palette(&data[48..96], precision);
    let byte = |i: usize| data.get(i).copied().unwrap_or(0);

    // Mask of non-zero byte pairs, 320x400, filled in 4x4 blocks.
    const MASK_WIDTH: usize = 320;
    let mut mask = vec![false; MASK_WIDTH * 400];
    let mut flag_b = FLAG_B;
    for block in 0..8000 {
        if data[FLAG_A + block / 8] & (0x80 >> (block % 8)) == 0 {
            continue;
        }
        let bits = u16::from_be_bytes([byte(flag_b), byte(flag_b + 1)]);
        flag_b += 2;
        let (bx, by) = (block % 80 * 4, block / 80 * 4);
        for i in 0..16 {
            mask[(by + i / 4) * MASK_WIDTH + bx + i % 4] = bits & (0x8000 >> i) != 0;
        }
    }
    let mut source = flag_b;
    let mut bytes: Vec<u8> = mask
        .iter()
        .map(|&set| {
            if set {
                source += 1;
                byte(source - 1)
            } else {
                0
            }
        })
        .collect();
    for i in xor_rows * MASK_WIDTH..bytes.len() {
        bytes[i] ^= bytes[i - xor_rows * MASK_WIDTH];
    }
    let pixels = bytes
        .iter()
        .flat_map(|&b| [palette[(b >> 4) as usize], palette[(b & 15) as usize]])
        .collect();
    Ok(Picture {
        width: 640,
        height: 400,
        pixels,
        double_width: false,
        double_height: false,
    }
    .into_image())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mag(
        machine: u8,
        flags: u8,
        mode: u8,
        right: u16,
        palette: &[u8],
        streams: &[u8],
    ) -> Vec<u8> {
        let mut data = b"MAKI02  TESTcomment\x1a\0".to_vec();
        let flag_a = 32 + palette.len() as u32;
        data.extend([machine, flags, mode]);
        for v in [0u16, 0, right, 0] {
            data.extend(v.to_le_bytes());
        }
        // Flag A: 2 bytes of zeros, flag B empty, then colours.
        for v in [flag_a, flag_a + 2, 0, flag_a + 2, streams.len() as u32] {
            data.extend(v.to_le_bytes());
        }
        data.extend(palette);
        data.extend([0, 0]);
        data.extend(streams);
        data
    }

    #[test]
    fn mag_reads_colour_stream() {
        let mut palette = [0u8; 48];
        palette[3..6].copy_from_slice(&[0, 0xff, 0]); // colour 1: red
        let data = mag(0x00, 0, 0, 7, &palette, &[0x10, 0x01, 0, 0]);
        let image = decode_mag(&data, Machine::Pc98).unwrap();
        assert_eq!((image.width(), image.height()), (8, 1));
        assert_eq!(&image.rgb()[..6], &[0xff, 0, 0, 0, 0, 0]);
        assert_eq!(&image.rgb()[9..12], &[0xff, 0, 0]);
        assert_eq!(
            decode_mag(&data, Machine::Msx),
            Err(DecodeError::Unrecognized)
        );
    }

    #[test]
    fn mag_rejects_truncated_header() {
        let data = mag(0x00, 0, 0, 7, &[0; 48], &[]);
        for len in 0..60 {
            assert!(decode_mag(&data[..len], Machine::Pc98).is_err());
        }
    }
}
