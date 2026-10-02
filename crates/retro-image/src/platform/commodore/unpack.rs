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
    escape_rle_counted(packed, escape, run, len).map(|(out, _)| out)
}

/// As [`escape_rle`], also returning how many bytes of `packed` were used.
pub(super) fn escape_rle_counted(
    packed: &[u8],
    escape: u8,
    run: Run,
    len: usize,
) -> Option<(Vec<u8>, usize)> {
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
    Some((out, i))
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

/// Unpacks like [`backward_rle`] until the output's start is reached;
/// `None` if `packed` runs out first. The flag tells whether `packed` was
/// used up exactly: no bytes left over and no run crossing the start.
pub(super) fn backward_rle_filled(
    packed: &[u8],
    escape: u8,
    len: usize,
) -> Option<(Vec<u8>, bool)> {
    let mut out = alloc::vec![0; len];
    let mut end = len;
    let mut bytes = packed.iter().rev().copied();
    let mut exact = true;
    while end > 0 {
        let byte = bytes.next()?;
        if byte == escape {
            let count = bytes.next()?;
            let value = bytes.next()?;
            let count = if count == 0 { 256 } else { usize::from(count) };
            exact &= count <= end;
            let start = end.saturating_sub(count);
            out[start..end].fill(value);
            end = start;
        } else {
            end -= 1;
            out[end] = byte;
        }
    }
    Some((out, exact && bytes.next().is_none()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backward_filled_reports_exact_use() {
        let packed = [5, 3, 0xfe, 9];
        assert_eq!(
            backward_rle_filled(&packed, 0xfe, 4),
            Some((alloc::vec![5, 5, 5, 9], true))
        );
        assert_eq!(backward_rle_filled(&packed, 0xfe, 5), None);
        assert_eq!(
            backward_rle_filled(&packed, 0xfe, 3),
            Some((alloc::vec![5, 5, 9], false))
        );
        assert_eq!(
            backward_rle_filled(&[1, 9], 0xfe, 1),
            Some((alloc::vec![9], false))
        );
    }

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
