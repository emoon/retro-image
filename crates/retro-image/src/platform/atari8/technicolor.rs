//! Technicolor Dream: a luminance file (`.LUM`) and a hue file (`.COL`).
//!
//! Sources:
//! - Just Solve "Technicolor Dream"
//!   (<http://fileformats.archiveteam.org/wiki/Technicolor_Dream>; two files
//!   of 4766 bytes, 80x119, 256 colours).
//! - Observed from `recoil2png` output: each file is a 6-byte header
//!   (ignored) and 119 lines of 40 bytes, one 4-bit value per pixel. Every
//!   line is shown as two scanlines with pixels 4 wide: a hue scanline (the
//!   `.COL` hue with the average luminance of this and the previous line,
//!   rounded down) and a luminance scanline (hue and this line's luminance).
//!   Without a `.COL` the luminance alone is shown, on both scanlines.
//! - Packed files (any other size) were reverse engineered from samples:
//!   after the same header, (value, count) byte pairs repeat each value
//!   count times. The packed `HAYWAIN.LUM`/`.COL` from the program's disk
//!   unpack to exactly RECOIL's 4766-byte `HAYWAIN` sample pair. A last run
//!   may overshoot the 4760 bytes, and bytes after it are disk slack.

use super::palette::rgb;
use crate::{Companions, DecodeError, Image};
use alloc::vec::Vec;

const RAW_LEN: usize = HEADER + PLANE;
const HEADER: usize = 6;
const LINES: usize = 119;
const PLANE: usize = LINES * 40;

pub(super) fn decode_lum(data: &[u8], companions: &dyn Companions) -> Result<Image, DecodeError> {
    let luminances = plane(data).ok_or(DecodeError::Unrecognized)?;
    let hues = companions.get("col").and_then(|col| plane(&col));
    let nibble = |plane: &[u8], line: usize, x: usize| {
        let byte = plane[line * 40 + x / 2];
        if x.is_multiple_of(2) {
            byte >> 4
        } else {
            byte & 0x0f
        }
    };
    let mut image = Image::new(80, 2 * LINES as u32);
    for line in 0..LINES {
        for x in 0..80 {
            let now = nibble(&luminances, line, x);
            let (top, bottom) = match &hues {
                Some(col) => {
                    let hue = nibble(col, line, x) << 4;
                    let before = line.checked_sub(1).map_or(0, |l| nibble(&luminances, l, x));
                    (rgb(hue | ((now + before) / 2)), rgb(hue | now))
                }
                None => (rgb(now), rgb(now)),
            };
            image.set(x as u32, 2 * line as u32, top);
            image.set(x as u32, 2 * line as u32 + 1, bottom);
        }
    }
    image.scaled(4, 1)
}

/// The 119 lines of a file after its header, unpacked if needed.
fn plane(data: &[u8]) -> Option<Vec<u8>> {
    let body = data.get(HEADER..)?;
    if data.len() == RAW_LEN {
        return Some(body.to_vec());
    }
    let mut plane = Vec::with_capacity(PLANE + 255);
    for pair in body.as_chunks::<2>().0 {
        if plane.len() >= PLANE {
            break;
        }
        plane.resize(plane.len() + usize::from(pair[1]), pair[0]);
    }
    (plane.len() >= PLANE).then(|| {
        plane.truncate(PLANE);
        plane
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NoCompanions;

    struct Col(Vec<u8>);

    impl Companions for Col {
        fn get_named(&self, _file_name: &str) -> Option<Vec<u8>> {
            None
        }
        fn get(&self, extension: &str) -> Option<Vec<u8>> {
            (extension == "col").then(|| self.0.clone())
        }
    }

    #[test]
    fn hue_scanline_averages_luminance_with_line_above() {
        let mut lum = alloc::vec![0u8; RAW_LEN];
        lum[HEADER] = 0x50;
        lum[HEADER + 40] = 0xc0;
        let mut col = alloc::vec![0u8; RAW_LEN];
        col[HEADER + 40] = 0xa0;
        let image = decode_lum(&lum, &Col(col)).unwrap();
        assert_eq!((image.width(), image.height()), (320, 238));
        assert_eq!(image.get(0, 2), rgb(0xa8));
        assert_eq!(image.get(3, 3), rgb(0xac));
        let alone = decode_lum(&lum, &NoCompanions).unwrap();
        assert_eq!(alone.get(0, 0), rgb(0x05));
        assert!(decode_lum(&lum[1..], &NoCompanions).is_err());
    }

    #[test]
    fn unpacks_value_count_pairs() {
        let mut packed = alloc::vec![0u8; HEADER];
        packed.extend_from_slice(&[0x50, 1]);
        for _ in 0..PLANE / 255 + 1 {
            packed.extend_from_slice(&[0, 255]);
        }
        let mut raw = alloc::vec![0u8; RAW_LEN];
        raw[HEADER] = 0x50;
        assert_eq!(
            decode_lum(&packed, &NoCompanions),
            decode_lum(&raw, &NoCompanions)
        );
        packed.truncate(packed.len() - 2);
        assert!(decode_lum(&packed, &NoCompanions).is_err());
    }
}
