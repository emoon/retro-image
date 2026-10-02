//! Raw screen dumps of the OS graphics modes: GR7, GR8, GR9, G10, G11, MIC.
//!
//! Sources:
//! - Modes: De Re Atari ch. 2 and App. E; Mapping the Atari App. 15 (colour
//!   registers 708-712).
//! - GR7, GR8, GR9: Just Solve "GR*" and AtariWiki File Suffix (sizes).
//! - G10: Just Solve "GR*" (7689 bytes = screen + registers 704-712).
//! - MIC: Graph2Font manual (screen + colours 712, 708, 709, 710).
//! - Accepted sizes, the 5-byte MIC tail, the GR8 colour tail, default colours
//!   and the fixed GR9/G11 luminances: observed from `recoil2png` output.

use super::antic::Bitmap;
use super::palette::{register_rgb, rgb};
use crate::{DecodeError, Image};

const LINE: usize = 40;
const MAX_LINES: usize = 240;

/// Splits a dump of whole 40-byte lines (1 to 240) from the bytes after them.
fn lines(data: &[u8]) -> Result<(Bitmap<'_>, &[u8]), DecodeError> {
    let lines = data.len() / LINE;
    if !(1..=MAX_LINES).contains(&lines) {
        return Err(DecodeError::Unrecognized);
    }
    let (screen, tail) = data.split_at(lines * LINE);
    let bitmap = Bitmap {
        data: screen,
        bytes_per_line: LINE,
        lines,
        bits: 1,
    };
    Ok((bitmap, tail))
}

/// Graphics 8: 320 pixels per line, 2 luminances of one hue. A 7682-byte
/// file stores background and foreground luminance after 192 lines.
pub(super) fn decode_gr8(data: &[u8]) -> Result<Image, DecodeError> {
    let (bitmap, tail) = lines(data)?;
    let (background, foreground) = match (data.len(), tail) {
        (7682, &[background, foreground]) => (background & 0x0e, foreground & 0x0e),
        _ => (0x00, 0x0e),
    };
    Ok(bitmap.render(1, 1, |_, value| {
        rgb(if value == 0 { background } else { foreground })
    }))
}

/// Graphics 9: 80 pixels of 16 grey luminances.
pub(super) fn decode_gr9(data: &[u8]) -> Result<Image, DecodeError> {
    let (bitmap, _) = lines(data)?;
    Ok(Bitmap { bits: 4, ..bitmap }.render(4, 1, |_, value| rgb(value)))
}

/// Graphics 11: 80 pixels of 16 hues at luminance 6; hue 0 is black.
pub(super) fn decode_g11(data: &[u8]) -> Result<Image, DecodeError> {
    let (bitmap, _) = lines(data)?;
    Ok(Bitmap { bits: 4, ..bitmap }.render(4, 1, |_, value| {
        if value == 0 {
            0
        } else {
            register_rgb(value << 4 | 6)
        }
    }))
}

/// Graphics 10: 80 pixels indexing the 9 registers 704-712 stored after the screen.
pub(super) fn decode_g10(data: &[u8]) -> Result<Image, DecodeError> {
    let screen_len = data.len().checked_sub(9).ok_or(DecodeError::Unrecognized)?;
    if screen_len % LINE != 0 {
        return Err(DecodeError::Unrecognized);
    }
    let (bitmap, registers) =
        lines(&data[..screen_len]).map(|(bitmap, _)| (bitmap, &data[screen_len..]))?;
    Ok(Bitmap { bits: 4, ..bitmap }.render(4, 1, |_, value| {
        register_rgb(registers[g10_register(value)])
    }))
}

/// GTIA mode 10 maps values 9-11 to the background and 12-15 to playfield 0-3.
fn g10_register(value: u8) -> usize {
    match value {
        0..=8 => usize::from(value),
        9..=11 => 8,
        _ => usize::from(value) - 8,
    }
}

/// Graphics 7: 160x96, 4 colours: screen then background and playfield 0-2.
pub(super) fn decode_gr7(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 3844 {
        return Err(DecodeError::Unrecognized);
    }
    let (bitmap, tail) = lines(data)?;
    let colors = [tail[0], tail[1], tail[2], tail[3]];
    Ok(Bitmap { bits: 2, ..bitmap }
        .render(2, 2, |_, value| register_rgb(colors[usize::from(value)])))
}

/// Micro Illustrator / Graphics 15: 160 pixels, 4 colours. A 4-byte tail is
/// background and playfield 0-2; a 5-byte tail is playfield 0-2, background
/// and an unused byte; otherwise grey defaults apply.
pub(super) fn decode_mic(data: &[u8]) -> Result<Image, DecodeError> {
    let (bitmap, tail) = lines(data)?;
    let colors = match *tail {
        [] | [_, _, _] => [0x00, 0x04, 0x08, 0x0c],
        [background, pf0, pf1, pf2] | [pf0, pf1, pf2, background, _] => [background, pf0, pf1, pf2],
        _ => return Err(DecodeError::Unrecognized),
    };
    Ok(Bitmap { bits: 2, ..bitmap }
        .render(2, 1, |_, value| register_rgb(colors[usize::from(value)])))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn g10_folds_registers() {
        let mapped: [usize; 16] = core::array::from_fn(|v| g10_register(v as u8));
        assert_eq!(mapped, [0, 1, 2, 3, 4, 5, 6, 7, 8, 8, 8, 8, 4, 5, 6, 7]);
    }

    #[test]
    fn mic_tail_orders() {
        let mut data = [0u8; 44];
        data[0] = 0b0001_1011;
        data[40..].copy_from_slice(&[0x02, 0x14, 0x26, 0x38]);
        let four = decode_mic(&data).unwrap();
        assert_eq!(&four.rgb()[..3], &[0x22, 0x22, 0x22]);
        let mut five = [0u8; 45];
        five[..40].copy_from_slice(&data[..40]);
        five[40..].copy_from_slice(&[0x14, 0x26, 0x38, 0x02, 0xff]);
        assert_eq!(decode_mic(&five).unwrap(), four);
        assert_eq!(decode_mic(&[0; 41]), Err(DecodeError::Unrecognized));
    }

    #[test]
    fn rejects_bad_sizes() {
        assert!(decode_gr8(&[0; 39]).is_err());
        assert!(decode_gr8(&[0; 9640]).is_err());
        assert!(decode_g10(&[0; 7690]).is_err());
        assert!(decode_g10(&[0; 9]).is_err());
        assert!(decode_gr7(&[0; 3840]).is_err());
    }
}
