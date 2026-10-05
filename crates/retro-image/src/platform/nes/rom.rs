//! NES ROM images (`.nes` iNES and NES 2.0, `.unf`/`.unif` UNIF): the
//! CHR-ROM drawn as a sheet of tiles.
//!
//! Sources: the nesdev wiki pages "INES" (<https://www.nesdev.org/wiki/INES>),
//! "NES 2.0" (<https://www.nesdev.org/wiki/NES_2.0>) and "UNIF"
//! (<https://www.nesdev.org/wiki/UNIF>), no license shown, facts only; the
//! tile encoding is the one of [`super::chr`].
//! - iNES: a 16-byte header, `NES` and `0x1A`, then the PRG-ROM size in 16 KiB
//!   units at byte 4 and the CHR-ROM size in 8 KiB units at byte 5 (0 means
//!   the board has CHR-RAM and the file holds no tiles). Bit 2 of byte 6 says
//!   a 512-byte trainer follows the header. The file continues with the
//!   PRG-ROM, then the CHR-ROM.
//! - NES 2.0: byte 7 has `(b7 & 0x0C) == 0x08`, and the size fields grow by
//!   the nibbles of byte 9 (PRG in the low one, CHR in the high one). When
//!   the high nibble of a size is `0xF` the size is not in units but
//!   `2^E * (MM * 2 + 1)` bytes, with `E` in bits 2-7 and `MM` in bits 0-1 of
//!   the low byte. The nesdev procedure accepts the NES 2.0 reading only if
//!   its sizes fit the file; otherwise the header is read as iNES.
//! - UNIF: `UNIF`, a 32-byte header, then chunks of a 4-byte id, a
//!   little-endian `u32` length and the data. The CHR-ROMs are the chunks
//!   `CHR0` to `CHRF`; they are joined in that order.
//!
//! Anything after the CHR-ROM (PlayChoice data, miscellaneous ROM) is
//! ignored. A ROM without CHR-ROM is rejected, as is one whose file is shorter
//! than its header says.
//!
//! Checked against the 263 test ROMs of `christopherpow/nes-test-roms` in
//! `corpus/extra/nintendo-rom-icons` (iNES and five NES 2.0 headers). No
//! UNIF sample was available, so that part follows the wiki page alone.

use alloc::vec::Vec;

use super::chr;
use crate::bytes::le32;
use crate::{DecodeError, Image};

const INES_MAGIC: &[u8] = b"NES\x1a";
const UNIF_MAGIC: &[u8] = b"UNIF";
const INES_HEADER_LEN: usize = 16;
const TRAINER_LEN: usize = 512;
const PRG_UNIT: usize = 0x4000;
const CHR_UNIT: usize = 0x2000;
const UNIF_HEADER_LEN: usize = 32;

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    if data.starts_with(INES_MAGIC) {
        chr::sheet(ines_chr(data)?)
    } else if data.starts_with(UNIF_MAGIC) {
        chr::sheet(&unif_chr(data)?)
    } else {
        Err(DecodeError::Unrecognized)
    }
}

/// The CHR-ROM of an iNES or NES 2.0 file.
fn ines_chr(data: &[u8]) -> Result<&[u8], DecodeError> {
    let fail = DecodeError::Unrecognized;
    let header = data.get(..INES_HEADER_LEN).ok_or(fail)?;
    let trainer = if header[6] & 4 != 0 { TRAINER_LEN } else { 0 };
    let body = (data.len() - INES_HEADER_LEN).saturating_sub(trainer);
    let (prg, chr) = rom_sizes(header, body);
    let start = INES_HEADER_LEN + trainer + prg;
    let chr = data.get(start..start.checked_add(chr).ok_or(fail)?);
    chr.filter(|chr| !chr.is_empty()).ok_or(fail)
}

/// The PRG-ROM and CHR-ROM sizes in bytes: the NES 2.0 reading if the header
/// says NES 2.0 and the sizes fit the `body` that follows the header and
/// trainer, the iNES one otherwise.
fn rom_sizes(header: &[u8], body: usize) -> (usize, usize) {
    if header[7] & 0x0c == 0x08 {
        let prg = rom_size(header[4], header[9] & 15, PRG_UNIT);
        let chr = rom_size(header[5], header[9] >> 4, CHR_UNIT);
        if let (Some(prg), Some(chr)) = (prg, chr)
            && prg.saturating_add(chr) <= body
        {
            return (prg, chr);
        }
    }
    (
        usize::from(header[4]) * PRG_UNIT,
        usize::from(header[5]) * CHR_UNIT,
    )
}

/// A NES 2.0 ROM size in bytes from its low byte, its high nibble and the
/// size of one unit.
fn rom_size(low: u8, high: u8, unit: usize) -> Option<usize> {
    if high != 0xf {
        return Some((usize::from(high) << 8 | usize::from(low)) * unit);
    }
    let multiplier = usize::from(low & 3) * 2 + 1;
    1usize
        .checked_shl(u32::from(low >> 2))?
        .checked_mul(multiplier)
}

/// The CHR-ROM chunks of a UNIF file, joined in the order of their numbers.
fn unif_chr(data: &[u8]) -> Result<Vec<u8>, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let mut banks: [&[u8]; 16] = [&[]; 16];
    let mut rest = data.get(UNIF_HEADER_LEN..).ok_or(fail)?;
    while !rest.is_empty() {
        let id = rest.get(..4).ok_or(fail)?;
        let len = le32(rest, 4).ok_or(fail)? as usize;
        let end = 8usize.checked_add(len).ok_or(fail)?;
        let chunk = rest.get(8..end).ok_or(fail)?;
        if let Some(bank) = id
            .strip_prefix(b"CHR")
            .and_then(|n| char::from(n[0]).to_digit(16))
            && banks[bank as usize].is_empty()
        {
            banks[bank as usize] = chunk;
        }
        rest = &rest[end..];
    }
    let joined = banks.concat();
    if joined.is_empty() {
        return Err(fail);
    }
    Ok(joined)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tile whose first pixel has color 1, which no other tile has.
    fn tile(marker: u8) -> [u8; 16] {
        let mut tile = [0; 16];
        tile[0] = marker;
        tile
    }

    fn ines(header: [u8; 16], rest: &[u8]) -> Vec<u8> {
        let mut file = header.to_vec();
        file.extend_from_slice(rest);
        file
    }

    fn header(prg: u8, chr: u8, flags6: u8, flags7: u8) -> [u8; 16] {
        let mut header = [0; 16];
        header[..4].copy_from_slice(INES_MAGIC);
        (header[4], header[5], header[6], header[7]) = (prg, chr, flags6, flags7);
        header
    }

    #[test]
    fn ines_chr_follows_the_trainer_and_prg() {
        let mut rest = alloc::vec![0xaa; TRAINER_LEN + PRG_UNIT];
        rest.extend_from_slice(&tile(0x80));
        rest.resize(TRAINER_LEN + PRG_UNIT + CHR_UNIT, 0);
        let file = ines(header(1, 1, 4, 0), &rest);
        let image = decode(&file).unwrap();
        // 512 tiles of an 8 KiB bank, 16 to a row.
        assert_eq!((image.width(), image.height()), (128, 256));
        assert_eq!(image.get(0, 0), 0x555555);
        // Without the trainer flag the CHR starts 512 bytes early, in the
        // 0xAA filler, whose first pixel has color 3.
        let shifted = decode(&ines(header(1, 1, 0, 0), &rest)).unwrap();
        assert_eq!(shifted.get(0, 0), 0xffffff);
    }

    #[test]
    fn a_rom_without_chr_rom_or_with_a_short_file_is_rejected() {
        assert!(decode(&ines(header(1, 0, 0, 0), &[0; PRG_UNIT])).is_err());
        assert!(decode(&ines(header(1, 1, 0, 0), &[0; PRG_UNIT + CHR_UNIT - 1])).is_err());
        assert!(decode(&INES_MAGIC[..3]).is_err());
    }

    #[test]
    fn nes2_sizes_use_the_high_nibbles_and_the_exponent_form() {
        // Plain: the nibble 1 in byte 9 adds 256 units.
        assert_eq!(rom_size(2, 1, CHR_UNIT), Some(258 * CHR_UNIT));
        // Exponent form: E = 14, MM = 1 gives 2^14 * 3 bytes.
        assert_eq!(rom_size(14 << 2 | 1, 0xf, CHR_UNIT), Some(3 << 14));
        assert_eq!(rom_size(63 << 2 | 3, 0xf, CHR_UNIT), None);
        // A NES 2.0 CHR of 0x4000 bytes in exponent form: E = 14, MM = 0.
        let mut head = header(0, 0, 0, 0x08);
        head[5] = 14 << 2;
        head[9] = 0xf0;
        let mut rest = alloc::vec![0; 0x4000];
        rest[0] = 0x80;
        assert_eq!(decode(&ines(head, &rest)).unwrap().get(0, 0), 0x555555);
    }

    #[test]
    fn a_nes2_header_that_does_not_fit_the_file_is_read_as_ines() {
        // Byte 9 would make the CHR 256 units larger.
        let mut head = header(0, 1, 0, 0x08);
        head[9] = 0x10;
        let mut rest = alloc::vec![0; CHR_UNIT];
        rest[0] = 0x80;
        assert_eq!(decode(&ines(head, &rest)).unwrap().get(0, 0), 0x555555);
    }

    fn chunk(id: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut chunk = id.to_vec();
        chunk.extend_from_slice(&(data.len() as u32).to_le_bytes());
        chunk.extend_from_slice(data);
        chunk
    }

    #[test]
    fn unif_joins_the_chr_chunks_in_number_order() {
        let mut file = UNIF_MAGIC.to_vec();
        file.resize(UNIF_HEADER_LEN, 0);
        file.extend(chunk(b"MAPR", b"NROM\0"));
        file.extend(chunk(b"CHR1", &tile(0x40)));
        file.extend(chunk(b"PRG0", &[0; 32]));
        file.extend(chunk(b"CHR0", &tile(0x80)));
        let image = decode(&file).unwrap();
        assert_eq!((image.width(), image.height()), (128, 8));
        assert_eq!(image.get(0, 0), 0x555555);
        assert_eq!(image.get(9, 0), 0x555555);
        assert_eq!(image.get(8, 0), 0);
        // A chunk that runs past the end, and a file without CHR.
        assert!(decode(&file[..file.len() - 1]).is_err());
        let mut no_chr = UNIF_MAGIC.to_vec();
        no_chr.resize(UNIF_HEADER_LEN, 0);
        no_chr.extend(chunk(b"PRG0", &[0; 32]));
        assert!(decode(&no_chr).is_err());
    }
}
