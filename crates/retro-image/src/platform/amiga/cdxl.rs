//! CDXL, the video format of the Commodore CDTV and CD32 (and the CD-ROM
//! add-on for the Amiga), shown as its first frame.
//!
//! Sources:
//! - The field layout of the 32-byte frame header (frame size, previous
//!   frame size, frame number, width, height, planes, palette size, sound
//!   size) and the encoding and arrangement bits: the MultimediaWiki page
//!   <https://wiki.multimedia.cx/index.php/CDXL>, which is community written.
//! - Reverse engineered from Sembiance's `video/cdxl` samples (10 files,
//!   which differ from the wiki as noted) with `ffmpeg -i x -frames:v 1`
//!   run as a black box for the pixels: the header is
//!   `type(1) info(1) frame size(4) previous size(4) frame number(4)
//!   width(2) height(2) reserved(1) planes(1) palette size(2) sound
//!   size(2)`, all big-endian, followed by the 12-bit `0RGB` palette words,
//!   then the bitplanes, then the 8-bit sound.
//! - Each plane row is padded to 16 pixels (a width of 225 takes 30 bytes,
//!   a width of 68 takes 10). With info bits 5-7 zero the planes follow one
//!   another; the one sample with 0x80 there (`optologo.cdxl`) interleaves
//!   the planes row by row, like ILBM. Info bit 0 is HAM (6 planes: HAM6, 8
//!   planes: HAM8); 0 is a plain palette picture.
//! - `optologo.cdxl` has a `1` in the byte before the plane count and 388
//!   bytes more than its header accounts for, and `Maku.XL` 24 bytes more per
//!   frame; both are after the picture and are not read. The wiki's
//!   description does not cover either.
//!
//! - HAM8 uses the expansion shared with ILBM (the 6 data bits repeated into
//!   the low 2). ffmpeg instead keeps the low 2 bits of the held color, as
//!   the AGA chip does: the two HAM8 samples (`vista1.xl`, `optologo.cdxl`)
//!   differ from its output by at most 3 levels in the modified channel
//!   (checked: 0 mismatches against ffmpeg with the held low bits), and
//!   the other eight samples are pixel-identical to it.
//!
//! Not decoded: YUV and DCTV-style encodings, and the byte-planar, chunky
//! and byte-line arrangements (no samples). The format has no signature, so
//! the decoder is gated on its extension and a strict header check.
//!
//! RECOIL has no CDXL support.

use alloc::vec::Vec;

use super::chunky::{Pixels, Rows, render};
use crate::bytes::{be16, be32};
use crate::image::{planar_pixels, rgb444};
use crate::{DecodeError, Image};

const HEADER_LEN: usize = 32;
const ENCODING_MASK: u8 = 0x07;
const ENCODING_HAM: u8 = 1;
const ARRANGEMENT_MASK: u8 = 0xe0;
const BIT_PLANAR: u8 = 0x00;
const BIT_LINE: u8 = 0x80;

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let info = *data.get(1).ok_or(fail)?;
    let frame_len = be32(data, 2).ok_or(fail)? as usize;
    let width = usize::from(be16(data, 14).ok_or(fail)?);
    let height = usize::from(be16(data, 16).ok_or(fail)?);
    let planes = usize::from(*data.get(19).ok_or(fail)?);
    let palette_len = usize::from(be16(data, 20).ok_or(fail)?);
    let sound_len = usize::from(be16(data, 22).ok_or(fail)?);

    let mode = match (info & ENCODING_MASK, planes) {
        (ENCODING_HAM, 6) => Pixels::Ham6,
        (ENCODING_HAM, 8) => Pixels::Ham8,
        (0, 1..=8) => Pixels::Indexed8,
        _ => return Err(fail),
    };
    let rows = Rows {
        width,
        height,
        bytes_per_line: width,
    };
    rows.len(mode)?;
    let row_len = width.div_ceil(16) * 2;
    let video_len = row_len * planes * height;
    // The frame holds the header, palette, picture and sound, in that order;
    // it may be longer (see above), never shorter.
    let picture_end = HEADER_LEN + palette_len + video_len;
    if palette_len % 2 != 0 || frame_len < picture_end + sound_len {
        return Err(fail);
    }
    let interleaved = match info & ARRANGEMENT_MASK {
        BIT_PLANAR => false,
        BIT_LINE => true,
        _ => return Err(fail),
    };
    let words = data.get(HEADER_LEN..HEADER_LEN + palette_len).ok_or(fail)?;
    let video = data
        .get(HEADER_LEN + palette_len..picture_end)
        .ok_or(fail)?;

    let mut palette = [0u32; 256];
    for (entry, word) in palette.iter_mut().zip(words.as_chunks::<2>().0) {
        *entry = rgb444(u16::from_be_bytes(*word));
    }
    let indices: Vec<u8> = planar_pixels(video, width, height, row_len, planes, |plane, y| {
        if interleaved {
            (y * planes + plane) * row_len
        } else {
            (plane * height + y) * row_len
        }
    })
    .into_iter()
    .map(|v| v as u8)
    .collect();
    render(mode, rows, &indices, &palette)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A frame of 16x2 pixels, 2 planes, a 4-color palette (black, red,
    /// green, blue) and the given picture bytes; `info` sets the encoding.
    fn frame(info: u8, video: &[u8]) -> Vec<u8> {
        let palette = [0x0000u16, 0x0f00, 0x00f0, 0x000f];
        let size = HEADER_LEN + 8 + video.len();
        let mut out = alloc::vec![1, info];
        out.extend_from_slice(&(size as u32).to_be_bytes());
        out.extend_from_slice(&[0; 4]); // previous frame size
        out.extend_from_slice(&1u32.to_be_bytes()); // frame number
        out.extend_from_slice(&16u16.to_be_bytes());
        out.extend_from_slice(&2u16.to_be_bytes());
        out.extend_from_slice(&[0, 2]); // reserved, planes
        out.extend_from_slice(&8u16.to_be_bytes());
        out.extend_from_slice(&[0; 10]); // sound size and reserved
        for color in palette {
            out.extend_from_slice(&color.to_be_bytes());
        }
        out.extend_from_slice(video);
        out
    }

    const RED: [u8; 3] = [255, 0, 0];
    const GREEN: [u8; 3] = [0, 255, 0];
    const BLUE: [u8; 3] = [0, 0, 255];

    #[test]
    fn planes_follow_one_another_or_interleave_by_row() {
        // Row 0: pixel 0 = 1 (plane 0 bit), pixel 1 = 2 (plane 1 bit),
        // pixel 2 = 3. Row 1: pixel 0 = 3.
        let planar = [
            0b1010_0000,
            0,
            0b1000_0000,
            0, // plane 0 rows 0 and 1
            0b0110_0000,
            0,
            0b1000_0000,
            0, // plane 1 rows 0 and 1
        ];
        let image = decode(&frame(0, &planar)).unwrap();
        assert_eq!(&image.rgb()[..9], [RED, GREEN, BLUE].concat().as_slice());
        assert_eq!(&image.rgb()[16 * 3..16 * 3 + 3], BLUE);
        let interleaved = [
            0b1010_0000,
            0,
            0b0110_0000,
            0, // row 0: plane 0, plane 1
            0b1000_0000,
            0,
            0b1000_0000,
            0, // row 1
        ];
        assert_eq!(decode(&frame(0x80, &interleaved)).unwrap(), image);
    }

    #[test]
    fn rejects_unsupported_encodings_and_short_frames() {
        let video = [0u8; 8];
        assert!(decode(&frame(2, &video)).is_err(), "YUV");
        assert!(decode(&frame(0x40, &video)).is_err(), "chunky");
        assert!(decode(&frame(0, &video[..7])).is_err(), "short picture");
        let mut zero_planes = frame(0, &video);
        zero_planes[19] = 0;
        assert!(decode(&zero_planes).is_err());
        // The frame size must cover the picture.
        let mut small = frame(0, &video);
        small[2..6].copy_from_slice(&40u32.to_be_bytes());
        assert!(decode(&small).is_err());
    }
}
