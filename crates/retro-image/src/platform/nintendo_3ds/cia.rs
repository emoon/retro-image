//! Nintendo 3DS installable archive (`.cia`): the icon in its meta section.
//!
//! Sources: GBATEK, "3DS Files - Title Installation Archive (CIA)"
//! (<https://problemkaputt.de/gbatek.htm>, no license stated, so facts only),
//! cross-checked with 3dbrew, "CIA" (<https://www.3dbrew.org/wiki/CIA>,
//! facts only). The header is `u32` header size (0x2020), `u16` type and
//! `u16` version (both 0), then `u32` sizes of the certificate chain, ticket,
//! TMD and meta sections and a `u64` size of the content. The sections follow
//! the header in the order certificates, ticket, TMD, content, meta, each
//! starting on a 0x40-byte boundary, the header included. The meta section
//! holds the SMDH block at +0x400. Meta sizes of 0, 8 or 0x200 are the
//! documented dummies and carry no icon.
//!
//! A CIA has no magic, so the format is chosen by its `.cia` extension. A
//! file is accepted only when the sizes fit the file and the meta section
//! holds an SMDH block. Reverse engineered from samples: the
//! Universal-Updater `.cia` in `corpus/extra/nintendo-rom-icons`, whose icon
//! matches the one in its `.3dsx`.

use super::smdh;
use crate::bytes::{le16, le32};
use crate::{DecodeError, Image};

const HEADER_SIZE: u32 = 0x2020;
const ALIGNMENT: u64 = 0x40;
const SMDH_AT: usize = 0x400;

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    const FAIL: DecodeError = DecodeError::Invalid;
    let field = |at| le32(data, at).map(u64::from).ok_or(FAIL);
    if le32(data, 0) != Some(HEADER_SIZE) || le16(data, 4) != Some(0) || le16(data, 6) != Some(0) {
        return Err(FAIL);
    }
    let (certificates, ticket, tmd, meta) = (field(8)?, field(0xc)?, field(0x10)?, field(0x14)?);
    let content = data
        .get(0x18..0x20)
        .and_then(|bytes| bytes.try_into().ok())
        .map(u64::from_le_bytes)
        .ok_or(FAIL)?;
    // The meta section starts after the other sections, each rounded up to
    // the alignment.
    let aligned = |size: u64| size.div_ceil(ALIGNMENT).saturating_mul(ALIGNMENT);
    let start = [u64::from(HEADER_SIZE), certificates, ticket, tmd, content]
        .into_iter()
        .fold(0u64, |at, size| at.saturating_add(aligned(size)));
    let meta_at = usize::try_from(start).map_err(|_| FAIL)?;
    if meta < (SMDH_AT + smdh::LEN) as u64 {
        return Err(FAIL);
    }
    let meta_end = start.saturating_add(meta);
    if meta_end > data.len() as u64 {
        return Err(FAIL);
    }
    smdh::icon(data.get(meta_at + SMDH_AT..).ok_or(FAIL)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn archive(meta: u32) -> alloc::vec::Vec<u8> {
        let content_end = 0x2040 + 0x40 + 0x80 + 0x80 + 0x40;
        let mut file = alloc::vec![0; content_end];
        file[..4].copy_from_slice(&HEADER_SIZE.to_le_bytes());
        // 3 bytes of certificates, 0x41 of ticket, 0x80 of TMD, 0x10 of
        // content: sections of 0x40, 0x80, 0x80 and 0x40 bytes.
        file[8..12].copy_from_slice(&3u32.to_le_bytes());
        file[0xc..0x10].copy_from_slice(&0x41u32.to_le_bytes());
        file[0x10..0x14].copy_from_slice(&0x80u32.to_le_bytes());
        file[0x14..0x18].copy_from_slice(&meta.to_le_bytes());
        file[0x18..0x1c].copy_from_slice(&0x10u32.to_le_bytes());
        let mut block = alloc::vec![0; SMDH_AT];
        block.extend_from_slice(&smdh::test_block(&[(1, 0xf800)]));
        file.extend_from_slice(&block);
        file
    }

    #[test]
    fn the_meta_section_follows_the_aligned_sections() {
        let file = archive(0x3ac0);
        assert_eq!(decode(&file).unwrap().get(1, 0), 0xff0000);
        // A dummy meta section holds no icon.
        assert!(decode(&archive(0x200)).is_err());
        assert!(decode(&file[..file.len() - 1]).is_err());
        let mut typed = file;
        typed[4] = 1;
        assert!(decode(&typed).is_err());
    }
}
