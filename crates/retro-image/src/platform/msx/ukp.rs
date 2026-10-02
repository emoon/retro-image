//! The "ukp" packer that MSX-FAN disk magazines used for their pictures.
//!
//! No documentation was found. Reverse engineered from three MSX-FAN 03
//! samples (a Graph Saurus Screen 7 and Screen 5 page and a BASIC `COPY`
//! Screen 7 file), each of which unpacks to exactly its stated size, and
//! whose unpacked SR7 and GL7 decode in `recoil2png` like the originals:
//! - 18-byte header: `"ukp" 1A`, `10 00 01 00 01 00 00 00`, the unpacked
//!   size (LE32), `20`, and an escape byte;
//! - then literal bytes, except `escape value count`, which repeats `value`
//!   `count` times (0 meaning 256).

use alloc::borrow::Cow;
use alloc::vec::Vec;

const SIGNATURE: &[u8] = b"ukp\x1a\x10\0\x01\0\x01\0\0\0";

/// Largest unpacked size accepted: a full 64 KiB VRAM dump plus its header.
const MAX_SIZE: usize = 0x10007;

/// `data` itself, or what it unpacks to if it is packed with ukp. `None`
/// for a malformed ukp file.
pub(super) fn unwrap(data: &[u8]) -> Option<Cow<'_, [u8]>> {
    if !data.starts_with(b"ukp\x1a") {
        return Some(Cow::Borrowed(data));
    }
    unpack(data).map(Cow::Owned)
}

fn unpack(data: &[u8]) -> Option<Vec<u8>> {
    let header = data.get(..0x12)?;
    if !header.starts_with(SIGNATURE) || header[0x10] != 0x20 {
        return None;
    }
    let size = u32::from_le_bytes(header[0x0c..0x10].try_into().ok()?) as usize;
    if size > MAX_SIZE {
        return None;
    }
    let escape = header[0x11];
    let mut out = Vec::with_capacity(size);
    let mut stream = &data[0x12..];
    while out.len() < size {
        match stream {
            [byte, value, count, rest @ ..] if *byte == escape => {
                let count = if *count == 0 { 256 } else { *count as usize };
                out.extend(core::iter::repeat_n(*value, count));
                stream = rest;
            }
            [byte, rest @ ..] if *byte != escape => {
                out.push(*byte);
                stream = rest;
            }
            _ => return None,
        }
    }
    (out.len() == size && stream.is_empty()).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packed(size: u32, escape: u8, stream: &[u8]) -> Vec<u8> {
        let mut data = SIGNATURE.to_vec();
        data.extend(size.to_le_bytes());
        data.extend([0x20, escape]);
        data.extend(stream);
        data
    }

    #[test]
    fn unpacks_literals_and_runs() {
        let data = packed(260, 1, &[0xfe, 1, 7, 3, 1, 9, 0]);
        let out = unwrap(&data).unwrap();
        assert_eq!(&out[..4], &[0xfe, 7, 7, 7]);
        assert_eq!(out.len(), 260);
        assert!(out[4..].iter().all(|&b| b == 9));
    }

    #[test]
    fn passes_other_data_through_and_rejects_bad_streams() {
        assert_eq!(unwrap(b"plain").unwrap().as_ref(), b"plain");
        assert!(unwrap(&packed(3, 1, &[5, 6])).is_none(), "truncated");
        assert!(unwrap(&packed(2, 1, &[5, 6, 7])).is_none(), "trailing data");
        assert!(
            unwrap(&packed(3, 1, &[1, 5, 4])).is_none(),
            "run past the size"
        );
        assert!(unwrap(&packed(2, 1, &[5, 1])).is_none(), "cut escape");
    }
}
