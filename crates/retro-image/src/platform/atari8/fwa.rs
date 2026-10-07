//! FWA: Fun with Art pictures, 160x192 in 4 colors with colors changed by
//! display list interrupts.
//!
//! Sources:
//! - Just Solve "Fun with Art" (<http://fileformats.archiveteam.org/wiki/Fun_with_Art>)
//!   and ANTIC magazine's "Rapid Graphics Converter"
//!   (<https://www.atarimagazines.com/v4n7/rapidgraphicsconverter.html>):
//!   a Graphics 15 screen with per-line colors changed by DLIs.
//! - De Re Atari ch. 2 (display lists, <https://www.atariarchives.org/dere/chapt02.php>)
//!   and the GTIA color register addresses, Mapping the Atari App. 15
//!   (<https://www.atariarchives.org/mapping/appendix15.php>).
//! - The layout was reverse engineered from the corpus samples and
//!   `recoil2png` output on modified copies. The file is a memory image:
//!   `FE FE`, the colors (background, playfield 0-2), a 202-byte display
//!   list at offset 6 (three 8-line blanks, an `LMS $5000` mode E line, 101
//!   mode E lines, `LMS $6000`, 89 more lines, `JVB`; any line may have the
//!   DLI bit), 54 bytes of viewer code that nothing reads, the screen (4080
//!   bytes for lines 0-101 at offset 262, 4096 bytes later the other 3600),
//!   the length of the handler data (16 bits), then one DLI handler per
//!   interrupt. A handler is the
//!   6502 code `PHA TXA PHA`, `LDA #c`, `STA WSYNC`, `STA reg`, optionally
//!   more `LDA #c`/`STA reg` pairs, and `JSR $06CA`, where the registers are
//!   COLPF2, COLPF1, COLPF0 and COLBK (`$D018`, `$D017`, `$D016`, `$D01A`),
//!   each used at most once and only in that order. The file ends with the
//!   last handler; there must be at least as many handlers as DLI lines. The
//!   registers the n-th handler writes take effect from the line after the
//!   n-th DLI line (the `WSYNC` waits out the line).

use super::antic::Bitmap;
use super::palette::register_rgb;
use crate::bytes::le16;
use crate::{DecodeError, Image};
use alloc::vec::Vec;

const SCREEN: usize = 262;
const SECOND_HALF: usize = SCREEN + 4096;
const TRAILER: usize = SECOND_HALF + 3600;
const HANDLERS: usize = TRAILER + 2;
const LINES: usize = 192;
/// First line of the screen at `$6000`.
const SPLIT: usize = 102;
const DISPLAY_LIST: usize = 6;

/// COLPF2, COLPF1, COLPF0, COLBK: the registers' low address bytes, in the
/// order a handler writes them, and the color of each pixel value.
const REGISTERS: [u8; 4] = [0x18, 0x17, 0x16, 0x1a];

pub(super) fn decode_fwa(data: &[u8]) -> Result<Image, DecodeError> {
    let handler_len = le16(data, TRAILER).map(usize::from);
    let expected = data.len().checked_sub(HANDLERS);
    if !data.starts_with(&[0xfe, 0xfe]) || handler_len.is_none() || handler_len != expected {
        return Err(DecodeError::Invalid);
    }
    let interrupts = interrupt_lines(data).ok_or(DecodeError::Invalid)?;
    let handlers = parse_handlers(&data[HANDLERS..]).ok_or(DecodeError::Invalid)?;
    if handlers.len() < interrupts.iter().filter(|&&dli| dli).count() {
        return Err(DecodeError::Invalid);
    }
    // Registers COLBK, COLPF0, COLPF1, COLPF2 as color values.
    let mut colors = [data[2], data[3], data[4], data[5]];
    let mut handlers = handlers.into_iter();
    let mut image = Image::new(320, LINES as u32)?;
    for (line, &dli) in interrupts.iter().enumerate() {
        let start = if line < SPLIT {
            SCREEN + 40 * line
        } else {
            SECOND_HALF + 40 * (line - SPLIT)
        };
        let row = Bitmap {
            data: &data[start..start + 40],
            bytes_per_line: 40,
            lines: 1,
            bits: 2,
        };
        for x in 0..160 {
            let rgb = register_rgb(colors[usize::from(row.pixel(x, 0))]);
            image.set(2 * x as u32, line as u32, rgb);
            image.set(2 * x as u32 + 1, line as u32, rgb);
        }
        if dli {
            for (register, value) in handlers.next().unwrap_or_default() {
                // Pixel value of each register: COLPF2 = 3, COLPF1 = 2, COLPF0 = 1.
                colors[[3, 2, 1, 0][register]] = value;
            }
        }
    }
    Ok(image)
}

/// Checks the display list and returns which of the 192 lines have the DLI
/// bit set.
fn interrupt_lines(data: &[u8]) -> Option<Vec<bool>> {
    let mut lines = Vec::with_capacity(LINES);
    let mut pos = DISPLAY_LIST;
    if data.get(pos..pos + 3)? != [0x70, 0x70, 0x70] {
        return None;
    }
    pos += 3;
    for (count, address) in [(SPLIT, [0x00, 0x50]), (LINES - SPLIT, [0x00, 0x60])] {
        // The segment's first line is the LMS instruction (mode E, `$40`).
        let lms = *data.get(pos)?;
        if lms & 0x7f != 0x4e || data.get(pos + 1..pos + 3)? != address {
            return None;
        }
        lines.push(lms & 0x80 != 0);
        pos += 3;
        for _ in 1..count {
            let byte = *data.get(pos)?;
            if byte & 0x7f != 0x0e {
                return None;
            }
            lines.push(byte & 0x80 != 0);
            pos += 1;
        }
    }
    (*data.get(pos)? == 0x41).then_some(lines)
}

/// The registers (as indexes into [`REGISTERS`]) and colors a handler writes.
type Writes = Vec<(usize, u8)>;

/// The DLI handlers up to the end of `data`, each as (register index in
/// [`REGISTERS`], color) writes. `None` if the data isn't exactly handlers.
fn parse_handlers(data: &[u8]) -> Option<Vec<Writes>> {
    let mut handlers = Vec::new();
    let mut pos = 0;
    while pos < data.len() {
        let bytes = &data[pos..];
        let rest = bytes.strip_prefix(&[0x48, 0x8a, 0x48])?;
        let (writes, rest) = parse_writes(rest)?;
        let rest = rest.strip_prefix(&[0x20, 0xca, 0x06])?;
        handlers.push(writes);
        pos = data.len() - rest.len();
    }
    Some(handlers)
}

/// `LDA #c, STA WSYNC, STA reg` and further `LDA #c, STA reg` pairs.
fn parse_writes(mut rest: &[u8]) -> Option<(Writes, &[u8])> {
    let mut writes = Writes::new();
    while let [0xa9, color, tail @ ..] = rest {
        let tail = if writes.is_empty() {
            tail.strip_prefix(&[0x8d, 0x0a, 0xd4])?
        } else {
            tail
        };
        let [0x8d, register, 0xd0, tail @ ..] = tail else {
            return None;
        };
        let index = REGISTERS.iter().position(|r| r == register)?;
        if writes
            .last()
            .is_some_and(|&(previous, _)| index <= previous)
        {
            return None;
        }
        writes.push((index, *color));
        rest = tail;
    }
    (!writes.is_empty()).then_some((writes, rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picture(handlers: &[u8], dli_line: Option<usize>) -> Vec<u8> {
        let mut data = alloc::vec![0u8; HANDLERS];
        data[..6].copy_from_slice(&[0xfe, 0xfe, 0x00, 0x0e, 0x00, 0x00]);
        let mut list = alloc::vec![0x70, 0x70, 0x70, 0x4e, 0x00, 0x50];
        list.extend((1..SPLIT).map(|l| if dli_line == Some(l) { 0x8e } else { 0x0e }));
        list.extend_from_slice(&[0x4e, 0x00, 0x60]);
        list.extend((SPLIT + 1..LINES).map(|l| if dli_line == Some(l) { 0x8e } else { 0x0e }));
        list.extend_from_slice(&[0x41, 0x00, 0x06]);
        data[DISPLAY_LIST..DISPLAY_LIST + list.len()].copy_from_slice(&list);
        data[TRAILER..HANDLERS].copy_from_slice(&(handlers.len() as u16).to_le_bytes());
        data.extend_from_slice(handlers);
        data
    }

    #[test]
    fn handler_changes_colour_below_its_line() {
        // Every pixel is playfield 0 (value 1); the handler sets COLPF0.
        let handler = [
            0x48, 0x8a, 0x48, 0xa9, 0x00, 0x8d, 0x0a, 0xd4, 0x8d, 0x16, 0xd0, 0x20, 0xca, 0x06,
        ];
        let mut data = picture(&handler, Some(2));
        data[SCREEN..SECOND_HALF].fill(0x55);
        data[3] = 0x0e;
        let image = decode_fwa(&data).unwrap();
        assert_eq!(image.get(0, 2), register_rgb(0x0e));
        assert_eq!(image.get(0, 3), register_rgb(0x00));
        // A DLI needs its handler.
        assert!(decode_fwa(&picture(&[], Some(2))).is_err());
        assert!(decode_fwa(&picture(&handler[..13], Some(2))).is_err());
    }
}
