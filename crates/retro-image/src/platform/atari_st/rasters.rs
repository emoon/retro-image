//! NEOchrome Master rasters: the `RAST` chunk of its IFF pictures and the
//! `.RST` file saved next to a `.NEO` picture.
//!
//! Source: `neochrom.txt`, <https://temlib.org/AtariForumWiki/index.php/Neochrome_Master>:
//! up to 200 records of a line number and 16 palette words; the first
//! record holds the palette from the top, a later record with line 0 is
//! inactive, records may be unsorted and a line of `$FFFF` ends the list.

use alloc::vec::Vec;

use super::common::{Resolution, decode_screen_by_line, st_rgb, uses_ste_bits, words};
use crate::{Companions, Image};

const RECORD_LEN: usize = 34;

/// Expands raster records into 16 palette words per line for `height`
/// lines, or `None` if there is no record.
pub(super) fn line_palette_words(records: &[u8], height: usize) -> Option<Vec<u16>> {
    let records: Vec<(u16, Vec<u16>)> = records
        .as_chunks::<RECORD_LEN>()
        .0
        .iter()
        .map(|r| (u16::from_be_bytes([r[0], r[1]]), words(&r[2..])))
        .take_while(|&(line, _)| line != 0xffff)
        .collect();
    let (_, first) = records.first()?;
    let mut current = first;
    let mut out = Vec::with_capacity(height * 16);
    for y in 0..height {
        // The last record for a line wins.
        if let Some((_, palette)) = records[1..]
            .iter()
            .rev()
            .find(|&&(line, _)| line != 0 && usize::from(line) == y)
        {
            current = palette;
        }
        out.extend_from_slice(current);
    }
    Some(out)
}

/// Colours for [`line_palette_words`]: `RAST` chunks switch to STE colours
/// when any word uses an STE bit, `.RST` files are always plain ST
/// (both observed from `recoil2png` output).
pub(super) fn line_colors(words: &[u16], ste_detection: bool) -> Vec<u32> {
    let ste = ste_detection && uses_ste_bits(words.iter().copied());
    words.iter().map(|&w| st_rgb(w, ste)).collect()
}

/// A low-resolution NEOchrome screen shown with the rasters of the `.RST`
/// companion file, if there is one.
pub(super) fn with_rst(
    resolution: Resolution,
    bitmap: &[u8],
    companions: &dyn Companions,
) -> Option<Image> {
    if resolution != Resolution::Low {
        return None;
    }
    let rst = companions.get("rst")?;
    let height = resolution.height() as usize;
    let colors = line_colors(&line_palette_words(&rst, height)?, false);
    decode_screen_by_line(resolution, bitmap, |y| {
        colors.get(y * 16..(y + 1) * 16).map(<[u32]>::to_vec)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(line: u16, color0: u16) -> Vec<u8> {
        let mut r = Vec::from(line.to_be_bytes());
        r.extend_from_slice(&color0.to_be_bytes());
        r.resize(RECORD_LEN, 0);
        r
    }

    #[test]
    fn records_apply_from_their_line_until_the_end_marker() {
        let mut data = record(0, 1);
        data.extend(record(2, 2));
        data.extend(record(0, 9)); // inactive
        data.extend(record(0xffff, 0));
        data.extend(record(3, 9)); // after the end marker
        let words = line_palette_words(&data, 4).unwrap();
        let color0: Vec<u16> = words.chunks(16).map(|p| p[0]).collect();
        assert_eq!(color0, [1, 1, 2, 2]);
    }
}
