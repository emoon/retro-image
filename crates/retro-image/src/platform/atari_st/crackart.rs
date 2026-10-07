//! CrackArt pictures (`CA1`-`CA3`).
//!
//! Sources:
//! - <https://temlib.org/AtariForumWiki/index.php/CrackArt_file_format>
//! - <http://fileformats.archiveteam.org/wiki/Crack_Art>

use alloc::vec::Vec;

use super::common::{Resolution, SCREEN_LEN, decode_screen, palette_words};
use crate::bytes::be16;
use crate::{DecodeError, Image};

pub(super) fn decode_ca(data: &[u8]) -> Result<Image, DecodeError> {
    decode(data).ok_or(DecodeError::Invalid)
}

fn decode(data: &[u8]) -> Option<Image> {
    if data.get(..2)? != b"CA" {
        return None;
    }
    let compressed = *data.get(2)?;
    let resolution = Resolution::from_index((*data.get(3)?).into())?;
    let colors = match resolution {
        Resolution::High => 0,
        _ => resolution.colors(),
    };
    let mut words = palette_words(data, 4, colors)?;
    if words.is_empty() {
        words.push(0);
    }
    let body = &data[4 + colors * 2..];
    let bitmap = match compressed {
        0 => body.get(..SCREEN_LEN)?.to_vec(),
        1 => unpack(body, SCREEN_LEN)?,
        _ => return None,
    };
    decode_screen(resolution, &bitmap, &words)
}

/// Writes bytes at positions stepping by a fixed offset, wrapping to the
/// next unwritten start position at the end of the buffer.
struct Writer {
    out: Vec<u8>,
    start: usize,
    pos: usize,
    offset: usize,
    written: usize,
}

impl Writer {
    fn put(&mut self, value: u8, count: usize) {
        for _ in 0..count {
            if self.written == self.out.len() {
                return;
            }
            self.out[self.pos] = value;
            self.written += 1;
            self.pos += self.offset;
            if self.pos >= self.out.len() {
                self.start += 1;
                self.pos = self.start;
            }
        }
    }

    fn full(&self) -> bool {
        self.written == self.out.len()
    }
}

/// CrackArt RLE: escape byte, delta (fill) byte, step offset, commands.
pub(super) fn unpack(data: &[u8], len: usize) -> Option<Vec<u8>> {
    let escape = *data.first()?;
    let delta = *data.get(1)?;
    let offset = usize::from(be16(data, 2)?);
    let mut writer = Writer {
        out: alloc::vec![delta; len],
        start: 0,
        pos: 0,
        offset,
        written: 0,
    };
    if offset == 0 {
        return Some(writer.out);
    }
    let mut pos = 4;
    let mut next = || {
        let b = data.get(pos).copied();
        pos += 1;
        b
    };
    while !writer.full() {
        let cmd = next()?;
        if cmd != escape {
            writer.put(cmd, 1);
            continue;
        }
        let control = next()?;
        match control {
            0 => {
                let n = usize::from(next()?);
                let b = next()?;
                writer.put(b, n + 1);
            }
            1 => {
                let n = usize::from(next()?) << 8 | usize::from(next()?);
                let b = next()?;
                writer.put(b, n + 1);
            }
            2 => {
                let high = next()?;
                if high == 0 {
                    break;
                }
                let n = usize::from(high) << 8 | usize::from(next()?);
                writer.put(delta, n + 1);
            }
            c if c == escape => writer.put(escape, 1),
            c => {
                let b = next()?;
                writer.put(b, usize::from(c) + 1);
            }
        }
    }
    Some(writer.out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offset_steps_wrap_to_next_column() {
        // escape 0xff, delta 0, offset 2 over 4 bytes: positions 0, 2, 1, 3.
        let data = [0xff, 0, 0, 2, 1, 2, 3, 4];
        assert_eq!(unpack(&data, 4), Some(alloc::vec![1, 3, 2, 4]));
    }

    #[test]
    fn escape_two_zero_stops() {
        let data = [0xff, 7, 0, 1, 1, 0xff, 2, 0];
        assert_eq!(unpack(&data, 3), Some(alloc::vec![1, 7, 7]));
    }
}
