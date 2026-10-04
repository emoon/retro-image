//! Print Shop graphics (Broderbund, 1984): 48-pixel-wide monochrome clip
//! art as stored on the Commodore 64 disks, one file per picture.
//!
//! Sources:
//! - GoDot's PrintMaster/Print Shop loader page,
//!   <https://www.godot64.de/german/l_pmaster.htm> (GoDot is MIT-licensed):
//!   the `.gra` family has a 48x45 "type B" picture, uncompressed hires,
//!   without PrintMaster's row header byte.
//! - Layout checked by reverse engineering the 132 files in
//!   `corpus/extra/printshop-c64/` (dumped from the 1984 Print Shop disk
//!   images, whose files have no extension): all load at `$5800` and there
//!   is no header, only rows of 6 bytes, most significant bit leftmost, set
//!   bits black. 60 files of 272 and 274 bytes hold 45 rows (270 bytes; the
//!   274-byte ones carry 2 trailing bytes). 71 files of 291 bytes hold 48
//!   rows (288 bytes, 1 trailing byte): the last rows of pictures such as
//!   BUNNY are still drawn, so the height is not 45. The renders were reviewed visually; `recoil2png` rejects them.
//!   The 88x52 "type A" picture of the GoDot page has no sample here and is
//!   not decoded.

use crate::image::BitOrder;
use crate::{DecodeError, Image};

const LOAD_ADDRESS: [u8; 2] = [0x00, 0x58];
const ROW_BYTES: usize = 6;

/// Rows by file size (including the load address).
fn rows(file_len: usize) -> Option<usize> {
    match file_len {
        272 | 274 => Some(45),
        291 => Some(48),
        _ => None,
    }
}

pub(super) fn decode_print_shop(data: &[u8]) -> Result<Image, DecodeError> {
    let height = rows(data.len()).ok_or(DecodeError::Unrecognized)?;
    let bitmap = data
        .strip_prefix(&LOAD_ADDRESS)
        .ok_or(DecodeError::Unrecognized)?;
    Image::from_bits(
        (ROW_BYTES * 8) as u32,
        height as u32,
        &bitmap[..height * ROW_BYTES],
        ROW_BYTES,
        BitOrder::MsbFirst,
        [0xffffff, 0x000000],
    )
}
