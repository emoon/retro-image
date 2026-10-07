//! Multicolor pictures wrapped in a self-displaying viewer: a BASIC
//! `SYS 2061` line, a machine code viewer, then the picture in Koala order
//! (bitmap, screen RAM, color RAM) at a fixed offset.
//!
//! Sources: reverse engineered from CSDb samples (416 files of 10500 bytes,
//! 387 of 10608 bytes), no decoder code used. The viewers' operands give the
//! layout: both copy the bitmap to `$6000`, the screen to `$4400` and the
//! colors to `$D800`, set `$D016` bit 4 (multicolor) and `$D011` bit 5
//! (bitmap), and read the border and background colors (`$D020`/`$D021`)
//! from bytes in the file. In the 10500-byte viewer these are at
//! `$09E6`/`$09E7`, followed by the bitmap at `$09F2`; in the 10608-byte
//! viewer the bitmap is at `$0A38` and the colors follow it at
//! `$2978`/`$2979`. Both layouts were checked by rendering samples with
//! different backgrounds; the pictures are standard Koala multicolor
//! bitmaps.

use super::bitmap::Multicolor;
use crate::{DecodeError, Image};

/// BASIC line `SYS 2061` after a variable line number.
fn has_sys_stub(data: &[u8]) -> bool {
    data.starts_with(&[0x01, 0x08, 0x0b, 0x08]) && data.get(6..14) == Some(b"\x9e2061\0\0\0")
}

/// A viewer family: its size, the code bytes that identify it, and where the
/// picture sits.
struct Viewer {
    size: usize,
    /// Code right after the BASIC line.
    code: &'static [u8],
    /// Offset and bytes of viewer code that ends the file, if the family has
    /// any.
    tail: Option<(usize, &'static [u8])>,
    picture: Multicolor,
}

const CODE_10500: &[u8] = &[
    0xa9, 0x14, 0x8d, 0x18, 0xd0, 0xa2, 0x00, 0xa9, 0x20, 0x9d, 0x00, 0x04, 0x9d, 0x00, 0x05, 0x9d,
    0x00, 0x06, 0x9d, 0xe8, 0x06, 0xca, 0xd0, 0xf1,
];

const CODE_10608: &[u8] = &[
    0xa2, 0x19, 0xb5, 0x02, 0x9d, 0x6f, 0x31, 0xca, 0x10, 0xf8, 0xa9, 0x0e, 0x20, 0xd2, 0xff, 0xa5,
    0x01, 0x48, 0x29, 0xf8, 0x09, 0x06, 0x85, 0x01, 0x20, 0x65, 0x08,
];

/// The 10608-byte viewer's final code, after the picture.
const TAIL_10608: &[u8] = &[
    0x8d, 0x58, 0x31, 0x8e, 0x59, 0x31, 0x8d, 0x5f, 0x31, 0x8e, 0x60, 0x31, 0x88, 0xb9, 0xff, 0xff,
    0x8d, 0x69, 0x31, 0x88, 0xb9, 0xff, 0xff, 0x8d, 0x68, 0x31, 0x8c, 0x6b, 0x31, 0x20, 0xff, 0xff,
    0xa0, 0xff, 0xd0, 0xe8, 0x60,
];

/// Both viewers load at `$0801`: file offset = address - `$0801` + 2.
const LOAD: u16 = 0x0801;

/// Bitmap at offset 499 (`$09F2`), then screen and colors; border and
/// background are bytes 487 and 488.
const VIEWER_10500: Viewer = Viewer {
    size: 10500,
    code: CODE_10500,
    tail: None,
    picture: Multicolor {
        load: LOAD,
        sizes: &[10500],
        bitmap: 0x09f2,
        screen: 0x09f2 + 8000,
        color: 0x09f2 + 9000,
        background: 0x09e7,
    },
};

/// Bitmap at offset 569 (`$0A38`), then border, background, screen and
/// colors; 37 bytes of viewer code end the file.
const VIEWER_10608: Viewer = Viewer {
    size: 10608,
    code: CODE_10608,
    tail: Some((10571, TAIL_10608)),
    picture: Multicolor {
        load: LOAD,
        sizes: &[10608],
        bitmap: 0x0a38,
        screen: 0x297a,
        color: 0x297a + 1000,
        background: 0x2979,
    },
};

impl Viewer {
    fn decode(&self, data: &[u8]) -> Result<Image, DecodeError> {
        let code_ok = data.len() == self.size
            && has_sys_stub(data)
            && data.get(14..14 + self.code.len()) == Some(self.code);
        let tail_ok = self
            .tail
            .is_none_or(|(at, tail)| data.get(at..) == Some(tail));
        if !code_ok || !tail_ok {
            return Err(DecodeError::Invalid);
        }
        self.picture.decode(data)
    }
}

/// Koala-order viewer of 10500 bytes.
pub(super) fn decode_10500(data: &[u8]) -> Result<Image, DecodeError> {
    VIEWER_10500.decode(data)
}

/// Koala-order viewer of 10608 bytes.
pub(super) fn decode_10608(data: &[u8]) -> Result<Image, DecodeError> {
    VIEWER_10608.decode(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(viewer: &Viewer) -> alloc::vec::Vec<u8> {
        let mut data = alloc::vec![0u8; viewer.size];
        data[..14].copy_from_slice(&[
            0x01, 0x08, 0x0b, 0x08, 0xf0, 0x02, 0x9e, b'2', b'0', b'6', b'1', 0, 0, 0,
        ]);
        data[14..14 + viewer.code.len()].copy_from_slice(viewer.code);
        if let Some((at, tail)) = viewer.tail {
            data[at..].copy_from_slice(tail);
        }
        data
    }

    #[test]
    fn viewers_place_koala_data_and_background() {
        let mut data = file(&VIEWER_10500);
        data[488] = 2; // background: red
        let image = decode_10500(&data).unwrap();
        assert_eq!((image.width(), image.height()), (320, 200));
        assert_eq!(image.get(0, 0), 0x68372b);
        data[499] = 0x40; // first pixel pair: %01 takes the screen RAM high nibble
        data[8499] = 0x10; // white
        assert_eq!(decode_10500(&data).unwrap().get(0, 0), 0xffffff);

        let mut data = file(&VIEWER_10608);
        data[8570] = 6; // background: blue
        assert_eq!(decode_10608(&data).unwrap().get(0, 0), 0x352879);
    }

    #[test]
    fn other_sizes_and_code_are_rejected() {
        let mut data = file(&VIEWER_10500);
        assert!(decode_10608(&data).is_err());
        data[20] ^= 1;
        assert!(decode_10500(&data).is_err());
        assert!(decode_10500(&data[..10499]).is_err());
    }
}
