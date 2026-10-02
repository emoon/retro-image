//! Workbench icon (`.info`): the first image of a classic DiskObject.
//!
//! Sources:
//! - Layout (78-byte DiskObject with magic `$E310`, optional 56-byte
//!   DrawerData, 20-byte Image header, planar rows padded to 16 bits):
//!   RKM Libraries ch. 14
//!   (<http://www.theflatnet.de/pub/cbm/amiga/AmigaDevDocs/lib_14.html>) and
//!   Deark's `amigaicon.c` (<https://github.com/jsummers/deark>, MIT licence).
//! - Workbench 1.x pens (blue, white, black, orange) and doubled lines for the
//!   high-resolution screen: observed from `recoil2png` output.

use super::iff::{be16, be32};
use crate::{DecodeError, Image};

const DISK_OBJECT_LEN: usize = 78;
const DRAWER_DATA_LEN: usize = 56;
const PALETTE: [u32; 4] = [0x55aaff, 0xffffff, 0x000000, 0xff8800];

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let object = data.get(..DISK_OBJECT_LEN).ok_or(fail)?;
    if be16(&object[0..2]) != 0xe310 {
        return Err(fail);
    }
    let has_drawer = be32(&object[66..70]) != 0;
    let start = DISK_OBJECT_LEN + if has_drawer { DRAWER_DATA_LEN } else { 0 };
    let header = data.get(start..start + 20).ok_or(fail)?;
    let width = usize::from(be16(&header[4..6]));
    let height = usize::from(be16(&header[6..8]));
    let depth = usize::from(be16(&header[8..10]));
    // Only 2-plane icons have a known palette.
    if width == 0 || height == 0 || depth != 2 {
        return Err(fail);
    }
    let row_len = width.div_ceil(16) * 2;
    let plane_len = row_len * height;
    let planes = data
        .get(start + 20..start + 20 + plane_len * depth)
        .ok_or(fail)?;
    let mut image = Image::new(width as u32, height as u32 * 2);
    for y in 0..height {
        for x in 0..width {
            let index = (0..depth).fold(0, |v, p| {
                v | usize::from(planes[p * plane_len + y * row_len + x / 8] >> (7 - x % 8) & 1) << p
            });
            let color = PALETTE[index];
            image.set(x as u32, y as u32 * 2, color);
            image.set(x as u32, y as u32 * 2 + 1, color);
        }
    }
    Ok(image)
}
