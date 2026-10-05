//! Z's Staff Kid98 `ZIM` pictures (NEC PC-98, 640x400, 16 colors).
//!
//! No layout is published (emk's HTML5 viewer page says the format is a
//! proprietary one with no public spec; only its prose was read, see
//! <http://web.archive.org/web/20170105145627/https://emk.name/2016/01/zim.html>,
//! and none of its code). The layout was reverse engineered from
//! `lockonstar.zim` by black-box probing of `recoil2png` with hand-mutated and
//! synthesised files; see `docs/research/msx-japanese.md`, "Wave 5: Japanese".
//!
//! Layout, all little-endian:
//! - `"FORMAT-A"` at 0; 0x1FA and 0x200..0x204 are zero; 0x204/0x206 are
//!   width - 1 and height - 1 (only 640 wide is accepted); 0x214 is 1 and
//!   0x216 non-zero (zero makes RECOIL draw a black picture, so it is refused).
//!   Everything else in the 0x1FC-byte text area (names, `"PC9801"`, a date) is
//!   ignored.
//! - 16 palette entries of 4 bytes at 0x218: blue, red, green, unused, each
//!   a full 8-bit component.
//! - At 0x258 a count `n` of 16-bit words that are skipped (RECOIL ignores them).
//! - Then one block per line: width (640), x (0), y, size, bytes per line (320),
//!   followed by `size - 2` bytes of packed line.
//! - A packed line is a three-level bitmap tree (MSB first) and literals: one
//!   byte whose bits select which of 8 first-level bytes follow; each of those
//!   selects which of 8 leaf bytes follow; each leaf bit says whether the
//!   matching delta byte of the 320-byte line is a literal (else 0).
//! - The 320 deltas `t` become `u[i] = t[i] ^ u[i - 4]`, then `p[i] = u[i] ^
//!   u[i - 1]` over the whole line. `p` holds the four bit planes of the
//!   line, 80 bytes each, plane 3 first (MSB-first pixels).

use alloc::vec::Vec;

use crate::bytes::le16;
use crate::image::planar_pixels;
use crate::{DecodeError, Image};

const SIGNATURE: &[u8] = b"FORMAT-A";
const WIDTH: usize = 640;
const MAX_HEIGHT: usize = 400;
const LINE_BYTES: usize = WIDTH / 2;
const PLANE_BYTES: usize = WIDTH / 8;
const PALETTE: usize = 0x218;
const TABLE_COUNT: usize = 0x258;

/// Unpacks one line into its four planes' bytes (plane 3 first).
fn unpack_line(packed: &[u8]) -> Option<[u8; LINE_BYTES]> {
    let &top = packed.first()?;
    let n1 = top.count_ones() as usize;
    let l1 = packed.get(1..1 + n1)?;
    let n2: usize = l1.iter().map(|b| b.count_ones() as usize).sum();
    let leaves = packed.get(1 + n1..1 + n1 + n2)?;
    let mut literals = packed.get(1 + n1 + n2..)?.iter();

    let mut delta = [0u8; LINE_BYTES];
    let mut leaf = leaves.iter();
    let mut second = l1.iter();
    for group in (0..8).filter(|g| top & (0x80 >> g) != 0) {
        let mid = *second.next()?;
        for sub in (0..8).filter(|s| mid & (0x80 >> s) != 0) {
            let bits = *leaf.next()?;
            for bit in (0..8).filter(|b| bits & (0x80 >> b) != 0) {
                let at = (group * 8 + sub) * 8 + bit;
                *delta.get_mut(at)? = *literals.next()?;
            }
        }
    }
    if literals.next().is_some() {
        return None;
    }

    let mut chained = delta;
    for i in 4..LINE_BYTES {
        chained[i] ^= chained[i - 4];
    }
    let mut line = chained;
    for i in 1..LINE_BYTES {
        line[i] ^= chained[i - 1];
    }
    Some(line)
}

pub(in crate::platform) fn decode_zim(data: &[u8]) -> Result<Image, DecodeError> {
    let bad = DecodeError::Unrecognized;
    if !data.starts_with(SIGNATURE) || data.len() < TABLE_COUNT + 2 {
        return Err(bad);
    }
    let word = |at: usize| {
        le16(data, at)
            .map(usize::from)
            .ok_or(DecodeError::Unrecognized)
    };
    let height = word(0x206)? + 1;
    if word(0x1fa)? != 0
        || word(0x200)? != 0
        || word(0x202)? != 0
        || word(0x204)? + 1 != WIDTH
        || height > MAX_HEIGHT
        || word(0x214)? != 1
        || word(0x216)? == 0
    {
        return Err(bad);
    }
    let palette: Vec<u32> = (0..16)
        .map(|i| {
            let at = PALETTE + i * 4;
            u32::from(data[at + 1]) << 16 | u32::from(data[at + 2]) << 8 | u32::from(data[at])
        })
        .collect();

    let mut pos = TABLE_COUNT + 2 + word(TABLE_COUNT)? * 2;
    let mut lines = Vec::with_capacity(LINE_BYTES * height);
    for y in 0..height {
        let (w, x, row, size, bytes) = (
            word(pos)?,
            word(pos + 2)?,
            word(pos + 4)?,
            word(pos + 6)?,
            word(pos + 8)?,
        );
        if w != WIDTH || x != 0 || row != y || bytes != LINE_BYTES || size < 2 {
            return Err(bad);
        }
        let packed = data.get(pos + 10..pos + 8 + size).ok_or(bad)?;
        let line = unpack_line(packed).ok_or(bad)?;
        lines.extend_from_slice(&line);
        pos += 8 + size;
    }
    // The line stream ends with a zero word; anything after it is ignored.
    if le16(data, pos) != Some(0) {
        return Err(bad);
    }
    // Each line stores its planes from the highest bit down.
    let indices: Vec<u8> = planar_pixels(&lines, WIDTH, height, PLANE_BYTES, 4, |plane, y| {
        y * LINE_BYTES + (3 - plane) * PLANE_BYTES
    })
    .into_iter()
    .map(|v| v as u8)
    .collect();
    Image::from_indexed(WIDTH as u32, height as u32, &indices, &palette)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unpacks_tree_and_deltas() {
        // One literal, 0x80, at delta byte 4: the chain repeats it every 4
        // bytes and the difference pass doubles each into a pair of bytes.
        let line = unpack_line(&[0x80, 0x80, 0x08, 0x80]).unwrap();
        assert_eq!(&line[..8], &[0, 0, 0, 0, 0x80, 0x80, 0, 0]);
        assert_eq!(line[LINE_BYTES - 3], 0x80);
        // Leftover literals are refused.
        assert!(unpack_line(&[0x00, 0x01]).is_none());
        // A bit pointing past the 320 byte line is refused.
        assert!(unpack_line(&[0x01, 0x01, 0x01, 0x01, 0xff]).is_none());
    }
}
