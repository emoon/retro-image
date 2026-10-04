//! Run-length unpackers used by C64 paint programs.
//!
//! Sources: Codebase64 "C64 Graphics File Format Specs"
//! (<http://codebase.c64.org/doku.php?id=base:c64_grafix_files_specs_list_v0.03>)
//! for the escape-byte schemes; details (count 0 meaning 256, stopping at
//! the expected size) checked against sample files. The exact-length
//! backward variant ([`backward_rle_filled`]) was reverse engineered from
//! the Super Hires samples (SIF, packed SHX) by probing `recoil2png` with
//! repacked copies. The flag-table backward packer ([`flag_table_rle`]) was
//! reverse engineered from a disassembly of the depacker stub in the packed
//! True Paint samples (the code is in the files; no outside source).
//! The escape-last packer ([`escape_last_rle`]) was read from the original
//! programs: the save routine of Flimatic 3.7 (`FLIMATIC.D64`,
//! `000_FLIMATIC_3.7_SHP.prg`) and Zoomatic 5.7 (`ZOOMATIC.D64`,
//! `000_ZOOMATIC_5.7__PD.prg`, run through a 6502 emulator to get past its
//! cruncher) and the loader of Showmatic (`002_SHOWMATIC_____PD.prg`),
//! all from the CSDb tools archive <https://csdb.dk>. The disassembly was
//! done by the project maintainer's permission; no code was copied.

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

/// Unpacks the escape-last RLE of Zoomatic and Flimatic: a load address,
/// then bytes where `value count escape` is a run (count 0 = 256) and every
/// other byte is a literal, and the escape byte itself as the last byte of
/// the file. The programs' loaders read it from the end backwards, which
/// is why a count or value equal to the escape byte is no problem. The
/// result is the last `len` bytes unpacked; `None` if the data runs out
/// before. Extra data before the wanted part is ignored.
pub(super) fn escape_last_rle(data: &[u8], len: usize) -> Option<Vec<u8>> {
    let (&escape, rest) = data.split_last()?;
    backward_rle_filled(rest.get(2..)?, escape, len).map(|(out, _)| out)
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

/// Unpacks the self-running True Paint packer's payload. It is read from the
/// last byte down; nine flag bytes `flags` pick the codes, and `values`
/// holds the bytes for the "emit twice" codes (indexes 5..=8). Where a flag
/// appears twice the highest index wins. Output is returned in file order
/// (the read order, reversed). `None` on truncation or output past `limit`.
///
/// Codes by flag index: 0 literal escape, 1 byte three times, 2 `n+2` zeros,
/// 3 three zeros, 4 `count` then byte repeated `count+2` times (count 0 ends
/// the stream), 5..=8 the table value twice.
pub(super) fn flag_table_rle(
    packed: &[u8],
    flags: &[u8; 9],
    values: &[u8; 4],
    limit: usize,
) -> Option<Vec<u8>> {
    let mut bytes = packed.iter().rev().copied();
    let mut out = Vec::new();
    loop {
        let byte = bytes.next()?;
        match flags.iter().rposition(|&f| f == byte) {
            None => out.push(byte),
            Some(0) => out.push(bytes.next()?),
            Some(1) => out.extend([bytes.next()?; 3]),
            Some(2) => {
                let n = usize::from(bytes.next()?) + 2;
                out.resize(out.len() + n, 0);
            }
            Some(3) => out.extend([0; 3]),
            Some(4) => {
                let n = usize::from(bytes.next()?);
                if n == 0 {
                    break;
                }
                let value = bytes.next()?;
                out.resize(out.len() + n + 2, value);
            }
            Some(i) => out.extend([values[i - 5]; 2]),
        }
        if out.len() > limit {
            return None;
        }
    }
    out.reverse();
    Some(out)
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

    #[test]
    fn flag_table_codes() {
        let flags = [1, 2, 3, 4, 5, 6, 7, 8, 9];
        let values = [0xa0, 0xa1, 0xa2, 0xa3];
        // Read order: 6 (table value twice), 0x42, 5 1 9 (9 three times), 5 0 (end).
        let packed = [0, 5, 9, 1, 5, 0x42, 6];
        assert_eq!(
            flag_table_rle(&packed, &flags, &values, 100),
            Some(alloc::vec![9, 9, 9, 0x42, 0xa0, 0xa0])
        );
        assert_eq!(flag_table_rle(&[1], &flags, &values, 100), None);
    }
}
