//! Centauri Logo-Editor (`.cle`).
//!
//! Sources: no format documentation found
//! (<http://fileformats.archiveteam.org/wiki/Centauri_Logo_Editor> only names
//! the tool). Reverse engineered from 9 samples by black-box probing of
//! `recoil2png`: mutating single bytes shows load address `$6000` (ignored),
//! an 8000-byte multicolor bitmap at file offset 2 in the usual cell order,
//! then three color bytes at offsets 8002-8004: the colors of bit pairs
//! `01` (high nibble) and `10` (low nibble), `11` (low nibble) and the
//! background `00` (low nibble). Offsets 8005-8193 (padding in most
//! samples) do not change the output.

use super::vic2::{BITMAP_LEN, Bitmap, Frame, SCREEN_LEN};
use crate::{DecodeError, Image};

const LEN: usize = 8194;
const COLORS: usize = 2 + BITMAP_LEN;

pub(super) fn decode_cle(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != LEN || data[..2] != [0x00, 0x60] {
        return Err(DecodeError::Unrecognized);
    }
    let screen = [data[COLORS]; SCREEN_LEN];
    let color = [data[COLORS + 1] & 15; SCREEN_LEN];
    let bitmap = Bitmap::multicolor(&data[2..COLORS], &screen, &color, data[COLORS + 2] & 15);
    let frame = Frame::multicolor(&bitmap, 200).ok_or(DecodeError::Unrecognized)?;
    Ok(frame.to_image(0))
}
