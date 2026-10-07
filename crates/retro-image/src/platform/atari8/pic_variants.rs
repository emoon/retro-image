//! Atari 8-bit `.PIC` files that are not Koala or Magic Painter: raw
//! Micro Illustrator, Graphics 8, and BLASTER pictures with per-line colors.
//!
//! Sources: reverse engineered from samples and `recoil2png` output
//! (`docs/research/gaps-corpus-atari8.md`, section 4), by hypothesis
//! decoding and byte-mutation probing:
//! - BARAHIR.PIC: a 7685-byte Micro Illustrator screen (see `screen.rs`).
//! - SCHALT.PIC: a 7680-byte Graphics 8 screen.
//! - BLASTER.PIC (exactly 4325 bytes): COLPF0-2, an unused byte and COLBK for
//!   line 0, 96 lines of Graphics 15 (160x96), then 480 table bytes: 96
//!   flags and 96-entry tables for PF0, PF1, PF2 and BK. Line `y` > 0 loads
//!   entry `y - 1` of the tables unless bit 7 of flag `y - 1` is set, which
//!   leaves the registers as they were.
//!
//! A 7680-byte `.PIC` named `PAINT?` is APAC 80x96 for RECOIL; the decoder
//! API has no file name, so those files come out as Graphics 8 here.

use super::antic::Bitmap;
use super::palette::register_rgb;
use super::screen::{decode_gr8, decode_mic};
use crate::{DecodeError, Image, NoCompanions};

const LINES: usize = 96;
const LINE: usize = 40;

/// Raw Micro Illustrator screen with its 5-byte color tail.
pub(super) fn decode_mic_pic(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 7685 {
        return Err(DecodeError::Invalid);
    }
    decode_mic(data, &NoCompanions)
}

/// Bare Graphics 8 screen.
pub(super) fn decode_gr8_pic(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 7680 {
        return Err(DecodeError::Invalid);
    }
    decode_gr8(data)
}

/// BLASTER: Graphics 15 with color registers reloaded per line.
pub(super) fn decode_blaster(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 5 + LINES * LINE + 5 * LINES {
        return Err(DecodeError::Invalid);
    }
    let (head, rest) = data.split_at(5);
    let (screen, tables) = rest.split_at(LINES * LINE);
    let table = |index: usize, y: usize| tables[index * LINES + y];
    // Registers per line: background, playfield 0-2.
    let mut registers = [head[4], head[0], head[1], head[2]];
    let mut colors = [[0; 4]; LINES];
    for (y, line) in colors.iter_mut().enumerate() {
        if y > 0 && table(0, y - 1) & 0x80 == 0 {
            registers = [
                table(4, y - 1),
                table(1, y - 1),
                table(2, y - 1),
                table(3, y - 1),
            ];
        }
        *line = registers;
    }
    let bitmap = Bitmap {
        data: screen,
        bytes_per_line: LINE,
        lines: LINES,
        bits: 2,
    };
    bitmap.render(2, 2, |y, value| register_rgb(colors[y][usize::from(value)]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blaster_flag_keeps_registers() {
        let mut data = alloc::vec![0u8; 4325];
        data[..5].copy_from_slice(&[0x28, 0x2a, 0x2c, 0, 0x02]);
        // Line 1 loads entry 0 (PF0 = 0x48); line 2 is held by flag 1.
        data[5 + 3840 + 96] = 0x48;
        data[5 + 3840 + 1] = 0x80;
        data[5 + 3840 + 96 + 1] = 0x66;
        // Pixel value 1 (PF0) on lines 0, 1 and 2.
        for y in 0..3 {
            data[5 + y * 40] = 0x40;
        }
        let image = decode_blaster(&data).unwrap();
        assert_eq!((image.width(), image.height()), (320, 192));
        let top = image.get(0, 0);
        assert_eq!(top, register_rgb(0x28));
        assert_eq!(image.get(0, 2), register_rgb(0x48));
        assert_eq!(image.get(0, 4), register_rgb(0x48));
        data.pop();
        assert!(decode_blaster(&data).is_err());
    }
}
