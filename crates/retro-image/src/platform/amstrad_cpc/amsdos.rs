//! AMSDOS file headers.
//!
//! Source: 128-byte header, checksum of bytes 0-66 stored at 67-68, file
//! length (24 bits) at 64-66: <https://cpctech.cpcwiki.de/docs/allhead.html>.
//! Files copied off disk images often keep the padding of their last
//! record (128 or 1024 bytes) after the length the header gives.

const HEADER_LEN: usize = 128;

/// The file's contents if `data` starts with an AMSDOS header: a matching
/// checksum, plus a user number (byte 0) of 0-15 and a non-zero sum so
/// that blank data, which trivially "matches", doesn't count. The contents
/// end at the header's length, dropping padding; a truncated file keeps
/// what is there.
pub(super) fn amsdos_body(data: &[u8]) -> Option<&[u8]> {
    let header = data.get(..HEADER_LEN)?;
    let sum = header[..67]
        .iter()
        .fold(0u16, |sum, &b| sum.wrapping_add(u16::from(b)));
    if header[0] > 15 || sum == 0 || sum != u16::from_le_bytes([header[67], header[68]]) {
        return None;
    }
    let body = &data[HEADER_LEN..];
    let len =
        usize::from(header[64]) | usize::from(header[65]) << 8 | usize::from(header[66]) << 16;
    Some(body.get(..len).unwrap_or(body))
}

/// The extension of the file name in the AMSDOS header, upper case, with
/// the attribute flags (bit 7: read-only, hidden, archived) cleared.
pub(super) fn amsdos_extension(data: &[u8]) -> Option<[u8; 3]> {
    amsdos_body(data)?;
    let mut extension = [0; 3];
    for (out, &byte) in extension.iter_mut().zip(&data[9..12]) {
        *out = (byte & 0x7f).to_ascii_uppercase();
    }
    Some(extension)
}

/// File contents without the AMSDOS header, if one is present.
pub(super) fn strip_amsdos(data: &[u8]) -> &[u8] {
    amsdos_body(data).unwrap_or(data)
}

/// Whether `data` starts with an AMSDOS header. Lets decoders of other
/// platforms' files with shared extensions (`.SCR`) turn CPC files away.
pub(crate) fn has_amsdos_header(data: &[u8]) -> bool {
    amsdos_body(data).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_header(user: u8, len: usize, body: &[u8]) -> alloc::vec::Vec<u8> {
        let mut data = alloc::vec![0u8; HEADER_LEN];
        data[0] = user;
        data[1..12].copy_from_slice(b"A       sCr");
        data[11] |= 0x80;
        data[64..67].copy_from_slice(&(len as u32).to_le_bytes()[..3]);
        let sum: u16 = data[..67].iter().map(|&b| u16::from(b)).sum();
        data[67..69].copy_from_slice(&sum.to_le_bytes());
        data.extend_from_slice(body);
        data
    }

    #[test]
    fn strips_only_valid_amsdos_header() {
        let mut data = with_header(0, 2, &[1, 2]);
        assert_eq!(strip_amsdos(&data), &[1, 2]);
        data[67] ^= 1;
        assert_eq!(strip_amsdos(&data).len(), data.len());
        assert!(!has_amsdos_header(&with_header(16, 2, &[1, 2])));
        assert!(!has_amsdos_header(&[0; HEADER_LEN]));
    }

    #[test]
    fn body_ends_at_header_length() {
        assert_eq!(
            amsdos_body(&with_header(0, 2, &[1, 2, 0, 0])),
            Some(&[1, 2][..])
        );
        assert_eq!(amsdos_body(&with_header(0, 9, &[1, 2])), Some(&[1, 2][..]));
    }

    #[test]
    fn extension_ignores_case_and_attributes() {
        assert_eq!(amsdos_extension(&with_header(0, 0, &[])), Some(*b"SCR"));
        assert_eq!(amsdos_extension(&[0; HEADER_LEN]), None);
    }
}
