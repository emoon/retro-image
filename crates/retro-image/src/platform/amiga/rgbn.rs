//! Impulse RGBN and RGB8: run-length coded 12- and 24-bit IFF pictures
//! (Turbo Silver, Imagine).
//!
//! Source: <https://wiki.amigaos.net/wiki/RGBN_and_RGB8_IFF_Image_Data>
//! (BMHD with 13 or 25 planes and compression 4, CAMG, the BODY entry layouts
//! and repeat counts). Behaviours the page leaves open were probed with
//! synthetic files fed to `recoil2png` (no real sample exists): the 4-bit
//! channels are expanded by 17, the genlock bit is ignored, a run past the
//! end of the picture is clamped, a short body is rejected, a 16-bit RGBN
//! count of 0 means 65536, and CAMG hires/interlace scale the picture like
//! ILBM.

use super::iff::find;
use super::ilbm::{Header, scale_factors};
use crate::bytes::{be16, be32};
use crate::image::check_scaled;
use crate::{DecodeError, Image};

/// Which of the two entry layouts a FORM uses.
#[derive(Clone, Copy)]
pub(super) enum Kind {
    /// 16-bit entries: 12 colour bits, genlock, 3-bit count.
    Rgbn,
    /// 32-bit entries: 24 colour bits, genlock, 7-bit count.
    Rgb8,
}

impl Kind {
    fn planes(self) -> usize {
        match self {
            Kind::Rgbn => 13,
            Kind::Rgb8 => 25,
        }
    }
}

pub(super) fn decode(kind: Kind, contents: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let header = Header::parse(contents).ok_or(fail)?;
    if header.planes != kind.planes() || header.compression != 4 {
        return Err(fail);
    }
    let body = find(contents, b"BODY").ok_or(fail)?;
    let camg = find(contents, b"CAMG")
        .and_then(|c| be32(c, 0))
        .unwrap_or(0);
    let (sx, sy) = scale_factors(camg);
    check_scaled(header.width, header.height, sx as usize, sy as usize)?;
    let total = header.width * header.height;
    // Every entry is at least 2 bytes and repeats at most 65536 pixels.
    if total > body.len().saturating_mul(32768) {
        return Err(fail);
    }
    let mut image = Image::new(header.width as u32, header.height as u32);
    let (mut pos, mut done) = (0, 0);
    while done < total {
        let (color, count, used) = match kind {
            Kind::Rgbn => rgbn_entry(body, pos).ok_or(fail)?,
            Kind::Rgb8 => rgb8_entry(body, pos).ok_or(fail)?,
        };
        pos += used;
        let end = total.min(done + count);
        for i in done..end {
            image.set((i % header.width) as u32, (i / header.width) as u32, color);
        }
        done = end;
    }
    Ok(image.scaled(sx, sy))
}

/// The colour, repeat count and byte length of the RGBN entry at `pos`.
fn rgbn_entry(body: &[u8], pos: usize) -> Option<(u32, usize, usize)> {
    let word = be16(body, pos)?;
    let expand = |n: u16| u32::from(n & 15) * 17;
    let color = expand(word >> 12) << 16 | expand(word >> 8) << 8 | expand(word >> 4);
    match word & 7 {
        0 => match *body.get(pos + 2)? {
            0 => match be16(body, pos + 3)? {
                0 => Some((color, 65536, 5)),
                n => Some((color, usize::from(n), 5)),
            },
            n => Some((color, usize::from(n), 3)),
        },
        n => Some((color, usize::from(n), 2)),
    }
}

/// The colour, repeat count and byte length of the RGB8 entry at `pos`.
fn rgb8_entry(body: &[u8], pos: usize) -> Option<(u32, usize, usize)> {
    let long = be32(body, pos)?;
    // A zero count is not defined for RGB8.
    match long & 0x7f {
        0 => None,
        n => Some((long >> 8, n as usize, 4)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    fn form(kind: &[u8; 4], planes: u8, w: u16, h: u16, camg: u32, body: &[u8]) -> Vec<u8> {
        let mut c = Vec::new();
        c.extend_from_slice(b"BMHD\0\0\0\x14");
        c.extend_from_slice(&w.to_be_bytes());
        c.extend_from_slice(&h.to_be_bytes());
        c.extend_from_slice(&[0, 0, 0, 0, planes, 0, 4, 0, 0, 0, 10, 11, 1, 0x40, 0, 200]);
        c.extend_from_slice(b"CAMG\0\0\0\x04");
        c.extend_from_slice(&camg.to_be_bytes());
        c.extend_from_slice(b"BODY");
        c.extend_from_slice(&(body.len() as u32).to_be_bytes());
        c.extend_from_slice(body);
        if body.len() % 2 == 1 {
            c.push(0);
        }
        let mut out = b"FORM".to_vec();
        out.extend_from_slice(&(c.len() as u32 + 4).to_be_bytes());
        out.extend_from_slice(kind);
        out.extend_from_slice(&c);
        out
    }

    fn decode_form(data: &[u8], kind: Kind) -> Result<Image, DecodeError> {
        let (_, contents) = super::super::iff::form(data).unwrap();
        decode(kind, contents)
    }

    #[test]
    fn rgbn_runs_and_extended_counts() {
        // Red x4 (3-bit count); green x4 (byte count); blue with a word
        // count of 9, clamped to the 4 pixels left.
        let body = [
            0xf0, 0x04, // red, count 4
            0x0f, 0x00, 4, // green, byte count 4
            0x00, 0xf8, 0, 0, 9, // blue with genlock set, word count 9 (clamped to 4)
        ];
        let data = form(b"RGBN", 13, 4, 3, 0, &body);
        let image = decode_form(&data, Kind::Rgbn).unwrap();
        assert_eq!((image.width(), image.height()), (4, 3));
        assert_eq!(image.get(0, 0), 0xff0000);
        assert_eq!(image.get(0, 1), 0x00ff00);
        assert_eq!(image.get(0, 2), 0x0000ff);
    }

    #[test]
    fn rgb8_runs_and_zero_count() {
        let body = [0x12, 0x34, 0x56, 4, 0xff, 0xff, 0xff, 4];
        let data = form(b"RGB8", 25, 4, 2, 0, &body);
        let image = decode_form(&data, Kind::Rgb8).unwrap();
        assert_eq!(image.get(3, 0), 0x123456);
        assert_eq!(image.get(0, 1), 0xffffff);
        let data = form(b"RGB8", 25, 4, 1, 0, &[1, 2, 3, 0]);
        assert!(decode_form(&data, Kind::Rgb8).is_err());
    }

    #[test]
    fn rejects_short_body_and_wrong_header() {
        let data = form(b"RGBN", 13, 4, 2, 0, &[0xf0, 0x04]);
        assert!(decode_form(&data, Kind::Rgbn).is_err());
        let data = form(b"RGBN", 12, 4, 1, 0, &[0xf0, 0x04]);
        assert!(decode_form(&data, Kind::Rgbn).is_err());
    }

    #[test]
    fn hires_camg_doubles_rows() {
        let data = form(b"RGBN", 13, 4, 1, 0x8000, &[0xf0, 0x04]);
        let image = decode_form(&data, Kind::Rgbn).unwrap();
        assert_eq!((image.width(), image.height()), (4, 2));
    }
}
