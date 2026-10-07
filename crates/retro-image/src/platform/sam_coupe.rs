//! SAM Coupe screens in the four video modes.
//!
//! Sources:
//! - Video modes, CLUT color bits, mode 3/4 pixel packing and the ROM's
//!   palette table (`PALTAB`, 40 bytes) followed by the line interrupt
//!   color table (`LINICOLS`): SAM Coupe Technical Manual v3.0,
//!   <https://sam.speccy.cz/systech/sam-coupe_tech-man_v3-0.pdf>.
//! - SimCoupe screenshot (SSX) sizes, screen data followed by the CLUT
//!   (16 entries, 4 in mode 3), and the raw 512x192 color dump: obo's post,
//!   <https://spectrumcomputing.co.uk/forums/viewtopic.php?t=1926>.
//! - SAM BASIC `SCREEN$` files (screen memory, then 40 palette bytes, then
//!   4-byte line interrupt records up to 0xFF; mode 2 attributes 8192 bytes
//!   in), 3-bit channel widening, mode 3 and raw output doubled to 512x384,
//!   which line interrupts take effect, and LCE interlacing: reverse
//!   engineered from samples and `recoil2png` output.

use alloc::vec::Vec;

use crate::image::widen_channel;
use crate::{DecodeError, Format, Image};

pub(super) static FORMATS: &[Format] = &[
    Format::new("SAM Coupe", "Mode 1", &["ss1"], |data| {
        decode_mode(data, Mode::One)
    }),
    Format::new("SAM Coupe", "Mode 2", &["ss2"], |data| {
        decode_mode(data, Mode::Two)
    }),
    Format::new("SAM Coupe", "Mode 3", &["ss3"], |data| {
        decode_mode(data, Mode::Three)
    }),
    Format::new("SAM Coupe", "Mode 4", &["ss4", "scs4"], |data| {
        decode_mode(data, Mode::Four)
    }),
    Format::new("SAM Coupe", "SimCoupe screenshot", &["ssx"], decode_ssx),
    Format::new("SAM Coupe", "256x384 interlace", &["lce"], decode_lce),
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    One,
    Two,
    Three,
    Four,
}

impl Mode {
    /// Length of the screen memory in a `SCREEN$` file.
    fn memory_len(self) -> usize {
        match self {
            Mode::One => 6912,
            Mode::Two => MODE2_FILE_ATTRIBUTES + 6144,
            Mode::Three | Mode::Four => 24576,
        }
    }

    /// Length of the screen data in an SSX screenshot.
    fn ssx_len(self) -> usize {
        match self {
            Mode::One => 6912,
            Mode::Two => 12288,
            Mode::Three | Mode::Four => 24576,
        }
    }

    fn clut_len(self) -> usize {
        if self == Mode::Three { 4 } else { 16 }
    }
}

const HEIGHT: usize = 192;
const BITMAP_LEN: usize = 6144;
/// In `SCREEN$` files, mode 2 attributes keep their memory offset.
const MODE2_FILE_ATTRIBUTES: usize = 8192;
const PALETTE_TABLE_LEN: usize = 40;

/// `SCREEN$` file or SSX screenshot of one mode.
fn decode_mode(data: &[u8], mode: Mode) -> Result<Image, DecodeError> {
    if data.len() == mode.ssx_len() + mode.clut_len() {
        let (screen, clut) = data.split_at(mode.ssx_len());
        let mut clut16 = [0; 16];
        clut16[..clut.len()].copy_from_slice(clut);
        return render(mode, screen, BITMAP_LEN, &Palette::fixed(clut16));
    }
    let (screen, palette, len) = screen_file(data, mode)?;
    if len != data.len() {
        return Err(DecodeError::Invalid);
    }
    render(mode, screen, MODE2_FILE_ATTRIBUTES, &palette)
}

/// Parses the `SCREEN$` file at the start of `data`: screen memory,
/// palette and the file's length.
fn screen_file(data: &[u8], mode: Mode) -> Result<(&[u8], Palette, usize), DecodeError> {
    let screen = data.get(..mode.memory_len()).ok_or(DecodeError::Invalid)?;
    let (mut palette, palette_len) = Palette::from_screen_file(&data[mode.memory_len()..])?;
    if mode == Mode::Three {
        // Pixel values 1 and 2 select CLUT entries 2 and 1 (observed from
        // `recoil2png` output; SSX screenshots don't swap them).
        palette.mode3_entries = [0, 2, 1, 3];
    }
    Ok((screen, palette, mode.memory_len() + palette_len))
}

/// LCE: two mode 4 `SCREEN$` files shown interlaced as 512x384, the first
/// on even lines, each pixel doubled horizontally.
fn decode_lce(data: &[u8]) -> Result<Image, DecodeError> {
    let (first, first_palette, first_len) = screen_file(data, Mode::Four)?;
    let (second, second_palette, second_len) = screen_file(&data[first_len..], Mode::Four)?;
    if first_len + second_len != data.len() {
        return Err(DecodeError::Invalid);
    }
    interlace(&[
        render(Mode::Four, first, 0, &first_palette)?,
        render(Mode::Four, second, 0, &second_palette)?,
    ])
}

/// Shows two 256x192 fields interlaced as 512x384: the first on even lines,
/// each pixel doubled horizontally. Also used by the Spectrum LCE variant.
pub(super) fn interlace(fields: &[Image; 2]) -> Result<Image, DecodeError> {
    let mut image = Image::new(512, 2 * HEIGHT as u32)?;
    for (field, frame) in (0..).zip(fields) {
        for y in 0..frame.height() {
            for x in 0..frame.width() {
                let color = frame.get(x, y);
                image.set(2 * x, 2 * y + field, color);
                image.set(2 * x + 1, 2 * y + field, color);
            }
        }
    }
    Ok(image)
}

const RAW_WIDTH: usize = 512;

fn decode_ssx(data: &[u8]) -> Result<Image, DecodeError> {
    for mode in [Mode::One, Mode::Two, Mode::Three, Mode::Four] {
        if data.len() == mode.ssx_len() + mode.clut_len() {
            return decode_mode(data, mode);
        }
    }
    if data.len() != RAW_WIDTH * HEIGHT {
        return Err(DecodeError::Invalid);
    }
    // One SAM color byte per pixel of the 512x192 display.
    let mut image = Image::new(RAW_WIDTH as u32, HEIGHT as u32)?;
    for (i, &value) in data.iter().enumerate() {
        let (x, y) = ((i % RAW_WIDTH) as u32, (i / RAW_WIDTH) as u32);
        image.set(x, y, color(value));
    }
    image.scaled(1, 2)
}

/// CLUT contents per scan line.
struct Palette {
    lines: Vec<[u8; 16]>,
    /// CLUT entry of each mode 3 pixel value.
    mode3_entries: [u8; 4],
}

impl Palette {
    fn fixed(clut: [u8; 16]) -> Self {
        Self {
            lines: alloc::vec![clut; HEIGHT],
            mode3_entries: [0, 1, 2, 3],
        }
    }

    /// Palette table (16 colors, 4 mode 3 colors, the same again for
    /// the flash phase), then line interrupt records (line, CLUT entry,
    /// color, flash color) ending with 0xFF. Changes apply from the line
    /// after the given one (observed from `recoil2png` output). Returns the
    /// palette and the number of bytes used.
    fn from_screen_file(tail: &[u8]) -> Result<(Self, usize), DecodeError> {
        let table = tail.get(..PALETTE_TABLE_LEN).ok_or(DecodeError::Invalid)?;
        let mut clut = [0; 16];
        clut.copy_from_slice(&table[..16]);
        let mut changes = Vec::new();
        let mut records = tail[PALETTE_TABLE_LEN..].chunks(4);
        loop {
            match records.next() {
                Some([0xff, ..]) => break,
                Some(&[line, entry, value, _]) if entry < 16 => {
                    changes.push((usize::from(line), usize::from(entry), value));
                }
                _ => return Err(DecodeError::Invalid),
            }
        }
        let used = PALETTE_TABLE_LEN + 4 * changes.len() + 1;
        // Stable, so changes within a line apply in file order.
        changes.sort_by_key(|c| c.0);
        let mut pending = changes.iter().peekable();
        let mut lines = Vec::with_capacity(HEIGHT);
        for y in 0..HEIGHT {
            while let Some(&(_, entry, value)) = pending.next_if(|c| c.0 < y) {
                clut[entry] = value;
            }
            lines.push(clut);
        }
        let palette = Self {
            lines,
            mode3_entries: [0, 1, 2, 3],
        };
        Ok((palette, used))
    }

    fn get(&self, y: usize, entry: u8) -> u32 {
        color(self.lines[y][usize::from(entry & 15)])
    }
}

/// CLUT color: bits 0 blue, 1 red, 2 green (low bits), 3 bright (lowest
/// bit of every channel), 4 blue, 5 red, 6 green (high bits). Each 3-bit
/// channel is widened by repeating its bits.
fn color(value: u8) -> u32 {
    let bright = (value >> 3) & 1;
    let channel = |low: u8, high: u8| {
        let level = ((value >> high) & 1) << 2 | ((value >> low) & 1) << 1 | bright;
        widen_channel(u32::from(level), 3)
    };
    channel(1, 5) << 16 | channel(2, 6) << 8 | channel(0, 4)
}

/// Renders a screen. `attributes` is where mode 2 attributes start.
fn render(
    mode: Mode,
    screen: &[u8],
    attributes: usize,
    palette: &Palette,
) -> Result<Image, DecodeError> {
    let width = if mode == Mode::Three { 512 } else { 256 };
    let mut image = Image::new(width as u32, HEIGHT as u32)?;
    for y in 0..HEIGHT {
        for x in 0..width {
            let entry = match mode {
                Mode::One | Mode::Two => {
                    let (bitmap, attribute) = if mode == Mode::One {
                        let offset = (y & 0xc0) << 5 | (y & 7) << 8 | (y & 0x38) << 2;
                        (
                            screen[offset + x / 8],
                            screen[BITMAP_LEN + y / 8 * 32 + x / 8],
                        )
                    } else {
                        (screen[y * 32 + x / 8], screen[attributes + y * 32 + x / 8])
                    };
                    let bright = (attribute >> 3) & 8;
                    if bitmap & (0x80 >> (x % 8)) != 0 {
                        bright | attribute & 7
                    } else {
                        bright | (attribute >> 3) & 7
                    }
                }
                Mode::Three => {
                    let value = (screen[y * 128 + x / 4] >> (6 - 2 * (x % 4))) & 3;
                    palette.mode3_entries[usize::from(value)]
                }
                Mode::Four => (screen[y * 128 + x / 2] >> (4 - 4 * (x % 2))) & 15,
            };
            image.set(x as u32, y as u32, palette.get(y, entry));
        }
    }
    // Mode 3 pixels are half as wide as they are high.
    if mode == Mode::Three {
        image.scaled(1, 2)
    } else {
        Ok(image)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_combines_bits_and_bright() {
        assert_eq!(color(0x00), 0x000000);
        assert_eq!(color(0x10), 0x000092);
        assert_eq!(color(0x78), 0xb6b6b6);
        assert_eq!(color(0x7f), 0xffffff);
    }

    #[test]
    fn screen_file_palette_changes_from_next_line() {
        let mut tail = alloc::vec![0u8; PALETTE_TABLE_LEN];
        tail.extend_from_slice(&[10, 3, 0x7f, 0x7f, 0xff]);
        let (palette, len) = Palette::from_screen_file(&tail).unwrap();
        assert_eq!(len, tail.len());
        assert_eq!(palette.get(10, 3), 0);
        assert_eq!(palette.get(11, 3), 0xffffff);
    }
}
