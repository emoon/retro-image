//! Amstrad PCW (Joyce) pictures: MicroDesign areas and pages, Stop Press
//! and MicroDesign CUT, The Desktop Publisher GRF, Stop Press canvas SPC.
//! All are 1-bit. A set bit is white in MicroDesign areas and pages (the format
//! sheet says so) and assumed white in SPC (the lit pixel of the PCW screen);
//! it is black in CUT and assumed black in GRF, see below.
//!
//! Sources:
//! - MicroDesign 3 page (`.MDP`) and area (`.MDA`) file specifications,
//!   John Elliott's page quoting Creative Technology's 1992 format sheet
//!   (<http://www.chiark.greenend.org.uk/~jacobn/cpm/mdaspec.html>): the
//!   128-byte stamp (`.MDA` or `.MDP`, `MicroDesignPCW`, version `v1.00` for
//!   MicroDesign 2 or `v1.30` for MicroDesign 3), height in lines and width in
//!   bytes at 128 and 130, the MicroDesign 2 run-length coding (`00` or `FF`
//!   followed by a count, 0 meaning 256) and the MicroDesign 3 line types
//!   (all-same byte, PackBits-style blocks, blocks XORed with the line above).
//!   The same page gives the CUT and GRF layouts, from John Elliott's own
//!   experiments (it calls them "in no way official"): CUT is height code
//!   `h1` and width code `w1` words, then rows of `(w + 8) / 8` bytes with
//!   `h = (h1 + 3) / 2` and `w = w1 + 2`; GRF is width and height words, then
//!   rows of `ceil(w / 8)` bytes.
//! - Stop Press Canvas, Just Solve the File Format Problem
//!   (<http://fileformats.archiveteam.org/wiki/Stop_Press_Canvas>, CC0),
//!   quoting John Elliott's SPC2BMP source: 720x256 pixels in 32 blocks of
//!   720 bytes, eight interleaved lines per block (line `k` is bytes `k`,
//!   `k + 8`, ...).
//! - Checked on samples in `corpus/extra/misc-computers/pcw`: five
//!   MicroDesign 2 areas (the run-length stream ends exactly at the end of the
//!   data, up to the padding noted below), `DIAGRAM.CUT` from John Elliott's
//!   `mdaspec.com` and `joyce.spc` from the Joyce emulator's Z80 utilities.
//!   The MicroDesign 3 coding, MDP pages and GRF have no real sample here and
//!   are decoded from the format sheet only.
//!
//! Choices of this crate:
//! - Polarity. The page does not give it for CUT, GRF or SPC. `DIAGRAM.CUT` is
//!   the sheet's own figure of the MicroDesign 2 coding: it draws the bytes
//!   `0F`, `CC`, `F0` ... as cells whose zero bits (black in the sheet's text)
//!   are solid blocks and whose one bits (white) are hollow boxes, on a page
//!   whose margin is zero bits. That reads as black ink on white paper only if
//!   a set bit is black, so CUT is decoded that way (one sample, so this is
//!   evidence, not a specification). GRF is "substantially similar" to CUT
//!   and assumed the same, unconfirmed; SPC assumed white, unconfirmed.
//! - Files copied from CP/M disks are padded to a multiple of 128 bytes
//!   (every sample is), so the headerless CUT and GRF accept a length that is
//!   the data size rounded up to 128.
//! - A PCW screen pixel is about twice as tall as wide, as on the other 2:1
//!   machines of this crate. SPC is a literal screen, so its lines are
//!   doubled, which makes the Joyce splash picture look right. The
//!   MicroDesign pictures and CUT and GRF stay at 1:1: they are print
//!   bitmaps, and the real samples (a bust, a cross, lettering) would look
//!   stretched when doubled.
//! - The headerless formats go last in the registry, after the other claimants
//!   of `.cut`, `.grf` and `.spc`.

use alloc::vec::Vec;

use crate::bytes::le16;
use crate::image::check_size;
use crate::{BitOrder, DecodeError, Format, Image};

pub(super) static FORMATS: &[Format] = &[
    Format::new(
        "Amstrad PCW",
        "MicroDesign area and page",
        &["mda", "mdp"],
        decode_microdesign,
    )
    .signature(),
    Format::new("Amstrad PCW", "CUT", &["cut"], decode_cut),
    Format::new("Amstrad PCW", "The Desktop Publisher", &["grf"], decode_grf),
    Format::new("Amstrad PCW", "Stop Press canvas", &["spc"], decode_spc),
];

const FAIL: DecodeError = DecodeError::Unrecognized;
const STAMP_LEN: usize = 128;
/// Width and height (or their codes) before the rows of CUT and GRF.
const HEADER_LEN: usize = 4;
const PIXELS_AT: usize = STAMP_LEN + 4;
const SCREEN_WIDTH: usize = 720;
const SCREEN_HEIGHT: usize = 256;
/// CP/M file records: copied files are padded to a whole number of them.
const RECORD_LEN: usize = 128;
/// Colors of a clear and a set bit: a set bit is white.
const SET_IS_WHITE: [u32; 2] = [0x000000, 0xffffff];
/// A set bit is black, as on paper.
const SET_IS_BLACK: [u32; 2] = [0xffffff, 0x000000];

fn decode_microdesign(data: &[u8]) -> Result<Image, DecodeError> {
    // `.MDA` or `.MDP`, the program name, `v1.` and the minor digit that
    // tells MicroDesign 2 ("0") from MicroDesign 3 ("3").
    let stamp = data.get(..STAMP_LEN).ok_or(FAIL)?;
    let known = matches!(&stamp[..4], b".MDA" | b".MDP")
        && &stamp[4..18] == b"MicroDesignPCW"
        && &stamp[18..21] == b"v1.";
    if !known {
        return Err(FAIL);
    }
    let height = usize::from(le16(data, STAMP_LEN).ok_or(FAIL)?);
    let row_len = usize::from(le16(data, STAMP_LEN + 2).ok_or(FAIL)?);
    check_size(row_len * 8, height)?;
    let packed = data.get(PIXELS_AT..).ok_or(FAIL)?;
    let bitmap = match stamp[21] {
        b'0' => unpack_runs(packed, row_len * height)?,
        b'3' => unpack_lines(packed, row_len, height)?,
        _ => return Err(FAIL),
    };
    bits(row_len * 8, height, &bitmap, row_len, SET_IS_WHITE)
}

/// MicroDesign 2: bytes `00` and `FF` are followed by a repeat count (0 is
/// 256); a run may continue into the next line and past the end.
fn unpack_runs(packed: &[u8], size: usize) -> Result<Vec<u8>, DecodeError> {
    let mut out = Vec::with_capacity(size);
    let mut packed = packed.iter();
    while out.len() < size {
        let byte = *packed.next().ok_or(FAIL)?;
        if byte == 0x00 || byte == 0xff {
            let count = match *packed.next().ok_or(FAIL)? {
                0 => 256,
                count => usize::from(count),
            };
            let count = count.min(size - out.len());
            out.resize(out.len() + count, byte);
        } else {
            out.push(byte);
        }
    }
    Ok(out)
}

/// MicroDesign 3: every line starts with its type, 0 one byte repeated over
/// the line, 1 blocks of data, 2 blocks of data XORed with the line above.
fn unpack_lines(packed: &[u8], row_len: usize, height: usize) -> Result<Vec<u8>, DecodeError> {
    let mut out: Vec<u8> = Vec::with_capacity(row_len * height);
    let mut packed = packed.iter();
    let mut next = || packed.next().copied().ok_or(FAIL);
    for _ in 0..height {
        let start = out.len();
        match next()? {
            0 => {
                let fill = next()?;
                out.resize(start + row_len, fill);
            }
            kind @ (1 | 2) => {
                while out.len() - start < row_len {
                    // 0x00-0x7f: control + 1 literal bytes; 0x81-0xff: the
                    // next byte, 257 - control times (control is -127 to -1).
                    let control = next()?;
                    let count = match control {
                        0x80 => return Err(FAIL),
                        0..0x80 => usize::from(control) + 1,
                        _ => 257 - usize::from(control),
                    };
                    if out.len() - start + count > row_len {
                        return Err(FAIL);
                    }
                    if control >= 0x80 {
                        let byte = next()?;
                        out.resize(out.len() + count, byte);
                    } else {
                        for _ in 0..count {
                            let byte = next()?;
                            out.push(byte);
                        }
                    }
                }
                if kind == 2 && start > 0 {
                    for i in start..start + row_len {
                        out[i] ^= out[i - row_len];
                    }
                }
            }
            _ => return Err(FAIL),
        }
    }
    Ok(out)
}

/// A CUT file: height and width codes, then rows of `(w + 8) / 8` bytes.
fn decode_cut(data: &[u8]) -> Result<Image, DecodeError> {
    let height = (usize::from(le16(data, 0).ok_or(FAIL)?) + 3) / 2;
    let width = usize::from(le16(data, 2).ok_or(FAIL)?) + 2;
    headerless(data, width, height, (width + 8) / 8)
}

/// A GRF file: width and height in pixels, then rows of `ceil(w / 8)` bytes.
fn decode_grf(data: &[u8]) -> Result<Image, DecodeError> {
    let width = usize::from(le16(data, 0).ok_or(FAIL)?);
    let height = usize::from(le16(data, 2).ok_or(FAIL)?);
    headerless(data, width, height, width.div_ceil(8))
}

/// Rows after a 4-byte header, in a file of exactly that size or one padded
/// to whole CP/M records.
fn headerless(
    data: &[u8],
    width: usize,
    height: usize,
    row_len: usize,
) -> Result<Image, DecodeError> {
    check_size(width, height)?;
    let end = HEADER_LEN + row_len * height;
    let padded = end.next_multiple_of(RECORD_LEN);
    if data.len() != end && data.len() != padded {
        return Err(FAIL);
    }
    bits(width, height, &data[HEADER_LEN..end], row_len, SET_IS_BLACK)
}

/// A 720x256 screen dump, shown with the lines doubled.
fn decode_spc(data: &[u8]) -> Result<Image, DecodeError> {
    let row_len = SCREEN_WIDTH / 8;
    if data.len() != row_len * SCREEN_HEIGHT {
        return Err(FAIL);
    }
    // Each block of 8 lines is stored byte-interleaved.
    let mut bitmap = Vec::with_capacity(data.len());
    for block in data.chunks_exact(row_len * 8) {
        for line in 0..8 {
            bitmap.extend(block.iter().skip(line).step_by(8));
        }
    }
    bits(SCREEN_WIDTH, SCREEN_HEIGHT, &bitmap, row_len, SET_IS_WHITE)?.scaled(1, 2)
}

fn bits(
    width: usize,
    height: usize,
    bitmap: &[u8],
    row_len: usize,
    colors: [u32; 2],
) -> Result<Image, DecodeError> {
    Image::from_bits(
        width as u32,
        height as u32,
        bitmap,
        row_len,
        BitOrder::MsbFirst,
        colors,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(version: &[u8; 5], height: u16, row_len: u16, packed: &[u8]) -> Vec<u8> {
        let mut data = b".MDAMicroDesignPCWv1.00\r\n1234567\r\n".to_vec();
        data[18..23].copy_from_slice(version);
        data.resize(STAMP_LEN, 0);
        data.extend_from_slice(&height.to_le_bytes());
        data.extend_from_slice(&row_len.to_le_bytes());
        data.extend_from_slice(packed);
        data
    }

    #[test]
    fn microdesign_2_runs_cross_line_ends_and_zero_means_256() {
        // 2 bytes (16 pixels) wide, 4 lines: the 00 run ends line 1 and
        // fills line 2, FF 00 is 256 bytes and is cut at the image end.
        let packed = [0xaa, 0x00, 0x02, 0x0f, 0xff, 0x00];
        let image = decode_microdesign(&area(b"v1.00", 4, 2, &packed)).unwrap();
        assert_eq!((image.width(), image.height()), (16, 4));
        assert_eq!(image.get(0, 0), 0xffffff);
        assert_eq!(image.get(1, 0), 0);
        assert_eq!(image.get(11, 1), 0);
        assert_eq!(image.get(12, 1), 0xffffff);
        assert_eq!(image.get(0, 2), 0xffffff);
        assert_eq!(image.get(15, 3), 0xffffff);
        // A stream that ends early is not an image.
        assert!(decode_microdesign(&area(b"v1.00", 4, 2, &packed[..5])).is_err());
    }

    #[test]
    fn microdesign_3_lines_use_the_three_types() {
        // The format sheet's examples, 7 bytes wide: a data line, an XOR line
        // against it and a solid line.
        let packed = [
            0x01, 0x03, 0x0f, 0xcc, 0xf0, 0x00, 0xfe, 0xff, // data
            0x02, 0xff, 0x00, 0x01, 0x3c, 0x03, 0xfe, 0x00, // difference
            0x00, 0xff, // all the same byte
        ];
        let image = decode_microdesign(&area(b"v1.30", 3, 7, &packed)).unwrap();
        assert_eq!((image.width(), image.height()), (56, 3));
        // Line 0 is 0F CC F0 00 FF FF FF.
        assert_eq!(image.get(4, 0), 0xffffff);
        assert_eq!(image.get(3, 0), 0);
        assert_eq!(image.get(55, 0), 0xffffff);
        // Line 1 is line 0 XOR 00 00 3C 03 00 00 00, so 0F CC CC 03 FF FF FF.
        assert_eq!(image.get(16, 1), 0xffffff);
        assert_eq!(image.get(18, 1), 0);
        assert_eq!(image.get(29, 1), 0);
        assert_eq!(image.get(30, 1), 0xffffff);
        // Line 2 is all white.
        assert_eq!(image.get(0, 2), 0xffffff);
        assert_eq!(image.get(55, 2), 0xffffff);
        // A block running past the line is rejected.
        assert!(decode_microdesign(&area(b"v1.30", 1, 1, &[0x01, 0xfe, 0xff])).is_err());
    }

    #[test]
    fn microdesign_needs_its_stamp() {
        let mut data = area(b"v1.00", 4, 2, &[0x00, 0x08]);
        assert!(decode_microdesign(&data).is_ok());
        data[5] = b'X';
        assert!(decode_microdesign(&data).is_err());
    }

    #[test]
    fn cut_rows_have_one_byte_more_than_whole_bytes_and_files_may_be_padded() {
        // h1 = 3 gives 3 rows, w1 = 6 gives 8 pixels in rows of 2 bytes.
        let mut data = alloc::vec![3, 0, 6, 0];
        data.extend_from_slice(&[0x80, 0x00, 0x01, 0xff, 0x00, 0x00]);
        let image = decode_cut(&data).unwrap();
        assert_eq!((image.width(), image.height()), (8, 3));
        // A set bit is black on white paper.
        assert_eq!(image.get(0, 0), 0);
        assert_eq!(image.get(7, 1), 0);
        assert_eq!(image.get(0, 2), 0xffffff);
        data.resize(RECORD_LEN, 0);
        assert!(decode_cut(&data).is_ok());
        data.push(0);
        assert!(decode_cut(&data).is_err());
    }

    #[test]
    fn grf_rows_have_whole_bytes_and_a_set_bit_is_black_like_cut() {
        // 9 x 2: rows of 2 bytes.
        let mut data = alloc::vec![9, 0, 2, 0, 0x80, 0x80, 0x00, 0x00];
        let image = decode_grf(&data).unwrap();
        assert_eq!((image.width(), image.height()), (9, 2));
        assert_eq!((image.get(0, 0), image.get(8, 0)), (0, 0));
        assert_eq!(image.get(1, 1), 0xffffff);
        data.push(0);
        assert!(decode_grf(&data).is_err());
    }

    #[test]
    fn spc_lines_are_deinterleaved_and_doubled() {
        let mut data = alloc::vec![0u8; 23040];
        // Block 0, byte 8 is the second byte of the top line (pixels 8-15);
        // byte 1 is the first byte of the second line.
        data[8] = 0x80;
        data[1] = 0x01;
        let image = decode_spc(&data).unwrap();
        assert_eq!((image.width(), image.height()), (720, 512));
        assert_eq!(image.get(8, 0), 0xffffff);
        assert_eq!(image.get(8, 1), 0xffffff);
        assert_eq!(image.get(7, 2), 0xffffff);
        assert_eq!(image.get(7, 3), 0xffffff);
        assert!(decode_spc(&data[..23039]).is_err());
    }
}
