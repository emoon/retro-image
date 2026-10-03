//! Dr. Halo CUT and PAL.
//!
//! Sources:
//! - Encyclopedia of Graphics File Formats, "Dr. Halo":
//!   <https://www.fileformat.info/format/drhalo/egff.htm>. CUT: width,
//!   height and a zero reserved word (little-endian), then per scan line a
//!   16-bit byte count and run-length data: a control byte with bit 7 set
//!   repeats the next byte (control & 0x7f) times, otherwise that many literal
//!   bytes follow; a control byte of 0 or 0x80 ends the line. PAL: 40-byte
//!   header (`AH`, version, size, type 0x0A, ..., MaxRed/MaxGreen/MaxBlue at
//!   0x0e, 0x10, 0x12) followed by RGB triplets.
//! - Overview: <https://www.graphicsacademy.com/format_drhalo.php>.
//!
//! The CUT file has no magic, so it is chosen by extension and validated by
//! structure: every line must decode to exactly the width. Without a PAL the
//! picture is a grey ramp. PAL values are scaled from their declared maxima
//! (`MaxRed` etc.) to 8 bits.
//!
//! The Dr. Halo PIC variant is not implemented: no specification could be
//! retrieved and no sample was available.
//!
//! No sample CUT/PAL files were available, so the decoder is verified by
//! unit tests built from the specification above.

use alloc::vec::Vec;

use crate::bytes::le16;
use crate::image::check_size;
use crate::{Companions, DecodeError, Image};

const CUT_HEADER_LEN: usize = 6;
const PAL_HEADER_LEN: usize = 40;

pub(super) fn decode_cut(data: &[u8], companions: &dyn Companions) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let word = |at| le16(data, at).map(usize::from).ok_or(fail);
    let (width, height) = (word(0)?, word(2)?);
    if width == 0 || height == 0 || word(4)? != 0 {
        return Err(fail);
    }
    check_size(width, height)?;
    // Each line costs at least its count word and a terminator.
    if height > data.len() / 3 {
        return Err(fail);
    }
    let mut pixels = Vec::with_capacity(width * height);
    let mut pos = CUT_HEADER_LEN;
    for _ in 0..height {
        let len = word(pos)?;
        let line = data.get(pos + 2..pos + 2 + len).ok_or(fail)?;
        pos += 2 + len;
        unpack_line(line, width, &mut pixels)?;
    }
    let palette = companions
        .get("pal")
        .and_then(|pal| parse_pal(&pal))
        .unwrap_or_else(|| (0..256u32).map(|v| v * 0x01_01_01).collect());
    Image::from_indexed(width as u32, height as u32, &pixels, &palette)
}

/// Appends exactly `width` pixels decoded from one line's run data.
fn unpack_line(line: &[u8], width: usize, out: &mut Vec<u8>) -> Result<(), DecodeError> {
    let fail = DecodeError::Unrecognized;
    let start = out.len();
    let mut pos = 0;
    while let Some(&control) = line.get(pos) {
        pos += 1;
        let n = usize::from(control & 0x7f);
        if n == 0 {
            break;
        }
        if out.len() - start + n > width {
            return Err(fail);
        }
        if control & 0x80 != 0 {
            let value = *line.get(pos).ok_or(fail)?;
            pos += 1;
            out.resize(out.len() + n, value);
        } else {
            out.extend_from_slice(line.get(pos..pos + n).ok_or(fail)?);
            pos += n;
        }
    }
    if out.len() - start == width {
        Ok(())
    } else {
        Err(fail)
    }
}

/// Palette entries of a PAL file, or `None` if it isn't one.
fn parse_pal(data: &[u8]) -> Option<Vec<u32>> {
    if data.get(..2)? != b"AH" {
        return None;
    }
    let max = |at| match le16(data, at) {
        Some(m @ 1..=255) => u32::from(m),
        _ => 255,
    };
    let (mr, mg, mb) = (max(0x0e), max(0x10), max(0x12));
    let scale = |v: u8, m: u32| (u32::from(v) * 255 / m).min(255);
    let colors: Vec<u32> = data
        .get(PAL_HEADER_LEN..)?
        .as_chunks::<3>()
        .0
        .iter()
        .take(256)
        .map(|c| scale(c[0], mr) << 16 | scale(c[1], mg) << 8 | scale(c[2], mb))
        .collect();
    (!colors.is_empty()).then_some(colors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NoCompanions;

    fn cut() -> Vec<u8> {
        // 4x2: line 1 is a run of four 3s; line 2 is 1, 2 then a run of two 9s.
        let mut d = alloc::vec![4, 0, 2, 0, 0, 0];
        d.extend_from_slice(&[3, 0, 0x84, 3, 0]);
        d.extend_from_slice(&[6, 0, 2, 1, 2, 0x82, 9, 0]);
        d
    }

    #[test]
    fn decodes_with_grey_default_palette() {
        let image = decode_cut(&cut(), &NoCompanions).unwrap();
        assert_eq!((image.width(), image.height()), (4, 2));
        assert_eq!(&image.rgb()[..3], &[3, 3, 3]);
        assert_eq!(&image.rgb()[12..15], &[1, 1, 1]);
        assert_eq!(&image.rgb()[21..24], &[9, 9, 9]);
    }

    #[test]
    fn pal_companion_is_scaled() {
        struct Pal;
        impl Companions for Pal {
            fn get(&self, ext: &str) -> Option<Vec<u8>> {
                (ext == "pal").then(|| {
                    let mut p = alloc::vec![0u8; PAL_HEADER_LEN];
                    p[..2].copy_from_slice(b"AH");
                    p[0x0e] = 63;
                    p[0x10] = 63;
                    p[0x12] = 63;
                    p.extend_from_slice(&[0; 9]);
                    p.extend_from_slice(&[63, 0, 21]);
                    p.resize(PAL_HEADER_LEN + 30, 0);
                    p
                })
            }
        }
        let image = decode_cut(&cut(), &Pal).unwrap();
        assert_eq!(&image.rgb()[..3], &[255, 0, 85]);
    }

    #[test]
    fn rejects_wrong_line_widths() {
        let mut d = cut();
        d[8] = 0x83; // first run now yields 3 pixels
        assert!(decode_cut(&d, &NoCompanions).is_err());
        assert!(decode_cut(&cut()[..10], &NoCompanions).is_err());
    }
}
