//! Softel EP1 teletext pages (Flair).
//!
//! Source: the Teletext Wiki's reverse-engineered description,
//! <https://teletext.wiki.zxnet.co.uk/wiki/EP1_format> (CC BY-SA 4.0; only
//! the facts are used). Header `FE 01`, language, `0xCA` if level 1.5
//! enhancement packets follow (else 0), and a 16-bit offset from the end of
//! the 6-byte header to 960 bytes of page data for rows 0-23. Language and
//! enhancement packets are ignored. No real EP1 sample was available; the
//! corpus files are converted from TTI pages (see the corpus manifest).

use super::{COLUMNS, Dialect, Page};
use crate::bytes::le16;
use crate::{DecodeError, Image};

const HEADER: usize = 6;
const PAGE_ROWS: usize = 24;

pub(in crate::platform) fn decode_ep1(data: &[u8]) -> Result<Image, DecodeError> {
    if data.get(..2) != Some(&[0xfe, 0x01]) || !matches!(data.get(3), Some(0 | 0xca)) {
        return Err(DecodeError::Unrecognized);
    }
    let offset = le16(data, 4).ok_or(DecodeError::Unrecognized)?;
    let start = HEADER + usize::from(offset);
    let body = data
        .get(start..start + PAGE_ROWS * COLUMNS)
        .ok_or(DecodeError::Unrecognized)?;
    let mut page = Page::blank();
    for (row, line) in body.as_chunks::<COLUMNS>().0.iter().enumerate() {
        page.set_row(row, line.iter().copied());
    }
    page.render(Dialect::Broadcast)
}
