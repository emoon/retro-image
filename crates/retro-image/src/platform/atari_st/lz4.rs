//! LZ4 frame decompression (for `PL4`).
//!
//! Sources: the LZ4 frame format and block format specifications,
//! <https://github.com/lz4/lz4/blob/dev/doc/lz4_Frame_format.md> and
//! <https://github.com/lz4/lz4/blob/dev/doc/lz4_Block_format.md>.

use alloc::vec::Vec;

const MAGIC: u32 = 0x184d_2204;

fn le32(data: &[u8], pos: usize) -> Option<u32> {
    let b = data.get(pos..pos + 4)?;
    Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// Decompresses an LZ4 frame, stopping once `limit` bytes are produced.
pub(super) fn decompress_frame(data: &[u8], limit: usize) -> Option<Vec<u8>> {
    if le32(data, 0)? != MAGIC {
        return None;
    }
    let flags = *data.get(4)?;
    if flags >> 6 != 1 {
        return None;
    }
    let block_checksum = flags & 0x10 != 0;
    let content_size = flags & 0x08 != 0;
    let dict_id = flags & 0x01 != 0;
    // FLG, BD, optional content size and dictionary id, header checksum.
    let mut pos = 4 + 2 + if content_size { 8 } else { 0 } + if dict_id { 4 } else { 0 } + 1;
    let mut out = Vec::new();
    loop {
        let size = le32(data, pos)?;
        pos += 4;
        if size == 0 {
            break;
        }
        let len = (size & 0x7fff_ffff) as usize;
        let block = data.get(pos..pos.checked_add(len)?)?;
        pos += len + if block_checksum { 4 } else { 0 };
        if size & 0x8000_0000 != 0 {
            out.extend_from_slice(block);
        } else {
            decompress_block(block, &mut out, limit)?;
        }
        if out.len() >= limit {
            break;
        }
    }
    out.truncate(limit);
    Some(out)
}

/// Appends one LZ4 block's output to `out` (earlier output may be
/// referenced by matches).
fn decompress_block(block: &[u8], out: &mut Vec<u8>, limit: usize) -> Option<()> {
    let mut pos = 0;
    let length = |pos: &mut usize, mut n: usize| -> Option<usize> {
        if n == 15 {
            loop {
                let b = *block.get(*pos)?;
                *pos += 1;
                n = n.checked_add(usize::from(b))?;
                if b != 255 {
                    break;
                }
            }
        }
        Some(n)
    };
    while pos < block.len() {
        let token = block[pos];
        pos += 1;
        let literals = length(&mut pos, usize::from(token >> 4))?;
        out.extend_from_slice(block.get(pos..pos.checked_add(literals)?)?);
        pos += literals;
        if pos == block.len() || out.len() > limit {
            break;
        }
        let offset = usize::from(u16::from_le_bytes([*block.get(pos)?, *block.get(pos + 1)?]));
        pos += 2;
        // Nothing past `limit` is kept, so don't expand huge matches.
        let count = (length(&mut pos, usize::from(token & 15))? + 4).min(limit + 1 - out.len());
        if offset == 0 || offset > out.len() {
            return None;
        }
        let start = out.len() - offset;
        for i in 0..count {
            let b = out[start + i];
            out.push(b);
        }
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literals_and_overlapping_match() {
        // "ab" then a match of length 6 at offset 2 -> "abababab".
        let mut out = Vec::new();
        decompress_block(&[0x22, b'a', b'b', 2, 0, 0x00], &mut out, 100).unwrap();
        assert_eq!(out, b"abababab");
    }

    #[test]
    fn rejects_bad_offset() {
        let mut out = Vec::new();
        assert_eq!(decompress_block(&[0x10, b'a', 5, 0], &mut out, 100), None);
    }
}
