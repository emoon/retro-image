//! SGI image files (`.sgi`, `.rgb`, `.rgba`, `.bw`), also called IRIS RGB.
//!
//! Sources:
//! - Paul Haeberli, "The SGI Image File Format" version 1.00
//!   (<https://paulbourke.net/dataformats/sgirgb/sgiversion.html>): 512-byte
//!   big-endian header (magic 474, storage 0 verbatim or 1 RLE, bytes per
//!   channel 1 or 2, dimension 1 to 3, x, y and z size, pixel minimum and
//!   maximum, image name, color map id), one plane per channel, rows stored
//!   bottom to top; RLE files hold a table of row offsets and one of row
//!   lengths (both `y * z` long, channel by channel), and each row is a
//!   series of packets with a count in the low seven bits and a copy flag in
//!   bit 7, ended by a zero count; 16-bit rows use the same packets on
//!   16-bit units.
//!
//! One channel is gray, two are gray and alpha, three RGB and four RGBA; alpha
//! is kept. 16-bit samples scale to 8 bits by rounding. Only color map id 0
//! (normal) is accepted, and the pixel minimum and maximum are ignored: samples
//! are not stretched (`greytest.rgb` has a maximum of 146 and Pillow shows it
//! unstretched too). The length table is not used, since each row ends with a
//! zero count or at the picture width.
//!
//! Verification: no RECOIL oracle. Output matches Pillow's SGI reader and
//! Deark's pixel for pixel on the 15 samples with 8-bit channels. On the
//! 16-bit `norle-16.sgi` both take the high byte of each sample, so they
//! differ from our rounding by at most 2 levels (the output equals an
//! independent numpy parse that rounds). See the divergence file
//! `unix-rasters.tsv`.

use super::to_byte;
use crate::bytes::{be16, be32};
use crate::image::check_size;
use crate::{DecodeError, Image};

const FAIL: DecodeError = DecodeError::Invalid;
const MAGIC: u16 = 474;
const HEADER_LEN: usize = 512;

struct Header {
    rle: bool,
    /// Two bytes per sample.
    wide: bool,
    width: usize,
    height: usize,
    channels: usize,
}

fn parse_header(data: &[u8]) -> Result<Header, DecodeError> {
    if be16(data, 0) != Some(MAGIC) || data.len() < HEADER_LEN {
        return Err(FAIL);
    }
    let (storage, bytes_per_channel) = (data[2], data[3]);
    let dimension = be16(data, 4).ok_or(FAIL)?;
    let size = |at| be16(data, at).map(usize::from).ok_or(FAIL);
    let (width, y, z) = (size(6)?, size(8)?, size(10)?);
    let color_map = be32(data, 104).ok_or(FAIL)?;
    if storage > 1 || !matches!(bytes_per_channel, 1 | 2) || color_map != 0 {
        return Err(FAIL);
    }
    let (height, channels) = match dimension {
        1 => (1, 1),
        2 => (y, 1),
        3 => (y, z),
        _ => return Err(FAIL),
    };
    if !(1..=4).contains(&channels) {
        return Err(FAIL);
    }
    check_size(width, height)?;
    let header = Header {
        rle: storage == 1,
        wide: bytes_per_channel == 2,
        width,
        height,
        channels,
    };
    // The tables or the planes must fit in the file, which keeps a few bytes
    // from describing a huge picture in verbatim form.
    let rows = height * channels;
    let needed = if header.rle {
        rows * 8
    } else {
        rows * width * usize::from(bytes_per_channel)
    };
    if needed > data.len() - HEADER_LEN {
        return Err(FAIL);
    }
    Ok(header)
}

pub(super) fn decode_sgi(data: &[u8]) -> Result<Image, DecodeError> {
    let header = parse_header(data)?;
    let Header {
        width,
        height,
        channels,
        ..
    } = header;
    let mut image = Image::new(width as u32, height as u32)?;
    let mut planes = alloc::vec![0u16; width * channels];
    for y in 0..height {
        // Row 0 of the file is the bottom of the picture.
        let file_row = height - 1 - y;
        for (channel, plane) in planes.chunks_exact_mut(width).enumerate() {
            read_row(data, &header, channel * height + file_row, plane)?;
        }
        let max = if header.wide { 0xffff } else { 0xff };
        let at =
            |channel: usize, x: usize| u32::from(to_byte(planes[channel * width + x].into(), max));
        for x in 0..width {
            let color = match channels {
                1 | 2 => at(0, x) * 0x01_0101,
                _ => at(0, x) << 16 | at(1, x) << 8 | at(2, x),
            };
            let alpha = match channels {
                2 => at(1, x),
                4 => at(3, x),
                _ => 255,
            };
            image.set_argb(x as u32, y as u32, alpha << 24 | color);
        }
    }
    Ok(image)
}

/// Reads row `index` (channel by channel, bottom row first) into `out`.
fn read_row(
    data: &[u8],
    header: &Header,
    index: usize,
    out: &mut [u16],
) -> Result<(), DecodeError> {
    let unit = if header.wide { 2 } else { 1 };
    let sample = |row: &[u8], i: usize| -> Option<u16> {
        if header.wide {
            be16(row, i * 2)
        } else {
            row.get(i).map(|&b| u16::from(b))
        }
    };
    if !header.rle {
        let start = HEADER_LEN + index * header.width * unit;
        let row = data.get(start..).ok_or(FAIL)?;
        for (i, value) in out.iter_mut().enumerate() {
            *value = sample(row, i).ok_or(FAIL)?;
        }
        return Ok(());
    }
    let start = be32(data, HEADER_LEN + index * 4).ok_or(FAIL)? as usize;
    let row = data.get(start..).ok_or(FAIL)?;
    let (mut pos, mut done) = (0, 0);
    while done < out.len() {
        let head = sample(row, pos).ok_or(FAIL)?;
        pos += 1;
        let count = usize::from(head & 0x7f).min(out.len() - done);
        if head & 0x7f == 0 {
            return Err(FAIL); // the row ended before the picture width
        }
        if head & 0x80 != 0 {
            for value in &mut out[done..done + count] {
                *value = sample(row, pos).ok_or(FAIL)?;
                pos += 1;
            }
        } else {
            let value = sample(row, pos).ok_or(FAIL)?;
            pos += 1;
            out[done..done + count].fill(value);
        }
        done += count;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::CLEAR;
    use alloc::vec::Vec;

    fn header(storage: u8, bpc: u8, dimension: u16, size: [u16; 3]) -> Vec<u8> {
        let mut h = alloc::vec![0u8; HEADER_LEN];
        h[..2].copy_from_slice(&MAGIC.to_be_bytes());
        h[2] = storage;
        h[3] = bpc;
        h[4..6].copy_from_slice(&dimension.to_be_bytes());
        for (i, v) in size.iter().enumerate() {
            h[6 + 2 * i..8 + 2 * i].copy_from_slice(&v.to_be_bytes());
        }
        h
    }

    #[test]
    fn verbatim_rows_run_bottom_to_top_and_channels_are_planes() {
        // 2x2 RGB: each plane has the bottom row first.
        let mut file = header(0, 1, 3, [2, 2, 3]);
        file.extend_from_slice(&[10, 11, 12, 13]); // red: bottom 10 11, top 12 13
        file.extend_from_slice(&[0; 4]);
        file.extend_from_slice(&[1, 1, 1, 1]);
        let image = decode_sgi(&file).unwrap();
        assert_eq!(image.get(0, 0), 0x0c0001);
        assert_eq!(image.get(1, 1), 0x0b0001);
    }

    #[test]
    fn rle_rows_use_copy_and_repeat_packets() {
        // 4x1 gray: copy 2 (7, 8), repeat 2 of 9, end.
        let mut file = header(1, 1, 2, [4, 1, 1]);
        let row = [0x82, 7, 8, 2, 9, 0];
        file.extend_from_slice(&520u32.to_be_bytes()); // start of row 0
        file.extend_from_slice(&(row.len() as u32).to_be_bytes());
        file.extend_from_slice(&row);
        let image = decode_sgi(&file).unwrap();
        let grays: Vec<u32> = (0..4).map(|x| image.get(x, 0) & 0xff).collect();
        assert_eq!(grays, [7, 8, 9, 9]);
        // A row that ends early is an error.
        let mut short = file.clone();
        short[520 + 3] = 0;
        assert!(decode_sgi(&short).is_err());
    }

    #[test]
    fn sixteen_bit_alpha_scales_and_is_kept() {
        // 1x1 gray and alpha at 16 bits: white, fully transparent.
        let mut file = header(0, 2, 3, [1, 1, 2]);
        file.extend_from_slice(&[0xff, 0xff, 0, 0]);
        assert_eq!(decode_sgi(&file).unwrap().get_argb(0, 0), CLEAR);
        let mut opaque = header(0, 2, 3, [1, 1, 2]);
        opaque.extend_from_slice(&[0x80, 0, 0xff, 0xff]);
        assert_eq!(decode_sgi(&opaque).unwrap().get(0, 0), 0x808080);
    }

    #[test]
    fn rejects_bad_headers() {
        let good = header(0, 1, 2, [1, 1, 1]);
        assert!(decode_sgi(&[good.clone(), alloc::vec![5]].concat()).is_ok());
        assert!(decode_sgi(&good).is_err());
        let mut bad = [good.clone(), alloc::vec![5]].concat();
        bad[3] = 3;
        assert!(decode_sgi(&bad).is_err());
        let mut mapped = [good, alloc::vec![5]].concat();
        mapped[107] = 2;
        assert!(decode_sgi(&mapped).is_err());
    }
}
