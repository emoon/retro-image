//! Tiny Stuff pictures (`TNY`, `TN1`-`TN6`).
//!
//! Sources:
//! - <https://temlib.org/AtariForumWiki/index.php/Tiny_file_format>
//! - <http://fileformats.archiveteam.org/wiki/Tiny_Stuff>

use alloc::vec::Vec;

use super::common::{Resolution, SCREEN_LEN, decode_screen, palette_words};
use crate::bytes::be16;
use crate::{DecodeError, Image};

const WORDS: usize = SCREEN_LEN / 2;

pub(super) fn decode_tny(data: &[u8]) -> Result<Image, DecodeError> {
    decode(data).ok_or(DecodeError::Invalid)
}

fn decode(data: &[u8]) -> Option<Image> {
    let mode = *data.first()?;
    let (resolution, mut pos) = match mode {
        0..=2 => (Resolution::from_index(mode.into())?, 1),
        3..=5 => (Resolution::from_index((mode - 3).into())?, 5),
        _ => return None,
    };
    let words = palette_words(data, pos, 16)?;
    pos += 32;
    let control_len = usize::from(be16(data, pos)?);
    let data_len = usize::from(be16(data, pos + 2)?) * 2;
    pos += 4;
    let control = data.get(pos..pos + control_len)?;
    let values = data.get(pos + control_len..pos + control_len + data_len)?;
    let columns: Vec<u8> = unpack(control, values)?
        .iter()
        .flat_map(|word| word.to_be_bytes())
        .collect();
    decode_screen(resolution, &from_columns(&columns)?, &words)
}

/// Reorders a 32000-byte screen stored as four sets of word columns
/// (also used by QuantumPaint) into normal screen order.
pub(super) fn from_columns(columns: &[u8]) -> Option<Vec<u8>> {
    let columns = columns.get(..SCREEN_LEN)?;
    let mut bitmap = alloc::vec![0u8; SCREEN_LEN];
    for (i, word) in columns.as_chunks::<2>().0.iter().enumerate() {
        let (y, column) = column_position(i);
        let offset = (y * 80 + column) * 2;
        bitmap[offset..offset + 2].copy_from_slice(word);
    }
    Some(bitmap)
}

/// Screen line and word column of the `i`-th unpacked word: four sets of
/// columns (0, 4, 8, ...), (1, 5, 9, ...), ... each column 200 words tall.
fn column_position(i: usize) -> (usize, usize) {
    let y = i % 200;
    let column_in_set = i / 200 % 20;
    let set = i / 4000;
    (y, set + column_in_set * 4)
}

/// Expands control bytes and data words into 16000 words.
fn unpack(control: &[u8], values: &[u8]) -> Option<Vec<u16>> {
    let mut out = Vec::with_capacity(WORDS);
    let mut values = values
        .as_chunks::<2>()
        .0
        .iter()
        .map(|w| u16::from_be_bytes([w[0], w[1]]));
    let mut pos = 0;
    let word_at = |pos: usize| be16(control, pos);
    while out.len() < WORDS {
        let x = *control.get(pos)? as i8;
        pos += 1;
        match x {
            i8::MIN..=-1 => {
                for _ in 0..(-isize::from(x)) {
                    out.push(values.next()?);
                }
            }
            0 => {
                let count = word_at(pos)?;
                pos += 2;
                let value = values.next()?;
                out.extend(core::iter::repeat_n(value, count.into()));
            }
            1 => {
                let count = word_at(pos)?;
                pos += 2;
                for _ in 0..count {
                    out.push(values.next()?);
                }
            }
            _ => {
                let value = values.next()?;
                out.extend(core::iter::repeat_n(value, x as usize));
            }
        }
    }
    out.truncate(WORDS);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_are_grouped_in_four_sets() {
        assert_eq!(column_position(0), (0, 0));
        assert_eq!(column_position(199), (199, 0));
        assert_eq!(column_position(200), (0, 4));
        assert_eq!(column_position(4000), (0, 1));
        assert_eq!(column_position(15999), (199, 79));
    }

    #[test]
    fn unpack_rejects_truncated_data() {
        assert_eq!(unpack(&[5], &[0, 1]), None);
    }
}
