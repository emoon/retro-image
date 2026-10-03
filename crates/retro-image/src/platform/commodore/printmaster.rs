//! PrintMaster clip art (`.gra`): 88×52 monochrome pictures.
//!
//! Sources:
//! - GoDot PrintMaster/Print Shop loader page,
//!   <https://www.godot64.de/german/l_pmaster.htm> (GoDot is MIT-licensed):
//!   hires, uncompressed, 88×52 pixels for PrintMaster, a row header byte
//!   `$8B` in front of every PrintMaster row, background white by default.
//!   The page also lists Print Shop variants (88×52 and 48×45) that no
//!   sample here has, so they are not decoded.
//! - Layout checked on 59 CSDb samples (all 631 bytes): a load address
//!   (`$6800` in 58 files, one `$8750`, so it is ignored), the five header
//!   bytes `58 00 34 00 B4`, then 52 rows of `$8B` plus 11 bytes (88 pixels,
//!   most significant bit leftmost, set bits black). `$34` is the height;
//!   the last header byte does not match the width GoDot lists and is not
//!   used. Output reviewed visually.

use crate::{DecodeError, Image};

const WIDTH_BYTES: usize = 11;
const HEIGHT: usize = 52;
const HEADER: [u8; 5] = [0x58, 0x00, 0x34, 0x00, 0xb4];
const ROW_MARK: u8 = 0x8b;
const SIZE: usize = 2 + HEADER.len() + HEIGHT * (1 + WIDTH_BYTES);

pub(super) fn decode_gra(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != SIZE || data[2..2 + HEADER.len()] != HEADER {
        return Err(DecodeError::Unrecognized);
    }
    let (rows, _) = data[2 + HEADER.len()..].as_chunks::<{ 1 + WIDTH_BYTES }>();
    if rows.iter().any(|row| row[0] != ROW_MARK) {
        return Err(DecodeError::Unrecognized);
    }
    let mut image = Image::new((WIDTH_BYTES * 8) as u32, HEIGHT as u32);
    for (y, row) in rows.iter().enumerate() {
        for (x, byte) in row[1..].iter().enumerate() {
            for bit in 0..8 {
                let black = byte & (0x80 >> bit) != 0;
                let color = if black { 0 } else { 0xffffff };
                image.set((x * 8 + bit) as u32, y as u32, color);
            }
        }
    }
    Ok(image)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file() -> alloc::vec::Vec<u8> {
        let mut data = alloc::vec![0u8; SIZE];
        data[1] = 0x68;
        data[2..7].copy_from_slice(&HEADER);
        for row in data[7..].as_chunks_mut::<12>().0 {
            row[0] = ROW_MARK;
        }
        data
    }

    #[test]
    fn draws_msb_left_black_on_white() {
        let mut data = file();
        data[8] = 0x80;
        data[8 + 12 + 10] = 0x01; // row 1, last pixel
        let image = decode_gra(&data).unwrap();
        assert_eq!((image.width(), image.height()), (88, 52));
        assert_eq!(image.get(0, 0), 0);
        assert_eq!(image.get(1, 0), 0xffffff);
        assert_eq!(image.get(87, 1), 0);
    }

    #[test]
    fn rejects_bad_row_marks_header_and_size() {
        let mut data = file();
        data[7 + 12 * 3] = 0;
        assert!(decode_gra(&data).is_err());
        let mut data = file();
        data[4] = 0x33;
        assert!(decode_gra(&data).is_err());
        assert!(decode_gra(&file()[..SIZE - 1]).is_err());
    }
}
