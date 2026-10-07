//! NUFLI (`.nuf`, packed `.nup`; Crest's NUFLI Editor / mufflon): 320×200 hires FLI with a
//! new screen RAM every second line, six X-expanded hires sprites under the
//! bitmap for a third color, and two more sprites (hires and multicolor)
//! covering the FLI bug in the leftmost 24 pixels.
//!
//! Sources:
//! - Memory map (bitmap split between `$6000` and `$3400`, the screen RAM
//!   and the sprite pointers of each line pair, the six 101-entry sprite
//!   color tables, the sprite row counter, the FLI-bug sprites and their
//!   color switch codes and initial colors): pynuvie,
//!   <https://github.com/anarkiwi/pynuvie>, files `src/nuvie/nufli.py`
//!   (<https://github.com/anarkiwi/pynuvie/blob/main/src/nuvie/nufli.py>) and
//!   `src/nuvie/_flibug.py`
//!   (<https://github.com/anarkiwi/pynuvie/blob/main/src/nuvie/_flibug.py>),
//!   and its format notes
//!   <https://github.com/anarkiwi/pynuvie/blob/main/docs/FORMAT.md>.
//! - Mode description: C64-Wiki, <https://www.c64-wiki.com/wiki/NUFLI>.
//! - Reverse engineered by probing `recoil2png` with modified copies of the
//!   three pynuvie sample files and with random data: sprite data is fetched
//!   through the pointers in each line pair's screen RAM (`recoil2png`
//!   rejects files whose fetched sprite data lies outside `$2000-$79FF`, but
//!   only where a pixel needs it); a color table entry with a nonzero high
//!   nibble keeps the previous sprite color (entry 0 always sets it);
//!   entry `k` colors lines `2k-1` and `2k`; FLI-bug color switches (high
//!   nibble 7: hires sprite, 5/6/E: multicolor `01`/`11`/`10`) apply from
//!   line `2k` (from `2k-1` when in the first table); in the FLI-bug
//!   columns lines 2-7 of each character row show light gray for both
//!   bitmap colors.
//! - `.nup` (packed): reverse engineered from the 13 NUP files on the
//!   Crest Slide Story tool disk and the Deadly Chords disk
//!   (<https://csdb.dk/release/?id=81153>, <https://csdb.dk/release/?id=243096>)
//!   by unpacking them and comparing `recoil2png`'s output for the `.nup`
//!   with that for the unpacked `.nuf`, and by probing it with modified
//!   copies (the byte at offset 2 must be `$FD`).
//!
//! The tables below were translated from pynuvie into Rust and restructured
//! (the screen table became a formula); the FLI-bug decoding and the color
//! timing are new. pynuvie's notice:
//!
//! ```text
//! pynuvie, Copyright Josh Bailey
//!
//! Licensed under the Apache License, Version 2.0 (the "License");
//! you may not use this file except in compliance with the License.
//! You may obtain a copy of the License at
//!
//!     http://www.apache.org/licenses/LICENSE-2.0
//!
//! Unless required by applicable law or agreed to in writing, software
//! distributed under the License is distributed on an "AS IS" BASIS,
//! WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//! See the License for the specific language governing permissions and
//! limitations under the License.
//! ```

use super::unpack::backward_rle_filled;
use super::vic2;
use crate::{DecodeError, Image};
use alloc::vec::Vec;

const LOAD: usize = 0x2000;
const END: usize = 0x7a00;
pub(super) const HEIGHT: usize = 200;
const BUG: usize = vic2::FLI_BUG;
/// Color tables of the six underlay sprites, one entry per line pair plus
/// the initial color.
pub(super) const COLOR_TABLES: [usize; 6] = [0x2400, 0x2480, 0x2800, 0x2880, 0x2c00, 0x2c80];
/// Initial colors of the FLI-bug sprites: hires, then multicolor bit
/// pairs `01`, `11`, `10`.
const BUG_COLORS: [usize; 4] = [0x3ff7, 0x3ff1, 0x3ff0, 0x3ff6];
/// High nibbles of color-table entries that switch a FLI-bug color, in
/// the order of [`BUG_COLORS`].
const BUG_SWITCHES: [u8; 4] = [0x7, 0x5, 0x6, 0xe];

/// The screen RAM row of line pair `lp` (two pixel lines).
pub(super) fn screen(lp: usize) -> usize {
    let row = lp / 4 * 40;
    let base = if lp < 64 {
        [
            0x5c00, 0x5800, 0x5400, 0x5000, 0x4c00, 0x4800, 0x4400, 0x4000,
        ][lp % 8]
    } else {
        let k = if lp < 84 { lp % 4 } else { (lp + 1) % 4 };
        0x2000 + k * 0x400
    };
    base + row
}

/// Memory address of the bitmap byte holding pixel (`x`, `y`).
pub(super) fn bitmap_addr(x: usize, y: usize) -> usize {
    let offset = y / 8 * 320 + x / 8 * 8 + y % 8;
    if offset < 0x1400 {
        0x6000 + offset
    } else {
        0x3400 + offset - 0x1400
    }
}

/// Color of underlay sprite `s` for each line, reading memory through
/// `byte`: entry 0 is the initial color, a nonzero high nibble keeps the
/// previous color, entry `k` colors lines `2k-1` and `2k`.
pub(super) fn underlay_colors(
    byte: impl Fn(usize) -> Option<u8>,
    s: usize,
) -> Option<[u8; HEIGHT]> {
    let mut by_entry = [0u8; 101];
    let mut color = 0;
    for (k, slot) in by_entry.iter_mut().enumerate() {
        let entry = byte(COLOR_TABLES[s] + k)?;
        if k == 0 || entry >> 4 == 0 {
            color = entry & 15;
        }
        *slot = color;
    }
    Some(core::array::from_fn(|y| by_entry[y.div_ceil(2)]))
}

/// Sprite row counter (byte offset in the 64-byte block) of each line: the
/// sprites are Y-expanded and repositioned so the counter runs on.
pub(super) fn sprite_rows() -> [u8; HEIGHT] {
    let mut rows = [0; HEIGHT];
    let mut row = 5u8;
    for (y, slot) in rows.iter_mut().enumerate() {
        *slot = row;
        if y % 2 == 0 {
            row += 3;
        }
        if row > 0x3f {
            row &= 0x3f;
        } else if row == 0x3f {
            row = 0;
        }
    }
    rows
}

struct Nufli<'a> {
    /// Memory `$2000-$79FF`.
    mem: &'a [u8],
    rows: [u8; HEIGHT],
}

impl Nufli<'_> {
    fn byte(&self, addr: usize) -> u8 {
        self.mem[addr - LOAD]
    }

    fn bitmap_set(&self, x: usize, y: usize) -> bool {
        self.byte(bitmap_addr(x, y)) & (0x80 >> (x % 8)) != 0
    }

    /// Byte `index` of hardware sprite `sprite` on line `y`, fetched through
    /// the pointer in the line pair's screen RAM; `None` outside the file.
    fn sprite(&self, sprite: usize, index: usize, y: usize) -> Option<u8> {
        let screen = screen(y / 2) & !0x3ff;
        let pointer = usize::from(self.byte(screen + 0x3f8 + sprite));
        let row = usize::from(self.rows[y] + index as u8) & 0x3f;
        let addr = (screen & 0xc000) + pointer * 64 + row;
        (LOAD..END).contains(&addr).then(|| self.byte(addr))
    }

    fn underlay_colors(&self, s: usize) -> [u8; HEIGHT] {
        underlay_colors(|addr| Some(self.byte(addr)), s)
            .unwrap_or_else(|| unreachable!("table addresses lie inside the frame"))
    }

    /// FLI-bug sprite colors (in [`BUG_COLORS`] order) for each line.
    fn bug_colors(&self) -> Vec<[u8; 4]> {
        let mut colors = BUG_COLORS.map(|addr| self.byte(addr) & 15);
        let switch = |colors: &mut [u8; 4], entry: u8| {
            if let Some(i) = BUG_SWITCHES.iter().position(|&s| s == entry >> 4) {
                colors[i] = entry & 15;
            }
        };
        let mut lines = alloc::vec![colors; HEIGHT];
        for k in 1..=100 {
            let entries = COLOR_TABLES.map(|table| self.byte(table + k));
            let mut early = colors;
            switch(&mut early, entries[0]);
            if let Some(line) = lines.get_mut(2 * k - 1) {
                *line = early;
            }
            entries.iter().for_each(|&entry| switch(&mut colors, entry));
            if let Some(line) = lines.get_mut(2 * k) {
                *line = colors;
            }
        }
        lines
    }

    fn pixel(&self, x: usize, y: usize, underlay: &[[u8; HEIGHT]], bug: &[u8; 4]) -> Option<u8> {
        let color = self.byte(screen(y / 2) + x / 8);
        // The FLI bug shows light gray, except on a character row's first
        // line pair, where the VIC fetches the screen RAM normally.
        let (ink, paper) = if x < BUG && y % 8 >= 2 {
            (15, 15)
        } else {
            (color >> 4, color & 15)
        };
        if self.bitmap_set(x, y) {
            return Some(ink);
        }
        if x < BUG {
            if self.sprite(0, x / 8, y)? & (0x80 >> (x % 8)) != 0 {
                return Some(bug[0]);
            }
            let pair = self.sprite(7, x / 8, y)? >> (6 - (x & 6)) & 3;
            return Some(match pair {
                1 => bug[1],
                3 => bug[2],
                2 => bug[3],
                _ => paper,
            });
        }
        let s = (x - BUG) / 48;
        if s < 6 {
            let column = (x - BUG) % 48 / 2;
            if self.sprite(1 + s, column / 8, y)? & (0x80 >> (column % 8)) != 0 {
                return Some(underlay[s][y]);
            }
        }
        Some(paper)
    }
}

pub(super) fn decode_nufli(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 2 + END - LOAD {
        return Err(DecodeError::Invalid);
    }
    decode_frame(&data[2..])
}

/// `.nup`: two ignored bytes (the load address `$0FFF`), the byte `$FD`, an
/// escape byte, then the memory `$2000-$79FF` packed backwards (`value
/// count escape`, count 0 = 256). The stream must fill the memory exactly.
pub(super) fn decode_nup(data: &[u8]) -> Result<Image, DecodeError> {
    let [_, _, 0xfd, escape, packed @ ..] = data else {
        return Err(DecodeError::Invalid);
    };
    match backward_rle_filled(packed, *escape, END - LOAD) {
        Some((mem, true)) => decode_frame(&mem),
        _ => Err(DecodeError::Invalid),
    }
}

/// Decodes the memory `$2000-$79FF` (`mem` is exactly that long).
fn decode_frame(mem: &[u8]) -> Result<Image, DecodeError> {
    let nufli = Nufli {
        mem,
        rows: sprite_rows(),
    };
    let underlay: Vec<[u8; HEIGHT]> = (0..6).map(|s| nufli.underlay_colors(s)).collect();
    let bug = nufli.bug_colors();
    let pixels = (0..HEIGHT)
        .flat_map(|y| (0..vic2::WIDTH).map(move |x| (x, y)))
        .map(|(x, y)| nufli.pixel(x, y, &underlay, &bug[y]))
        .collect::<Option<Vec<u8>>>()
        .ok_or(DecodeError::Invalid)?;
    Ok(vic2::image(vic2::WIDTH, HEIGHT, pixels))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screen_rows_follow_line_pairs() {
        assert_eq!(screen(0), 0x5c00);
        assert_eq!(screen(4), 0x4c28);
        assert_eq!(screen(64), 0x2280);
        assert_eq!(screen(84), 0x2748);
        assert_eq!(screen(87), 0x2348);
        assert_eq!(screen(99), 0x23c0);
    }

    #[test]
    fn sprite_rows_wrap_like_the_counter() {
        let rows = sprite_rows();
        assert_eq!(&rows[..5], &[5, 8, 8, 11, 11]);
        assert!(rows.iter().all(|&r| r < 0x3f));
    }

    #[test]
    fn pointers_outside_the_file_are_rejected() {
        let mut data = alloc::vec![0u8; 2 + END - LOAD];
        // All pointers 0: bank 1 points at $4000 (inside), bank 0 at $0000.
        assert!(decode_nufli(&data).is_err());
        // Cover every pixel with set bitmap bits: no sprite fetch needed.
        data[2 + 0x6000 - LOAD..2 + 0x7400 - LOAD].fill(0xff);
        data[2 + 0x3400 - LOAD..2 + 0x3f40 - LOAD].fill(0xff);
        assert!(decode_nufli(&data).is_ok());
    }

    #[test]
    fn nup_stream_must_fill_the_frame_exactly() {
        // 90 runs of 256 `$FF` bytes: set bitmap bits cover every pixel, so
        // no sprite data is fetched.
        let mut data = alloc::vec![0x00, 0x00, 0xfd, 0xaa];
        for _ in 0..(END - LOAD) / 256 {
            data.extend([0xff, 0x00, 0xaa]);
        }
        assert!(decode_nup(&data).is_ok());
        let mut extra = data.clone();
        extra.insert(4, 0x12);
        assert!(decode_nup(&extra).is_err());
        assert!(decode_nup(&data[..data.len() - 3]).is_err());
        data[2] = 0xfe;
        assert!(decode_nup(&data).is_err());
    }
}
