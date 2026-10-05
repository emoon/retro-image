//! GRASP GL animation files (`.GL`, Bridges / Paul Mace, DOS): the first
//! still picture inside.
//!
//! Sources:
//! - Container: Deark's `graspgl` module in `grasp.c`
//!   (<https://github.com/jsummers/deark>, MIT license, notice below) and the
//!   survey note `docs/research/gaps-computers-extra.md` C5. A little-endian
//!   word gives the size of the index, a multiple of 17 bytes; each 17-byte
//!   entry is a little-endian 32-bit offset and a 13-byte NUL-padded name;
//!   the last entry is all offset 0. At each offset a 32-bit length is
//!   followed by the member. Deark calls a file whose first member follows
//!   the index right away a strong match; the Amiga variant (`41 47 01 00`,
//!   big-endian) is not handled, no sample has it.
//! - Reverse engineered from the 47 samples (Sembiance `video/grasp`): in
//!   every file the members follow each other from the end of the index, so
//!   each offset is the previous offset plus 4 plus the previous length. The
//!   last member ends at the end of the file, except in `FEMA.GL`, which is
//!   cut short. This chain, with the index size and the terminator, is
//!   what makes the file recognizable by content.
//! - Members are PCPaint pictures (`.PIC`, and `.CLP` files that are mostly
//!   PIC data, some a real clip, see `pcpaint.rs`), PCX (`.PCX`, `.PCC`),
//!   GIF, GRASP fonts, scripts and the animation frames (`.DFF`, delta
//!   frames). Only the pictures are decoded, and only the first one that
//!   decodes is shown, the same choice as for other archives of unrelated
//!   pictures. A member is tried if the extension of its stored name is one
//!   of those of a picture, and decoded by its content.
//!   `ACORN.GL` and `KITE.GL` hold only frames and text, so they are
//!   rejected. PCPaint 3.1's own `.OVR` files (cursors and pattern clips)
//!   use this container too, so eight of them in `corpus/extra/pcpaint-ovr/`
//!   show their first clip.
//! - Checked on all 47 samples: the member shown matches Deark's decode of
//!   that member pixel for pixel (see `dos-clipart.tsv`). Pictures without a
//!   palette of their own use the PCPaint defaults.

// Parts of this file follow Deark's modules/grasp.c
// (Deark, https://github.com/jsummers/deark):
//
// Copyright (C) 2016 Jason Summers
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

use alloc::vec::Vec;

use super::{gif, pcpaint, pcx};
use crate::bytes::{le16, le32};
use crate::{DecodeError, Image};

const FAIL: DecodeError = DecodeError::Unrecognized;
const INDEX_AT: usize = 2;
const ENTRY_LEN: usize = 17;
const NAME_LEN: usize = 13;

struct Member<'a> {
    name: &'a [u8],
    data: &'a [u8],
}

/// The members that lie inside the file, in order, after checking that the
/// index is well formed and that the members follow each other from its end.
/// A file cut short ends the list early.
fn members(data: &[u8]) -> Result<Vec<Member<'_>>, DecodeError> {
    let index_len = usize::from(le16(data, 0).ok_or(FAIL)?);
    if index_len == 0 || index_len % ENTRY_LEN != 0 {
        return Err(FAIL);
    }
    let index = data.get(INDEX_AT..INDEX_AT + index_len).ok_or(FAIL)?;
    let (terminator, entries) = index.as_chunks::<ENTRY_LEN>().0.split_last().ok_or(FAIL)?;
    if entries.is_empty() || le32(terminator, 0) != Some(0) {
        return Err(FAIL);
    }
    let mut members = Vec::new();
    let mut at = INDEX_AT + index_len;
    for entry in entries {
        let name = &entry[4..4 + NAME_LEN];
        let name = name.split(|&b| b == 0).next().unwrap_or_default();
        let named = !name.is_empty() && name.iter().all(|&b| b >= 0x20);
        if !named || le32(entry, 0).and_then(|o| usize::try_from(o).ok()) != Some(at) {
            return Err(FAIL);
        }
        let Some(len) = le32(data, at).and_then(|len| usize::try_from(len).ok()) else {
            break;
        };
        let start = at + 4;
        let end = start.checked_add(len).ok_or(FAIL)?;
        let Some(body) = data.get(start..end) else {
            break;
        };
        members.push(Member { name, data: body });
        at = end;
    }
    Ok(members)
}

/// The extension of a member's stored name, lower case.
fn extension(name: &[u8]) -> Vec<u8> {
    let ext = name.rsplit(|&b| b == b'.').next().unwrap_or_default();
    ext.to_ascii_lowercase()
}

/// Members named like a picture are decoded by what they hold: the stored
/// names are not reliable (`M1.PCX` in `treat.gl` is a PCPaint picture, and a
/// `.CLP` is usually one too). The clip, which has no magic number, is the
/// last resort.
fn decode_member(member: &Member<'_>) -> Result<Image, DecodeError> {
    if !matches!(
        extension(member.name).as_slice(),
        b"pic" | b"clp" | b"pcx" | b"pcc" | b"gif"
    ) {
        return Err(FAIL);
    }
    pcpaint::decode_pic(member.data)
        .or_else(|_| pcx::decode_pcx(member.data))
        .or_else(|_| gif::decode_gif(member.data))
        .or_else(|_| pcpaint::decode_clp(member.data))
}

pub(super) fn decode_gl(data: &[u8]) -> Result<Image, DecodeError> {
    members(data)?
        .iter()
        .find_map(|member| decode_member(member).ok())
        .ok_or(FAIL)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A GL file of (name, member) pairs.
    fn file(members: &[(&str, &[u8])]) -> Vec<u8> {
        let index_len = (members.len() + 1) * ENTRY_LEN;
        let mut index = Vec::new();
        let mut body = Vec::new();
        let mut at = INDEX_AT + index_len;
        for (name, member) in members {
            index.extend_from_slice(&(at as u32).to_le_bytes());
            let mut field = name.as_bytes().to_vec();
            field.resize(NAME_LEN, 0);
            index.extend_from_slice(&field);
            body.extend_from_slice(&(member.len() as u32).to_le_bytes());
            body.extend_from_slice(member);
            at += 4 + member.len();
        }
        index.extend_from_slice(&[0; ENTRY_LEN]);
        let mut data = (index_len as u16).to_le_bytes().to_vec();
        data.extend(index);
        data.extend(body);
        data
    }

    /// An uncompressed 8x1 PCPaint clip: file length, width, height, a zero
    /// word, the 1-bit plane flag and one row.
    fn clip() -> Vec<u8> {
        let mut data = alloc::vec![0, 0, 8, 0, 1, 0, 0, 0, 0, 0, 1, 0x80];
        data[0] = data.len() as u8;
        data
    }

    #[test]
    fn the_first_decodable_picture_is_shown() {
        let data = file(&[("a.dff", b"frames"), ("b.txt", b"x"), ("c.clp", &clip())]);
        let image = decode_gl(&data).unwrap();
        assert_eq!((image.width(), image.height()), (8, 1));
        assert!(decode_gl(&file(&[("a.txt", &clip())])).is_err());
    }

    #[test]
    fn the_index_and_the_member_chain_must_be_consistent() {
        let good = file(&[("a.clp", &clip()), ("b.clp", &clip())]);
        assert!(decode_gl(&good).is_ok());
        // The second offset points one byte too far.
        let mut bad = good.clone();
        bad[2 + ENTRY_LEN] += 1;
        assert!(members(&bad).is_err());
        // No terminator.
        let mut bad = good.clone();
        bad[2 + 2 * ENTRY_LEN] = 1;
        assert!(members(&bad).is_err());
        assert!(members(&good[..1]).is_err());
        assert!(members(&[0, 0]).is_err());
        assert!(members(&[17, 0]).is_err());
    }

    #[test]
    fn a_file_cut_short_still_shows_what_is_there() {
        let data = file(&[("a.clp", &clip()), ("b.clp", &clip())]);
        let cut = &data[..data.len() - 3];
        assert_eq!(members(cut).unwrap().len(), 1);
        assert!(decode_gl(cut).is_ok());
    }
}
