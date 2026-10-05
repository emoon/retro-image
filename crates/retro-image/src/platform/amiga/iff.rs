//! EA IFF 85 container: `FORM` group and its chunks.
//!
//! Source: EA IFF 85 standard
//! (<https://wiki.amigaos.net/wiki/EA_IFF_85_Standard_for_Interchange_Format_Files>).

use crate::bytes::be32;

/// Returns the FORM type and its contents, if `data` is an IFF FORM.
///
/// The declared FORM length is clamped to the data present. If it ends inside
/// the last chunk, the length is wrong (two Deluxe Paint ACBM files declare 68
/// bytes too few) and the contents run to the end of the data. Bytes after a
/// complete FORM are left out.
pub(super) fn form(data: &[u8]) -> Option<([u8; 4], &[u8])> {
    if data.get(..4)? != b"FORM" {
        return None;
    }
    group(data)
}

/// Returns the type and contents of any group chunk (`FORM`, `LIST`, `CAT `),
/// with the declared length handled as described for [`form`].
pub(super) fn group(data: &[u8]) -> Option<([u8; 4], &[u8])> {
    if data.len() < 12 {
        return None;
    }
    let len = be32(data, 4)? as usize;
    let declared = data.len().min(8usize.saturating_add(len)).max(12);
    let end = if last_chunk_is_cut(&data[12..declared]) {
        data.len()
    } else {
        declared
    };
    let kind = data[8..12].try_into().ok()?;
    Some((kind, &data[12..end]))
}

/// Whether the last chunk of `data` claims more bytes than `data` holds.
fn last_chunk_is_cut(mut data: &[u8]) -> bool {
    while data.len() >= 8 {
        let Some(len) = be32(data, 4).map(|len| len as usize) else {
            return false;
        };
        let rest = &data[8..];
        if len > rest.len() {
            return true;
        }
        data = &rest[len.saturating_add(len & 1).min(rest.len())..];
    }
    false
}

/// Iterates over `(id, body)` chunks; a body running past the end is clamped.
pub(super) fn chunks(mut data: &[u8]) -> impl Iterator<Item = ([u8; 4], &[u8])> {
    core::iter::from_fn(move || {
        if data.len() < 8 {
            return None;
        }
        let id: [u8; 4] = data[..4].try_into().ok()?;
        let len = be32(data, 4)? as usize;
        let rest = &data[8..];
        let body = &rest[..len.min(rest.len())];
        let next = len.saturating_add(len & 1).min(rest.len());
        data = &rest[next..];
        Some((id, body))
    })
}

/// The first chunk with `id`.
pub(super) fn find<'a>(contents: &'a [u8], id: &[u8; 4]) -> Option<&'a [u8]> {
    chunks(contents)
        .find(|(i, _)| i == id)
        .map(|(_, body)| body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walks_padded_chunks() {
        let data = b"FORM\0\0\0\x18ILBMAAAA\0\0\0\x01xpBBBB\0\0\0\x02yz";
        let (kind, contents) = form(data).unwrap();
        assert_eq!(&kind, b"ILBM");
        let all: alloc::vec::Vec<_> = chunks(contents).collect();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0], (*b"AAAA", &b"x"[..]));
        assert_eq!(all[1], (*b"BBBB", &b"yz"[..]));
    }

    #[test]
    fn short_form_length_runs_on_when_it_cuts_a_chunk() {
        // The FORM says 0x10 bytes but BODY needs 0x14.
        let data = b"FORM\0\0\0\x10ILBMBODY\0\0\0\x08abcdefgh";
        let (_, contents) = form(data).unwrap();
        assert_eq!(find(contents, b"BODY"), Some(&b"abcdefgh"[..]));
    }

    #[test]
    fn bytes_after_a_complete_form_are_left_out() {
        let data = b"FORM\0\0\0\x0cILBMAAAA\0\0\0\0trailing";
        let (_, contents) = form(data).unwrap();
        assert_eq!(contents, b"AAAA\0\0\0\0");
        // A length that ends inside a chunk header does not count as a cut chunk.
        let data = b"FORM\0\0\0\x08ILBMAAA\0\0\0\0\0trailing";
        assert_eq!(form(data).unwrap().1, b"AAA\0");
    }

    #[test]
    fn clamps_overlong_chunk() {
        let contents = b"BODY\x7f\0\0\0abc";
        assert_eq!(find(contents, b"BODY"), Some(&b"abc"[..]));
    }
}
