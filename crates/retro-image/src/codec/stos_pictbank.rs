//! STOS / AMOS packed picture streams ("Picture Packer", `Pac.Pic.`).
//!
//! Source: Deark `fmtutil_decompress_stos_pictbank` in `fmtutil-rle.c`
//! (<https://github.com/jsummers/deark>, MIT licence, Copyright (C) 2016-2026
//! Jason Summers).

use alloc::vec::Vec;

/// Three streams: picture bytes, RLE bit masks and "points" bits. Each RLE
/// bit says whether the next output byte is a new picture byte or a repeat;
/// each points bit says whether the next 8 RLE bits are a new mask byte or a
/// repeat of the previous one. The first byte of both byte streams is read
/// up front, as the original decompressor does.
pub(crate) fn unpack(
    data: &[u8],
    pic_pos: usize,
    rle_pos: usize,
    points_pos: usize,
    len: usize,
) -> Option<Vec<u8>> {
    let mut pic = data.get(pic_pos..)?.iter().copied();
    let mut rle = data.get(rle_pos..)?.iter().copied();
    let mut points = data
        .get(points_pos..)?
        .iter()
        .flat_map(|&b| (0..8).rev().map(move |i| b >> i & 1));
    // Every output byte costs at least a bit of RLE data, read every 8 bytes
    // from a points bit.
    if len > data.len().saturating_mul(64) {
        return None;
    }
    let mut pic_byte = pic.next()?;
    let mut rle_byte = rle.next()?;
    let mut out = Vec::with_capacity(len);
    let mut mask = 0u8;
    for i in 0..len {
        if i % 8 == 0 {
            if points.next()? != 0 {
                rle_byte = rle.next()?;
            }
            mask = rle_byte;
        }
        if mask & 0x80 != 0 {
            pic_byte = pic.next()?;
        }
        mask <<= 1;
        out.push(pic_byte);
    }
    Some(out)
}
