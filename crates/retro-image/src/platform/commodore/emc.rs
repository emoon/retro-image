//! EMC-editor (Masters Design Group, Magic Disk 64): unpacked multicolor
//! FLI.
//!
//! Sources:
//! - Memory map (load `$4000`, eight screen RAMs, bitmap, color RAM,
//!   background always black): GoDot's Magic Disk EMC loader page,
//!   <https://www.godot64.de/german/l_mdisk.htm>.
//! - The page gives 17410 bytes; the sample has 17412 (load address, the
//!   `$4000-$83FF` memory and two trailing bytes), and `recoil2png` accepts
//!   only that size. The picture shows lines 4-195 (found by matching
//!   `recoil2png` output).

use super::fli::{Bg, Fli};
use crate::{DecodeError, Image};

const EMC: Fli = Fli {
    load: 0x4000,
    sizes: &[2 + 0x4400 + 2],
    bitmap: 0x6000,
    screens: 0x4000,
    color: Some(0x8000),
    background: Bg::Black,
    height: 192,
    skip: 4,
};

pub(super) fn decode_emc(data: &[u8]) -> Result<Image, DecodeError> {
    EMC.decode(data)
}
