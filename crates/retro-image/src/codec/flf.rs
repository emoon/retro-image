//! The Turbo Rascal Syntax Error (TRSE) "Fluff" image container (`.flf`),
//! shared by the platform decoders that register it.
//!
//! Sources: reverse engineered from 17 samples (the 16 in RECOIL's sample
//! set plus `hflogo.flf`) by black-box probing of `recoil2png` with
//! hand-mutated copies. TRSE's own source is GPL-3 and was not read. See
//! `docs/research/amiga-apple-misc.md`, "Wave 5: FLF".
//!
//! Layout as far as it was found:
//! - 7 bytes `FLUFF64`, then 4 bytes (`02 00 00 00`, or `0a d7 23 3c`)
//!   that `recoil2png` ignores, then one image type byte at offset 11.
//! - What follows depends on the type (see the platform modules). Pictures
//!   with a palette or an ink table end in a 256-byte block that starts
//!   `40 45 00 00`; the pen/ink table is at its offset 192.
//!
//! This module holds the parts several platforms share: the container
//! check, the paletted 8-bit layout and the 8x8 colour-cell layout.

use crate::{DecodeError, Image};
use alloc::vec::Vec;

const MAGIC: &[u8; 7] = b"FLUFF64";
/// Offset of the image type byte.
const KIND: usize = 11;
/// Offset of the byte after the type byte.
pub(crate) const PAYLOAD: usize = KIND + 1;
/// Length of the optional (for some types, required) closing block.
pub(crate) const TRAILER_LEN: usize = 256;

/// A file that starts with the Fluff magic.
pub(crate) struct Fluff<'a> {
    /// The image type byte.
    pub(crate) kind: u8,
    data: &'a [u8],
}

impl<'a> Fluff<'a> {
    pub(crate) fn parse(data: &'a [u8]) -> Result<Self, DecodeError> {
        if data.len() <= KIND || !data.starts_with(MAGIC) {
            return Err(DecodeError::Unrecognized);
        }
        Ok(Self {
            kind: data[KIND],
            data,
        })
    }

    /// The byte at an absolute file offset.
    pub(crate) fn byte(&self, offset: usize) -> Result<u8, DecodeError> {
        self.data
            .get(offset)
            .copied()
            .ok_or(DecodeError::Unrecognized)
    }

    /// `len` bytes at an absolute file offset, and everything after them.
    pub(crate) fn split(
        &self,
        offset: usize,
        len: usize,
    ) -> Result<(&'a [u8], &'a [u8]), DecodeError> {
        let rest = self.data.get(offset..).ok_or(DecodeError::Unrecognized)?;
        if rest.len() < len {
            return Err(DecodeError::Unrecognized);
        }
        Ok(rest.split_at(len))
    }
}

/// What may follow the picture: nothing or the closing block.
///
/// With `required`, only the closing block is accepted. Returns it (empty
/// when absent).
pub(crate) fn trailer(rest: &[u8], required: bool) -> Result<&[u8], DecodeError> {
    match rest.len() {
        0 if !required => Ok(rest),
        TRAILER_LEN => Ok(rest),
        _ => Err(DecodeError::Unrecognized),
    }
}

/// A picture of one palette index per pixel from offset 13, then a colour
/// count byte (0 means 256) and that many RGB triples, then the optional
/// closing block. Indices past the palette are black.
pub(crate) fn decode_paletted(
    fluff: &Fluff,
    width: usize,
    height: usize,
) -> Result<Image, DecodeError> {
    let (pixels, rest) = fluff.split(PAYLOAD + 1, width * height)?;
    let (&count, rest) = rest.split_first().ok_or(DecodeError::Unrecognized)?;
    let count = if count == 0 { 256 } else { usize::from(count) };
    if rest.len() < count * 3 {
        return Err(DecodeError::Unrecognized);
    }
    let (rgb, rest) = rest.split_at(count * 3);
    trailer(rest, false)?;
    let mut palette = [0u32; 256];
    for (entry, c) in palette.iter_mut().zip(rgb.as_chunks::<3>().0) {
        *entry = u32::from(c[0]) << 16 | u32::from(c[1]) << 8 | u32::from(c[2]);
    }
    Image::from_indexed(width as u32, height as u32, pixels, &palette)
}

/// Bytes of one colour cell: 8 bitmap rows, then 4 colour numbers.
pub(crate) const CELL_LEN: usize = 12;

/// How a cell's bitmap bits select its colours.
#[derive(Clone, Copy)]
pub(crate) enum CellMode {
    /// One bit per pixel; bit `b` selects colour number `b` (0 or 1).
    Hires,
    /// Two bits per pixel, the leftmost pixel in the low bits and each
    /// drawn twice as wide; value `v` selects colour number `v`.
    Multicolor,
}

/// `cols` x `rows` cells of 8x8 pixels, row by row. A colour number only
/// has to be a valid palette index if a pixel uses it (unused numbers are
/// `$ff` in the samples).
pub(crate) fn decode_cells(
    cells: &[u8],
    cols: usize,
    rows: usize,
    mode: CellMode,
    palette: &[u32],
) -> Result<Image, DecodeError> {
    if cells.len() != cols * rows * CELL_LEN {
        return Err(DecodeError::Unrecognized);
    }
    let width = cols * 8;
    let mut indices: Vec<u8> = alloc::vec![0; width * rows * 8];
    for (n, cell) in cells.as_chunks::<CELL_LEN>().0.iter().enumerate() {
        let (bitmap, colors) = cell.split_at(8);
        let (x0, y0) = (n % cols * 8, n / cols * 8);
        for (y, &byte) in bitmap.iter().enumerate() {
            let row = &mut indices[(y0 + y) * width + x0..][..8];
            match mode {
                CellMode::Hires => {
                    for (x, out) in row.iter_mut().enumerate() {
                        *out = colors[usize::from(byte >> (7 - x) & 1)];
                    }
                }
                CellMode::Multicolor => {
                    for (x, pair) in row.as_chunks_mut::<2>().0.iter_mut().enumerate() {
                        pair.fill(colors[usize::from(byte >> (2 * x) & 3)]);
                    }
                }
            }
        }
    }
    Image::from_indexed(width as u32, (rows * 8) as u32, &indices, palette)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multicolor_cell_pixels_run_from_the_low_bits() {
        let mut cell = [0u8; CELL_LEN];
        cell[0] = 0b0010_0111; // pixels: 3, 1, 2, 0
        cell[8..].copy_from_slice(&[10, 11, 12, 13]);
        let palette: [u32; 16] = core::array::from_fn(|n| n as u32);
        let image = decode_cells(&cell, 1, 1, CellMode::Multicolor, &palette).unwrap();
        let row: [u32; 8] = core::array::from_fn(|x| image.get(x as u32, 0));
        assert_eq!(row, [13, 13, 11, 11, 12, 12, 10, 10]);
    }

    #[test]
    fn colour_numbers_only_matter_when_used() {
        let mut cell = [0u8; CELL_LEN];
        cell[8..].copy_from_slice(&[1, 0xff, 0xff, 0xff]);
        let palette = [0u32, 1];
        assert!(decode_cells(&cell, 1, 1, CellMode::Multicolor, &palette).is_ok());
        cell[0] = 1;
        assert!(decode_cells(&cell, 1, 1, CellMode::Multicolor, &palette).is_err());
    }

    #[test]
    fn closing_block_is_all_or_nothing() {
        assert!(trailer(&[], false).is_ok());
        assert!(trailer(&[], true).is_err());
        assert!(trailer(&[0; TRAILER_LEN], true).is_ok());
        assert!(trailer(&[0; 7], false).is_err());
    }
}
