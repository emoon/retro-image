//! More interlace and multi-frame bitmap formats: IGE, ILD, ING, HR, MGA, BGP
//! and CCI.
//!
//! Sources:
//! - Format names, sizes and mode descriptions: Just Solve pages
//!   "Interlace Graphics Editor"
//!   (<http://fileformats.archiveteam.org/wiki/Interlace_Graphics_Editor>),
//!   "Interlace Logo Designer"
//!   (<http://fileformats.archiveteam.org/wiki/Interlace_Logo_Designer>),
//!   "ING 15" (<http://fileformats.archiveteam.org/wiki/ING_15>),
//!   "Bugbiter APAC239i"
//!   (<http://fileformats.archiveteam.org/wiki/Bugbiter_APAC239i>), the
//!   RECOIL format list (<https://recoil.sourceforge.net/formats.html>, names
//!   and sizes only), and the AtariAge thread on APAC 80x240
//!   (<https://atariage.com/forums/topic/216997-apac-256-color-mode-80x240-interlaced-has-this-been-done/>).
//! - Everything else was reverse engineered from the corpus samples
//!   (WARRIOR.IGE, willy.ild, HIP3D1.ING, GIRL.MGA and the MegaColorEditor
//!   files, CH2016.BGP, LUKE1.BGP) and by black-box probing of `recoil2png`
//!   with hand-made files (uniform frames with one colour register set at a
//!   time, one-byte changes to see which bytes are read, size scans).
//!
//! Layouts found:
//! - IGE: a DOS binary-load header `FF FF F6 A3 FF BB`, the two bytes
//!   `FF 5F`, four colour registers per frame (frame 1 then frame 2, indexed by
//!   pixel value), then two frames of 96 lines x 32 bytes (128 pixels, 2 bits
//!   each, drawn 2 wide). The frames are averaged.
//! - ILD: exactly 8195 bytes: two frames of 128 lines x 32 bytes (2 bits) shown
//!   in greys 0, 6, 2, 10 for pixel values 0-3, drawn 2 wide and averaged;
//!   the 3 trailing bytes are not read.
//! - ING: two frames of 200 lines x 40 bytes (2 bits), then four colour
//!   registers shared by both frames. Anything after them is ignored.
//! - HR: exactly 16384 bytes: two 1-bit frames of 256 lines x 32 bytes, of
//!   which 239 lines are shown; clear is black, set is white, the frames are
//!   averaged (3 shades).
//! - MGA: exactly 7856 bytes: an 80x96 APAC picture (see `apac.rs`) of alternating
//!   luminance and hue lines, followed by 176 unread bytes.
//! - BGP: `BUGBITER_APAC239I_PICTURE_V1.0`, `FF`, width 80, height 239, four
//!   unread bytes, the title length (16 bits), the title, the plane size
//!   9560 (16 bits, `58 25`), then the 239 luminance lines and 239 hue lines
//!   of an interlaced APAC picture.
//! - CCI (Champions' Interlace, packed): `CIN 1.2 ` and four chunks, each a
//!   16-bit length (counting the next field), a 16-bit field whose meaning
//!   is unknown (it looks like a token count; not read), and run-length
//!   tokens: a byte below 0x80 is followed by that many plus one literal
//!   bytes, a byte from 0x80 by one byte repeated (low 7 bits plus one)
//!   times. Found by unpacking SUNV2.CCI and matching it to the unpacked
//!   THESUNV2.CIN, which renders identically. The chunks unpack to 3840 bytes
//!   each: the Graphics 15 columns of even lines, then of odd lines (96 bytes
//!   per column, left to right), then the 7680 bytes of the hue plane, also by
//!   column (192 per column), then the 1024 bytes of per-line colour tables.
//!   Together that is a 16384-byte CIN picture. DRACONUS.CCI's odd-line chunk
//!   unpacks to 2 bytes too many; they are dropped, as is any data after the
//!   fourth chunk (probed: only the 1024-byte table variant is accepted).

use super::antic::Bitmap;
use super::apac::decode_cin;
use super::apac::{INTERLACED, Scanlines, apac_80x96, deinterleave, nibble};
use super::palette::{register_rgb, rgb};
use crate::bytes::le16;
use crate::{DecodeError, Image};
use alloc::vec;
use alloc::vec::Vec;

/// Two frames of 2-bit pixels, 32 bytes per line, drawn 2 wide.
fn narrow_frames(
    data: &[u8],
    lines: usize,
    color: impl Fn(usize, u8) -> u32,
) -> Result<Image, DecodeError> {
    let frame = |data: &[u8], which: usize| {
        let bitmap = Bitmap {
            data,
            bytes_per_line: 32,
            lines,
            bits: 2,
        };
        bitmap.render(2, 1, |_, value| color(which, value))
    };
    let (first, second) = data.split_at(32 * lines);
    Ok(Image::blend(&[&frame(first, 0)?, &frame(second, 1)?]))
}

/// Interlace Graphics Editor: a binary-load header, `FF 5F`, 2 x 4 colour
/// registers, then two 128x96 frames.
pub(super) fn decode_ige(data: &[u8]) -> Result<Image, DecodeError> {
    const MAGIC: [u8; 8] = [0xff, 0xff, 0xf6, 0xa3, 0xff, 0xbb, 0xff, 0x5f];
    if data.len() != 16 + 2 * 32 * 96 || data[..8] != MAGIC {
        return Err(DecodeError::Unrecognized);
    }
    let colors = &data[8..16];
    narrow_frames(&data[16..], 96, |frame, value| {
        register_rgb(colors[4 * frame + usize::from(value)])
    })
}

/// Interlace Logo Designer: two 128x128 frames in four greys.
pub(super) fn decode_ild(data: &[u8]) -> Result<Image, DecodeError> {
    const GREYS: [u8; 4] = [0x00, 0x06, 0x02, 0x0a];
    if data.len() != 2 * 32 * 128 + 3 {
        return Err(DecodeError::Unrecognized);
    }
    narrow_frames(data, 128, |_, value| {
        register_rgb(GREYS[usize::from(value)])
    })
}

/// ING 15: two 160x200 frames, then four colour registers.
pub(super) fn decode_ing(data: &[u8]) -> Result<Image, DecodeError> {
    const FRAME: usize = 8000;
    if data.len() < 2 * FRAME + 4 {
        return Err(DecodeError::Unrecognized);
    }
    let colors = &data[2 * FRAME..2 * FRAME + 4];
    let frame = |data: &[u8]| {
        let bitmap = Bitmap {
            data: &data[..FRAME],
            bytes_per_line: 40,
            lines: 200,
            bits: 2,
        };
        bitmap.render(2, 1, |_, value| register_rgb(colors[usize::from(value)]))
    };
    Ok(Image::blend(&[&frame(data)?, &frame(&data[FRAME..])?]))
}

/// Atari HR: two 256x239 one-bit frames, averaged into black, grey and white.
pub(super) fn decode_hr(data: &[u8]) -> Result<Image, DecodeError> {
    const FRAME: usize = 8192;
    if data.len() != 2 * FRAME {
        return Err(DecodeError::Unrecognized);
    }
    let white = register_rgb(0x0e);
    let frame = |data: &[u8]| {
        let bitmap = Bitmap {
            data,
            bytes_per_line: 32,
            lines: 239,
            bits: 1,
        };
        bitmap.render(1, 1, |_, value| if value == 0 { 0 } else { white })
    };
    Ok(Image::blend(&[&frame(data)?, &frame(&data[FRAME..])?]))
}

/// MGA: an 80x96 APAC picture of alternating luminance and hue lines (the
/// reverse of APA's order), plus 176 unread bytes.
pub(super) fn decode_mga(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 7680 + 176 {
        return Err(DecodeError::Unrecognized);
    }
    let (luminance, hue) = deinterleave(&data[..7680]);
    apac_80x96(&hue, &luminance)
}

/// Bugbiter APAC239i: a header with a title, then a size word and 239
/// luminance lines, a second size word and 239 hue lines, interlaced.
pub(super) fn decode_bgp(data: &[u8]) -> Result<Image, DecodeError> {
    const MAGIC: &[u8] = b"BUGBITER_APAC239I_PICTURE_V1.0\xff\x50\xef";
    const PLANE: usize = 239 * 40;
    let rest = data
        .strip_prefix(MAGIC)
        .and_then(|rest| rest.get(4..))
        .ok_or(DecodeError::Unrecognized)?;
    let title = usize::from(le16(rest, 0).ok_or(DecodeError::Unrecognized)?);
    // Each plane has its own 16-bit size word.
    let size = (PLANE as u16).to_le_bytes();
    let planes = rest
        .get(2 + title..)
        .and_then(|rest| rest.strip_prefix(&size))
        .filter(|planes| planes.len() == 2 * PLANE + 2)
        .ok_or(DecodeError::Unrecognized)?;
    let (luminance, hue) = planes.split_at(PLANE);
    let hue = hue.strip_prefix(&size).ok_or(DecodeError::Unrecognized)?;
    let picture = Scanlines {
        lines: 239,
        luminance: |y, x| nibble(luminance, y, x / 2),
        hue: |y, x| nibble(hue, y, x / 2),
        top: |x| rgb(nibble(luminance, 0, x / 2)),
    };
    picture.render(INTERLACED)
}

/// Champions' Interlace, packed: unpacks to a 16384-byte CIN picture.
pub(super) fn decode_cci(data: &[u8]) -> Result<Image, DecodeError> {
    const LINES: usize = 192;
    let mut rest = data
        .strip_prefix(b"CIN 1.2 ")
        .ok_or(DecodeError::Unrecognized)?;
    let mut chunk = |size: usize| -> Result<Vec<u8>, DecodeError> {
        let length = usize::from(le16(rest, 0).ok_or(DecodeError::Unrecognized)?);
        let (packed, after) = rest
            .get(4..)
            .and_then(|body| body.split_at_checked(length.checked_sub(2)?))
            .ok_or(DecodeError::Unrecognized)?;
        rest = after;
        unpack(packed, size)
    };
    let even = chunk(LINES / 2 * 40)?;
    let odd = chunk(LINES / 2 * 40)?;
    let hue = chunk(LINES * 40)?;
    let tables = chunk(1024)?;
    // Columns of one parity of lines back to lines.
    let mut cin = vec![0u8; 16384];
    for (parity, columns) in [even, odd].iter().enumerate() {
        for (i, &byte) in columns.iter().enumerate() {
            let (x, y) = (i / (LINES / 2), 2 * (i % (LINES / 2)) + parity);
            cin[y * 40 + x] = byte;
        }
    }
    for (i, &byte) in hue.iter().enumerate() {
        let (x, y) = (i / LINES, i % LINES);
        cin[LINES * 40 + y * 40 + x] = byte;
    }
    cin[2 * LINES * 40..].copy_from_slice(&tables);
    decode_cin(&cin)
}

/// Unpacks run-length tokens to exactly `size` bytes; anything beyond is
/// dropped.
fn unpack(mut packed: &[u8], size: usize) -> Result<Vec<u8>, DecodeError> {
    let mut out = Vec::with_capacity(size);
    while out.len() < size {
        let (&control, rest) = packed.split_first().ok_or(DecodeError::Unrecognized)?;
        let count = usize::from(control & 0x7f) + 1;
        if control & 0x80 != 0 {
            let (&byte, rest) = rest.split_first().ok_or(DecodeError::Unrecognized)?;
            out.resize(out.len() + count, byte);
            packed = rest;
        } else {
            let literal = rest.get(..count).ok_or(DecodeError::Unrecognized)?;
            out.extend_from_slice(literal);
            packed = &rest[count..];
        }
    }
    out.truncate(size);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ing_uses_shared_registers() {
        let mut data = vec![0u8; 16004];
        data[0] = 0x40; // frame 1, pixel 1 = value 1
        data[16000..].copy_from_slice(&[0x00, 0x0e, 0x00, 0x00]);
        let image = decode_ing(&data).unwrap();
        // Frame 2 shows value 0 (black) there.
        assert_eq!(image.get(0, 0), 0x777777);
        assert_eq!(image.get(2, 0), 0);
        assert!(decode_ing(&data[..16003]).is_err());
    }

    #[test]
    fn hr_averages_one_bit_frames() {
        let mut data = vec![0u8; 16384];
        data[0] = 0x80;
        data[8192] = 0x80;
        let image = decode_hr(&data).unwrap();
        assert_eq!(image.get(0, 0), 0xeeeeee);
        data[8192] = 0;
        assert_eq!(decode_hr(&data).unwrap().get(0, 0), 0x777777);
        assert_eq!((image.width(), image.height()), (256, 239));
    }

    #[test]
    fn cci_unpacks_literals_and_runs() {
        assert_eq!(
            unpack(&[0x01, 0xaa, 0xbb, 0x82, 0x55], 5).unwrap(),
            [0xaa, 0xbb, 0x55, 0x55, 0x55]
        );
        // Extra bytes of the last run are dropped; missing ones are an error.
        assert_eq!(unpack(&[0x85, 0x01], 2).unwrap(), [1, 1]);
        assert!(unpack(&[0x02, 0x01], 3).is_err());
        assert!(decode_cci(b"CIN 1.2 ").is_err());
    }

    #[test]
    fn bgp_needs_plane_size() {
        let mut data = b"BUGBITER_APAC239I_PICTURE_V1.0\xff\x50\xef\0\0\0\0\x02\0hi".to_vec();
        data.extend_from_slice(&[0x58, 0x25]);
        data.resize(data.len() + 9560, 0);
        data.extend_from_slice(&[0x58, 0x25]);
        data.resize(data.len() + 9560, 0);
        let image = decode_bgp(&data).unwrap();
        assert_eq!((image.width(), image.height()), (320, 239));
        data.pop();
        assert!(decode_bgp(&data).is_err());
    }
}
