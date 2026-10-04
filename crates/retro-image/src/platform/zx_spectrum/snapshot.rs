//! Z80 and SNA snapshots: the 6912-byte screen is cut out of RAM and drawn
//! like an `.scr`. Nothing else in the snapshot is looked at, apart from the
//! machine type and the 128K screen-select bit needed to find the screen.
//!
//! Sources:
//! - Z80 (v1, v2 and v3 headers, `ED ED` run-length coding, 16K page
//!   blocks, hardware table, page numbers): World of Spectrum Z80 format
//!   description, <https://worldofspectrum.org/faq/reference/z80format.htm>.
//! - SNA (27-byte header, 48K and 128K layouts): World of Spectrum snapshot
//!   formats, <https://worldofspectrum.org/faq/reference/formats.htm>.
//! - 128K screen select (bit 3 of port 0x7FFD picks bank 7 over bank 5; the
//!   Pentagon, Scorpion and +3 use the same bit): 128K hardware description,
//!   <https://sinclair.wiki.zxnet.co.uk/wiki/Spectrum_128K_hardware>.
//! - Survey and sample notes: `docs/research/next-zx-misc.md`. There is no
//!   RECOIL oracle for these formats; the output was checked against the
//!   specs and by eye on the samples in `corpus/extra/zx-snapshots`.
//!
//! Left out: Timex machines (Z80 hardware 14, 15, 128), SamRam and Didaktik,
//! whose screen mode is not part of what is read here. SZX is in `szx.rs`.

use alloc::vec::Vec;

use super::screen::SCR_LEN;
use super::standard::decode_scr;
use crate::bytes::le16;
use crate::{DecodeError, Image};

pub(super) const BANK_LEN: usize = 0x4000;
const Z80_HEADER_LEN: usize = 30;
const SNA_HEADER_LEN: usize = 27;
/// Screen select in the port 0x7FFD value.
pub(super) const SHADOW_SCREEN: u8 = 0x08;
/// Z80 page numbers of banks 5 and 7 on 128K machines; page 8 is also the
/// 0x4000 page of 48K machines.
const PAGE_BANK_5: u8 = 8;
const PAGE_BANK_7: u8 = 10;

/// Z80 snapshot, any of the three versions.
pub(super) fn decode_z80(data: &[u8]) -> Result<Image, DecodeError> {
    let header = data
        .get(..Z80_HEADER_LEN)
        .ok_or(DecodeError::Unrecognized)?;
    if le16(header, 6) != Some(0) {
        return decode_z80_v1(header, &data[Z80_HEADER_LEN..]);
    }
    let extension = usize::from(le16(data, 30).ok_or(DecodeError::Unrecognized)?);
    let version_3 = match extension {
        23 => false,
        54 | 55 => true,
        _ => return Err(DecodeError::Unrecognized),
    };
    let hardware = *data.get(34).ok_or(DecodeError::Unrecognized)?;
    let port_7ffd = *data.get(35).ok_or(DecodeError::Unrecognized)?;
    let page = if z80_has_banks(hardware, version_3)? && port_7ffd & SHADOW_SCREEN != 0 {
        PAGE_BANK_7
    } else {
        PAGE_BANK_5
    };
    let mut blocks = data
        .get(32 + extension..)
        .ok_or(DecodeError::Unrecognized)?;
    while let (Some(length), Some(&number)) = (le16(blocks, 0), blocks.get(2)) {
        let raw = length == 0xffff;
        let size = if raw { BANK_LEN } else { usize::from(length) };
        let body = blocks.get(3..3 + size).ok_or(DecodeError::Unrecognized)?;
        if number == page {
            return if raw {
                decode_scr(&body[..SCR_LEN])
            } else {
                decode_scr(&z80_unpack(body)?)
            };
        }
        blocks = &blocks[3 + size..];
    }
    Err(DecodeError::Unrecognized)
}

/// Whether the machine has the 128K paging port (so the screen select bit
/// applies). Machines whose screen mode the snapshot does not describe here
/// are rejected.
fn z80_has_banks(hardware: u8, version_3: bool) -> Result<bool, DecodeError> {
    match (version_3, hardware) {
        (false, 0 | 1) | (true, 0 | 1 | 3) => Ok(false),
        (false, 3 | 4) | (true, 4..=10 | 12 | 13) => Ok(true),
        _ => Err(DecodeError::Unrecognized),
    }
}

fn decode_z80_v1(header: &[u8], body: &[u8]) -> Result<Image, DecodeError> {
    // Byte 12 of 255 means 1 for old files.
    let compressed = header[12] != 255 && header[12] & 0x20 != 0;
    if compressed {
        decode_scr(&z80_unpack(body)?)
    } else if body.len() >= 3 * BANK_LEN {
        decode_scr(&body[..SCR_LEN])
    } else {
        Err(DecodeError::Unrecognized)
    }
}

/// Unpacks the first 6912 bytes of a Z80 compressed block: `ED ED count
/// value` is a run, any other byte (a lone `ED` included) stands for itself.
fn z80_unpack(packed: &[u8]) -> Result<Vec<u8>, DecodeError> {
    let mut out = Vec::with_capacity(SCR_LEN);
    let mut at = 0;
    while out.len() < SCR_LEN {
        let byte = *packed.get(at).ok_or(DecodeError::Unrecognized)?;
        if byte == 0xed && packed.get(at + 1) == Some(&0xed) {
            let count = *packed.get(at + 2).ok_or(DecodeError::Unrecognized)?;
            let value = *packed.get(at + 3).ok_or(DecodeError::Unrecognized)?;
            if count == 0 {
                return Err(DecodeError::Unrecognized);
            }
            out.resize((out.len() + usize::from(count)).min(SCR_LEN), value);
            at += 4;
        } else {
            out.push(byte);
            at += 1;
        }
    }
    Ok(out)
}

/// SNA snapshot, 48K or 128K.
pub(super) fn decode_sna(data: &[u8]) -> Result<Image, DecodeError> {
    let ram = data
        .get(SNA_HEADER_LEN..)
        .ok_or(DecodeError::Unrecognized)?;
    if ram.len() == 3 * BANK_LEN {
        return decode_scr(&ram[..SCR_LEN]);
    }
    // 128K: banks 5, 2 and the paged one, PC, port 0x7FFD, TR-DOS flag, then
    // every other bank in ascending order.
    let port_7ffd = *ram.get(3 * BANK_LEN + 2).ok_or(DecodeError::Unrecognized)?;
    let paged = port_7ffd & 7;
    let mut trailing = (0..8).filter(|&bank| !matches!(bank, 5 | 2) && bank != paged);
    if ram.len() != 3 * BANK_LEN + 4 + trailing.clone().count() * BANK_LEN {
        return Err(DecodeError::Unrecognized);
    }
    let start = if port_7ffd & SHADOW_SCREEN == 0 {
        0 // bank 5 comes first
    } else if paged == 7 {
        2 * BANK_LEN
    } else {
        let index = trailing.by_ref().take_while(|&bank| bank != 7).count();
        3 * BANK_LEN + 4 + index * BANK_LEN
    };
    decode_scr(&ram[start..start + SCR_LEN])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unpack_expands_runs_and_keeps_lone_ed() {
        let mut data = alloc::vec![0xed, 0xed, 0x03, 0x07, 0xed, 0x01, 0x05];
        data.extend(core::iter::repeat_n(0x11, SCR_LEN));
        let out = z80_unpack(&data).unwrap();
        assert_eq!(&out[..6], &[7, 7, 7, 0xed, 0x01, 0x05]);
        assert_eq!(out.len(), SCR_LEN);
    }

    #[test]
    fn sna_128k_finds_shadow_screen_in_trailing_banks() {
        let paged_at = SNA_HEADER_LEN + 3 * BANK_LEN + 2;
        let mut data = alloc::vec![0u8; paged_at + 2 + 5 * BANK_LEN];
        data[paged_at] = SHADOW_SCREEN; // bank 0 paged, shadow screen shown
        // Trailing banks are 1, 3, 4, 6, 7: bank 7 is the fifth.
        let bank_7 = paged_at + 2 + 4 * BANK_LEN;
        data[bank_7 + 6144] = 0x38; // white paper in the first cell
        let image = decode_sna(&data).unwrap();
        assert_eq!(&image.rgb()[..3], &[0xcd; 3], "bank 7 was shown");
        assert!(decode_sna(&data[..data.len() - 1]).is_err());
    }
}
