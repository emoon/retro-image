//! ZX Spectrum Profi 512x240 16-color screens (GRF).
//!
//! Sources:
//! - 512x240 with color attributes per 8x1 cell: speccy.info Profi
//!   overview (Russian), Wayback Machine snapshot
//!   <http://web.archive.org/web/20250924164348/https://speccy.info/Profi>
//!   (the live site blocks automated fetches), see also
//!   `docs/research/sinclair-cpc-bbc-misc.md`; zx-image README (CC0),
//!   <https://github.com/moroz1999/zx-image>, lists GRF as "hi-res 16 colors".
//! - Layout, reverse engineered from the sample and `recoil2png` output:
//!   a 128-byte header (width 512 and height 240 as u16, then fixed bytes;
//!   RECOIL accepts only this exact header and file size), a 16-entry
//!   GRB332 palette at offset 10, then for each line 64 pairs of a bitmap
//!   byte and its attribute (ink in bits 2-0 plus bit 6 for entries 8-15,
//!   paper in bits 5-3 plus bit 7). Shown with rows doubled.

use super::timex::grb332;
use crate::{DecodeError, Image};

const HEADER: [u8; 10] = [0x00, 0x02, 0xf0, 0x00, 0x04, 0x00, 0x80, 0x00, 0x01, 0x13];
const HEADER_LEN: usize = 128;
const PALETTE: usize = 10;
const WIDTH: usize = 512;
const HEIGHT: usize = 240;
const GRF_LEN: usize = HEADER_LEN + WIDTH / 8 * 2 * HEIGHT;

pub(super) fn decode_grf(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != GRF_LEN || !data.starts_with(&HEADER) {
        return Err(DecodeError::Unrecognized);
    }
    let palette: [u32; 16] = core::array::from_fn(|i| grb332(data[PALETTE + i]));
    let mut image = Image::new(WIDTH as u32, HEIGHT as u32);
    for (cell, pair) in data[HEADER_LEN..].as_chunks::<2>().0.iter().enumerate() {
        let (bits, attribute) = (pair[0], pair[1]);
        let ink = attribute & 7 | (attribute >> 3) & 8;
        let paper = (attribute >> 3) & 7 | (attribute >> 4) & 8;
        let (left, y) = (cell % (WIDTH / 8) * 8, cell / (WIDTH / 8));
        for x in 0..8 {
            let entry = if bits & (0x80 >> x) != 0 { ink } else { paper };
            image.set((left + x) as u32, y as u32, palette[usize::from(entry)]);
        }
    }
    image.scaled(1, 2)
}
