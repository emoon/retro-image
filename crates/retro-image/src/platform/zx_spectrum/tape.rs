//! TAP and TZX tapes: the loading screen is the first standard-speed data
//! block holding a 6912-byte screen (flag 0xFF, 6912 bytes, checksum).
//! Tapes without such a block (packed or custom-loader screens, pulse-level
//! data) are rejected.
//!
//! Sources:
//! - TAP (blocks of `u16 length`, flag, data, checksum; the 19-byte header
//!   block with type, name, length and start address): World of Spectrum
//!   snapshot and tape formats, <https://worldofspectrum.org/faq/reference/formats.htm>.
//! - TZX (`ZXTape!` 0x1A header, block ids and their length fields):
//!   TZX format 1.20, <https://worldofspectrum.org/TZXformat.html>.
//! - Survey and sample notes: `docs/research/next-zx-misc.md`. There is no
//!   RECOIL oracle; checked against the specs and by eye on the samples in
//!   `corpus/extra/zx-snapshots`.

use super::screen::{BITMAP_LEN, SCR_LEN};
use super::standard::decode_scr;
use crate::bytes::{le16, le32};
use crate::{DecodeError, Image};

const TZX_SIGNATURE: &[u8] = b"ZXTape!\x1a";
const TZX_HEADER_LEN: usize = 10;
/// Flag byte, screen, checksum.
const SCREEN_BLOCK_LEN: usize = SCR_LEN + 2;
/// Spectrum header block: flag 0, type, name, length, start, parameter, checksum.
const HEADER_BLOCK_LEN: usize = 19;
const CODE: u8 = 3;
const SCREEN_ADDRESS: u16 = 0x4000;
const ATTRIBUTE_RUN_PERCENT: usize = 80;

/// TAP tape.
pub(super) fn decode_tap(data: &[u8]) -> Result<Image, DecodeError> {
    let mut rest = data;
    find_screen(core::iter::from_fn(|| {
        let length = usize::from(le16(rest, 0)?);
        let block = rest.get(2..2 + length)?;
        rest = &rest[2 + length..];
        Some(block)
    }))
}

/// TZX tape.
pub(super) fn decode_tzx(data: &[u8]) -> Result<Image, DecodeError> {
    if !data.starts_with(TZX_SIGNATURE) {
        return Err(DecodeError::Invalid);
    }
    let mut rest = &data[TZX_HEADER_LEN.min(data.len())..];
    // Blocks without a standard-speed payload are skipped, not ends of tape.
    let blocks = core::iter::from_fn(|| {
        let (&id, body) = rest.split_first()?;
        let size = tzx_body_len(id, body)?;
        let body = body.get(..size)?;
        rest = &rest[1 + size..];
        Some(tzx_standard_data(id, body).unwrap_or(&[]))
    });
    find_screen(blocks)
}

/// The payload (flag, data, checksum) of the standard-speed data blocks
/// 0x10 (normal), 0x11 (turbo) and 0x14 (pure data).
fn tzx_standard_data(id: u8, body: &[u8]) -> Option<&[u8]> {
    match id {
        0x10 => body.get(4..),
        0x11 => body.get(18..),
        0x14 => body.get(10..),
        _ => None,
    }
}

/// Length of a block's body (everything after the id byte), `None` if the
/// length field is cut off. Unknown ids follow the format's rule of a `u32`
/// length first.
fn tzx_body_len(id: u8, body: &[u8]) -> Option<usize> {
    let u8_at = |at| body.get(at).map(|&v| usize::from(v));
    let u16_at = |at| le16(body, at).map(usize::from);
    let u24_at = |at| Some(u16_at(at)? | usize::from(*body.get(at + 2)?) << 16);
    let u32_at = |at| usize::try_from(le32(body, at)?).ok();
    match id {
        0x10 => Some(4 + u16_at(2)?),
        0x11 => Some(18 + u24_at(15)?),
        0x12 | 0x2a => Some(4),
        0x13 => Some(1 + 2 * u8_at(0)?),
        0x14 => Some(10 + u24_at(7)?),
        0x15 => Some(8 + u24_at(5)?),
        0x20 | 0x23 | 0x24 => Some(2),
        0x21 | 0x30 => Some(1 + u8_at(0)?),
        0x22 | 0x25 | 0x27 => Some(0),
        0x26 => Some(2 + 2 * u16_at(0)?),
        0x28 | 0x32 => Some(2 + u16_at(0)?),
        0x2b => Some(5),
        0x31 => Some(2 + u8_at(1)?),
        0x33 => Some(1 + 3 * u8_at(0)?),
        0x34 => Some(8),
        0x35 => Some(14 + u32_at(10)?),
        0x40 => Some(4 + u24_at(1)?),
        0x5a => Some(9),
        _ => Some(4 + u32_at(0)?),
    }
}

/// The first screen among the data blocks, in tape order. A block directly
/// after a header block counts if that header announces CODE of 6912 bytes
/// at the screen address, or elsewhere when the bytes look like a screen
/// (see [`looks_like_screen`]): tapes load pictures at a scratch address to
/// copy them to the display later, but 6912-byte blocks that are not pictures
/// exist too.
fn find_screen<'a>(blocks: impl Iterator<Item = &'a [u8]>) -> Result<Image, DecodeError> {
    let mut previous: &[u8] = &[];
    for block in blocks {
        let is_screen = block.len() == SCREEN_BLOCK_LEN
            && block[0] == 0xff
            && follows_header_of_screen(previous, &block[1..=SCR_LEN]);
        if is_screen {
            return decode_scr(&block[1..=SCR_LEN]);
        }
        previous = block;
    }
    Err(DecodeError::Invalid)
}

/// Whether the 6912-byte `screen` may be taken as one, given the block before
/// it: always without a header, never after a header announcing other data.
fn follows_header_of_screen(previous: &[u8], screen: &[u8]) -> bool {
    if previous.len() != HEADER_BLOCK_LEN {
        return true;
    }
    let is_screen_code =
        previous[0] == 0 && previous[1] == CODE && le16(previous, 12) == Some(SCR_LEN as u16);
    is_screen_code && (le16(previous, 14) == Some(SCREEN_ADDRESS) || looks_like_screen(screen))
}

/// Attribute runs: in a picture most attribute bytes repeat their left
/// neighbour (0.96 in the one sample picture loaded off-screen), in other
/// data they do not (0.64 in the one sample that is not a picture). Reverse
/// engineered from those two `corpus/extra/zx-snapshots/tap` files, so the
/// threshold is a judgement call.
fn looks_like_screen(screen: &[u8]) -> bool {
    let attributes = &screen[BITMAP_LEN..];
    let repeats = attributes.windows(2).filter(|w| w[0] == w[1]).count();
    repeats * 100 >= (attributes.len() - 1) * ATTRIBUTE_RUN_PERCENT
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::zx_spectrum::screen::ATTRIBUTES_LEN;

    fn screen_block() -> alloc::vec::Vec<u8> {
        let mut block = alloc::vec![0u8; SCREEN_BLOCK_LEN];
        block[0] = 0xff;
        block
    }

    #[test]
    fn tap_screen_after_header_needs_a_matching_header() {
        let mut header = alloc::vec![0u8; HEADER_BLOCK_LEN];
        header[1] = CODE;
        header[12..14].copy_from_slice(&6912u16.to_le_bytes());
        header[14..16].copy_from_slice(&0x4000u16.to_le_bytes());
        let mut tap = alloc::vec![];
        for block in [header, screen_block()] {
            tap.extend((block.len() as u16).to_le_bytes());
            tap.extend(block);
        }
        assert!(decode_tap(&tap).is_ok());
        // Loaded elsewhere: kept only while the bytes look like a picture.
        tap[2 + 15] = 0x80;
        assert!(decode_tap(&tap).is_ok());
        let attributes = tap.len() - 1 - ATTRIBUTES_LEN;
        for (i, byte) in tap[attributes..attributes + ATTRIBUTES_LEN]
            .iter_mut()
            .enumerate()
        {
            *byte = i as u8 & 1;
        }
        assert!(decode_tap(&tap).is_err());
    }

    #[test]
    fn tzx_skips_other_blocks_to_a_turbo_screen() {
        let mut tzx = TZX_SIGNATURE.to_vec();
        tzx.extend([1, 20]);
        tzx.extend([0x30, 3, b'a', b'b', b'c']); // text description
        tzx.extend([0x20, 0, 0]); // pause
        let block = screen_block();
        tzx.push(0x11);
        tzx.extend([0; 15]);
        tzx.extend(&(block.len() as u32).to_le_bytes()[..3]);
        tzx.extend(block);
        assert!(decode_tzx(&tzx).is_ok());
        assert!(decode_tzx(&tzx[..tzx.len() - 1]).is_err());
    }
}
