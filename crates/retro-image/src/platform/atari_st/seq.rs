//! Cyber Paint Sequence animations (`SEQ`): the first non-empty frame.
//!
//! Sources:
//! - <https://temlib.org/AtariForumWiki/index.php?title=Cyber_Paint_Sequence_file_format>
//!   (file and frame headers, change boxes, control words, column layout)
//! - <https://www.fileformat.info/format/atari/egff.htm> (EGFF Atari summary)
//! - Reverse engineered from sample files (RECOIL does not read `SEQ`): a
//!   table of one big-endian frame offset per frame sits between the 128-byte
//!   file header and the first frame; a literal run is a control word with the
//!   high bit set and the count in the low 15 bits (sign-magnitude); a box
//!   holds `ceil(width / 16)` words per column even when `x` is not a
//!   multiple of 16, so its pixels start at bit `x` rather than word `x / 16`.
//!   Every frame stream in the samples ends exactly at its stored length.
//!
//! Only the first frame that has a non-empty box is drawn: that frame covers
//! the blank screen, so no earlier frame is needed.

use alloc::vec;
use alloc::vec::Vec;

use super::common::{palette_words, st_palette};
use crate::bytes::{be16, be32};
use crate::{DecodeError, Image};

const HEADER_LEN: usize = 128;
const FRAME_HEADER_LEN: usize = 128;
const SCREEN_WIDTH: usize = 320;
const SCREEN_HEIGHT: usize = 200;
const PLANES: usize = 4;
const PALETTE_OFFSET: usize = 4;

pub(super) fn decode_seq(data: &[u8]) -> Result<Image, DecodeError> {
    decode(data).ok_or(DecodeError::Invalid)
}

/// One frame's header fields and data.
struct Frame<'a> {
    palette: Vec<u16>,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    compressed: bool,
    payload: &'a [u8],
}

fn decode(data: &[u8]) -> Option<Image> {
    if !matches!(be16(data, 0)?, 0xfedb | 0xfedc) {
        return None;
    }
    let count = usize::try_from(be32(data, 4)?).ok()?;
    // The offset table must fit in the file, which also bounds `count`.
    if count == 0 || HEADER_LEN.checked_add(count.checked_mul(4)?)? > data.len() {
        return None;
    }
    for index in 0..count {
        let offset = usize::try_from(be32(data, HEADER_LEN + index * 4)?).ok()?;
        let frame = parse_frame(data, offset)?;
        if frame.width != 0 && frame.height != 0 {
            return draw(&frame);
        }
    }
    None
}

fn parse_frame(data: &[u8], offset: usize) -> Option<Frame<'_>> {
    let header = data.get(offset..offset.checked_add(FRAME_HEADER_LEN)?)?;
    // Resolution must be low.
    if be16(header, 2)? != 0 {
        return None;
    }
    let x = usize::from(be16(header, 54)?);
    let y = usize::from(be16(header, 56)?);
    let width = usize::from(be16(header, 58)?);
    let height = usize::from(be16(header, 60)?);
    let (operation, storage) = (header[62], header[63]);
    if operation > 1 || storage > 1 {
        return None;
    }
    let length = usize::try_from(be32(header, 64)?).ok()?;
    let start = offset + FRAME_HEADER_LEN;
    let payload = data.get(start..start.checked_add(length)?)?;
    if x + width > SCREEN_WIDTH || y + height > SCREEN_HEIGHT {
        return None;
    }
    Some(Frame {
        palette: palette_words(header, PALETTE_OFFSET, 16)?,
        x,
        y,
        width,
        height,
        compressed: storage == 1,
        payload,
    })
}

fn draw(frame: &Frame<'_>) -> Option<Image> {
    let groups = frame.width.div_ceil(16);
    let total = PLANES * groups * frame.height;
    let columns = if frame.compressed {
        unpack(frame.payload, total)?
    } else {
        // Uncompressed data holds exactly the expanded words.
        if frame.payload.len() != total * 2 {
            return None;
        }
        frame
            .payload
            .as_chunks::<2>()
            .0
            .iter()
            .map(|word| u16::from_be_bytes(*word))
            .collect()
    };
    let mut indices = vec![0u8; SCREEN_WIDTH * SCREEN_HEIGHT];
    // Plane-major, then word column, then scanline.
    for plane in 0..PLANES {
        for group in 0..groups {
            let column = (plane * groups + group) * frame.height;
            for row in 0..frame.height {
                let word = columns[column + row];
                let line = (frame.y + row) * SCREEN_WIDTH + frame.x + group * 16;
                for bit in 0..16 {
                    if group * 16 + bit >= frame.width {
                        break;
                    }
                    indices[line + bit] |= u8::from((word >> (15 - bit)) & 1 != 0) << plane;
                }
            }
        }
    }
    Image::from_indexed(
        SCREEN_WIDTH as u32,
        SCREEN_HEIGHT as u32,
        &indices,
        &st_palette(&frame.palette),
    )
    .ok()
}

/// Expands control-word runs into exactly `total` words, consuming the whole
/// stream. A set high bit means that many literal words follow; otherwise
/// the next word repeats that many times.
fn unpack(stream: &[u8], total: usize) -> Option<Vec<u16>> {
    let mut out = Vec::with_capacity(total);
    let mut pos = 0;
    while pos < stream.len() {
        let control = be16(stream, pos)?;
        pos += 2;
        let run = usize::from(control & 0x7fff);
        if run > total - out.len() {
            return None;
        }
        if control & 0x8000 != 0 {
            for _ in 0..run {
                out.push(be16(stream, pos)?);
                pos += 2;
            }
        } else {
            let word = be16(stream, pos)?;
            pos += 2;
            out.resize(out.len() + run, word);
        }
    }
    (out.len() == total).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A one-frame file drawing a 20x2 box at (3, 1): plane 0 set everywhere.
    fn sample(stream: &[u8], width: u16) -> Vec<u8> {
        let mut data = vec![0u8; HEADER_LEN];
        data[..2].copy_from_slice(&0xfedbu16.to_be_bytes());
        data[7] = 1;
        data.extend_from_slice(&132u32.to_be_bytes());
        let mut frame = vec![0u8; FRAME_HEADER_LEN];
        frame[6..8].copy_from_slice(&0x0777u16.to_be_bytes());
        frame[54..64].copy_from_slice(&[0, 3, 0, 1, (width >> 8) as u8, width as u8, 0, 2, 0, 1]);
        frame[64..68].copy_from_slice(&(stream.len() as u32).to_be_bytes());
        data.extend_from_slice(&frame);
        data.extend_from_slice(stream);
        data
    }

    #[test]
    fn draws_unaligned_box_from_runs() {
        // Plane 0: two columns x two rows of 0xffff (repeat 4), then three
        // planes of zeros (repeat 12).
        let stream = [0, 4, 0xff, 0xff, 0, 12, 0, 0];
        let image = decode_seq(&sample(&stream, 20)).unwrap();
        assert_eq!((image.width(), image.height()), (320, 200));
        let lit = |x: u32, y: u32| image.get(x, y) != 0;
        assert!(!lit(2, 1) && lit(3, 1) && lit(3 + 19, 2) && !lit(3 + 20, 2) && !lit(3, 3));
    }

    #[test]
    fn rejects_overlong_and_short_streams() {
        assert!(decode_seq(&sample(&[0, 17, 0, 0], 20)).is_err());
        assert!(decode_seq(&sample(&[0, 4, 0, 0], 20)).is_err());
        assert!(decode_seq(&sample(&[0x80, 2, 0, 0], 20)).is_err());
    }
}
