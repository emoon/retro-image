//! Fixed-width integers read from byte slices, `None` past the end.
//!
//! No external format knowledge: little- and big-endian readers.

fn array<const N: usize>(data: &[u8], at: usize) -> Option<[u8; N]> {
    data.get(at..at.checked_add(N)?)?.try_into().ok()
}

/// Little-endian 16-bit value at `at`.
pub(crate) fn le16(data: &[u8], at: usize) -> Option<u16> {
    array(data, at).map(u16::from_le_bytes)
}

/// Little-endian 32-bit value at `at`.
pub(crate) fn le32(data: &[u8], at: usize) -> Option<u32> {
    array(data, at).map(u32::from_le_bytes)
}

/// Big-endian 16-bit value at `at`.
pub(crate) fn be16(data: &[u8], at: usize) -> Option<u16> {
    array(data, at).map(u16::from_be_bytes)
}

/// Big-endian 32-bit value at `at`.
pub(crate) fn be32(data: &[u8], at: usize) -> Option<u32> {
    array(data, at).map(u32::from_be_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_both_byte_orders_and_rejects_out_of_range() {
        let data = [0x12, 0x34, 0x56, 0x78];
        assert_eq!(le16(&data, 1), Some(0x5634));
        assert_eq!(be16(&data, 2), Some(0x5678));
        assert_eq!(le32(&data, 0), Some(0x7856_3412));
        assert_eq!(be32(&data, 0), Some(0x1234_5678));
        assert_eq!(be16(&data, 3), None);
        assert_eq!(le32(&data, 1), None);
        assert_eq!(be16(&data, usize::MAX), None);
    }
}
