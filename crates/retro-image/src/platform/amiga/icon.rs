//! Workbench icon (`.info`): the first image of a classic DiskObject.
//!
//! Sources:
//! - Layout (78-byte DiskObject with magic `$E310`, optional 56-byte
//!   DrawerData, 20-byte Image header, planar rows padded to 16 bits):
//!   RKM Libraries ch. 14
//!   (<http://www.theflatnet.de/pub/cbm/amiga/AmigaDevDocs/lib_14.html>) and
//!   Deark's `amigaicon.c` (<https://github.com/jsummers/deark>, MIT licence).
//! - The icon revision in the low byte of the gadget's UserData (+44): Deark.
//! - Workbench 1.x pens (blue, white, black, orange) for revision 0, 2.x
//!   pens (grey, black, white, blue) otherwise, and doubled lines for the
//!   high-resolution screen: observed from `recoil2png` output. The 2.x
//!   blue (`$3B67A2`, from Deark) is not used by any 2-plane sample.
//! - 3-plane 2.x icons: the 2.x pens followed by the other four MagicWB
//!   colours (dark grey, light grey, beige, pink). The order is observed
//!   from `recoil2png` output; beige (`$AA907C`, unused by the samples) is
//!   from Deark's MagicWB palette.

use alloc::vec::Vec;

use crate::bytes::{be16, be32};
use crate::{DecodeError, Image};

const DISK_OBJECT_LEN: usize = 78;
const DRAWER_DATA_LEN: usize = 56;
const PALETTE_1X: [u32; 4] = [0x55aaff, 0xffffff, 0x000000, 0xff8800];
const PALETTE_2X: [u32; 8] = [
    0x959595, 0x000000, 0xffffff, 0x3b67a2, 0x7b7b7b, 0xafafaf, 0xaa907c, 0xffa997,
];

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let object = data.get(..DISK_OBJECT_LEN).ok_or(fail)?;
    // Magic and version 1.
    if be16(object, 0).ok_or(fail)? != 0xe310 || be16(object, 2).ok_or(fail)? != 1 {
        return Err(fail);
    }
    let has_drawer = be32(object, 66).ok_or(fail)? != 0;
    let palette: &[u32] = if object[47] == 0 {
        &PALETTE_1X
    } else {
        &PALETTE_2X
    };
    let start = DISK_OBJECT_LEN + if has_drawer { DRAWER_DATA_LEN } else { 0 };
    let header = data.get(start..start + 20).ok_or(fail)?;
    let width = usize::from(be16(header, 4).ok_or(fail)?);
    let height = usize::from(be16(header, 6).ok_or(fail)?);
    let depth = usize::from(be16(header, 8).ok_or(fail)?);
    // Only depths whose colours all have a known pen.
    if width == 0 || height == 0 || !(2..=3).contains(&depth) || 1 << depth > palette.len() {
        return Err(fail);
    }
    let row_len = width.div_ceil(16) * 2;
    let plane_len = row_len * height;
    let planes = data
        .get(start + 20..start + 20 + plane_len * depth)
        .ok_or(fail)?;
    let indices: Vec<u8> = (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .map(|(x, y)| {
            (0..depth).fold(0, |v, p| {
                v | (planes[p * plane_len + y * row_len + x / 8] >> (7 - x % 8) & 1) << p
            })
        })
        .collect();
    Ok(Image::from_indexed(width as u32, height as u32, &indices, palette)?.scaled(1, 2))
}
