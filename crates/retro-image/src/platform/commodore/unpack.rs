//! Run-length unpackers used by C64 paint programs.
//!
//! Sources: Codebase64 "C64 Graphics File Format Specs"
//! (<http://codebase.c64.org/doku.php?id=base:c64_grafix_files_specs_list_v0.03>)
//! for the escape-byte schemes; details (count 0 meaning 256, stopping at
//! the expected size) checked against sample files.

use alloc::vec::Vec;

/// Order of the fields after the escape byte.
#[derive(Clone, Copy)]
pub(super) enum Run {
    /// `ESC value count`
    ValueCount,
    /// `ESC count value`
    CountValue,
    /// `ESC count-1 value`
    CountMinusOneValue,
}

/// Unpacks `ESC a b` runs (count 0 = 256 unless stored minus one) and
/// literal bytes from `packed` until `len` bytes are produced or the input
/// ends. Fails on a truncated run.
pub(super) fn escape_rle(packed: &[u8], escape: u8, run: Run, len: usize) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(len);
    let mut i = 0;
    while out.len() < len {
        let Some(&byte) = packed.get(i) else {
            break;
        };
        if byte == escape {
            let (a, b) = (*packed.get(i + 1)?, *packed.get(i + 2)?);
            let (value, count) = match run {
                Run::ValueCount => (a, b),
                Run::CountValue => (b, a),
                Run::CountMinusOneValue => (b, a.wrapping_add(1)),
            };
            let count = if count == 0 { 256 } else { usize::from(count) };
            out.extend(core::iter::repeat_n(value, count));
            i += 3;
        } else {
            out.push(byte);
            i += 1;
        }
    }
    out.truncate(len);
    Some(out)
}

/// Unpacks data packed backwards: `packed` is read from its last byte down,
/// where `ESC count value` runs (count 0 = 256) and literal bytes fill the
/// output from its end. Returns `len` bytes; any start not reached stays 0.
pub(super) fn backward_rle(packed: &[u8], escape: u8, len: usize) -> Option<Vec<u8>> {
    let mut out = alloc::vec![0; len];
    let mut end = len;
    let mut bytes = packed.iter().rev().copied();
    while end > 0 {
        let Some(byte) = bytes.next() else {
            break;
        };
        if byte == escape {
            let count = bytes.next()?;
            let value = bytes.next()?;
            let count = if count == 0 { 256 } else { usize::from(count) };
            let start = end.saturating_sub(count);
            out[start..end].fill(value);
            end = start;
        } else {
            end -= 1;
            out[end] = byte;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_runs_and_literals() {
        let packed = [1, 0xfe, 7, 3, 2];
        assert_eq!(
            escape_rle(&packed, 0xfe, Run::ValueCount, 5),
            Some(alloc::vec![1, 7, 7, 7, 2])
        );
        assert_eq!(
            escape_rle(&packed, 0xfe, Run::CountValue, 5),
            Some(alloc::vec![1, 3, 3, 3, 3, 3, 3, 3, 2][..5].to_vec())
        );
    }

    #[test]
    fn truncated_run_fails_and_short_input_stops() {
        assert_eq!(escape_rle(&[0xfe, 1], 0xfe, Run::ValueCount, 4), None);
        assert_eq!(
            escape_rle(&[1, 2], 0xfe, Run::ValueCount, 4),
            Some(alloc::vec![1, 2])
        );
    }
}
