//! ZX Spectrum Next screens (NXI, SL2, SLR) and the +3DOS file header.
//!
//! Sources:
//! - Sizes and layouts (NXI: palette first; SL2: palette last; 256x192 row
//!   by row; 320x256 and 640x256 stored column by column, 640x256 at 4 bits
//!   per pixel with the left pixel in the high nibble; SLR 128x96 rows;
//!   RGB332 default palette; RGB333 palette entries as `RRRGGGBB`,
//!   `0000000B`): ZX Spectrum Next wiki, <https://wiki.specnext.dev/File_Formats>,
//!   and SpectraLab `ZX_SPECTRUM_GRAPHICS_GUIDE.md` (MIT), sections NXI, SL2 and
//!   SLR, <https://github.com/Bedazzle/SpectraLab/blob/main/ZX_SPECTRUM_GRAPHICS_GUIDE.md>.
//! - SL2 files with a plain byte palette after the pixels (256 bytes after
//!   320x256, 16 bytes after 640x256): reverse engineered from the
//!   moroz1999/zx-image samples `sl2-320x256.sl2` and `sl2-640x256.sl2`
//!   (<https://github.com/moroz1999/zx-image/tree/master/example>); each
//!   byte is an RGB332 colour.
//! - +3DOS header (`PLUS3DOS`, 0x1A, issue, version, 32-bit LE total length,
//!   checksum in byte 127 = sum of bytes 0-126 modulo 256): Spectrum +3
//!   manual, <https://worldofspectrum.org/ZXSpectrum128+3Manual/chapter8pt27.html>.
//! - RGB333 widened by bit repetition: observed from `recoil2png` output.

use super::screen::{Frame, widen3};
use crate::{DecodeError, Image};

const HEADER_LEN: usize = 128;

/// Payload of a file with a valid +3DOS header, or the data itself when it
/// has none. A header whose length field or checksum is wrong is not one.
pub(super) fn strip_plus3dos(data: &[u8]) -> &[u8] {
    match data.split_first_chunk::<HEADER_LEN>() {
        Some((header, payload))
            if header.starts_with(b"PLUS3DOS\x1a")
                && u32::from_le_bytes([header[11], header[12], header[13], header[14]])
                    as usize
                    == data.len()
                && header[..HEADER_LEN - 1]
                    .iter()
                    .fold(0u8, |sum, &b| sum.wrapping_add(b))
                    == header[HEADER_LEN - 1] =>
        {
            payload
        }
        _ => data,
    }
}

#[derive(Clone, Copy)]
enum Mode {
    /// 256x192, a byte per pixel, row by row.
    Rows,
    /// 320x256, a byte per pixel, column by column.
    Columns,
    /// 640x256, a nibble per pixel, column by column.
    WideColumns,
}

#[derive(Clone, Copy)]
enum Palette<'a> {
    /// Index is an RGB332 colour.
    Default,
    /// 2 bytes per entry, RGB333.
    Rgb333(&'a [u8]),
    /// One RGB332 byte per entry.
    Bytes(&'a [u8]),
}

fn rgb333(high: u8, low: u8) -> u32 {
    let red = widen3(high >> 5);
    let green = widen3((high >> 2) & 7);
    let blue = widen3((high & 3) << 1 | low & 1);
    red << 16 | green << 8 | blue
}

/// RGB332 byte with its blue bits widened to three as the Next does.
fn rgb332(value: u8) -> u32 {
    rgb333(value, (value >> 1) & 1)
}

impl Palette<'_> {
    fn color(self, index: u8) -> u32 {
        let i = usize::from(index);
        match self {
            Palette::Default => rgb332(index),
            Palette::Rgb333(p) => p.get(i * 2..i * 2 + 2).map_or(0, |e| rgb333(e[0], e[1])),
            Palette::Bytes(p) => p.get(i).map_or(0, |&e| rgb332(e)),
        }
    }
}

/// Draws a fixed-size mode; callers have checked that `pixels` is long enough.
fn render(mode: Mode, pixels: &[u8], palette: Palette) -> Image {
    let (width, height) = match mode {
        Mode::Rows => (256, 192),
        Mode::Columns => (320, 256),
        Mode::WideColumns => (640, 256),
    };
    let mut frame = Frame::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let index = match mode {
                Mode::Rows => pixels[y * 256 + x],
                Mode::Columns => pixels[x * 256 + y],
                Mode::WideColumns => {
                    let byte = pixels[x / 2 * 256 + y];
                    if x % 2 == 0 { byte >> 4 } else { byte & 15 }
                }
            };
            frame.set(x, y, palette.color(index));
        }
    }
    frame.into_image()
}

const ROWS_LEN: usize = 256 * 192;
const COLUMNS_LEN: usize = 320 * 256;

/// NXI: palette first (512 bytes of RGB333, or 32 bytes for 640x256), then
/// the pixels; a bare 49152-byte screen uses the default palette.
pub(super) fn decode_nxi(data: &[u8]) -> Result<Image, DecodeError> {
    let (mode, palette_len) = match data.len() {
        ROWS_LEN => (Mode::Rows, 0),
        l if l == ROWS_LEN + 512 => (Mode::Rows, 512),
        l if l == COLUMNS_LEN + 512 => (Mode::Columns, 512),
        l if l == COLUMNS_LEN + 32 => (Mode::WideColumns, 32),
        _ => return Err(DecodeError::Unrecognized),
    };
    let (palette, pixels) = data.split_at(palette_len);
    let palette = if palette.is_empty() {
        Palette::Default
    } else {
        Palette::Rgb333(palette)
    };
    Ok(render(mode, pixels, palette))
}

/// SL2: pixels first, then an optional palette; the file may start with a
/// +3DOS header. A bare 81920-byte file is taken as 320x256 (it could also
/// be 640x256 at 4 bits).
pub(super) fn decode_sl2(data: &[u8]) -> Result<Image, DecodeError> {
    let data = strip_plus3dos(data);
    // (mode, pixel bytes, bytes per palette entry; 0 for the default palette)
    let (mode, pixels_len, entry_len) = match data.len() {
        ROWS_LEN => (Mode::Rows, ROWS_LEN, 0),
        l if l == ROWS_LEN + 512 => (Mode::Rows, ROWS_LEN, 2),
        COLUMNS_LEN => (Mode::Columns, COLUMNS_LEN, 0),
        l if l == COLUMNS_LEN + 256 => (Mode::Columns, COLUMNS_LEN, 1),
        l if l == COLUMNS_LEN + 512 => (Mode::Columns, COLUMNS_LEN, 2),
        l if l == COLUMNS_LEN + 16 => (Mode::WideColumns, COLUMNS_LEN, 1),
        l if l == COLUMNS_LEN + 32 => (Mode::WideColumns, COLUMNS_LEN, 2),
        _ => return Err(DecodeError::Unrecognized),
    };
    let (pixels, tail) = data.split_at(pixels_len);
    let palette = match entry_len {
        1 => Palette::Bytes(tail),
        2 => Palette::Rgb333(tail),
        _ => Palette::Default,
    };
    Ok(render(mode, pixels, palette))
}

/// SLR: 128x96 lores, a byte per pixel in the default palette.
pub(super) fn decode_slr(data: &[u8]) -> Result<Image, DecodeError> {
    let data = strip_plus3dos(data);
    if data.len() != 128 * 96 {
        return Err(DecodeError::Unrecognized);
    }
    let mut frame = Frame::new(128, 96);
    for (i, &index) in data.iter().enumerate() {
        frame.set(i % 128, i / 128, rgb332(index));
    }
    Ok(frame.into_image())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn with_header(payload: &[u8]) -> alloc::vec::Vec<u8> {
        let mut h = vec![0u8; HEADER_LEN];
        h[..9].copy_from_slice(b"PLUS3DOS\x1a");
        let total = (HEADER_LEN + payload.len()) as u32;
        h[11..15].copy_from_slice(&total.to_le_bytes());
        h[127] = h[..127].iter().fold(0u8, |s, &b| s.wrapping_add(b));
        h.extend_from_slice(payload);
        h
    }

    #[test]
    fn rgb333_repeats_bits() {
        assert_eq!(rgb333(0x00, 0), 0x000000);
        assert_eq!(rgb333(0xff, 1), 0xffffff);
        assert_eq!(rgb333(0x24, 0), 0x242400);
        assert_eq!(rgb332(0xff), 0xffffff);
    }

    #[test]
    fn plus3dos_header_needs_length_and_checksum() {
        let file = with_header(&[1, 2, 3]);
        assert_eq!(strip_plus3dos(&file), &[1, 2, 3]);
        let mut bad = file.clone();
        bad[127] ^= 1;
        assert_eq!(strip_plus3dos(&bad).len(), bad.len());
    }

    #[test]
    fn wide_columns_put_left_pixel_in_high_nibble() {
        let mut data = vec![0u8; COLUMNS_LEN + 16];
        data[0] = 0x10; // x 0 colour 1, x 1 colour 0
        data[COLUMNS_LEN + 1] = 0xff;
        let image = decode_sl2(&data).unwrap();
        assert_eq!(&image.rgb()[..3], &[255, 255, 255]);
        assert_eq!(&image.rgb()[3..6], &[0, 0, 0]);
    }
}
