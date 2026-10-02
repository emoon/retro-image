//! MUFLI Editor (`.muf`, `.mup`) and MUIFLI Editor (`.mui`): NUFLI's
//! predecessor, 296×200 hires FLI with a new screen RAM every second line
//! and six X-expanded sprites underneath the bitmap.
//!
//! Sources: reverse engineered from the eight MUP samples (CSDb MUFLI
//! Editor, <https://csdb.dk/release/?id=35737>) and five MUI samples
//! (CSDb Mufflon bonus pack, <https://csdb.dk/release/?id=93314>) by
//! black-box probing of `recoil2png` (flipping every byte of a file and
//! watching which pixels change, and building synthetic files); no format
//! description was found. The mode itself is described on Codebase64,
//! <http://codebase.c64.org/doku.php?id=base:ufli>.
//! - One frame is the memory `$2100-$75FF` (`$5500` bytes), laid out like
//!   [`super::nufli`]: bitmap `$6000` (5120 bytes) + `$3400`, screen RAM and
//!   sprite pointers per line pair (reusing its `screen` and `sprite_rows`),
//!   sprite data through the pointers (bytes 1-6 of each bank's pointer
//!   row), the six colour tables of NUFLI. The FLI-bug sprites are not
//!   drawn: the picture starts at column 3.
//! - A page of masks at `$3300`: entry `ceil(y / 2)` of a line has bit
//!   `s + 1` set to paint the sprite `s` (0-5) pixels of that line white,
//!   whatever the colour table says.
//! - `.mup`: two ignored bytes, an escape byte, then the frame packed
//!   backwards (`value count escape`, count 0 = 256). `.muf`: two ignored
//!   bytes, the frame and 64 ignored bytes (checked by building synthetic
//!   `.muf` files from the unpacked `.mup` samples; `recoil2png` gives the
//!   same pixels; no `.muf` sample exists).
//! - `.mui`: two ignored bytes and `$AC00` bytes holding two frames for an
//!   interlaced pair, blended. Frame 1 is the first `$5500` bytes (address
//!   `$2100` is file offset 0). Frame 2 has the same layout but is stored
//!   in pieces: addresses `$4D00-$75FF` start at file offset `$5600` and
//!   `$2100-$4CFF` at `$8000`.

use super::nufli::{screen, sprite_rows};
use super::unpack::backward_rle;
use super::vic2;
use crate::{DecodeError, Image};
use alloc::vec::Vec;

/// Start of the memory image of one frame.
const LOAD: usize = 0x2100;
/// Bytes of one frame (`$2100-$75FF`).
const LEN: usize = 0x5500;
const HEIGHT: usize = 200;
const BUG: usize = 24;
/// Colour tables of the six underlay sprites (as in NUFLI).
const COLOR_TABLES: [usize; 6] = [0x2400, 0x2480, 0x2800, 0x2880, 0x2c00, 0x2c80];

/// Where the bytes of a frame lie in the file's memory.
#[derive(Clone, Copy)]
enum Layout {
    /// Frame at `$2100` from the start of the data.
    First,
    /// Second frame of a MUIFLI file.
    Second,
}

impl Layout {
    /// Offset of the byte at memory address `addr` of the frame.
    fn offset(self, addr: usize) -> Option<usize> {
        let offset = addr.checked_sub(LOAD)?;
        match self {
            _ if addr >= LOAD + LEN => None,
            Layout::First => Some(offset),
            Layout::Second if addr < 0x4d00 => Some(offset + 0x8000),
            Layout::Second => Some(addr - 0x4d00 + 0x5600),
        }
    }
}

struct Frame<'a> {
    mem: &'a [u8],
    layout: Layout,
    rows: [u8; HEIGHT],
}

impl Frame<'_> {
    fn byte(&self, addr: usize) -> Option<u8> {
        self.mem.get(self.layout.offset(addr)?).copied()
    }

    fn bitmap_set(&self, x: usize, y: usize) -> Option<bool> {
        let offset = y / 8 * 320 + x / 8 * 8 + y % 8;
        let addr = if offset < 0x1400 {
            0x6000 + offset
        } else {
            0x3400 + offset - 0x1400
        };
        Some(self.byte(addr)? & (0x80 >> (x % 8)) != 0)
    }

    /// Colour of underlay sprite `s` for each line: the six tables work as
    /// in NUFLI (entry 0 is the initial colour, a nonzero high nibble keeps
    /// the previous colour, entry `k` colours lines `2k-1` and `2k`).
    fn underlay_colors(&self, s: usize) -> Option<[u8; HEIGHT]> {
        let table = COLOR_TABLES[s];
        let mut by_entry = [0u8; 101];
        let mut color = self.byte(table)? & 15;
        for (k, slot) in by_entry.iter_mut().enumerate() {
            let entry = self.byte(table + k)?;
            if k == 0 || entry >> 4 == 0 {
                color = entry & 15;
            }
            *slot = color;
        }
        Some(core::array::from_fn(|y| by_entry[y.div_ceil(2)]))
    }

    /// Colour of underlay sprite `s` on line `y`: white when the line's
    /// entry in the mask table at `$3300` has bit `s + 1` set.
    fn sprite_color(&self, colors: &[[u8; HEIGHT]; 6], s: usize, y: usize) -> Option<u8> {
        let mask = self.byte(0x3300 + y.div_ceil(2))?;
        Some(if mask >> (s + 1) & 1 != 0 {
            1
        } else {
            colors[s][y]
        })
    }

    fn pixel(&self, colors: &[[u8; HEIGHT]; 6], x: usize, y: usize) -> Option<u8> {
        let screen = screen(y / 2);
        let cell = self.byte(screen + x / 8)?;
        if self.bitmap_set(x, y)? {
            return Some(cell >> 4);
        }
        let s = x.checked_sub(BUG).map(|x| x / 48).filter(|&s| s < 6);
        if let Some(s) = s {
            let bank = screen & !0x3ff;
            let pointer = usize::from(self.byte(bank + 0x3f9 + s)?);
            let column = (x - BUG) % 48 / 2;
            let row = usize::from(self.rows[y] + (column / 8) as u8) & 0x3f;
            let addr = (bank & 0xc000) + pointer * 64 + row;
            if self.byte(addr)? & (0x80 >> (column % 8)) != 0 {
                return self.sprite_color(colors, s, y);
            }
        }
        Some(cell & 15)
    }

    fn pixels(&self) -> Option<Vec<u8>> {
        let mut colors = [[0; HEIGHT]; 6];
        for (s, slot) in colors.iter_mut().enumerate() {
            *slot = self.underlay_colors(s)?;
        }
        (0..HEIGHT)
            .flat_map(|y| (0..vic2::WIDTH).map(move |x| (x, y)))
            .map(|(x, y)| self.pixel(&colors, x, y))
            .collect()
    }
}

impl<'a> Frame<'a> {
    fn new(mem: &'a [u8], layout: Layout) -> Self {
        Frame {
            mem,
            layout,
            rows: sprite_rows(),
        }
    }

    fn to_frame(&self) -> Result<vic2::Frame, DecodeError> {
        let pixels = self.pixels().ok_or(DecodeError::Unrecognized)?;
        Ok(vic2::Frame::from_fn(HEIGHT, |x, y| {
            pixels[y * vic2::WIDTH + x]
        }))
    }
}

/// `.muf`: two ignored bytes, the frame and 64 ignored bytes.
pub(super) fn decode_muf(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 2 + LEN + 64 {
        return Err(DecodeError::Unrecognized);
    }
    Ok(Frame::new(&data[2..], Layout::First)
        .to_frame()?
        .to_image(BUG))
}

pub(super) fn decode_mup(data: &[u8]) -> Result<Image, DecodeError> {
    let [_, _, escape, packed @ ..] = data else {
        return Err(DecodeError::Unrecognized);
    };
    let mem = backward_rle(packed, *escape, LEN).ok_or(DecodeError::Unrecognized)?;
    Ok(Frame::new(&mem, Layout::First).to_frame()?.to_image(BUG))
}

pub(super) fn decode_mui(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 2 + 0xac00 {
        return Err(DecodeError::Unrecognized);
    }
    let mem = &data[2..];
    let first = Frame::new(mem, Layout::First).to_frame()?;
    let second = Frame::new(mem, Layout::Second).to_frame()?;
    Ok(first.blend(&second, BUG))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_frame_is_stored_in_two_pieces() {
        assert_eq!(Layout::First.offset(0x2100), Some(0));
        assert_eq!(Layout::First.offset(0x7600), None);
        assert_eq!(Layout::Second.offset(0x2100), Some(0x8000));
        assert_eq!(Layout::Second.offset(0x4cff), Some(0xabff));
        assert_eq!(Layout::Second.offset(0x4d00), Some(0x5600));
        assert_eq!(Layout::Second.offset(0x75ff), Some(0x7eff));
        assert_eq!(Layout::Second.offset(0x2000), None);
    }

    #[test]
    fn wrong_sizes_are_rejected() {
        assert!(decode_muf(&[0; 100]).is_err());
        assert!(decode_mui(&[0; 100]).is_err());
        assert!(decode_mup(&[0; 2]).is_err());
    }
}
