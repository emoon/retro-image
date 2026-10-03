//! Workbench icon (`.info`): the best image of a DiskObject.
//!
//! Sources:
//! - Layout (78-byte DiskObject with magic `$E310`, optional 56-byte
//!   DrawerData, 20-byte Image headers, planar rows padded to 16 bits, then
//!   the DefaultTool and ToolTypes texts, the ToolWindow text and the 6-byte
//!   DrawerData2): RKM Libraries ch. 14
//!   (<http://www.theflatnet.de/pub/cbm/amiga/AmigaDevDocs/lib_14.html>),
//!   Dirk Stöcker, "Amiga Icon Format" (2002), OS1.x/OS2.x section
//!   (<http://www.evillabs.net/index.php/Amiga_Icon_Formats>), and Deark's
//!   `modules/amigaicon.c` (<https://github.com/jsummers/deark>, MIT
//!   licence, notice below).
//! - The icon revision in the low byte of the gadget's UserData (+44), which
//!   also says whether DrawerData2 is present: Stöcker and Deark.
//! - Workbench 1.x pens (blue, white, black, orange) for revision 0, 2.x
//!   pens (grey, black, white, blue) otherwise, and doubled lines for the
//!   high-resolution screen: observed from `recoil2png` output. The 2.x
//!   blue (`$3B67A2`, from Deark) is not used by any 2-plane sample.
//! - 3-plane 2.x icons: the 2.x pens followed by the other four MagicWB
//!   colours (dark grey, light grey, beige, pink). The order is observed
//!   from `recoil2png` output; beige (`$AA907C`, unused by the samples) is
//!   from Deark's MagicWB palette.
//!
//! An icon may hold up to three versions of its picture, each with a normal
//! and a selected state: the classic planar image, a NewIcon in the tool
//! types (`newicon`) and an OS 3.5 GlowIcon in an IFF `FORM ICON` appended
//! to the file (`glowicon`). We show the normal state of the best one that
//! decodes: GlowIcon, then NewIcon, then classic. RECOIL shows only the
//! classic image. NewIcons and GlowIcons are drawn for square pixels and
//! are not doubled; their transparent colour becomes the Workbench 2.x
//! background pen, the colour the classic image's colour 0 shows as.

// Workbench pen values and the walk over the DiskObject's optional parts
// follow Deark's modules/amigaicon.c (Deark, https://github.com/jsummers/deark):
//
// Copyright (C) 2016-2026 Jason Summers
// <jason1@pobox.com>
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

mod glowicon;
mod newicon;

use alloc::vec::Vec;

use super::iff;
use crate::bytes::{be16, be32};
use crate::image::check_scaled;
use crate::{DecodeError, Image};

const DISK_OBJECT_LEN: usize = 78;
const DRAWER_DATA_LEN: usize = 56;
const DRAWER_DATA_2_LEN: usize = 6;
const IMAGE_HEADER_LEN: usize = 20;
const PALETTE_1X: [u32; 4] = [0x55aaff, 0xffffff, 0x000000, 0xff8800];
const PALETTE_2X: [u32; 8] = [
    0x959595, 0x000000, 0xffffff, 0x3b67a2, 0x7b7b7b, 0xafafaf, 0xaa907c, 0xffa997,
];
/// What transparent NewIcon and GlowIcon pixels show: the 2.x background pen.
const BACKGROUND: u32 = PALETTE_2X[0];

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let object = data.get(..DISK_OBJECT_LEN).ok_or(fail)?;
    // Magic and version 1.
    if be16(object, 0).ok_or(fail)? != 0xe310 || be16(object, 2).ok_or(fail)? != 1 {
        return Err(fail);
    }
    if let Some(extras) = Extras::find(data) {
        let glow = extras.glow.map(|form| glowicon::decode(form, BACKGROUND));
        if let Some(Ok(image)) = glow {
            return Ok(image);
        }
        if let Ok(image) = newicon::decode(&extras.tool_types, BACKGROUND) {
            return Ok(image);
        }
    }
    decode_classic(data)
}

/// Offset of the first Image header.
fn first_image(object: &[u8]) -> Option<usize> {
    let has_drawer = be32(object, 66)? != 0;
    Some(DISK_OBJECT_LEN + if has_drawer { DRAWER_DATA_LEN } else { 0 })
}

/// The first classic image, drawn with the Workbench pens.
fn decode_classic(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let palette: &[u32] = if data[47] == 0 {
        &PALETTE_1X
    } else {
        &PALETTE_2X
    };
    let start = first_image(data).ok_or(fail)?;
    let header = data.get(start..start + IMAGE_HEADER_LEN).ok_or(fail)?;
    let width = usize::from(be16(header, 4).ok_or(fail)?);
    let height = usize::from(be16(header, 6).ok_or(fail)?);
    let depth = usize::from(be16(header, 8).ok_or(fail)?);
    // Only depths whose colours all have a known pen.
    if width == 0 || height == 0 || !(2..=3).contains(&depth) || 1 << depth > palette.len() {
        return Err(fail);
    }
    check_scaled(width, height, 1, 2)?;
    let row_len = width.div_ceil(16) * 2;
    let plane_len = row_len * height;
    let start = start + IMAGE_HEADER_LEN;
    let planes = data.get(start..start + plane_len * depth).ok_or(fail)?;
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

/// The parts after the classic images that may hold better ones.
struct Extras<'a> {
    /// The ToolTypes texts without their terminating zero.
    tool_types: Vec<&'a [u8]>,
    /// The contents of a `FORM ICON` after the DiskObject's parts.
    glow: Option<&'a [u8]>,
}

impl<'a> Extras<'a> {
    /// Walks the DiskObject's optional parts; `None` if they are truncated.
    fn find(data: &'a [u8]) -> Option<Self> {
        let has = |at| be32(data, at).map(|pointer| pointer != 0);
        let has_drawer = has(66)?;
        let images = if has(26)? { 2 } else { 1 };
        let mut pos = first_image(data)?;
        for _ in 0..images {
            pos = pos.checked_add(image_len(data, pos)?)?;
        }
        if has(50)? {
            pos = skip_text(data, pos)?;
        }
        let mut tool_types = Vec::new();
        if has(54)? {
            // The entry count is stored as (count + 1) * 4.
            let count = (be32(data, pos)? as usize / 4).checked_sub(1)?;
            pos += 4;
            for _ in 0..count {
                let end = skip_text(data, pos)?;
                let text = &data[pos + 4..end];
                tool_types.push(text.split(|&b| b == 0).next().unwrap_or(text));
                pos = end;
            }
        }
        if has(70)? {
            pos = skip_text(data, pos)?;
        }
        if has_drawer && data[47] == 1 {
            pos += DRAWER_DATA_2_LEN;
        }
        let glow = data
            .get(pos..)
            .and_then(iff::form)
            .filter(|(kind, _)| kind == b"ICON")
            .map(|(_, contents)| contents);
        Some(Self { tool_types, glow })
    }
}

/// Bytes taken by the Image header at `pos` and its planes.
fn image_len(data: &[u8], pos: usize) -> Option<usize> {
    let width = usize::from(be16(data, pos.checked_add(4)?)?);
    let height = usize::from(be16(data, pos + 6)?);
    let depth = usize::from(be16(data, pos + 8)?);
    let planes = (width.div_ceil(16) * 2)
        .checked_mul(height)?
        .checked_mul(depth)?;
    planes.checked_add(IMAGE_HEADER_LEN)
}

/// The end of the text at `pos`: a 32-bit length (including the
/// terminating zero), then the text.
fn skip_text(data: &[u8], pos: usize) -> Option<usize> {
    let len = be32(data, pos)? as usize;
    let end = (pos + 4).checked_add(len)?;
    (end <= data.len()).then_some(end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn classic_icon_doubled_over_the_pixel_cap_is_rejected() {
        let (width, height) = (65535usize, 513usize);
        let mut data = vec![0; DISK_OBJECT_LEN];
        data[..4].copy_from_slice(&[0xe3, 0x10, 0, 1]);
        let mut image = vec![0; IMAGE_HEADER_LEN];
        image[4..6].copy_from_slice(&(width as u16).to_be_bytes());
        image[6..8].copy_from_slice(&(height as u16).to_be_bytes());
        image[9] = 2;
        data.extend(image);
        data.resize(data.len() + width.div_ceil(16) * 2 * height * 2, 0);
        assert!(matches!(decode(&data), Err(DecodeError::Unrecognized)));
    }

    /// A DiskObject with one 16x1, 2-plane image, the given tool types and
    /// trailing bytes.
    fn icon(tool_types: &[&[u8]], tail: &[u8]) -> Vec<u8> {
        let mut data = vec![0; DISK_OBJECT_LEN];
        data[..4].copy_from_slice(&[0xe3, 0x10, 0, 1]);
        data[47] = 1;
        let mut image = vec![0; IMAGE_HEADER_LEN];
        image[4..10].copy_from_slice(&[0, 16, 0, 1, 0, 2]);
        data.extend(image);
        data.extend([0x80, 0, 0x80, 0]); // pixel 0 has colour 3
        if !tool_types.is_empty() {
            data[54] = 1;
            let count = (tool_types.len() as u32 + 1) * 4;
            data.extend(count.to_be_bytes());
            for text in tool_types {
                data.extend((text.len() as u32 + 1).to_be_bytes());
                data.extend(*text);
                data.push(0);
            }
        }
        data.extend(tail);
        data
    }

    #[test]
    fn classic_image_with_doubled_lines() {
        let image = decode(&icon(&[], &[])).unwrap();
        assert_eq!((image.width(), image.height()), (16, 2));
        assert_eq!(image.get(0, 1), PALETTE_2X[3]);
        assert_eq!(image.get(1, 0), PALETTE_2X[0]);
    }

    #[test]
    fn finds_tool_types_and_glow_form() {
        let data = icon(&[b"A=1", b"IM1=x"], b"FORM\0\0\0\x04ICON");
        let extras = Extras::find(&data).unwrap();
        assert_eq!(extras.tool_types, [&b"A=1"[..], b"IM1=x"]);
        assert_eq!(extras.glow, Some(&b""[..]));
    }

    #[test]
    fn bad_newicon_falls_back_to_classic() {
        let image = decode(&icon(&[b"IM1=?"], &[])).unwrap();
        assert_eq!((image.width(), image.height()), (16, 2));
    }

    #[test]
    fn truncated_tool_types_still_show_classic() {
        let mut data = icon(&[b"IM1=B"], &[]);
        data.truncate(data.len() - 3);
        assert!(Extras::find(&data).is_none());
        assert!(decode(&data).is_ok());
    }
}
