//! AMSDOS file headers.
//!
//! Source: 128-byte header, checksum of bytes 0-66 stored at 67-68:
//! <https://cpctech.cpcwiki.de/docs/allhead.html>.

const AMSDOS_HEADER_LEN: usize = 128;

/// File contents without the AMSDOS header, if one is present.
pub(super) fn strip_amsdos(data: &[u8]) -> &[u8] {
    let Some(header) = data.get(..AMSDOS_HEADER_LEN) else {
        return data;
    };
    let sum = header[..67]
        .iter()
        .fold(0u16, |sum, &b| sum.wrapping_add(u16::from(b)));
    if sum == u16::from_le_bytes([header[67], header[68]]) {
        &data[AMSDOS_HEADER_LEN..]
    } else {
        data
    }
}

/// Whether `data` starts with an AMSDOS header: a matching checksum, plus a
/// user number (byte 0) of 0-15 and a non-zero sum so that blank data, which
/// trivially "matches", doesn't count. Lets decoders of other platforms'
/// files with shared extensions (`.SCR`) turn CPC files away.
pub(crate) fn has_amsdos_header(data: &[u8]) -> bool {
    let Some(header) = data.get(..AMSDOS_HEADER_LEN) else {
        return false;
    };
    let sum = header[..67]
        .iter()
        .fold(0u16, |sum, &b| sum.wrapping_add(u16::from(b)));
    header[0] <= 15 && sum != 0 && sum == u16::from_le_bytes([header[67], header[68]])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_only_valid_amsdos_header() {
        let mut data = alloc::vec![0u8; AMSDOS_HEADER_LEN + 2];
        data[1] = 5;
        assert_eq!(strip_amsdos(&data).len(), data.len());
        data[67] = 5;
        assert_eq!(strip_amsdos(&data).len(), 2);
    }
}
