//! Vertical word RLE (BMHD compression 2, `VDAT` chunks inside BODY), written
//! by DeluxePaint on the Atari ST.
//!
//! Source: layout reverse engineered from sample files and verified against
//! `recoil2png` output. One VDAT chunk per plane: a word holding the length of
//! the command block (including that word), signed command bytes, then data
//! words. Command 0: a data word gives a count of literal words; command 1:
//! a data word gives a count, then one word to repeat; a negative command
//! `-n` copies `n` literal words; a command `n >= 2` repeats one word `n` times.
//! Words fill a plane column by column (16 pixels wide), top to bottom.

use alloc::vec;
use alloc::vec::Vec;

use super::iff::chunks;
use crate::bytes::be16;

/// Unpacks a BODY into ILBM's interleaved layout (`row_len` bytes per plane row).
pub(super) fn unpack(body: &[u8], planes: usize, row_len: usize, height: usize) -> Option<Vec<u8>> {
    let mut out = vec![0u8; row_len * planes * height];
    let mut vdats = chunks(body).filter(|(id, _)| id == b"VDAT");
    let columns = row_len / 2;
    for plane in 0..planes {
        let (_, vdat) = vdats.next()?;
        let words = unpack_plane(vdat, columns * height)?;
        for (i, word) in words.iter().enumerate() {
            let (column, y) = (i / height, i % height);
            let at = (y * planes + plane) * row_len + column * 2;
            out[at..at + 2].copy_from_slice(&word.to_be_bytes());
        }
    }
    Some(out)
}

fn unpack_plane(vdat: &[u8], len: usize) -> Option<Vec<u16>> {
    let command_len = usize::from(be16(vdat, 0)?);
    let commands = vdat.get(2..command_len)?;
    let mut data = vdat[command_len..]
        .chunks_exact(2)
        .map(|w| u16::from_be_bytes([w[0], w[1]]));
    let mut out = Vec::with_capacity(len);
    for &command in commands {
        match command as i8 {
            0 => {
                let count = usize::from(data.next()?);
                for _ in 0..count {
                    out.push(data.next()?);
                }
            }
            1 => {
                let count = usize::from(data.next()?);
                let word = data.next()?;
                out.resize(out.len() + count, word);
            }
            n if n < 0 => {
                for _ in 0..-i32::from(n) {
                    out.push(data.next()?);
                }
            }
            n => {
                let word = data.next()?;
                out.resize(out.len() + n as usize, word);
            }
        }
        if out.len() >= len {
            break;
        }
    }
    out.resize(len, 0);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands() {
        // commands: repeat 3, literal 2, count-run, count-literal
        let vdat = [
            0, 6, 3, 0xfe, 1, 0, // command block
            0, 7, // repeated word
            0, 1, 0, 2, // literals
            0, 2, 0, 9, // run of 2 x 9
            0, 1, 0, 5, // literal of 1
        ];
        let words = unpack_plane(&vdat, 9).unwrap();
        assert_eq!(words, [7, 7, 7, 1, 2, 9, 9, 5, 0]);
    }
}
