//! STOS memory bank files (`.MBK`): a screen bank is a STOS packed screen, a
//! sprite bank is shown as a sheet of its low resolution frames.
//!
//! Sources:
//! - Sprite bank header, frame table and the `PALT` palette block: the Atari
//!   Forum Wiki page "STOS Memory Bank file format",
//!   <https://temlib.org/AtariForumWiki/index.php?title=STOS_Memory_Bank_file_format>
//!   (prose, no license stated; facts only).
//! - Reverse engineered from the samples in `corpus/extra/stos-mbk` (ggnkua's
//!   Atari ST sources archive, see its `MANIFEST.tsv`):
//!   - the file starts `Lionpoubnk`, a `u16` bank number at 12, then a `u32`
//!     at 14 holding flags in the top byte and the bank length (without this
//!     18-byte header) in the low 24 bits; the bank follows at 18;
//!   - a sprite frame holds its 1-bit mask first (`height` lines of `width`
//!     words, a set bit is transparent), then the color data, 4 planes
//!     interleaved per word like an ST screen;
//!   - frame data offsets count from the start of the frame table.
//! - A screen bank is the packed screen of `stos_pp.rs`.
//!
//! The wiki lists one frame table per resolution; only low resolution (4
//! planes) is read, as no sample has medium or high resolution frames. A
//! transparent pixel is drawn in palette color 0. Hotspots are ignored.
//! Multi-bank `.MBS` files and banks of other kinds (music, data) are
//! rejected.

use alloc::vec::Vec;

use super::common::{palette_words, st_palette};
use super::stos_pp;
use crate::bytes::{be16, be32};
use crate::image::check_size;
use crate::{DecodeError, Image};

const TAG: &[u8] = b"Lionpoubnk";
/// Where the bank starts.
const BANK_AT: usize = 18;
const SPRITE_MAGIC: u32 = 0x1986_1987;
/// Offset, from the bank start, of the low resolution table offset and of the
/// frame count; the offset counts from its own position.
const LOW_OFFSET_AT: usize = 4;
const LOW_COUNT_AT: usize = 16;
const FRAME_ENTRY_LEN: usize = 8;
const PALETTE_TAG: &[u8] = b"PALT";
const PLANES: usize = 4;
/// Sheet width the frames are wrapped at, unless one frame is wider.
const SHEET_WIDTH: usize = 320;

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    const FAIL: DecodeError = DecodeError::Invalid;
    if !data.starts_with(TAG) {
        return Err(FAIL);
    }
    let bank = data.get(BANK_AT..).ok_or(FAIL)?;
    if be32(bank, 0) == Some(SPRITE_MAGIC) {
        sprites(bank)
    } else {
        stos_pp::decode_pac(bank)
    }
}

/// One sprite frame: where its data starts and its size in words and lines.
struct Frame {
    data_at: usize,
    words: usize,
    lines: usize,
}

fn sprites(bank: &[u8]) -> Result<Image, DecodeError> {
    const FAIL: DecodeError = DecodeError::Invalid;
    let count = usize::from(be16(bank, LOW_COUNT_AT).ok_or(FAIL)?);
    let table = LOW_OFFSET_AT
        .checked_add(be32(bank, LOW_OFFSET_AT).ok_or(FAIL)? as usize)
        .ok_or(FAIL)?;
    let palette_at = table.checked_add(count * FRAME_ENTRY_LEN).ok_or(FAIL)?;
    if count == 0
        || bank.get(palette_at..palette_at.checked_add(4).ok_or(FAIL)?) != Some(PALETTE_TAG)
    {
        return Err(FAIL);
    }
    let palette = st_palette(&palette_words(bank, palette_at + PALETTE_TAG.len(), 16).ok_or(FAIL)?);

    let mut frames = Vec::with_capacity(count);
    for i in 0..count {
        let entry = table + i * FRAME_ENTRY_LEN;
        let data_at = table
            .checked_add(be32(bank, entry).ok_or(FAIL)? as usize)
            .ok_or(FAIL)?;
        let size = bank.get(entry + 4..entry + 6).ok_or(FAIL)?;
        let (words, lines) = (usize::from(size[0]), usize::from(size[1]));
        let end = data_at
            .checked_add(words * lines * (2 + 2 * PLANES))
            .ok_or(FAIL)?;
        if words == 0 || lines == 0 || bank.get(data_at..end).is_none() {
            return Err(FAIL);
        }
        frames.push(Frame {
            data_at,
            words,
            lines,
        });
    }

    let cell_width = frames.iter().map(|f| f.words * 16).max().unwrap_or(0);
    let cell_height = frames.iter().map(|f| f.lines).max().unwrap_or(0);
    let columns = (SHEET_WIDTH / cell_width).max(1).min(count);
    let (width, height) = (columns * cell_width, count.div_ceil(columns) * cell_height);
    check_size(width, height)?;
    let mut pixels = alloc::vec![palette[0]; width * height];
    for (i, frame) in frames.iter().enumerate() {
        let (left, top) = (i % columns * cell_width, i / columns * cell_height);
        let mask_len = frame.words * frame.lines * 2;
        let planes = frame.data_at + mask_len;
        for line in 0..frame.lines {
            for word in 0..frame.words {
                let cell = line * frame.words + word;
                let mask = be16(bank, frame.data_at + cell * 2).ok_or(FAIL)?;
                let planes: Vec<u16> = (0..PLANES)
                    .map(|p| be16(bank, planes + (cell * PLANES + p) * 2).ok_or(FAIL))
                    .collect::<Result<_, _>>()?;
                for bit in 0..16 {
                    if mask >> (15 - bit) & 1 == 0 {
                        let index = planes.iter().enumerate().fold(0, |v, (p, plane)| {
                            v | usize::from(plane >> (15 - bit) & 1) << p
                        });
                        pixels[(top + line) * width + left + word * 16 + bit] = palette[index];
                    }
                }
            }
        }
    }
    Image::from_colors(width as u32, height as u32, pixels.into_iter())
}
