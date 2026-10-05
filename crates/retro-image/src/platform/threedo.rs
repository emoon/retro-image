//! 3DO Interactive Multiplayer: IMAG pictures and cels (`.imag`, `.img`,
//! `.3do`, `.cel`, `.anim`), the first picture of a file.
//!
//! Sources:
//! - The chunk format, `IMAG` header, `CCB `, `PLUT` and `ANIM` chunks: "3DO
//!   File Format" in the 3DO Portfolio 2.5 documentation (`ppgfldr/smmfldr/
//!   cdmfldr/08CDM001.html`, mirrored at
//!   <https://3dodev.com/documentation/file_formats/media/container/3do> and in
//!   <https://github.com/trapexit/3do-devkit> `docs/3dosdk/`; prose only, its
//!   license is not stated).
//! - Cel flags, the preamble words and the packed and unpacked source data:
//!   the same documentation's Graphics Programmer's Guide (`5gpgc.html`,
//!   `5gpgd.html`, `5gpge.html`, `3gpgc.html` under `ppgfldr/ggsfldr/
//!   gpgfldr/`).
//! - Checked against 111 distinct sample files of `trapexit/3do-devkit` (data
//!   only: none of that repository's source was read), by looking at the
//!   decoded pictures. Learned from them: a `CCB ` chunk is always 80 bytes
//!   with the preamble words in it; the preamble leads the `PDAT` data
//!   exactly when the `CCBPRE` flag is clear; the row stride of unpacked
//!   cels is `WOFFSET + 2` words; pixel order 1 of an `IMAG` stores pairs of
//!   rows interleaved pixel by pixel.
//! - Platform survey: `docs/research/gaps-consoles.md` section 3.3.
//!
//! Supported: 16-bit `IMAG` pictures; 16-bit uncoded cels and coded cels of
//! 1, 2, 4, 6 and 8 bits, packed or unpacked (only 4, 6 and 8 bits and 16-bit
//! packed and unpacked cels were seen in samples). Not read: 8 and 24-bit
//! `IMAG` pictures, compressed `IMAG` data, the left/right memory format of
//! cels, 8-bit uncoded cels, pictures with a `VDL ` chunk and the 32-bit
//! "z24" images of the SDK slide show, which the documentation does not
//! describe. An animation or a file
//! of several cels shows the first picture. A cel pixel whose 15-bit color
//! is zero is transparent unless the CCB's `BGND` flag is set; transparent
//! pixels are composited onto the shared fill color.

mod cel;
mod imag;

use crate::bytes::be32;
use crate::{DecodeError, Format, Image};

/// One format entry for every extension: the chunk tags decide, because `.cel`
/// and `.img` also belong to other formats.
pub(super) static FORMATS: &[Format] = &[Format::new(
    "3DO",
    "IMAG and cel",
    &["imag", "img", "3do", "cel", "anim"],
    decode,
)
.signature()];

/// A chunk: its tag and the body after the 8-byte header, cut at the end of
/// the file.
struct Chunk<'a> {
    tag: [u8; 4],
    body: &'a [u8],
}

/// The chunks of a file in order. A `3DO ` wrapper chunk is entered rather
/// than skipped; chunks are padded to four bytes. Stops at the first chunk
/// whose size is not at least its header.
fn chunks(data: &[u8]) -> impl Iterator<Item = Chunk<'_>> {
    let mut pos: usize = 0;
    core::iter::from_fn(move || {
        let tag: [u8; 4] = data.get(pos..pos.checked_add(4)?)?.try_into().ok()?;
        let size = be32(data, pos + 4)? as usize;
        if size < 8 {
            return None;
        }
        let end = pos.saturating_add(size).min(data.len());
        let chunk = Chunk {
            tag,
            body: &data[pos + 8..end],
        };
        pos = if &tag == b"3DO " {
            pos + 8
        } else {
            pos.saturating_add(padded(size))
        };
        Some(chunk)
    })
}

/// A chunk size rounded up to the four bytes chunks are padded to, without
/// overflowing on sizes near the top of the range.
fn padded(size: usize) -> usize {
    size.saturating_add(3) & !3
}

fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    let first = chunks(data)
        .find(|chunk| &chunk.tag != b"3DO ")
        .ok_or(DecodeError::Unrecognized)?;
    match &first.tag {
        b"IMAG" => imag::decode(data),
        b"CCB " | b"ANIM" | b"OFST" | b"PLUT" | b"CPYR" | b"DESC" | b"KWRD" | b"CRDT" | b"XTRA" => {
            cel::decode(data)
        }
        _ => Err(DecodeError::Unrecognized),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn padding_a_chunk_size_never_overflows() {
        assert_eq!(padded(8), 8);
        assert_eq!(padded(9), 12);
        assert_eq!(padded(usize::MAX), usize::MAX & !3);
        assert_eq!(padded(usize::MAX - 2), usize::MAX & !3);
    }

    #[test]
    fn chunk_sizes_near_the_top_of_the_range_end_the_walk() {
        // Sizes whose padding to four bytes would overflow a 32-bit usize.
        for size in [0xffff_fffdu32, 0xffff_fffe, 0xffff_ffff] {
            let mut data = b"PLUT".to_vec();
            data.extend_from_slice(&size.to_be_bytes());
            data.extend_from_slice(&[0; 8]);
            assert_eq!(chunks(&data).count(), 1);
            assert!(decode(&data).is_err());
            assert!(crate::decode("a.cel", &data).is_err());
        }
    }
}
