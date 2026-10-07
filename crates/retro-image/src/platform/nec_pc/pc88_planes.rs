//! Shared drawing of the three-plane (blue, red, green) 640x200 PC-88
//! pictures used by the ArtMaster88 and DaVinci decoders.
//!
//! Source: reverse engineered from `_REMSM4.IMG` and `REMSM3.IMG` by black-box
//! probing of `recoil2png` (see `docs/research/msx-japanese.md`, "Wave 5:
//! Japanese"): bit 0 of the color is blue, bit 1 red, bit 2 green, each shown
//! as 0 or 255, and every line is drawn twice.

use alloc::vec::Vec;

use crate::image::planar_pixels;
use crate::{DecodeError, Image};

pub(super) const WIDTH: usize = 640;
pub(super) const LINES: usize = 200;
/// Bytes in one bit plane.
pub(super) const PLANE_BYTES: usize = WIDTH / 8 * LINES;

/// The eight digital colors: bit 0 of the index is blue, bit 1 red, bit 2 green.
pub(super) fn palette() -> Vec<u32> {
    (0..8u32)
        .map(|c| {
            let level = |bit: u32| (c >> bit & 1) * 0xff;
            level(1) << 16 | level(2) << 8 | level(0)
        })
        .collect()
}

/// The picture for three planes of `PLANE_BYTES` bytes each, MSB first.
pub(super) fn image(blue: &[u8], red: &[u8], green: &[u8]) -> Result<Image, DecodeError> {
    let planes = [blue, red, green].concat();
    let indices: Vec<u8> = planar_pixels(&planes, WIDTH, LINES, WIDTH / 8, 3, |plane, y| {
        plane * PLANE_BYTES + y * (WIDTH / 8)
    })?
    .into_iter()
    .map(|v| v as u8)
    .collect();
    Image::from_indexed(WIDTH as u32, LINES as u32, &indices, &palette())?.scaled(1, 2)
}
