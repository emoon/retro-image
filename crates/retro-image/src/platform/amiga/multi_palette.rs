//! Per-line palettes of ILBM pictures: PCHG, SHAM, CTBL and BEAM chunks.
//!
//! Sources:
//! - PCHG: ILBM spec, "PCHG" section
//!   (<https://wiki.amigaos.net/wiki/ILBM_IFF_Interleaved_Bitmap>): header,
//!   line mask, SmallLineChanges (register in the top 4 bits, 12-bit colour)
//!   and BigLineChanges.
//! - SHAM, CTBL, BEAM: the IFF chunk registry
//!   (<https://wiki.amigaos.net/wiki/IFF_FORM_and_Chunk_Registry>) names CTBL
//!   an array of `0RGB` words. The per-line layout (SHAM: a version word,
//!   then one 16-colour table per line or line pair) was reverse engineered
//!   from samples and verified against `recoil2png` output.

use alloc::vec::Vec;

use super::iff::{be16, be32, find};
use super::ilbm::{Palette, rgb12};

pub(super) enum LinePalettes<'a> {
    /// Complete colour tables of `colors` `0RGB` words, each used for
    /// `lines_per_table` lines.
    Tables {
        words: &'a [u8],
        colors: usize,
        lines_per_table: usize,
    },
    /// PCHG: register changes for lines from `start`.
    Changes {
        start: isize,
        lines: Vec<Vec<(usize, u32)>>,
    },
}

impl<'a> LinePalettes<'a> {
    pub(super) fn parse(contents: &'a [u8], height: usize) -> Option<Self> {
        if let Some(pchg) = find(contents, b"PCHG") {
            return parse_pchg(pchg);
        }
        let (words, colors) = if let Some(sham) = find(contents, b"SHAM") {
            (sham.get(2..)?, 16)
        } else {
            let table = find(contents, b"CTBL").or_else(|| find(contents, b"BEAM"))?;
            (table, table.len() / 2 / height)
        };
        let tables = words.len() / 2 / colors.max(1);
        if colors == 0 || tables == 0 || height % tables != 0 {
            return None;
        }
        Some(Self::Tables {
            words,
            colors,
            lines_per_table: height / tables,
        })
    }

    /// Updates `palette` for picture line `y`. Lines are visited in order.
    pub(super) fn apply(&self, y: usize, palette: &mut Palette) {
        match self {
            Self::Tables {
                words,
                colors,
                lines_per_table,
            } => {
                let start = y / lines_per_table * colors * 2;
                let table = &words[start..start + colors * 2];
                for (i, word) in table.chunks_exact(2).enumerate() {
                    palette.set(i, rgb12(be16(word)));
                }
            }
            Self::Changes { start, lines } => {
                let line = y as isize - start;
                if let Some(changes) = usize::try_from(line).ok().and_then(|l| lines.get(l)) {
                    for &(register, color) in changes {
                        palette.set(register, color);
                    }
                }
            }
        }
    }
}

const PCHG_12BIT: u16 = 1;
const PCHG_32BIT: u16 = 2;

fn parse_pchg(pchg: &[u8]) -> Option<LinePalettes<'static>> {
    let header = pchg.get(..20)?;
    let compression = be16(&header[0..2]);
    let flags = be16(&header[2..4]);
    let start = be16(&header[4..6]) as i16 as isize;
    let line_count = usize::from(be16(&header[6..8]));
    let data = match compression {
        0 => pchg[20..].to_vec(),
        _ => return None,
    };
    let mask_len = line_count.div_ceil(32) * 4;
    let mask = data.get(..mask_len)?;
    let mut pos = mask_len;
    let mut lines = Vec::with_capacity(line_count);
    for line in 0..line_count {
        let mut changes = Vec::new();
        if mask[line / 8] & (0x80 >> (line % 8)) != 0 {
            if flags & PCHG_12BIT != 0 {
                let low = usize::from(*data.get(pos)?);
                let high = usize::from(*data.get(pos + 1)?);
                pos += 2;
                for i in 0..low + high {
                    let word = be16(data.get(pos..pos + 2)?);
                    pos += 2;
                    let bank = if i < low { 0 } else { 16 };
                    changes.push((bank + usize::from(word >> 12), rgb12(word & 0xfff)));
                }
            } else if flags & PCHG_32BIT != 0 {
                let count = usize::from(be16(data.get(pos..pos + 2)?));
                pos += 2;
                for _ in 0..count {
                    let change = data.get(pos..pos + 6)?;
                    pos += 6;
                    let [_, r, b, g] = be32(&change[2..6]).to_be_bytes();
                    let color = u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b);
                    changes.push((usize::from(be16(&change[0..2])), color));
                }
            } else {
                return None;
            }
        }
        lines.push(changes);
    }
    Some(LinePalettes::Changes { start, lines })
}
