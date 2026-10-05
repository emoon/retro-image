//! YAFA animations (`FORM YAFA`), shown as their first frame.
//!
//! Sources:
//! - "YAFA, an IFF format for animations", release V1.0, 1996-05-26, by
//!   Michael Henke and Andreas Maschke (<https://aminet.net/docs/misc/YAFA-doc.lha>,
//!   `YAFA-doc.txt`): the `INFO` fields, the `DRGB` palette (a
//!   `LoadRGB32` table), the frame types, the palette stored behind each
//!   frame, and the ANIM-7 style delta frames.
//! - Checked on Sembiance's `video/iffYAFA` (13 files): eleven are chunky
//!   (frame type 3) with XPK FAST or NUKE frames, two (`a.yafa`,
//!   `1649768106_b.yafa`) are planar with the delta flag.
//!
//! Where the samples differ from the document:
//! - The document says the first two frames of a delta-compressed file are
//!   stored uncompressed. In both delta samples the first frame is delta
//!   coded already: `PROF` puts its end below the size of a plain frame, it
//!   starts with the 16-pointer table, and applying it to an empty bitmap
//!   gives a clean picture. A first frame is therefore taken as plain when
//!   `PROF` says it has the plain size, and as a delta against an empty
//!   bitmap otherwise.
//! - `INFO` flags 0x20 and 0x40 are set in some files; the document does not
//!   define them and they are ignored, as are the `ODD!` chunk, `TTBL` and
//!   the playback speed.
//!
//! Planar frames are read as whole bitplanes one after another (the document
//! says "simply raw bitplanes"; the delta samples confirm that planes are
//! separate buffers, but no plain planar frame was available). Delta frames of
//! chunky data (the document treats them as eight bitplanes) are not decoded.
//! RECOIL has no YAFA support.

use alloc::borrow::Cow;
use alloc::vec::Vec;

use super::chunky::{Pixels, Rows, render};
use super::iff::find;
use crate::bytes::{be16, be32};
use crate::codec::xpk;
use crate::image::planar_pixels;
use crate::{DecodeError, Image};

const FLAG_HAM: u16 = 1;
const FLAG_PALETTE_PER_FRAME: u16 = 2;
const FLAG_DELTA: u16 = 4;
/// The size in bits of a delta element: byte, word, long.
const FLAG_DELTA_WORD: u16 = 8;
const FLAG_DELTA_LONG: u16 = 16;

/// Largest palette a frame can carry: a `LoadRGB32` header and 256 colors.
const MAX_PALETTE_LEN: usize = 4 + 256 * 12;

#[derive(Clone, Copy, PartialEq, Eq)]
enum FrameType {
    Planar,
    PlanarXpk,
    ChunkyXpk,
    Chunky,
}

struct Info {
    width: usize,
    height: usize,
    depth: usize,
    frame_type: FrameType,
    flags: u16,
}

impl Info {
    fn parse(info: &[u8]) -> Option<Self> {
        let word = |i: usize| be16(info, i * 2).map(usize::from);
        let frame_type = match word(5)? {
            0 => FrameType::Planar,
            1 => FrameType::PlanarXpk,
            3 => FrameType::ChunkyXpk,
            4 => FrameType::Chunky,
            _ => return None,
        };
        let info = Self {
            width: word(0)?,
            height: word(1)?,
            depth: word(2)?,
            frame_type,
            flags: be16(info, 12)?,
        };
        let frames = word(4)?;
        (frames > 0 && (1..=8).contains(&info.depth)).then_some(info)
    }

    fn is_planar(&self) -> bool {
        matches!(self.frame_type, FrameType::Planar | FrameType::PlanarXpk)
    }

    fn is_xpk(&self) -> bool {
        matches!(self.frame_type, FrameType::PlanarXpk | FrameType::ChunkyXpk)
    }

    /// Bytes of one bitplane row.
    fn plane_row_len(&self) -> usize {
        self.width.div_ceil(16) * 2
    }

    /// Bytes of the pixels of one plain frame.
    fn frame_len(&self) -> usize {
        if self.is_planar() {
            self.plane_row_len() * self.depth * self.height
        } else {
            self.width * self.height
        }
    }

    /// Bytes of an element of a delta frame's data lists.
    fn delta_element(&self) -> usize {
        if self.flags & FLAG_DELTA_LONG != 0 {
            4
        } else if self.flags & FLAG_DELTA_WORD != 0 {
            2
        } else {
            1
        }
    }
}

pub(super) fn decode(contents: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let info = find(contents, b"INFO").and_then(Info::parse).ok_or(fail)?;
    let body = find(contents, b"BODY").ok_or(fail)?;
    let rows = Rows {
        width: info.width,
        height: info.height,
        bytes_per_line: info.width,
    };
    // Checks the picture size before anything is allocated from it.
    rows.len(Pixels::Indexed8)?;
    let frame_len = info.frame_len();
    let per_frame_palette = info.flags & FLAG_PALETTE_PER_FRAME != 0;

    let frame = if info.is_xpk() {
        let limit = frame_len
            + if per_frame_palette {
                MAX_PALETTE_LEN
            } else {
                0
            };
        Cow::Owned(xpk::unpack(body, limit).ok_or(fail)?)
    } else {
        Cow::Borrowed(body)
    };
    let pixels = if is_delta_frame(contents, &info) {
        // The palette of a delta frame would sit behind data of unknown length.
        if !info.is_planar() || per_frame_palette {
            return Err(fail);
        }
        undo_delta(&frame, &info).ok_or(fail)?
    } else {
        frame.get(..frame_len).ok_or(fail)?.to_vec()
    };

    let palette = if per_frame_palette {
        load_rgb32(frame.get(frame_len..).ok_or(fail)?)
    } else {
        find(contents, b"DRGB").and_then(load_rgb32)
    }
    .ok_or(fail)?;

    let indices = if info.is_planar() {
        let row_len = info.plane_row_len();
        let plane_len = row_len * info.height;
        planar_pixels(
            &pixels,
            info.width,
            info.height,
            row_len,
            info.depth,
            |plane, y| plane * plane_len + y * row_len,
        )
        .into_iter()
        .map(|v| v as u8)
        .collect()
    } else {
        pixels
    };
    let mode = match (info.flags & FLAG_HAM != 0, info.depth) {
        (true, 6) => Pixels::Ham6,
        (true, 8) => Pixels::Ham8,
        _ => Pixels::Indexed8,
    };
    render(mode, rows, &indices, &palette, None)
}

/// Whether the first frame is delta coded: the file has the delta flag and
/// `PROF` does not say the frame has the size of a plain one.
fn is_delta_frame(contents: &[u8], info: &Info) -> bool {
    let first_end = find(contents, b"PROF").and_then(|prof| be32(prof, 0));
    info.flags & FLAG_DELTA != 0 && first_end.is_none_or(|end| end as usize != info.frame_len())
}

/// A `LoadRGB32` table: a color count, the first register, then red, green
/// and blue as 32-bit values whose top bytes are the 8-bit colors. The
/// registers before the first stay black.
fn load_rgb32(table: &[u8]) -> Option<[u32; 256]> {
    let count = usize::from(be16(table, 0)?);
    let first = usize::from(be16(table, 2)?);
    let colors = table.get(4..4 + count * 12)?;
    let mut palette = [0u32; 256];
    for (i, rgb) in colors.as_chunks::<12>().0.iter().enumerate() {
        *palette.get_mut(first + i)? = u32::from_be_bytes([0, rgb[0], rgb[4], rgb[8]]);
    }
    Some(palette)
}

/// Applies an ANIM-7 style delta frame to empty bitplanes: eight plane
/// pointers to opcode lists, eight to data lists, then per plane a list of
/// vertical columns, each a count of ops that skip rows, copy items from the
/// data list (high bit set) or repeat one item (0, count).
fn undo_delta(frame: &[u8], info: &Info) -> Option<Vec<u8>> {
    let element = info.delta_element();
    let row_len = info.plane_row_len();
    if !row_len.is_multiple_of(element) {
        return None;
    }
    let plane_len = row_len * info.height;
    let mut planes = alloc::vec![0u8; plane_len * info.depth];
    for (plane, out) in planes.chunks_exact_mut(plane_len).enumerate() {
        let ops = be32(frame, plane * 4)? as usize;
        let data = be32(frame, 32 + plane * 4)? as usize;
        if ops == 0 {
            continue;
        }
        let (mut ops, mut data) = (ops, data);
        let next_byte = |at: &mut usize| {
            let byte = *frame.get(*at)?;
            *at += 1;
            Some(byte)
        };
        for column in 0..row_len / element {
            let mut row = 0;
            let mut put = |row: &mut usize, data: &mut usize| -> Option<()> {
                let item = frame.get(*data..data.checked_add(element)?)?;
                let at = *row * row_len + column * element;
                out.get_mut(at..at + element)?.copy_from_slice(item);
                *data += element;
                *row += 1;
                Some(())
            };
            for _ in 0..next_byte(&mut ops)? {
                match next_byte(&mut ops)? {
                    0 => {
                        // The same item for the next `count` rows.
                        let count = next_byte(&mut ops)?;
                        let item = data;
                        for _ in 0..count {
                            data = item;
                            put(&mut row, &mut data)?;
                        }
                    }
                    op if op & 0x80 != 0 => {
                        for _ in 0..op & 0x7f {
                            put(&mut row, &mut data)?;
                        }
                    }
                    skip => row += usize::from(skip),
                }
            }
        }
    }
    Some(planes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(id: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut out = id.to_vec();
        out.extend_from_slice(&(body.len() as u32).to_be_bytes());
        out.extend_from_slice(body);
        out.resize(out.len() + body.len() % 2, 0);
        out
    }

    fn info(width: u16, height: u16, depth: u16, frame_type: u16, flags: u16) -> Vec<u8> {
        let words = [width, height, depth, 1, 1, frame_type, flags];
        chunk(
            b"INFO",
            &words
                .iter()
                .flat_map(|w| w.to_be_bytes())
                .collect::<Vec<u8>>(),
        )
    }

    /// A `LoadRGB32` table of 4 colors: black, red, green, blue.
    fn colors() -> Vec<u8> {
        let mut table = alloc::vec![0, 4, 0, 0];
        for rgb in [[0, 0, 0], [255, 0, 0], [0, 255, 0], [0, 0, 255]] {
            for c in rgb {
                table.extend_from_slice(&[c, 0, 0, 0]);
            }
        }
        table
    }

    const RED: [u8; 3] = [255, 0, 0];
    const GREEN: [u8; 3] = [0, 255, 0];
    const BLUE: [u8; 3] = [0, 0, 255];

    #[test]
    fn chunky_frame_with_the_shared_palette() {
        let contents = [
            info(2, 2, 2, 4, 0),
            chunk(b"DRGB", &colors()),
            chunk(b"BODY", &[1, 2, 3, 1]),
        ]
        .concat();
        let image = decode(&contents).unwrap();
        assert_eq!(image.rgb(), [RED, GREEN, BLUE, RED].concat());
    }

    #[test]
    fn palette_per_frame_follows_the_pixels() {
        let mut body = alloc::vec![1u8, 2];
        body.extend_from_slice(&colors());
        let contents = [
            info(2, 1, 2, 4, FLAG_PALETTE_PER_FRAME),
            chunk(b"BODY", &body),
        ]
        .concat();
        assert_eq!(decode(&contents).unwrap().rgb(), [RED, GREEN].concat());
        // No DRGB and no palette behind the frame: nothing to draw with.
        let bare = [info(2, 1, 2, 4, 0), chunk(b"BODY", &[1, 2])].concat();
        assert!(decode(&bare).is_err());
    }

    #[test]
    fn planar_frames_hold_whole_planes() {
        // 16x1, two planes: plane 0 is 0xA000, plane 1 is 0x6000 (the first
        // pixels are 1, 2, 3, 0 ...).
        let body = [0xa0, 0x00, 0x60, 0x00];
        let contents = [
            info(16, 1, 2, 0, 0),
            chunk(b"DRGB", &colors()),
            chunk(b"BODY", &body),
        ]
        .concat();
        let image = decode(&contents).unwrap();
        assert_eq!(
            &image.rgb()[..12],
            [RED, GREEN, BLUE, [0, 0, 0]].concat().as_slice()
        );
    }

    #[test]
    fn delta_frame_against_an_empty_bitmap() {
        // 16x2, one plane, byte elements: two columns. Column 0 copies two
        // items (0x80, 0x40); column 1 repeats one item (0xff) over both rows.
        let mut frame = alloc::vec![0u8; 64];
        let ops: [u8; 5] = [1, 0x82, 1, 0x00, 0x02];
        frame[..4].copy_from_slice(&64u32.to_be_bytes());
        frame[32..36].copy_from_slice(&(64 + ops.len() as u32).to_be_bytes());
        frame.extend_from_slice(&ops);
        frame.extend_from_slice(&[0x80, 0x40, 0xff]);
        let contents = [
            info(16, 2, 1, 0, FLAG_DELTA),
            chunk(b"DRGB", &colors()),
            chunk(b"BODY", &frame),
        ]
        .concat();
        let image = decode(&contents).unwrap();
        // Row 0: pixel 0 set, then the second byte all ones: pixels 8-15.
        let pixel = |x: usize, y: usize| &image.rgb()[(y * 16 + x) * 3..][..3];
        assert_eq!(pixel(0, 0), RED);
        assert_eq!(pixel(1, 0), [0, 0, 0]);
        assert_eq!(pixel(9, 0), RED);
        assert_eq!(pixel(1, 1), RED);
        assert_eq!(pixel(0, 1), [0, 0, 0]);
        assert_eq!(pixel(15, 1), RED);
    }

    #[test]
    fn delta_pointers_at_the_end_of_the_address_space_are_rejected() {
        // The data list pointer plus an element size must not wrap (a panic
        // in a 32-bit debug build).
        let mut frame = alloc::vec![0u8; 64];
        frame[..4].copy_from_slice(&64u32.to_be_bytes());
        frame[32..36].copy_from_slice(&0xffff_ffffu32.to_be_bytes());
        frame.extend_from_slice(&[1, 0x81]); // one column: copy one item
        let contents = [
            info(16, 1, 1, 0, FLAG_DELTA),
            chunk(b"DRGB", &colors()),
            chunk(b"BODY", &frame),
        ]
        .concat();
        assert!(decode(&contents).is_err());
    }

    #[test]
    fn rejects_delta_chunky_frames_and_truncated_data() {
        let delta_chunky = [
            info(2, 1, 2, 4, FLAG_DELTA),
            chunk(b"DRGB", &colors()),
            chunk(b"BODY", &[0; 80]),
        ]
        .concat();
        assert!(decode(&delta_chunky).is_err());
        let short = [
            info(2, 2, 2, 4, 0),
            chunk(b"DRGB", &colors()),
            chunk(b"BODY", &[1, 2, 3]),
        ]
        .concat();
        assert!(decode(&short).is_err());
    }
}
