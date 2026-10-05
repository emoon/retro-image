//! `ARV` pictures (ARTV, NEC PC-98, 640x400, 16 colors): the 16-color
//! relative of ArtMaster88 (`SS_SIF` files).
//!
//! The archiveteam wiki page on ArtMaster88 only gives the signature
//! (<http://fileformats.archiveteam.org/wiki/ArtMaster88>). The layout was
//! reverse engineered from `TAIHO (from Gr.lzh).ARV` by black-box probing of
//! `recoil2png` with mutated copies; see `docs/research/msx-japanese.md`,
//! "Wave 5b: Japanese".
//!
//! Layout, little-endian:
//! - `"SS_SIF    0.0"` at 0 (the next byte, a version digit, is ignored),
//!   then `"IRB"` at 0x10 and `"BRG"` at 0x13; 640 and 400 at 0x18 and 0x1A.
//!   Bytes 0x16 and 0x17 do not change the picture.
//! - At 0x28 a chain of three records, each starting with its own length
//!   (including the two length bytes). The second holds the palette: 16
//!   entries of three 16-bit values (red, green, blue, each 0-15 and scaled by
//!   17; larger values are rejected). The first and third do not affect the
//!   picture.
//! - Four run-length packed bit planes of 80 * 400 bytes (blue, red, green,
//!   intensity), MSB first, packed as in ArtMaster88: two equal bytes are
//!   followed by a count of copies (0 meaning 256). Bytes after the last plane
//!   are ignored.

use alloc::vec::Vec;

use super::artmaster88::unpack_plane;
use crate::bytes::le16;
use crate::image::{planar_pixels, widen_channel};
use crate::{DecodeError, Image};

const SIGNATURE: &[u8] = b"SS_SIF    0.0";
const WIDTH: usize = 640;
const HEIGHT: usize = 400;
const PLANE_BYTES: usize = WIDTH / 8 * HEIGHT;
const RECORDS: usize = 0x28;
const PALETTE_RECORD: usize = 1;

pub(in crate::platform) fn decode_arv(data: &[u8]) -> Result<Image, DecodeError> {
    let bad = DecodeError::Unrecognized;
    if !data.starts_with(SIGNATURE)
        || data.get(0x10..0x13) != Some(b"IRB")
        || data.get(0x13..0x16) != Some(b"BRG")
        || le16(data, 0x18) != Some(WIDTH as u16)
        || le16(data, 0x1a) != Some(HEIGHT as u16)
    {
        return Err(bad);
    }
    let mut pos = RECORDS;
    let mut palette_at = 0;
    for record in 0..3 {
        if record == PALETTE_RECORD {
            palette_at = pos + 2;
        }
        let length = usize::from(le16(data, pos).ok_or(bad)?);
        if length < 2 {
            return Err(bad);
        }
        pos += length;
    }
    let palette = (0..16)
        .map(|i| {
            let component = |c: usize| match le16(data, palette_at + (i * 3 + c) * 2) {
                Some(v @ 0..=15) => Ok(widen_channel(u32::from(v), 4)),
                _ => Err(bad),
            };
            Ok(component(0)? << 16 | component(1)? << 8 | component(2)?)
        })
        .collect::<Result<Vec<u32>, _>>()?;

    let mut planes = Vec::with_capacity(4);
    for _ in 0..4 {
        planes.push(unpack_plane(data, &mut pos, PLANE_BYTES).ok_or(bad)?);
    }
    let planes = planes.concat();
    let indices: Vec<u8> = planar_pixels(&planes, WIDTH, HEIGHT, WIDTH / 8, 4, |plane, y| {
        plane * PLANE_BYTES + y * (WIDTH / 8)
    })
    .into_iter()
    .map(|v| v as u8)
    .collect();
    Image::from_indexed(WIDTH as u32, HEIGHT as u32, &indices, &palette)
}
