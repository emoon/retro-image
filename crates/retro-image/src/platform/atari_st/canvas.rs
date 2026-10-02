//! Canvas pictures: compressed (`CPT`), with raster palettes from an
//! `.HBL` file next to them, and full (`FUL`) files holding both.
//!
//! Sources:
//! - <https://temlib.org/AtariForumWiki/index.php/Canvas_file_format>
//! - Canvas 1.17 manual: <http://cd.textfiles.com/crawlycrypt1/graphics/canvas17/manual.txt>
//!   (an HBL file lists a palette number per group of scanlines, -1 for no
//!   change; a FUL file holds picture, HBL and animation data).
//! - Derived from sample files and `recoil2png` output: the HBL layout (see
//!   [`Hbl`]) and that a FUL file is the HBL data, 608 bytes of animation
//!   data, then the CPT data.

use alloc::vec::Vec;

use super::common::{
    Resolution, SCREEN_LEN, decode_screen, decode_screen_by_line, palette_words, scale3, vdi_pen,
};
use crate::bytes::be16;
use crate::{Companions, DecodeError, Image};

/// CPT, shown with the rasters of the `.HBL` file next to it if present.
pub(super) fn decode_cpt(data: &[u8], companions: &dyn Companions) -> Result<Image, DecodeError> {
    let picture = Picture::parse(data).ok_or(DecodeError::Unrecognized)?;
    let hbl = companions.get("hbl");
    let image = hbl
        .as_deref()
        .and_then(Hbl::parse)
        .and_then(|hbl| picture.decode_with_rasters(&hbl))
        .or_else(|| picture.decode());
    image.ok_or(DecodeError::Unrecognized)
}

/// Animation data between the HBL data and the picture in a FUL file.
const FUL_ANIMATION_LEN: usize = 608;

/// FUL: HBL data, animation data and a CPT picture.
pub(super) fn decode_ful(data: &[u8]) -> Result<Image, DecodeError> {
    let image = Hbl::parse(data).and_then(|hbl| {
        let picture = Picture::parse(data.get(hbl.len + FUL_ANIMATION_LEN..)?)?;
        picture.decode_with_rasters(&hbl)
    });
    image.ok_or(DecodeError::Unrecognized)
}

/// An unpacked CPT picture.
struct Picture {
    resolution: Resolution,
    words: Vec<u16>,
    bitmap: Vec<u8>,
}

impl Picture {
    /// 16 palette words, a resolution word, then runs of (count, offset,
    /// one 16-pixel unit), ended by a count of `$FFFF`, then the units the
    /// runs left unfilled, in order.
    fn parse(data: &[u8]) -> Option<Self> {
        let words = palette_words(data, 0, 16)?;
        let resolution = Resolution::from_index(be16(data, 32)?)?;
        // One unit is a 16-pixel group: one word per plane.
        let unit = resolution.planes() as usize * 2;
        let units = SCREEN_LEN / unit;
        let mut bitmap = alloc::vec![0u8; SCREEN_LEN];
        let mut filled = alloc::vec![false; units];
        let mut pos = 34;
        loop {
            let count = be16(data, pos)?;
            let offset = usize::from(be16(data, pos + 2)?);
            let value = data.get(pos + 4..pos + 4 + unit)?;
            pos += 4 + unit;
            if count == 0xffff {
                break;
            }
            // The offset counts units, not bytes (derived from sample files).
            let run = offset..offset + usize::from(count) + 1;
            let target = bitmap.get_mut(run.start * unit..run.end * unit)?;
            for chunk in target.chunks_exact_mut(unit) {
                chunk.copy_from_slice(value);
            }
            filled[run].fill(true);
        }
        for index in (0..units).filter(|&i| !filled[i]) {
            let value = data.get(pos..pos + unit)?;
            pos += unit;
            bitmap[index * unit..(index + 1) * unit].copy_from_slice(value);
        }
        Some(Self {
            resolution,
            words,
            bitmap,
        })
    }

    fn decode(&self) -> Option<Image> {
        decode_screen(self.resolution, &self.bitmap, &self.words)
    }

    /// The picture with the HBL palettes; high resolution has none.
    fn decode_with_rasters(&self, hbl: &Hbl) -> Option<Image> {
        if self.resolution == Resolution::High {
            return self.decode();
        }
        let default = super::common::screen_palette(self.resolution, &self.words);
        let colors = self.resolution.colors();
        decode_screen_by_line(self.resolution, &self.bitmap, |y| {
            Some(hbl.palette(y, colors).unwrap_or_else(|| default.clone()))
        })
    }
}

/// Lines covered by one entry of the HBL table.
const HBL_LINES: usize = 4;
const HBL_TABLE_LEN: usize = 400;
/// Offset of the first palette record.
const HBL_RECORDS: usize = 800;
const HBL_RECORD_LEN: usize = 48;

/// HBL data: a table of 200 words, one per four lines from the top
/// (`$FFFF` keeps the current palette), 400 unused bytes, then one palette
/// record per used table entry plus one, each 16 VDI pens of three bytes
/// (R, G, B, low three bits used). The records are stored in reverse:
/// the first used entry selects the last record. The palette numbers in
/// the table don't matter.
struct Hbl<'a> {
    /// Ordinal of the last used entry at or above each table position.
    entries: Vec<Option<usize>>,
    used: usize,
    records: &'a [u8],
    len: usize,
}

impl<'a> Hbl<'a> {
    fn parse(data: &'a [u8]) -> Option<Self> {
        let table = palette_words(data, 0, HBL_TABLE_LEN / 2)?;
        let mut used = 0;
        let mut current = None;
        let entries = table
            .iter()
            .map(|&word| {
                if word != 0xffff {
                    current = Some(used);
                    used += 1;
                }
                current
            })
            .collect();
        let len = HBL_RECORDS + (used + 1) * HBL_RECORD_LEN;
        Some(Self {
            entries,
            used,
            records: data.get(HBL_RECORDS..len)?,
            len,
        })
    }

    /// The `colors` hardware colours of line `y`, or `None` above the
    /// first used entry.
    fn palette(&self, y: usize, colors: usize) -> Option<Vec<u32>> {
        let position = y / HBL_LINES;
        let ordinal = (*self.entries.get(position)?)?;
        let record = &self.records[(self.used - ordinal) * HBL_RECORD_LEN..][..HBL_RECORD_LEN];
        // The palette set from the top of the screen maps pens like a
        // 16-colour mode even in medium resolution (observed from
        // `recoil2png` output).
        let pen_colors = if ordinal == 0 && self.first_entry_at_top() {
            16
        } else {
            colors
        };
        Some(
            (0..colors)
                .map(|index| {
                    let rgb = &record[vdi_pen(index, pen_colors) * 3..][..3];
                    scale3(rgb[0].into()) << 16 | scale3(rgb[1].into()) << 8 | scale3(rgb[2].into())
                })
                .collect(),
        )
    }

    fn first_entry_at_top(&self) -> bool {
        self.entries.first() == Some(&Some(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hbl_records_are_stored_in_reverse_order() {
        // Entries at lines 0 and 8; three records whose pen 0 is red 1, 2, 3.
        let mut data = alloc::vec![0xff; HBL_TABLE_LEN];
        data[0..2].copy_from_slice(&[0, 5]);
        data[4..6].copy_from_slice(&[0, 9]);
        data.resize(HBL_RECORDS, 0);
        for red in 1..=3 {
            let mut record = [0; HBL_RECORD_LEN];
            record[0] = red;
            data.extend_from_slice(&record);
        }
        let hbl = Hbl::parse(&data).unwrap();
        assert_eq!(hbl.len, data.len());
        let red = |y| hbl.palette(y, 16).unwrap()[0] >> 16;
        assert_eq!(red(0), scale3(3));
        assert_eq!(red(7), scale3(3));
        assert_eq!(red(8), scale3(2));
        assert_eq!(red(199), scale3(2));
        assert!(Hbl::parse(&data[..data.len() - 1]).is_none());
    }
}
