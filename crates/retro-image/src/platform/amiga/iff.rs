//! EA IFF 85 container: `FORM` group and its chunks.
//!
//! Source: EA IFF 85 standard
//! (<https://wiki.amigaos.net/wiki/EA_IFF_85_Standard_for_Interchange_Format_Files>).

use crate::bytes::be32;

/// Returns the FORM type and its contents, if `data` is an IFF FORM.
///
/// The declared FORM length is clamped to the data actually present.
pub(super) fn form(data: &[u8]) -> Option<([u8; 4], &[u8])> {
    if data.len() < 12 || &data[..4] != b"FORM" {
        return None;
    }
    let len = be32(data, 4)? as usize;
    let end = data.len().min(8usize.saturating_add(len));
    let kind = data[8..12].try_into().ok()?;
    Some((kind, &data[12..end.max(12)]))
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
    fn clamps_overlong_chunk() {
        let contents = b"BODY\x7f\0\0\0abc";
        assert_eq!(find(contents, b"BODY"), Some(&b"abc"[..]));
    }
}
