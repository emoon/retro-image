//! CGA graphics memory, as the PC formats that dump a screen store it.
//!
//! Sources:
//! - Wikipedia, "Color Graphics Adapter":
//!   <https://en.wikipedia.org/wiki/Color_Graphics_Adapter> (in the 320x200
//!   and 640x200 modes even scan lines live in the first bank of video
//!   memory and odd lines in the second, which starts 8192 bytes later).
//! - Deark `drhalo.c` (<https://github.com/jsummers/deark>, MIT license): the
//!   same layout for Dr. Halo pictures, and the four-bank Hercules variant.

use alloc::vec::Vec;

/// Distance between the banks of CGA video memory.
pub(super) const BANK_STRIDE: usize = 8192;

/// Rows `0..rows` of `row_len` bytes, row `i` taken from bank `i % banks`
/// (the banks are `bank_stride` bytes apart) at row `i / banks` of that bank.
/// Rows `data` is too short for are left out.
pub(super) fn deinterlace(
    data: &[u8],
    rows: usize,
    row_len: usize,
    banks: usize,
    bank_stride: usize,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(rows * row_len);
    for i in 0..rows {
        let at = (i / banks) * row_len + (i % banks) * bank_stride;
        out.extend_from_slice(data.get(at..at + row_len).unwrap_or(&[]));
    }
    out
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;

    #[test]
    fn cga_banks_are_interleaved() {
        let mut planes = vec![0u8; 2 * BANK_STRIDE];
        planes[0] = 1; // row 0
        planes[BANK_STRIDE] = 2; // row 1
        planes[80] = 3; // row 2
        let rows = deinterlace(&planes, 200, 80, 2, BANK_STRIDE);
        assert_eq!([rows[0], rows[80], rows[160]], [1, 2, 3]);
    }
}
