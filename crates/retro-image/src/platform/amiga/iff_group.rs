//! IFF groups that hold pictures: `LIST` and `CAT` at the top of a file, and
//! Deluxe Video's `FORM ANBM` animated brush, shown as their first picture.
//!
//! Sources:
//! - EA IFF 85 standard: `LIST` shares properties through `PROP` chunks,
//!   which apply to the `FORM`s of the same type that follow them in that
//!   `LIST`; `CAT` groups without properties
//!   (<https://wiki.amigaos.net/wiki/EA_IFF_85_Standard_for_Interchange_Format_Files>).
//! - ANBM: `FORM ANBM` with an `FSQN` chunk, then a `LIST ILBM` with a `PROP`
//!   and one `FORM ILBM` per frame
//!   (<https://wiki.amigaos.net/wiki/ANBM_IFF_Animated_Bitmap>).
//!
//! No real file of either kind was found (the `CAT` samples seen hold effect
//! data, not pictures), so this is checked only by unit tests: unverified.
//! The first `FORM` is taken whatever it is; a later one is not tried. A
//! property chunk the `FORM` has itself wins over the same one in a `PROP`,
//! and a later `PROP` over an earlier.

use alloc::vec::Vec;

use super::iff::chunks;
use crate::bytes::be32;

/// Nested groups followed, so a crafted file cannot recurse deeply.
const MAX_DEPTH: usize = 4;

/// The kind and chunks of the first `FORM` in the group `data`, followed by
/// the chunks of the `PROP`s that apply to it, so that a search for a chunk
/// finds the `FORM`'s own first. `None` unless `data` is a `LIST`, a `CAT` or
/// a `FORM ANBM`.
pub(super) fn first_form(data: &[u8]) -> Option<([u8; 4], Vec<u8>)> {
    let kind: [u8; 4] = data.get(8..12)?.try_into().ok()?;
    let group = match &data[..4] {
        b"LIST" | b"CAT " => true,
        b"FORM" => &kind == b"ANBM",
        _ => false,
    };
    let len = (be32(data, 4)? as usize).saturating_add(8).min(data.len());
    group
        .then(|| walk(data.get(12..len.max(12))?, &[], 0))
        .flatten()
}

/// The first `FORM` among `children`, with `props` (type and chunks of each
/// `PROP` seen, oldest first) and those of this group.
fn walk(children: &[u8], props: &[([u8; 4], &[u8])], depth: usize) -> Option<([u8; 4], Vec<u8>)> {
    if depth > MAX_DEPTH {
        return None;
    }
    let mut props = props.to_vec();
    for (id, body) in chunks(children) {
        if !matches!(&id, b"PROP" | b"FORM" | b"LIST" | b"CAT ") {
            continue;
        }
        let Some((kind, rest)) = body.split_first_chunk::<4>() else {
            continue;
        };
        match &id {
            b"PROP" => props.push((*kind, rest)),
            b"FORM" => {
                let mut contents = rest.to_vec();
                for (_, chunks) in props.iter().rev().filter(|(k, _)| k == kind) {
                    contents.extend_from_slice(chunks);
                }
                return Some((*kind, contents));
            }
            _ => {
                if let Some(found) = walk(rest, &props, depth + 1) {
                    return Some(found);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(id: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut out = id.to_vec();
        out.extend_from_slice(&(body.len() as u32).to_be_bytes());
        out.extend_from_slice(body);
        out.resize(out.len() + body.len() % 2, 0);
        out
    }

    fn group(id: &[u8; 4], kind: &[u8; 4], children: &[Vec<u8>]) -> Vec<u8> {
        chunk(id, &[kind.as_slice(), &children.concat()].concat())
    }

    #[test]
    fn a_list_gives_its_first_form_with_the_shared_properties_behind_it() {
        let prop = group(
            b"PROP",
            b"ILBM",
            &[chunk(b"BMHD", b"shared"), chunk(b"CMAP", b"pal")],
        );
        let first = group(
            b"FORM",
            b"ILBM",
            &[chunk(b"BMHD", b"own"), chunk(b"BODY", b"px")],
        );
        let second = group(b"FORM", b"ILBM", &[chunk(b"BODY", b"other")]);
        let list = group(b"LIST", b"ILBM", &[prop, first, second]);
        let (kind, contents) = first_form(&list).unwrap();
        assert_eq!(&kind, b"ILBM");
        // The FORM's BMHD comes first, then its BODY, then the PROP's chunks.
        let ids: Vec<[u8; 4]> = chunks(&contents).map(|(id, _)| id).collect();
        assert_eq!(ids, [*b"BMHD", *b"BODY", *b"BMHD", *b"CMAP"]);
        assert_eq!(chunks(&contents).next().unwrap().1, b"own");
    }

    #[test]
    fn cat_and_anbm_are_followed_and_other_properties_ignored() {
        let prop = group(b"PROP", b"ILBM", &[chunk(b"CMAP", b"pal")]);
        let other = group(b"PROP", b"8SVX", &[chunk(b"VHDR", b"snd")]);
        let frame = group(b"FORM", b"ILBM", &[chunk(b"BODY", b"px")]);
        let list = group(b"LIST", b"ILBM", &[other, prop, frame]);
        let anbm = group(b"FORM", b"ANBM", &[chunk(b"FSQN", b"seq"), list]);
        let (_, contents) = first_form(&anbm).unwrap();
        let ids: Vec<[u8; 4]> = chunks(&contents).map(|(id, _)| id).collect();
        assert_eq!(ids, [*b"BODY", *b"CMAP"]);
        let cat = group(
            b"CAT ",
            b"ILBM",
            &[group(b"FORM", b"ILBM", &[chunk(b"BODY", b"x")])],
        );
        assert!(first_form(&cat).is_some());
    }

    #[test]
    fn plain_forms_and_empty_groups_are_not_groups() {
        let form = group(b"FORM", b"ILBM", &[chunk(b"BODY", b"x")]);
        assert!(first_form(&form).is_none());
        assert!(first_form(&group(b"LIST", b"ILBM", &[chunk(b"BMHD", b"x")])).is_none());
        assert!(first_form(b"LIST\0\0\0\0").is_none());
        // A deep tower of lists ends instead of recursing without bound.
        let mut tower = group(b"FORM", b"ILBM", &[]);
        for _ in 0..40 {
            tower = group(b"LIST", b"ILBM", &[tower]);
        }
        assert!(first_form(&tower).is_none());
    }
}
