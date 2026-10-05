//! SZX (zx-state) snapshots: the 6912-byte screen is cut out of the RAM
//! page that holds it and drawn like an `.scr`.
//!
//! Sources:
//! - Header, machine identifiers, block layout (`id`, `size`, data; unknown
//!   blocks are skipped), `SPCR` (port 0x7FFD value), `RAMP` (flags bit 0 =
//!   zlib, page number, data) and `SCLD` (port 0xFF value): the zx-state
//!   File Format documentation by Jonathan Needle,
//!   <https://www.spectaculator.com/docs/zx-state/intro.shtml>. RAM pages
//!   are banks (0-7), not Z80 snapshot page numbers.
//! - 128K screen select (bit 3 of port 0x7FFD picks bank 7 over bank 5):
//!   <https://sinclair.wiki.zxnet.co.uk/wiki/Spectrum_128K_hardware>.
//! - Timex port 0xFF screen modes (bits 0-2): WoS Timex technical reference,
//!   <https://worldofspectrum.org/faq/reference/tmxreference.htm>.
//! - ULAplus `PLTT` block: <https://sinclair.wiki.zxnet.co.uk/wiki/ZX-State_format>.
//! - Survey and sample notes: `docs/research/next-zx-misc.md`. There is no
//!   RECOIL oracle for this format; the output was checked against the
//!   other snapshot formats and by eye on the samples in
//!   `corpus/extra/zx-snapshots/szx`. Zlib is `codec::inflate`.
//!
//! Left out, as `Unrecognized`: snapshots with a Timex screen mode other
//! than the standard one (the sample set has none, so the page layout is
//! untested), with ULAplus enabled, and of machine 11 (Spectrum SE) or
//! identifiers after 16.

use super::screen::SCR_LEN;
use super::snapshot::{BANK_LEN, SHADOW_SCREEN};
use super::standard::decode_scr;
use crate::bytes::{le16, le32};
use crate::codec::inflate;
use crate::{DecodeError, Image};

const HEADER_LEN: usize = 8;
const BLOCK_HEADER_LEN: usize = 8;
const RAM_COMPRESSED: u16 = 1;
const NORMAL_SCREEN_BANK: u8 = 5;
const SHADOW_SCREEN_BANK: u8 = 7;
/// Port 0xFF bits 0-2 are the Timex screen mode; 0 is the normal one.
const TIMEX_MODE_MASK: u8 = 7;
/// First flag bit of a `PLTT` block: ULAplus enabled.
const ULAPLUS_ENABLED: u8 = 1;

pub(super) fn decode_szx(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let header = data.get(..HEADER_LEN).ok_or(fail)?;
    if &header[..4] != b"ZXST" {
        return Err(fail);
    }
    let paged = has_paging_port(header[6]).ok_or(fail)?;
    let mut port_7ffd = 0;
    let mut ram: [Option<&[u8]>; 8] = [None; 8];
    let mut blocks = &data[HEADER_LEN..];
    while !blocks.is_empty() {
        let size = le32(blocks, 4).ok_or(fail)? as usize;
        let body = blocks
            .get(BLOCK_HEADER_LEN..)
            .and_then(|rest| rest.get(..size))
            .ok_or(fail)?;
        match &blocks[..4] {
            b"SPCR" => port_7ffd = *body.get(1).ok_or(fail)?,
            b"SCLD" if body.get(1).ok_or(fail)? & TIMEX_MODE_MASK != 0 => return Err(fail),
            b"PLTT" if body.first().ok_or(fail)? & ULAPLUS_ENABLED != 0 => return Err(fail),
            b"RAMP" => {
                let page = usize::from(*body.get(2).ok_or(fail)?);
                // Pages past 7 belong to bigger machines; they are not read.
                if let Some(slot) = ram.get_mut(page) {
                    *slot = Some(body);
                }
            }
            _ => {}
        }
        blocks = &blocks[BLOCK_HEADER_LEN + size..];
    }
    let bank = if paged && port_7ffd & SHADOW_SCREEN != 0 {
        SHADOW_SCREEN_BANK
    } else {
        NORMAL_SCREEN_BANK
    };
    let block = ram[usize::from(bank)].ok_or(fail)?;
    let page = &block[3..];
    if le16(block, 0).ok_or(fail)? & RAM_COMPRESSED != 0 {
        let page = inflate::zlib(page, BANK_LEN)
            .filter(|page| page.len() == BANK_LEN)
            .ok_or(fail)?;
        decode_scr(page.get(..SCR_LEN).ok_or(fail)?)
    } else {
        decode_scr(page.get(..SCR_LEN).ok_or(fail)?)
    }
}

/// Whether the machine has the 128K paging port, so that port 0x7FFD picks
/// the screen. `None` for machines this decoder doesn't draw.
fn has_paging_port(machine: u8) -> Option<bool> {
    match machine {
        // 16K, 48K, TC2048, TC2068, TS2068, NTSC 48K.
        0 | 1 | 8 | 9 | 12 | 15 => Some(false),
        // 128K, +2, +2A, +3, +3e, Pentagon 128, Scorpion, Pentagon 512 and
        // 1024, 128Ke.
        2..=7 | 10 | 13 | 14 | 16 => Some(true),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;
    use alloc::vec::Vec;

    use super::*;

    /// `ZXST` 1.4 for `machine`, then the blocks.
    fn file(machine: u8, blocks: &[(&[u8; 4], Vec<u8>)]) -> Vec<u8> {
        let mut data = b"ZXST\x01\x04".to_vec();
        data.extend_from_slice(&[machine, 0]);
        for (id, body) in blocks {
            data.extend_from_slice(*id);
            data.extend_from_slice(&(body.len() as u32).to_le_bytes());
            data.extend_from_slice(body);
        }
        data
    }

    /// A bank whose first pixel is white ink on black paper.
    fn bank(lit: bool) -> Vec<u8> {
        let mut bank = vec![0; BANK_LEN];
        if lit {
            bank[0] = 0x80;
            bank[6144] = 0x07;
        }
        bank
    }

    fn raw_ramp(page: u8, bank: &[u8]) -> Vec<u8> {
        let mut body = vec![0, 0, page];
        body.extend_from_slice(bank);
        body
    }

    /// A RAMP body holding `bank` as a zlib stream of one stored block.
    fn zlib_ramp(page: u8, bank: &[u8]) -> Vec<u8> {
        let (mut a, mut b) = (1u32, 0u32);
        for &byte in bank {
            a = (a + u32::from(byte)) % 65521;
            b = (b + a) % 65521;
        }
        let mut body = vec![1, 0, page, 0x78, 0x01, 0x01];
        body.extend_from_slice(&(bank.len() as u16).to_le_bytes());
        body.extend_from_slice(&(!(bank.len() as u16)).to_le_bytes());
        body.extend_from_slice(bank);
        body.extend_from_slice(&(b << 16 | a).to_be_bytes());
        body
    }

    fn spcr(port: u8) -> Vec<u8> {
        vec![0, port, 0, 0, 0, 0, 0, 0]
    }

    fn first_pixel(data: &[u8]) -> Vec<u8> {
        decode_szx(data).unwrap().rgb()[..3].to_vec()
    }

    #[test]
    fn shadow_screen_applies_to_128k_machines_only() {
        let blocks = |port| {
            [
                (b"SPCR", spcr(port)),
                (b"RAMP", raw_ramp(5, &bank(false))),
                (b"RAMP", zlib_ramp(7, &bank(true))),
            ]
        };
        assert_eq!(first_pixel(&file(2, &blocks(0x00))), [0, 0, 0]);
        assert_eq!(first_pixel(&file(2, &blocks(0x08))), [0xcd; 3]);
        // A 48K machine ignores the bit (and has no bank 7 anyway).
        assert_eq!(first_pixel(&file(1, &blocks(0x08))), [0, 0, 0]);
    }

    #[test]
    fn compressed_pages_are_inflated_and_checked() {
        let mut blocks = vec![(b"RAMP", zlib_ramp(5, &bank(true)))];
        assert_eq!(first_pixel(&file(1, &blocks)), [0xcd; 3]);
        // A page that inflates to less than 16K is rejected.
        blocks[0].1 = zlib_ramp(5, &bank(true)[..BANK_LEN - 1]);
        assert!(decode_szx(&file(1, &blocks)).is_err());
    }

    #[test]
    fn rejects_unsupported_states_and_bad_blocks() {
        let ram = (b"RAMP", raw_ramp(5, &bank(true)));
        let ok = file(1, &[(b"JOY\0", vec![0; 6]), ram.clone()]);
        assert!(decode_szx(&ok).is_ok());
        // Timex hi-color, ULAplus enabled, unknown machine, no screen page.
        let scld = (b"SCLD", vec![0, 2]);
        assert!(decode_szx(&file(8, &[scld, ram.clone()])).is_err());
        let pltt = (b"PLTT", vec![1; 66]);
        assert!(decode_szx(&file(1, &[pltt, ram.clone()])).is_err());
        assert!(decode_szx(&file(11, core::slice::from_ref(&ram))).is_err());
        assert!(decode_szx(&file(1, &[(b"RAMP", raw_ramp(2, &bank(true)))])).is_err());
        // A block running past the end of the file.
        assert!(decode_szx(&ok[..ok.len() - 1]).is_err());
        assert!(decode_szx(&ok[..HEADER_LEN + 3]).is_err());
    }
}
