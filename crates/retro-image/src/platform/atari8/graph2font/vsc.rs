//! Graph2Font VSC: a vertical scroll, a list of G2F pictures stacked.
//!
//! Sources:
//! - G2F manual (<https://g2f.atari8.info/instrukcja_eng.html>) for the
//!   scroll feature; the file layout is not documented there.
//! - Layout observed from the corpus samples (`katon.vsc`, `girl.vsc`) and by
//!   black-box probing of `recoil2png` with hand-made lists: the file is
//!   text, each G2F name (plain or VBXE) ended by CR LF. A name without its
//!   CR LF at the end is dropped. A list with a missing, non-G2F or MCH
//!   file is rejected. Names are case sensitive, the pictures are the full
//!   336x240 renders stacked with no limit on their number.

use super::g2f;
use super::{LINES, WIDTH};
use crate::image::check_size;
use crate::{Companions, DecodeError, Image};
use alloc::vec::Vec;

pub(in crate::platform::atari8) fn decode_vsc(
    data: &[u8],
    companions: &dyn Companions,
) -> Result<Image, DecodeError> {
    let names: Vec<&[u8]> = names(data).collect();
    if names.is_empty() {
        return Err(DecodeError::Unrecognized);
    }
    check_size(WIDTH, LINES * names.len())?;
    let mut stacked = Image::new(WIDTH as u32, (LINES * names.len()) as u32);
    for (index, name) in names.into_iter().enumerate() {
        let name = core::str::from_utf8(name).map_err(|_| DecodeError::Unrecognized)?;
        let file = companions
            .get_named(name)
            .ok_or(DecodeError::Unrecognized)?;
        let picture = g2f::decode_plain_or_vbxe(&file)?;
        for (y, row) in picture
            .rgb()
            .as_chunks::<{ WIDTH * 3 }>()
            .0
            .iter()
            .enumerate()
        {
            stacked
                .row_mut((index * LINES + y) as u32)
                .copy_from_slice(row);
        }
    }
    Ok(stacked)
}

/// The names of the list: the text before each CR LF.
fn names(data: &[u8]) -> impl Iterator<Item = &[u8]> {
    let mut rest = data;
    core::iter::from_fn(move || {
        let end = rest.windows(2).position(|w| w == b"\r\n")?;
        let name = &rest[..end];
        rest = &rest[end + 2..];
        Some(name)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_at_crlf_and_drops_an_unterminated_name() {
        let found: Vec<&[u8]> = names(b"a.g2f\r\nb.g2f\r\nc.g2f").collect();
        assert_eq!(found, [&b"a.g2f"[..], &b"b.g2f"[..]]);
        assert_eq!(names(b"a.g2f\n").count(), 0);
    }

    #[test]
    fn needs_the_listed_files() {
        assert!(decode_vsc(b"a.g2f\r\n", &crate::NoCompanions).is_err());
        assert!(decode_vsc(b"", &crate::NoCompanions).is_err());
    }
}
