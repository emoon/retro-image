//! Amstrad CPC snapshots (`SNA`, versions 1 to 3): the screen the CRTC
//! would be showing, drawn from the Gate Array pens and the CRTC registers
//! saved in the header.
//!
//! Sources:
//! - Snapshot layout (256-byte `MV - SNA` header, Gate Array pens at 0x2F,
//!   screen mode in the multi-configuration byte at 0x40, CRTC registers at
//!   0x43, memory dump size at 0x6B, memory dump at 0x100):
//!   <https://cpctech.cpcwiki.de/docs/snapshot.html>.
//! - Version 3 chunks (`MEM0`-`MEM8`, an 8-byte header of tag and 32-bit
//!   length) and their run-length coding (`0xE5 n v` repeats `v` n times,
//!   `0xE5 0` is a literal `0xE5`; a chunk of exactly 65536 bytes is not
//!   coded): <https://www.cpcwiki.eu/index.php/Snapshot>, as summarised in
//!   `docs/research/next-zx-misc.md`.
//! - CRTC screen addressing (R1 characters per line of 2 bytes, R6 rows, R9
//!   lines per row minus one, R12/R13 start address; page in R12 bits 5-4;
//!   2 KB wrap of a normal 16K screen, free-running counter when R12 bits
//!   3-2 select a 32K screen; each scan line is 0x800 further):
//!   <https://cpctech.cpcwiki.de/docs/screen.html> and
//!   <https://www.cpcwiki.eu/index.php/CRTC>. The 32K behavior agrees with
//!   the overscan screens in `overscan.rs`, which were read from loaders.
//! - Colors and pixel packing: `hardware.rs`.
//! - There is no RECOIL support for snapshots; the render was checked by eye
//!   on the samples in `corpus/extra/cpc-snapshots`.
//!
//! A snapshot is one instant: palette or mode changes made mid-frame by
//! raster code are lost, and the CPC Plus ASIC palette, sprites and split
//! screens (the `CPC+` chunk) are not applied. The video chip always reads the
//! first 64 KB, so the RAM configuration port is not needed.

use alloc::borrow::Cow;
use alloc::vec::Vec;

use super::hardware::{Mode, hardware_color, render};
use crate::bytes::{le16, le32};
use crate::{DecodeError, Image};

const HEADER_LEN: usize = 0x100;
const RAM_LEN: usize = 0x1_0000;
const MAGIC: &[u8; 8] = b"MV - SNA";
const PENS_AT: usize = 0x2f;
const MODE_AT: usize = 0x40;
const CRTC_AT: usize = 0x43;
const DUMP_KB_AT: usize = 0x6b;
const RLE_ESCAPE: u8 = 0xe5;

/// The first 64 KB of RAM, which is what the video chip reads.
fn video_ram(data: &[u8]) -> Option<Cow<'_, [u8]>> {
    let body = data.get(HEADER_LEN..)?;
    match le16(data, DUMP_KB_AT)? {
        64 | 128 => body.get(..RAM_LEN).map(Cow::Borrowed),
        0 => chunk(body, b"MEM0").map(Cow::Owned),
        _ => None,
    }
}

/// Contents of the first chunk tagged `id`, run-length decoded unless it
/// is exactly 64 KB long.
fn chunk(mut body: &[u8], id: &[u8; 4]) -> Option<Vec<u8>> {
    while body.len() >= 8 {
        let len = le32(body, 4)? as usize;
        let content = body.get(8..8usize.checked_add(len)?)?;
        if &body[..4] == id {
            return if len == RAM_LEN {
                Some(content.to_vec())
            } else {
                unpack_rle(content)
            };
        }
        body = &body[8 + len..];
    }
    None
}

/// Expands `0xE5 n v` runs to exactly 64 KB.
fn unpack_rle(mut rest: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(RAM_LEN);
    while let Some((&byte, tail)) = rest.split_first() {
        rest = tail;
        if byte != RLE_ESCAPE {
            out.push(byte);
            continue;
        }
        let (&count, tail) = rest.split_first()?;
        rest = tail;
        if count == 0 {
            out.push(RLE_ESCAPE);
        } else {
            let (&value, tail) = rest.split_first()?;
            rest = tail;
            out.resize(out.len() + usize::from(count), value);
        }
        if out.len() > RAM_LEN {
            return None;
        }
    }
    (out.len() == RAM_LEN).then_some(out)
}

/// Address of the byte at character `column` of character row `row`, scan
/// line `line` of the screen the CRTC scans out.
struct Crtc {
    start: usize,
    chars_per_line: usize,
    free_running: bool,
}

impl Crtc {
    fn address(&self, row: usize, column: usize, line: usize) -> usize {
        let counter = self.start + row * self.chars_per_line + column;
        let counter = if self.free_running {
            counter & 0x3fff
        } else {
            (self.start & 0x3000) | (counter & 0x3ff)
        };
        (counter & 0x3000) << 2 | (counter & 0x3ff) << 1 | line << 11
    }
}

pub(super) fn decode_sna(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    if !data.starts_with(MAGIC) {
        return Err(fail);
    }
    let ram = video_ram(data).ok_or(fail)?;
    let crtc_register = |n: usize| usize::from(data[CRTC_AT + n]);
    let chars_per_line = crtc_register(1);
    let rows = crtc_register(6);
    let lines_per_row = crtc_register(9) + 1;
    if !(1..=64).contains(&chars_per_line) || !(1..=64).contains(&rows) || lines_per_row > 8 {
        return Err(fail);
    }
    let mode = Mode::from_number(data[MODE_AT] & 3).ok_or(fail)?;
    let mut pens = [0; 16];
    for (pen, &value) in pens.iter_mut().zip(&data[PENS_AT..]) {
        *pen = hardware_color(value);
    }

    let r12 = crtc_register(12);
    let crtc = Crtc {
        start: (r12 & 0x3f) << 8 | crtc_register(13),
        chars_per_line,
        free_running: r12 >> 2 & 3 == 3,
    };
    let line_bytes = chars_per_line * 2;
    let mut screen = Vec::with_capacity(line_bytes * rows * lines_per_row);
    for row in 0..rows {
        for line in 0..lines_per_row {
            for column in 0..chars_per_line {
                let at = crtc.address(row, column, line);
                screen.extend_from_slice(&ram[at..at + 2]);
            }
        }
    }
    let width = line_bytes * mode.pixels_per_byte();
    render(
        mode,
        width,
        rows * lines_per_row,
        |y| &screen[y * line_bytes..][..line_bytes],
        &pens,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rle_expands_runs_and_literal_escape() {
        let mut packed = alloc::vec![0xe5, 0x00, 0x11, 0xe5, 0x02, 0x22];
        let mut rest = RAM_LEN - 4;
        while rest > 0 {
            let n = rest.min(255);
            packed.extend_from_slice(&[0xe5, n as u8, 0]);
            rest -= n;
        }
        let out = unpack_rle(&packed).unwrap();
        assert_eq!(out[..5], [0xe5, 0x11, 0x22, 0x22, 0x00]);
        assert_eq!(unpack_rle(&packed[..6]), None);
    }

    #[test]
    fn sixteen_k_screen_wraps_in_two_kilobytes() {
        let crtc = Crtc {
            start: 0x3f0,
            chars_per_line: 40,
            free_running: false,
        };
        // 0x3f0 + 16 wraps to character 0 of the same page.
        assert_eq!(crtc.address(0, 16, 1), 0x800);
        assert_eq!(crtc.address(0, 0, 0), 0x7e0);
    }

    #[test]
    fn overscan_screen_crosses_into_the_next_page() {
        // R12/R13 = 0x0d00: the counter reaches 0x1000 after 0x300 characters.
        let crtc = Crtc {
            start: 0x0d00,
            chars_per_line: 48,
            free_running: true,
        };
        assert_eq!(crtc.address(0, 0, 0), 0x200);
        assert_eq!(crtc.address(16, 0, 0), 0x4000);
    }
}
