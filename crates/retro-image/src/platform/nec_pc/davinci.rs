//! DaVinci `IMG` pictures (NEC PC-88, 640x200 shown at 640x400).
//!
//! The only public hint is the 8-bits.info game entry
//! (<https://www.8-bits.info/gamelist/PC88/info/info_n21XpU9XjMi7XL4U.php>), which says
//! nothing about the layout. Reverse engineered from `REMSM3.IMG` by black-box
//! probing of `recoil2png` with mutated copies; see `docs/research/msx-japanese.md`,
//! "Wave 5: Japanese".
//!
//! There is no header. The file is a run-length coded stream of 80 * 200 byte
//! columns, each a triple (blue, red, green) of plane bytes, MSB first, line by
//! line. A control byte with bit 7 set repeats the next triple `c & 0x7f` times;
//! otherwise `c` literal triples follow. The stream must be followed by exactly 35
//! more bytes (RECOIL ignores their content and refuses any other length).
//! Colors are the 8 digital PC-88 colors (0 or 255 per component).

use alloc::vec::Vec;

use super::pc88_planes::{self, PLANE_BYTES};
use crate::{DecodeError, Image};

const COLUMNS: usize = PLANE_BYTES;
const TRAILER: usize = 35;

/// Expands the stream into `COLUMNS` (blue, red, green) triples.
fn unpack(data: &[u8]) -> Option<Vec<[u8; 3]>> {
    let mut columns = Vec::with_capacity(COLUMNS);
    let mut pos = 0;
    while columns.len() < COLUMNS {
        let control = *data.get(pos)?;
        pos += 1;
        let count = usize::from(control & 0x7f);
        if count == 0 || columns.len() + count > COLUMNS {
            return None;
        }
        if control & 0x80 != 0 {
            let triple = *data.get(pos..pos + 3)?.as_array()?;
            pos += 3;
            columns.resize(columns.len() + count, triple);
        } else {
            let (triples, _) = data.get(pos..pos + count * 3)?.as_chunks::<3>();
            columns.extend_from_slice(triples);
            pos += count * 3;
        }
    }
    (data.len() == pos + TRAILER).then_some(columns)
}

pub(in crate::platform) fn decode_davinci(data: &[u8]) -> Result<Image, DecodeError> {
    let columns = unpack(data).ok_or(DecodeError::Unrecognized)?;
    let plane = |channel: usize| -> Vec<u8> { columns.iter().map(|c| c[channel]).collect() };
    pc88_planes::image(&plane(0), &plane(1), &plane(2))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn runs_literals_and_trailer() {
        // 16000 columns: a run of 127 (x125 = 15875), a run of 100, one literal, a run of 24.
        let mut data = Vec::new();
        for _ in 0..125 {
            data.extend([0xff, 1, 2, 3]);
        }
        data.extend([0xe4, 9, 9, 9]);
        data.extend([1, 4, 5, 6]);
        data.extend([0x80 | 24, 7, 7, 7]);
        data.extend(vec![0; TRAILER]);
        let columns = unpack(&data).unwrap();
        assert_eq!(columns.len(), COLUMNS);
        assert_eq!(columns[0], [1, 2, 3]);
        assert_eq!(columns[15999], [7, 7, 7]);
        assert_eq!(columns[15975], [4, 5, 6]);
        // The trailer length is exact and zero counts are refused.
        data.push(0);
        assert!(unpack(&data).is_none());
        data.truncate(data.len() - 2);
        assert!(unpack(&data).is_none());
        assert!(unpack(&[0x80, 1, 2, 3]).is_none());
    }
}
