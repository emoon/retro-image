//! Nintendo 3DS homebrew executable (`.3dsx`): the icon in its extended
//! header.
//!
//! Sources: GBATEK, "3DS Files - Title Homebrew Executables (3DSX)"
//! (<https://problemkaputt.de/gbatek.htm>, no license stated, so facts only):
//! `3DSX` at 0, a header size at 4 that is 0x20 or 0x2C, and when it is
//! larger than 0x20 the extended header holds the offset (0x20) and size
//! (0x24, 0x36C0) of an SMDH block. Reverse engineered from samples: the
//! Universal-Updater `.3dsx` in `corpus/extra/nintendo-rom-icons` has its SMDH
//! at 0x26D598, past the code and relocation tables.

use super::smdh;
use crate::bytes::{le16, le32};
use crate::{DecodeError, Image};

const MAGIC: &[u8] = b"3DSX";
const HEADER_SIZE_AT: usize = 4;
/// Header size of a file with the extended header.
const EXTENDED_HEADER_SIZE: u16 = 0x2c;
const SMDH_OFFSET_AT: usize = 0x20;
const SMDH_SIZE_AT: usize = 0x24;

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    if !data.starts_with(MAGIC) || le16(data, HEADER_SIZE_AT) < Some(EXTENDED_HEADER_SIZE) {
        return Err(fail);
    }
    let offset = le32(data, SMDH_OFFSET_AT).ok_or(fail)? as usize;
    if le32(data, SMDH_SIZE_AT) != Some(smdh::LEN as u32) {
        return Err(fail);
    }
    smdh::icon(data.get(offset..).ok_or(fail)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_icon_is_found_through_the_extended_header() {
        let mut file = alloc::vec![0; 0x30];
        file[..4].copy_from_slice(MAGIC);
        file[4] = 0x2c;
        file[SMDH_OFFSET_AT..SMDH_OFFSET_AT + 4].copy_from_slice(&0x30u32.to_le_bytes());
        file[SMDH_SIZE_AT..SMDH_SIZE_AT + 4].copy_from_slice(&0x36c0u32.to_le_bytes());
        file.extend_from_slice(&smdh::test_block(&[(1, 0xf800)]));
        assert_eq!(decode(&file).unwrap().get(1, 0), 0xff0000);
        // Without the extended header there is no icon.
        file[4] = 0x20;
        assert!(decode(&file).is_err());
        file[4] = 0x2c;
        assert!(decode(&file[..file.len() - 1]).is_err());
    }
}
