//! MacBinary wrapper: a 128-byte header in front of the data fork.
//!
//! Sources:
//! - MacBinary II standard,
//!   <https://files.stairways.com/other/macbinaryii-standard-info.txt>:
//!   byte 0 and bytes 74 and 82 are zero, byte 1 is the file name length
//!   (1-63), the file type is at +65, the data fork length (big-endian) at +83
//!   and the data fork follows the header.
//! - Header layout checked against the MacBinary-wrapped GIF, BMP and MacPaint
//!   samples in the corpus.
//!
//! The resource fork and the CRC are ignored.

use crate::bytes::be32;

const HEADER_LEN: usize = 128;

/// The data fork of a MacBinary file and its four-character file type.
pub(crate) struct MacBinary<'a> {
    pub(crate) file_type: [u8; 4],
    pub(crate) data_fork: &'a [u8],
}

impl<'a> MacBinary<'a> {
    /// `None` unless `data` starts with a consistent MacBinary header whose
    /// data fork lies fully inside the file.
    pub(crate) fn parse(data: &'a [u8]) -> Option<Self> {
        let header = data.get(..HEADER_LEN)?;
        if header[0] != 0 || !(1..=63).contains(&header[1]) || header[74] != 0 || header[82] != 0 {
            return None;
        }
        let len = usize::try_from(be32(header, 83)?).ok()?;
        Some(Self {
            file_type: header[65..69].try_into().ok()?,
            data_fork: data.get(HEADER_LEN..HEADER_LEN.checked_add(len)?)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;

    #[test]
    fn parses_the_data_fork_and_rejects_bare_data() {
        let mut file = vec![0u8; HEADER_LEN];
        file[1] = 4;
        file[65..69].copy_from_slice(b"GIFf");
        file[83..87].copy_from_slice(&3u32.to_be_bytes());
        file.extend_from_slice(b"abcXXXX");
        assert_eq!(MacBinary::parse(&file).unwrap().data_fork, b"abc");
        assert_eq!(MacBinary::parse(&file).unwrap().file_type, *b"GIFf");
        assert!(MacBinary::parse(b"GIF89a").is_none());
        // A fork running past the end of the file is not MacBinary.
        file[83..87].copy_from_slice(&99u32.to_be_bytes());
        assert!(MacBinary::parse(&file).is_none());
    }
}
