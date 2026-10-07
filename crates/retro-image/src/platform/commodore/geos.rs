//! GEOS geoPaint, Photo Album and Photo Scrap files in the CVT container
//! (`.cvt`).
//!
//! Sources:
//! - zimmers.net GEOS documents: geoPaint file format,
//!   <http://www.zimmers.net/anonftp/pub/cbm/geos/programming/documents/geoPaint%20format.txt>
//!   and <http://www.zimmers.net/geos/docs/paintfile.txt>, plus the GEOS
//!   VLIR/file structure notes at <https://ist.uwaterloo.ca/~schepers/formats/GEOS.TXT>
//!   (prose, no code).
//! - GEOS bitmap compression as in the GEOS Programmer's Reference Guide
//!   (the `$DC..$FF` pattern form did not occur in any sample; it is covered
//!   by a unit test only).
//! - Container and record layouts checked by reverse engineering the 18
//!   samples in `corpus/extra/commodore/zimmers-geos/`: file sizes match the
//!   block counts, every geoPaint record unpacks to exactly 1448 bytes, and
//!   the rendered pictures were reviewed visually. RECOIL rejects CVT files.
//!
//! Container: the 254-byte GEOS directory entry (`+0x15` structure, 0
//! sequential or 1 VLIR; `PRG formatted GEOS file` at `+0x1E`), the 254-byte
//! info block (class text such as `Paint Image V1.1` at `+75`), then either
//! the data (sequential) or a 254-byte VLIR table of 127 `(blocks, last
//! block bytes)` pairs followed by the records, each padded to whole
//! 254-byte blocks except that the file's last block is trimmed.
//!
//! geoPaint: records 0..45 are two card rows (16 lines) of a 640-pixel-wide
//! page. A record unpacks to 640+640 bitmap bytes in card order, 8 unused
//! bytes, then 80+80 color bytes (foreground high nibble, background low).
//! An empty record gets GEOS's default color `$BF`. Trailing empty records
//! are cropped.
//!
//! Photo Scrap: width in cards (1 byte), height in lines (16 bits), then
//! the bitmap in GEOS compression, rows in order, 1 = black. A Photo Album
//! is a VLIR file of scraps; the first one is rendered.

use super::vic2;
use crate::bytes::le16;
use crate::image::{BitOrder, check_size};
use crate::{DecodeError, Image};
use alloc::vec::Vec;

const BLOCK: usize = 254;
const TABLE_AT: usize = 2 * BLOCK;
const RECORDS_AT: usize = 3 * BLOCK;
const CLASS_AT: usize = BLOCK + 75;
const VLIR_ENTRIES: usize = 127;

const PAINT_RECORDS: usize = 45;
const PAINT_RECORD_LEN: usize = 1448;
const PAINT_COLORS_AT: usize = 1288;
const PAINT_WIDTH: usize = 640;
const DEFAULT_COLOR: u8 = 0xbf;

#[derive(Clone, Copy, PartialEq)]
enum Structure {
    Sequential,
    Vlir,
}

/// A validated CVT file.
struct Cvt<'a> {
    data: &'a [u8],
}

impl<'a> Cvt<'a> {
    /// Accepts a file with the given structure and info-block class text prefix.
    fn parse(data: &'a [u8], class: &[u8], structure: Structure) -> Result<Self, DecodeError> {
        let signature = data.get(0x1e..0x3a).ok_or(DecodeError::Invalid)?;
        let formatted = &signature[3..];
        let sig_ok = signature.starts_with(b"PRG") || signature.starts_with(b"SEQ");
        let structure_byte = u8::from(structure == Structure::Vlir);
        let class_text = data.get(CLASS_AT..CLASS_AT + class.len());
        if !sig_ok
            || !formatted.starts_with(b" formatted GEOS file V")
            || data[0x15] != structure_byte
            || class_text != Some(class)
        {
            return Err(DecodeError::Invalid);
        }
        Ok(Self { data })
    }

    /// The VLIR record payloads in table order; unused and empty records are
    /// empty slices.
    fn records(&self) -> Result<Vec<&'a [u8]>, DecodeError> {
        let table = self
            .data
            .get(TABLE_AT..RECORDS_AT)
            .ok_or(DecodeError::Invalid)?;
        let mut at = RECORDS_AT;
        let mut records = Vec::with_capacity(VLIR_ENTRIES);
        for entry in table.as_chunks::<2>().0.iter().take(VLIR_ENTRIES) {
            let end = at + usize::from(entry[0]) * BLOCK;
            // The file's last block is trimmed, so later records start past the end.
            let record = self
                .data
                .get(at.min(self.data.len())..end.min(self.data.len()))
                .ok_or(DecodeError::Invalid)?;
            records.push(record);
            at = end;
        }
        Ok(records)
    }

    /// The data of a sequential file.
    fn body(&self) -> &'a [u8] {
        &self.data[TABLE_AT.min(self.data.len())..]
    }
}

pub(super) fn decode_geopaint(data: &[u8]) -> Result<Image, DecodeError> {
    let records = Cvt::parse(data, b"Paint Image", Structure::Vlir)?.records()?;
    let pages = &records[..PAINT_RECORDS];
    let used = pages
        .iter()
        .rposition(|r| !r.is_empty())
        .ok_or(DecodeError::Invalid)?
        + 1;
    let height = used * 16;
    let palette: [u32; 16] = vic2::PALETTE;
    let mut pixels = alloc::vec![0u8; PAINT_WIDTH * height];
    for (index, record) in pages[..used].iter().enumerate() {
        let (bitmap, colors) = if record.is_empty() {
            (
                alloc::vec![0; 2 * PAINT_WIDTH],
                alloc::vec![DEFAULT_COLOR; 160],
            )
        } else {
            let unpacked = unpack_paint(record)?;
            let colors = unpacked[PAINT_COLORS_AT..].to_vec();
            (unpacked, colors)
        };
        for line in 0..16 {
            let row = &mut pixels[(index * 16 + line) * PAINT_WIDTH..][..PAINT_WIDTH];
            let card_row = line / 8;
            for (x, pixel) in row.iter_mut().enumerate() {
                let card = card_row * 80 + x / 8;
                let bits = bitmap[card * 8 + line % 8];
                let color = colors[card];
                *pixel = if bits & (0x80 >> (x % 8)) != 0 {
                    color >> 4
                } else {
                    color & 15
                };
            }
        }
    }
    Image::from_indexed(PAINT_WIDTH as u32, height as u32, &pixels, &palette)
}

pub(super) fn decode_photo_album(data: &[u8]) -> Result<Image, DecodeError> {
    let records = Cvt::parse(data, b"photo album", Structure::Vlir)?.records()?;
    records
        .into_iter()
        .filter(|r| !r.is_empty())
        .find_map(|r| decode_scrap_data(r).ok())
        .ok_or(DecodeError::Invalid)
}

pub(super) fn decode_photo_scrap(data: &[u8]) -> Result<Image, DecodeError> {
    decode_scrap_data(Cvt::parse(data, b"Photo Scrap", Structure::Sequential)?.body())
}

fn decode_scrap_data(data: &[u8]) -> Result<Image, DecodeError> {
    let (&cards, rest) = data.split_first().ok_or(DecodeError::Invalid)?;
    let height = usize::from(le16(rest, 0).ok_or(DecodeError::Invalid)?);
    let row_len = usize::from(cards);
    if row_len == 0 || height == 0 {
        return Err(DecodeError::Invalid);
    }
    check_size(row_len * 8, height)?;
    let bitmap = unpack_bitmap(&rest[2..], row_len * height)?;
    Image::from_bits(
        (row_len * 8) as u32,
        height as u32,
        &bitmap,
        row_len,
        BitOrder::MsbFirst,
        [0xffffff, 0x000000],
    )
}

/// geoPaint record compression: `$00` ends, `$01..$3F` literal bytes,
/// `$41..$7F` repeat the next 8-byte card, `$81..$FF` repeat the next byte.
/// Fails unless exactly one record (1448 bytes) comes out.
fn unpack_paint(data: &[u8]) -> Result<Vec<u8>, DecodeError> {
    let mut out = Vec::with_capacity(PAINT_RECORD_LEN);
    let mut at = 0;
    while out.len() < PAINT_RECORD_LEN {
        let code = *data.get(at).ok_or(DecodeError::Invalid)?;
        at += 1;
        match code {
            0x01..=0x3f => {
                let n = usize::from(code);
                out.extend_from_slice(data.get(at..at + n).ok_or(DecodeError::Invalid)?);
                at += n;
            }
            0x41..=0x7f => {
                let card = data.get(at..at + 8).ok_or(DecodeError::Invalid)?;
                at += 8;
                for _ in 0..code - 0x40 {
                    out.extend_from_slice(card);
                }
            }
            0x81..=0xff => {
                let byte = *data.get(at).ok_or(DecodeError::Invalid)?;
                at += 1;
                out.resize(out.len() + usize::from(code - 0x80), byte);
            }
            _ => return Err(DecodeError::Invalid),
        }
    }
    if out.len() == PAINT_RECORD_LEN {
        Ok(out)
    } else {
        Err(DecodeError::Invalid)
    }
}

/// GEOS bitmap compression: `0..=127` repeat the next byte, `128..=219`
/// copy `n - 128` literal bytes, `220..=255` repeat the following
/// length-prefixed pattern `n - 220` times. Fails when the data ends before
/// `len` bytes are out; a final run that overshoots is cut to `len`.
fn unpack_bitmap(data: &[u8], len: usize) -> Result<Vec<u8>, DecodeError> {
    let mut out = Vec::new();
    let mut at = 0;
    while out.len() < len {
        let code = *data.get(at).ok_or(DecodeError::Invalid)?;
        at += 1;
        match code {
            0..=127 => {
                let byte = *data.get(at).ok_or(DecodeError::Invalid)?;
                at += 1;
                out.resize(out.len() + usize::from(code), byte);
            }
            128..=219 => {
                let n = usize::from(code - 128);
                out.extend_from_slice(data.get(at..at + n).ok_or(DecodeError::Invalid)?);
                at += n;
            }
            220..=255 => {
                let n = usize::from(*data.get(at).ok_or(DecodeError::Invalid)?);
                let pattern = data.get(at + 1..at + 1 + n).ok_or(DecodeError::Invalid)?;
                at += 1 + n;
                for _ in 0..code - 220 {
                    out.extend_from_slice(pattern);
                }
            }
        }
    }
    // Some albums end the last run past the bitmap; the surplus is padding.
    out.truncate(len);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(class: &[u8], structure: u8) -> Vec<u8> {
        let mut data = alloc::vec![0u8; TABLE_AT];
        data[0x15] = structure;
        data[0x1e..0x3a].copy_from_slice(b"PRG formatted GEOS file V1.0");
        data[CLASS_AT..CLASS_AT + class.len()].copy_from_slice(class);
        data
    }

    #[test]
    fn bitmap_forms() {
        let packed = [3, 0xaa, 0x82, 1, 2, 221, 2, 7, 8];
        assert_eq!(
            unpack_bitmap(&packed, 7).unwrap(),
            [0xaa, 0xaa, 0xaa, 1, 2, 7, 8]
        );
        assert!(unpack_bitmap(&packed, 8).is_err());
        // A last run that overshoots is cut; missing data is an error.
        assert_eq!(unpack_bitmap(&[5, 1], 4).unwrap(), [1, 1, 1, 1]);
        assert!(unpack_bitmap(&[5], 4).is_err());
    }

    #[test]
    fn paint_forms() {
        // literal run, repeated card, repeated byte, then fill to the record size.
        let mut packed = alloc::vec![2, 9, 8, 0x42, 1, 2, 3, 4, 5, 6, 7, 8, 0x83, 7];
        let have = 2 + 2 * 8 + 3;
        let mut remaining = PAINT_RECORD_LEN - have;
        while remaining > 0 {
            let run = remaining.min(127);
            packed.extend_from_slice(&[0x80 + run as u8, 0]);
            remaining -= run;
        }
        let out = unpack_paint(&packed).unwrap();
        assert_eq!(out.len(), PAINT_RECORD_LEN);
        assert_eq!(&out[..4], &[9, 8, 1, 2]);
        assert_eq!(&out[10..18], &[1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(&out[18..21], &[7, 7, 7]);
        assert!(unpack_paint(&[0x40]).is_err());
    }

    #[test]
    fn scrap_renders_black_ones() {
        let mut data = header(b"Photo Scrap", 0);
        data.extend_from_slice(&[1, 2, 0, 0x82, 0x80, 0x01]);
        let image = decode_photo_scrap(&data).unwrap();
        assert_eq!((image.width(), image.height()), (8, 2));
        assert_eq!(&image.rgb()[..3], &[0, 0, 0]);
        assert_eq!(&image.rgb()[3..6], &[255, 255, 255]);
    }

    #[test]
    fn rejects_wrong_class_and_truncation() {
        let mut data = header(b"Photo Scrap", 0);
        assert!(decode_geopaint(&data).is_err());
        data.truncate(0x30);
        assert!(decode_photo_scrap(&data).is_err());
    }
}
