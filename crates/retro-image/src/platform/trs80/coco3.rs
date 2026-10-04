//! TRS-80 Color Computer 3 pictures: Davinci HRS, ColorMax MGE, The Rat
//! Graphics Design Package RAT and OS-9 VEF.
//!
//! Sources: the KAOS Toolkit image format documents, written by Chet
//! Simpson and published under the MIT licence:
//! - `AssetFoo/docs/images/hrs.dox`, `mge.dox`, `rat.dox` and `vef.dox` at
//!   <https://github.com/ChetSimpson/KAOSToolkit/tree/main/AssetFoo/docs/images>
//!   (headers, sizes, run-length schemes). They in turn cite The Rainbow,
//!   October to December 1990 ("Getting the Picture with OS-9",
//!   "Displaying Picture Files Using OS-9 Level II Graphics") and the
//!   MVCanvas VEF text, <http://www.sdc.org/~goosey/os9/vef.format.txt>.
//! - The GIME palette byte, `R1 G1 B1 R0 G0 B0` in bits 5..0 (RGB222):
//!   Lomont, "Color Computer 1/2/3 Hardware Programming",
//!   <https://www.lomont.org/software/misc/coco/Lomont_CoCoHardware.pdf>.
//!   The KAOS page for the colour format is a TODO stub.
//! - Pixel shape (640 wide: lines doubled, 160 wide: columns doubled): our
//!   choice, so every VEF mode comes out near 8:5.
//! - Checked against the KAOS test data (`corpus/extra/coco3`): the MGE
//!   `test1-*` files against the reference render `png/test1_base.png`, the
//!   other files by eye. Half-row overruns in compressed VEF files are
//!   clamped to the half-row, as the format notes advise.
//!
//! CM3 is in `cm3.rs`. Its compression is undocumented, so it
//! was learned from the KAOS Toolkit reader (MIT, notice in that file).
//!
//! Not decoded: MGE files whose
//! colour space is composite C4I2 (the KAOS page for it is a TODO stub and
//! no sample uses it). HRS, RAT and VEF don't record their colour space, so
//! RGB222 is assumed.
//!
//! The KAOS Toolkit licence applies to its documents:
//!
//! > MIT License
//! >
//! > Copyright (c) 2023 HyperTech Gaming and Chet Simpson
//! >
//! > Permission is hereby granted, free of charge, to any person obtaining a
//! > copy of this software and associated documentation files (the
//! > "Software"), to deal in the Software without restriction, including
//! > without limitation the rights to use, copy, modify, merge, publish,
//! > distribute, sublicense, and/or sell copies of the Software, and to permit
//! > persons to whom the Software is furnished to do so, subject to the
//! > following conditions:
//! >
//! > The above copyright notice and this permission notice shall be included
//! > in all copies or substantial portions of the Software.
//! >
//! > THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
//! > OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
//! > MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
//! > IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
//! > CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
//! > TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE
//! > SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

pub(super) mod cm3;

use alloc::vec::Vec;

use crate::{DecodeError, Image};

const COLORMAP_LEN: usize = 16;

/// The palette from a colour map of RGB222 bytes.
fn palette(colormap: &[u8]) -> Vec<u32> {
    colormap
        .iter()
        .map(|&c| {
            let channel =
                |high: u8, low: u8| u32::from(((c >> high) & 1) << 1 | ((c >> low) & 1)) * 85;
            channel(5, 2) << 16 | channel(4, 1) << 8 | channel(3, 0)
        })
        .collect()
}

/// One palette index per pixel from packed `bits`-bit pixels, leftmost in
/// the most significant bits.
fn unpack(packed: &[u8], bits: u32) -> Vec<u8> {
    let mask = (1u8 << bits) - 1;
    packed
        .iter()
        .flat_map(|&byte| (0..8 / bits).map(move |i| (byte >> (8 - bits * (i + 1))) & mask))
        .collect()
}

fn picture(
    width: u32,
    height: u32,
    bits: u32,
    colormap: &[u8],
    packed: &[u8],
) -> Result<Image, DecodeError> {
    Image::from_indexed(width, height, &unpack(packed, bits), &palette(colormap))
}

/// Davinci HRS: a colour map and 320x192 4-bit pixels.
pub(super) fn decode_hrs(data: &[u8]) -> Result<Image, DecodeError> {
    const PIXELS: usize = 320 / 2 * 192;
    if data.len() != COLORMAP_LEN + PIXELS {
        return Err(DecodeError::Unrecognized);
    }
    let (colormap, packed) = data.split_at(COLORMAP_LEN);
    picture(320, 192, 4, colormap, packed)
}

/// ColorMax MGE: a 51-byte header and 320x200 4-bit pixels, run-length coded
/// as `count, byte` pairs when the compression flag is 0.
pub(super) fn decode_mge(data: &[u8]) -> Result<Image, DecodeError> {
    const HEADER: usize = 51;
    const PIXELS: usize = 320 / 2 * 200;
    let fail = DecodeError::Unrecognized;
    let (header, body) = data.split_at_checked(HEADER).ok_or(fail)?;
    let (image_type, colorspace, compression) = (header[0], header[17], header[18]);
    // Composite C4I2 (colour space 1) is not documented, so not decoded.
    if image_type != 0 || colorspace != 0 {
        return Err(fail);
    }
    let packed = if compression == 0 {
        let mut packed = Vec::with_capacity(PIXELS);
        for pair in body.as_chunks::<2>().0 {
            let (count, byte) = (usize::from(pair[0]), pair[1]);
            if count == 0 {
                break;
            }
            if packed.len() + count > PIXELS {
                return Err(fail);
            }
            packed.resize(packed.len() + count, byte);
        }
        packed
    } else {
        body.get(..PIXELS).ok_or(fail)?.to_vec()
    };
    picture(320, 200, 4, &header[1..17], &packed)
}

/// The Rat Graphics Design Package: a 19-byte header and 320x199 4-bit
/// pixels, with `escape, count, byte` runs when the compression flag is set.
pub(super) fn decode_rat(data: &[u8]) -> Result<Image, DecodeError> {
    const HEADER: usize = 19;
    const PIXELS: usize = 320 / 2 * 199;
    let fail = DecodeError::Unrecognized;
    let (header, body) = data.split_at_checked(HEADER).ok_or(fail)?;
    let escape = header[0];
    let packed = if header[1] != 0 {
        let mut packed = Vec::with_capacity(PIXELS);
        let mut input = body.iter().copied();
        while let Some(byte) = input.next() {
            if byte == escape {
                let count = usize::from(input.next().ok_or(fail)?);
                let value = input.next().ok_or(fail)?;
                if packed.len() + count > PIXELS {
                    return Err(fail);
                }
                packed.resize(packed.len() + count, value);
            } else {
                if packed.len() == PIXELS {
                    return Err(fail);
                }
                packed.push(byte);
            }
        }
        packed
    } else {
        body.to_vec()
    };
    picture(320, 199, 4, &header[2..18], &packed)
}

/// OS-9 VEF: an 18-byte header and 200 rows of 1, 2 or 4-bit pixels, stored
/// as 400 length-prefixed half-row blocks of packets when the top bit of the
/// flags is set.
pub(super) fn decode_vef(data: &[u8]) -> Result<Image, DecodeError> {
    const HEADER: usize = 18;
    const HEIGHT: usize = 200;
    let fail = DecodeError::Unrecognized;
    let (header, body) = data.split_at_checked(HEADER).ok_or(fail)?;
    // Width and bits per pixel by image type.
    let (width, bits): (usize, u32) = match header[1] {
        0 => (320, 4),
        1 => (640, 2),
        2 => (160, 4),
        3 => (320, 2),
        4 => (640, 1),
        _ => return Err(fail),
    };
    let row_len = width * bits as usize / 8;
    let packed = if header[0] & 0x80 != 0 {
        unsquash(body, row_len / 2).ok_or(fail)?
    } else if body.len() == row_len * HEIGHT {
        body.to_vec()
    } else {
        return Err(fail);
    };
    let image = picture(width as u32, HEIGHT as u32, bits, &header[2..], &packed)?;
    // The screen is 4:3 whatever the mode, so 640-pixel modes have tall
    // pixels and the 160-pixel mode wide ones: square them up.
    match width {
        640 => image.scaled(1, 2),
        160 => image.scaled(2, 1),
        _ => Ok(image),
    }
}

/// The 400 half-row blocks of a compressed VEF. A packet running past the
/// end of its half-row is cut off there; a block must fill its half-row.
fn unsquash(body: &[u8], half_row: usize) -> Option<Vec<u8>> {
    let mut packed = Vec::with_capacity(half_row * 400);
    let mut rest = body;
    for _ in 0..400 {
        let (&len, after) = rest.split_first()?;
        let (block, after) = after.split_at_checked(usize::from(len))?;
        rest = after;
        let end = packed.len() + half_row;
        let mut packets = block;
        while packed.len() < end {
            let (&head, after) = packets.split_first()?;
            let count = usize::from(head & 0x7f).min(end - packed.len());
            if head & 0x80 == 0 {
                let (literal, after) = after.split_at_checked(usize::from(head))?;
                packed.extend_from_slice(&literal[..count]);
                packets = after;
            } else {
                let (&value, after) = after.split_first()?;
                packed.resize(packed.len() + count, value);
                packets = after;
            }
        }
    }
    Some(packed)
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;

    #[test]
    fn palette_decodes_rgb222() {
        let colors = palette(&[0x00, 0x3f, 0x20, 0x04, 0x38, 0x07]);
        assert_eq!(
            colors,
            [0x000000, 0xffffff, 0xaa0000, 0x550000, 0xaaaaaa, 0x555555]
        );
    }

    #[test]
    fn unpacks_msb_first() {
        assert_eq!(unpack(&[0x1e], 4), [1, 14]);
        assert_eq!(unpack(&[0b1110_0100], 2), [3, 2, 1, 0]);
        assert_eq!(unpack(&[0b1010_0001], 1), [1, 0, 1, 0, 0, 0, 0, 1]);
    }

    #[test]
    fn mge_runs_must_fill_the_picture_before_the_terminator() {
        let mut file = vec![0u8; 51];
        file.extend_from_slice(&[255, 0x01, 0, 0]);
        assert!(decode_mge(&file).is_err());
        file.truncate(51);
        for _ in 0..125 {
            file.extend_from_slice(&[255, 0x01, 1, 0x01]);
        }
        file.extend_from_slice(&[0, 0]);
        let image = decode_mge(&file).unwrap();
        assert_eq!((image.width(), image.height()), (320, 200));
        file.splice(51..51, [1, 1]);
        assert!(decode_mge(&file).is_err());
    }

    #[test]
    fn mge_composite_is_rejected() {
        let mut file = vec![0u8; 51 + 32000];
        file[17] = 1;
        file[18] = 1;
        assert!(decode_mge(&file).is_err());
        file[17] = 0;
        assert!(decode_mge(&file).is_ok());
    }

    #[test]
    fn vef_clamps_half_row_overrun() {
        // Type 0 has 80-byte half-rows; each block repeats a byte 90 times.
        let mut data = vec![0x80, 0];
        data.resize(18, 0);
        for _ in 0..400 {
            data.extend_from_slice(&[2, 128 + 90, 0x11]);
        }
        let image = decode_vef(&data).unwrap();
        assert_eq!((image.width(), image.height()), (320, 200));
        data[1] = 1; // 640 wide, same half-row length in bytes
        let image = decode_vef(&data).unwrap();
        assert_eq!((image.width(), image.height()), (640, 400));
        assert!(decode_vef(&data[..data.len() - 1]).is_err());
    }

    #[test]
    fn rat_escape_runs_fill_the_picture() {
        let mut data = vec![140, 1];
        data.resize(19, 0);
        for _ in 0..(31840 / 255) {
            data.extend_from_slice(&[140, 255, 0x22]);
        }
        data.extend_from_slice(&[140, (31840 % 255) as u8, 0x22]);
        assert_eq!(decode_rat(&data).unwrap().height(), 199);
        data.push(7);
        assert!(decode_rat(&data).is_err());
    }
}
