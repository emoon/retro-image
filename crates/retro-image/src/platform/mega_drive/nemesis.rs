//! Nemesis decompression: the run-length and prefix code compressor for
//! Mega Drive tile data.
//!
//! Source: the "Format" section of the Sega Retro article "Nemesis
//! compression" (<https://segaretro.org/Nemesis_compression>, also at
//! <https://info.sonicretro.org/Nemesis_compression>), licensed CC BY 4.0,
//! used for its prose. The article also prints a 68000 routine taken from a
//! Sonic 3 & Knuckles disassembly; that listing was not translated.
//! Checked against the 156 `.nem` files of the Sonic the Hedgehog
//! disassembly's `artnem/` directory (<https://github.com/sonicretro/s1disasm>,
//! used as sample data only, see `corpus/extra/small-consoles/mega-drive`):
//! every file decodes to exactly its declared tile count, and every
//! stream ends within one byte of the end of its file.
//!
//! Layout, from that prose:
//! - A big-endian word gives the number of 8 x 8 tiles; bit 15 selects XOR
//!   mode. The data is the nibbles of those tiles, a tile being 64 nibbles
//!   (32 bytes), 8 to a row.
//! - A code table follows. A byte with bit 7 set sets the nibble value to
//!   its low 4 bits. Any other byte holds a run length minus one in bits 4
//!   to 6 and a code length in bits 0 to 3, and the next byte is the code
//!   itself, in its low bits. `0xff` ends the table.
//! - The rest is a bit stream, most significant bit first. A code in the
//!   table stands for a run of its nibble. The six bits `111111` instead
//!   introduce 7 bits of inline data: run length minus one in the top 3 and
//!   the nibble in the low 4.
//! - In XOR mode every decoded row is XORed with the row before it (the
//!   first with zero) and that is the row's value.
//!
//! A run may cross a row boundary, and a run that goes past the declared
//! number of tiles is cut off.

use alloc::vec::Vec;

use crate::bytes::be16;

/// The longest code. A table with one slot per possible byte of stream
/// serves every code, since a code of `n` bits fills `2^(8 - n)` slots.
const MAX_CODE_LEN: usize = 8;
/// Bytes of one tile and pixels in one row.
const TILE_LEN: usize = 32;
const ROW_PIXELS: usize = 8;
/// Slots from here up start with the six bits of the inline marker.
const INLINE: usize = 0xfc;

/// What a code stands for.
#[derive(Clone, Copy)]
struct Run {
    /// Bits of the code.
    code_len: usize,
    nibble: u8,
    count: usize,
}

/// Reads the code table at `at`; returns it and where the bit stream starts.
/// `None` if the table is cut off, two codes overlap (a code must not be the
/// start of another), or a code collides with the inline marker.
fn code_table(data: &[u8], mut at: usize) -> Option<([Option<Run>; 256], usize)> {
    let mut table = [None; 256];
    let mut nibble = None;
    loop {
        let byte = *data.get(at)?;
        at += 1;
        if byte == 0xff {
            return Some((table, at));
        }
        if byte & 0x80 != 0 {
            nibble = Some(byte & 15);
            continue;
        }
        let code_len = usize::from(byte & 15);
        let code = *data.get(at)?;
        at += 1;
        if !(1..=MAX_CODE_LEN).contains(&code_len) || usize::from(code) >> code_len != 0 {
            return None;
        }
        let run = Run {
            code_len,
            nibble: nibble?,
            count: usize::from(byte >> 4 & 7) + 1,
        };
        let first = usize::from(code) << (MAX_CODE_LEN - code_len);
        let slots = table.get_mut(first..first + (1 << (MAX_CODE_LEN - code_len)))?;
        if first + slots.len() > INLINE || slots.iter().any(Option::is_some) {
            return None;
        }
        slots.fill(Some(run));
    }
}

/// The bit stream, most significant bit first.
struct Bits<'a> {
    data: &'a [u8],
    /// Bits consumed.
    at: usize,
}

impl Bits<'_> {
    /// The next 8 bits, zeros past the end.
    fn peek_byte(&self) -> usize {
        let byte = |i| u16::from(self.data.get(self.at / 8 + i).copied().unwrap_or(0));
        usize::from((byte(0) << 8 | byte(1)) >> (8 - self.at % 8) & 0xff)
    }

    /// Consumes `n` bits (at most 8); `None` if the data holds fewer.
    fn skip(&mut self, n: usize) -> Option<()> {
        self.at += n;
        (self.at <= self.data.len() * 8).then_some(())
    }
}

/// The tiles of a Nemesis stream, `32 * tile count` bytes, or `None` if it
/// is not a well-formed stream that holds that many tiles.
pub(super) fn decompress(data: &[u8]) -> Option<Vec<u8>> {
    let header = be16(data, 0)?;
    let xor = header & 0x8000 != 0;
    let rows = usize::from(header & 0x7fff) * TILE_LEN / 4;
    let (table, stream_at) = code_table(data, 2)?;
    let mut bits = Bits {
        data: &data[stream_at..],
        at: 0,
    };
    let mut out = Vec::new();
    let (mut row, mut filled, mut previous) = (0u32, 0, 0u32);
    while out.len() < rows * 4 {
        let (nibble, count) = match bits.peek_byte() {
            marker if marker >= INLINE => {
                bits.skip(6)?;
                let inline = bits.peek_byte() >> 1;
                bits.skip(7)?;
                (inline as u8 & 15, (inline >> 4) + 1)
            }
            next => {
                let run = table[next]?;
                bits.skip(run.code_len)?;
                (run.nibble, run.count)
            }
        };
        for _ in 0..count {
            row = row << 4 | u32::from(nibble);
            filled += 1;
            if filled == ROW_PIXELS {
                if xor {
                    row ^= previous;
                    previous = row;
                }
                out.extend_from_slice(&row.to_be_bytes());
                (row, filled) = (0, 0);
            }
        }
    }
    out.truncate(rows * 4);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One tile of eight identical rows `0x11111111`, in XOR or plain form,
    /// with one code `0` (nibble 1, run of 8) and the bit stream `0` x 8.
    fn one_tile(header: u8) -> Vec<u8> {
        // Table: nibble 1; run 8 (7 in bits 4-6), code length 1; code 0; end.
        // Stream: eight times the 1-bit code 0.
        alloc::vec![header, 1, 0x81, 0x71, 0x00, 0xff, 0x00]
    }

    #[test]
    fn a_run_fills_a_row_and_xor_mode_accumulates_rows() {
        let plain = decompress(&one_tile(0x00)).unwrap();
        assert_eq!(plain.len(), 32);
        assert!(plain.chunks(4).all(|row| row == [0x11; 4]));
        // XOR mode: the rows are 0x11111111 ^ previous, so they alternate.
        let xored = decompress(&one_tile(0x80)).unwrap();
        assert_eq!(&xored[..8], &[0x11, 0x11, 0x11, 0x11, 0, 0, 0, 0]);
    }

    /// The bits of `text` (`0` and `1`, anything else skipped), packed most
    /// significant first and padded with zeros.
    fn pack(text: &str) -> Vec<u8> {
        let bits: Vec<bool> = text
            .chars()
            .filter(|c| matches!(c, '0' | '1'))
            .map(|c| c == '1')
            .collect();
        bits.chunks(8)
            .map(|byte| {
                byte.iter()
                    .enumerate()
                    .map(|(i, &bit)| u8::from(bit) << (7 - i))
                    .sum()
            })
            .collect()
    }

    #[test]
    fn inline_data_follows_the_six_bit_marker() {
        // No codes. Each item is the marker 111111, then run 8 (111) of
        // nibble 5 (0101); a tile of 64 nibbles takes eight of them.
        let mut data = alloc::vec![0, 1, 0x80, 0xff];
        data.extend(pack(&"111111 111 0101 ".repeat(8)));
        let tiles = decompress(&data).unwrap();
        assert_eq!(tiles, [0x55; 32]);
        // The stream cannot hold a second tile.
        data[1] = 2;
        assert!(decompress(&data).is_none());
    }

    #[test]
    fn overlapping_and_inline_codes_are_rejected() {
        // Codes 0 (length 1) and 01 (length 2) overlap.
        let overlap = [0, 1, 0x80, 0x01, 0x00, 0x02, 0x01, 0xff, 0];
        assert!(decompress(&overlap).is_none());
        // Code 111111 collides with the inline marker.
        let marker = [0, 1, 0x80, 0x06, 0x3f, 0xff, 0];
        assert!(decompress(&marker).is_none());
        // A run before any nibble value was set.
        assert!(decompress(&[0, 1, 0x01, 0x00, 0xff, 0]).is_none());
    }
}
