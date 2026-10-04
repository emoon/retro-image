//! Small raw and lightly framed Atari 8-bit screens: TXS, FGE, KFX,
//! CUT, GR9P, RYS, KSS, GHG, PI8 and PI9.
//!
//! Sources:
//! - Format names, sizes and mode summaries: the RECOIL formats list
//!   (<https://recoil.sourceforge.net/formats.html>, a plain list of facts),
//!   Just Solve pages for Floor Designer
//!   (<http://fileformats.archiveteam.org/wiki/Floor_Designer>), TXS
//!   (<http://fileformats.archiveteam.org/wiki/TXS>), KFX
//!   (<http://fileformats.archiveteam.org/wiki/KFX_(Atari_graphics_format)>), Mamut
//!   (<http://fileformats.archiveteam.org/wiki/Mamut>) and Gephard Hires
//!   Graphics (<http://fileformats.archiveteam.org/wiki/Gephard_Hires_Graphics>).
//!   Display modes: De Re Atari ch. 2 and App. E
//!   (<https://www.atariarchives.org/dere/chapt02.php>,
//!   <https://www.atariarchives.org/dere/chaptE.php>).
//! - None of these has a published byte layout. Everything below was reverse
//!   engineered from the corpus samples (KOLO.TXS, LENNA.FGE, RYSUNEK.KFX,
//!   ATARI2.CUT, torus.gr9p, RYS06.RYS, lenna.kss, GOD.GHG, face2.ghg, mori.ghg,
//!   CLEANSE.PI8, MEMMAP.PI8, GOLDGATE/MONROE/NANCY/BOWGIRL.PI9) and then probed
//!   with `recoil2png` on hand-made files (black box): accepted sizes, header
//!   checks, which bytes are ignored, and the fixed colours.
//!
//! Layouts:
//! - TXS: 16x16 greys, one byte per pixel (0-15), behind the 6-byte binary-load
//!   header `FF FF 00 06 FF 06`, drawn 4x4. Bigger values are rejected.
//! - FGE: 64x40 greys, two pixels per byte (high nibble first), behind a 6-byte
//!   header that RECOIL does not look at, drawn 4x4. Exactly 1286 bytes.
//! - KFX 56x60 and CUT 96x99: bare 1-bit bitmaps (black and `0E` white).
//! - GR9P: 80x60 greys, two pixels per byte, 2400 bytes, drawn 4x4.
//! - RYS: 160x96 in the OS colours (Graphics 7 without the colour tail), 3840 bytes.
//! - KSS: 160x160, 40 bytes per line, then the colours of values 0-3. 6404 bytes,
//!   drawn 2x1.
//! - GHG: width (LE16, 1-320), height (1-200), then 1-bit lines of (width + 7) / 8
//!   bytes. A clear bit is `0C`, a set bit `02`.
//! - PI8: 7680 bytes are a Graphics 15 screen in grey colours (`00 04 08 0C`);
//!   7685 bytes are a Graphics 8 screen (black and `0E` white) plus 5 ignored
//!   bytes.
//! - PI9: a Graphics 9 screen with the black background and 4 (7684), 128 (7808) or 256
//!   (7936) ignored bytes; 7720 bytes are an interleaved APAC picture (as APC).
//! - ART (Artist by David Eaton, BLINKY.ART): `07`, COLOR0-2 (708-710), an
//!   ignored byte (711), the background (712), then 80 lines of Graphics 7 data
//!   (3206 bytes), drawn 2x2.
//! - ART (monochrome, DOG.ART): width in bytes - 1 (0-29), height - 1 (0-63),
//!   the bitmap, and one more byte. A set bit is black on a `0E` white page. It
//!   is tried before the Ascii-Art Editor, as in RECOIL.
//! - AGS (Atari Graphics Studio, ags0/ags1/gr9p.ags/ags2): `AGS`, a mode (`0B` or
//!   `13`), the width in bytes, the height (LE16), nine registers, then two
//!   planes of width x height bytes. Mode `13`: the first plane is a Graphics 9
//!   screen, drawn 4x4 (the second plane and the registers are unused). Mode
//!   `0B`: the planes are two Graphics 15 frames shown on alternate scanlines
//!   (frame 1 on even ones), each scanline with its own colours: registers 0-2
//!   and 3 (background) for frame 1, registers 4-6 and 7 for frame 2.

use super::antic::Bitmap;
use super::palette::{register_rgb, rgb};
use super::screen::{GREY_COLORS, OS_COLORS, bitmap, exactly, four_color, gtia9, hires};
use crate::bytes::le16;
use crate::image::check_size;
use crate::{BitOrder, DecodeError, Image};

/// A 4x4-scaled picture of `width` x `height` pixels, one grey level (0-15)
/// per entry of `levels`.
fn grey_blocks(levels: impl Iterator<Item = u8>, width: u32, height: u32) -> Image {
    let mut image = Image::new(width, height);
    for (i, level) in levels.enumerate() {
        image.set(i as u32 % width, i as u32 / width, rgb(level));
    }
    image.scaled(4, 4)
}

/// TXS: 16x16 greys.
pub(super) fn decode_txs(data: &[u8]) -> Result<Image, DecodeError> {
    let pixels = exactly(data, 262)?
        .strip_prefix(&[0xff, 0xff, 0x00, 0x06, 0xff, 0x06])
        .filter(|pixels| pixels.iter().all(|&level| level <= 15))
        .ok_or(DecodeError::Unrecognized)?;
    Ok(grey_blocks(pixels.iter().copied(), 16, 16))
}

/// Floor Designer: 64x40 greys behind an unchecked 6-byte header.
pub(super) fn decode_fge(data: &[u8]) -> Result<Image, DecodeError> {
    let screen = exactly(data, 1286)?;
    Ok(bitmap(&screen[6..], 32, 4).render(4, 4, |_, level| rgb(level)))
}

/// KFX: 56x60 mono.
pub(super) fn decode_kfx(data: &[u8]) -> Result<Image, DecodeError> {
    mono(exactly(data, 420)?, 56, 60, [rgb(0x00), rgb(0x0e)])
}

/// Cut Creator: 96x99 mono.
pub(super) fn decode_cut(data: &[u8]) -> Result<Image, DecodeError> {
    mono(exactly(data, 1188)?, 96, 99, [rgb(0x00), rgb(0x0e)])
}

fn mono(data: &[u8], width: u32, height: u32, colors: [u32; 2]) -> Result<Image, DecodeError> {
    Image::from_bits(
        width,
        height,
        data,
        width.div_ceil(8) as usize,
        BitOrder::MsbFirst,
        colors,
    )
}

/// Graphics 9+: 80x60 greys.
pub(super) fn decode_gr9p(data: &[u8]) -> Result<Image, DecodeError> {
    Ok(bitmap(exactly(data, 2400)?, 40, 4).render(4, 4, |_, level| rgb(level)))
}

/// Mamut: a Graphics 7 screen in the OS colours.
pub(super) fn decode_rys(data: &[u8]) -> Result<Image, DecodeError> {
    Ok(four_color(
        bitmap(exactly(data, 3840)?, 40, 2),
        2,
        2,
        OS_COLORS,
    ))
}

/// KSS-Paint: 160x160, then the colours of pixel values 0-3.
pub(super) fn decode_kss(data: &[u8]) -> Result<Image, DecodeError> {
    let (screen, colors) = exactly(data, 6404)?.split_at(6400);
    let colors = [colors[0], colors[1], colors[2], colors[3]];
    Ok(four_color(bitmap(screen, 40, 2), 2, 1, colors))
}

/// Gephard Hires Graphics: width (LE16), height, then the bitmap.
pub(super) fn decode_ghg(data: &[u8]) -> Result<Image, DecodeError> {
    let [w0, w1, height, ref bits @ ..] = *data else {
        return Err(DecodeError::Unrecognized);
    };
    let width = u32::from(u16::from_le_bytes([w0, w1]));
    if !(1..=320).contains(&width) || !(1..=200).contains(&height) {
        return Err(DecodeError::Unrecognized);
    }
    let row_len = width.div_ceil(8) as usize;
    if bits.len() != row_len * usize::from(height) {
        return Err(DecodeError::Unrecognized);
    }
    mono(
        bits,
        width,
        u32::from(height),
        [register_rgb(0x0c), register_rgb(0x02)],
    )
}

/// PI8: Graphics 15 in greys (7680 bytes) or Graphics 8 (7685 bytes).
pub(super) fn decode_pi8(data: &[u8]) -> Result<Image, DecodeError> {
    match data.len() {
        7680 => Ok(four_color(bitmap(data, 40, 2), 2, 1, GREY_COLORS)),
        7685 => Ok(hires(bitmap(&data[..7680], 40, 1), rgb(0x00), rgb(0x0e))),
        _ => Err(DecodeError::Unrecognized),
    }
}

/// PI9: Graphics 9, or an interleaved APAC picture in 7720 bytes.
pub(super) fn decode_pi9(data: &[u8]) -> Result<Image, DecodeError> {
    match data.len() {
        7684 | 7808 | 7936 => Ok(gtia9(
            Bitmap {
                data: &data[..7680],
                bytes_per_line: 40,
                lines: 192,
                bits: 4,
            },
            0x00,
        )),
        7720 => super::apac::decode_interleaved(data),
        _ => Err(DecodeError::Unrecognized),
    }
}

/// Artist by David Eaton: mode byte 7, registers 708-712, 80 Graphics 7 lines.
pub(super) fn decode_artist_art(data: &[u8]) -> Result<Image, DecodeError> {
    let data = exactly(data, 3206)?;
    let [7, c0, c1, c2, _, background] = data[..6] else {
        return Err(DecodeError::Unrecognized);
    };
    Ok(four_color(
        bitmap(&data[6..], 40, 2),
        2,
        2,
        [background, c0, c1, c2],
    ))
}

/// Monochrome ART: width and height minus one, bitmap, one spare byte.
pub(super) fn decode_mono_art(data: &[u8]) -> Result<Image, DecodeError> {
    let [wide, high, ..] = *data else {
        return Err(DecodeError::Unrecognized);
    };
    let (row_len, height) = (usize::from(wide) + 1, usize::from(high) + 1);
    if wide > 29 || high > 63 || data.len() != 3 + row_len * height {
        return Err(DecodeError::Unrecognized);
    }
    Image::from_bits(
        row_len as u32 * 8,
        height as u32,
        &data[2..data.len() - 1],
        row_len,
        BitOrder::MsbFirst,
        [rgb(0x0e), rgb(0x00)],
    )
}

/// Atari Graphics Studio.
pub(super) fn decode_ags(data: &[u8]) -> Result<Image, DecodeError> {
    let Some((&[b'A', b'G', b'S', mode, row_bytes], rest)) = data.split_first_chunk::<5>() else {
        return Err(DecodeError::Unrecognized);
    };
    let (row_bytes, height) = (
        usize::from(row_bytes),
        usize::from(le16(rest, 0).unwrap_or(0)),
    );
    let plane_len = row_bytes * height;
    let (Some(registers), Some(planes)) = (rest.get(2..11), rest.get(11..)) else {
        return Err(DecodeError::Unrecognized);
    };
    if plane_len == 0 || planes.len() != 2 * plane_len {
        return Err(DecodeError::Unrecognized);
    }
    // Both modes draw at most 8 pixels per row byte and 4 per line.
    check_size(8 * row_bytes, 4 * height)?;
    let (first, second) = planes.split_at(plane_len);
    match mode {
        0x13 => Ok(bitmap(first, row_bytes, 4).render(4, 4, |_, level| rgb(level))),
        0x0b => {
            let palettes = [
                [registers[3], registers[0], registers[1], registers[2]],
                [registers[7], registers[4], registers[5], registers[6]],
            ];
            let width = row_bytes as u32 * 4;
            let mut image = Image::new(width, 2 * height as u32);
            for (frame, plane) in [first, second].into_iter().enumerate() {
                let screen = bitmap(plane, row_bytes, 2);
                for y in 0..height {
                    for x in 0..width as usize {
                        let register = palettes[frame][usize::from(screen.pixel(x, y))];
                        image.set(x as u32, (2 * y + frame) as u32, register_rgb(register));
                    }
                }
            }
            Ok(image.scaled(2, 1))
        }
        _ => Err(DecodeError::Unrecognized),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn txs_checks_header_and_levels() {
        let mut data = [0u8; 262];
        data[..6].copy_from_slice(&[0xff, 0xff, 0x00, 0x06, 0xff, 0x06]);
        data[6] = 0x0d;
        let image = decode_txs(&data).unwrap();
        assert_eq!((image.width(), image.get(3, 3)), (64, rgb(0x0d)));
        data[7] = 16;
        assert!(decode_txs(&data).is_err());
        data[7] = 0;
        data[0] = 0;
        assert!(decode_txs(&data).is_err());
    }

    #[test]
    fn ghg_size_follows_the_header() {
        let mut data = [0u8; 3 + 2 * 3];
        data[..3].copy_from_slice(&[17, 0, 2]);
        data[3] = 0x80;
        let image = decode_ghg(&data).unwrap();
        assert_eq!((image.width(), image.height()), (17, 2));
        assert_eq!(image.get(0, 0), register_rgb(0x02));
        assert_eq!(image.get(1, 0), register_rgb(0x0c));
        assert!(decode_ghg(&data[..8]).is_err());
        data[..3].copy_from_slice(&[0x41, 1, 2]);
        assert!(decode_ghg(&data).is_err());
    }

    #[test]
    fn pi8_picks_the_mode_by_size() {
        assert_eq!(decode_pi8(&[0; 7680]).unwrap().width(), 320);
        assert_eq!(decode_pi8(&[0; 7685]).unwrap().width(), 320);
        assert!(decode_pi8(&[0; 7684]).is_err());
    }

    #[test]
    fn ags_rejects_huge_pictures() {
        let (row_bytes, height) = (255usize, 9000usize);
        let mut data = alloc::vec![0u8; 16 + 2 * row_bytes * height];
        data[..5].copy_from_slice(&[b'A', b'G', b'S', 0x13, row_bytes as u8]);
        data[5..7].copy_from_slice(&(height as u16).to_le_bytes());
        assert!(decode_ags(&data).is_err());
    }
}
