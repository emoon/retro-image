//! C64 OS screenshots (`.pet` version 2): a 40×25 text screen with its own
//! character set.
//!
//! Sources:
//! - Greg Naçu, "Image File Formats", <https://c64os.com/post/imageformats>:
//!   `PET` in PETSCII and a version digit, three 17-byte strings, 1000
//!   screen codes, 1000 colours, border, background, and (version 2) a
//!   2048-byte character set.
//! - Text mode semantics (set character pixels in the colour RAM colour,
//!   clear ones in the background): <https://www.cebix.net/VIC-Article.txt>.
//!   Versions 0 and 1 need the C64 character ROM and are not supported.

use super::vic2::{Frame, SCREEN_LEN};
use crate::{DecodeError, Image};

const HEADER: usize = 4 + 3 * 17;
const COLORS: usize = HEADER + SCREEN_LEN;
const BACKGROUND: usize = COLORS + SCREEN_LEN + 1;
const CHARSET: usize = BACKGROUND + 1;

pub(super) fn decode_c64os(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != CHARSET + 2048 || data[..4] != [0xd0, 0xc5, 0xd4, b'2'] {
        return Err(DecodeError::Unrecognized);
    }
    let background = data[BACKGROUND];
    let frame = Frame::from_fn(200, |x, y| {
        let cell = y / 8 * 40 + x / 8;
        let glyph = usize::from(data[HEADER + cell]);
        if data[CHARSET + glyph * 8 + y % 8] & (0x80 >> (x % 8)) != 0 {
            data[COLORS + cell] & 15
        } else {
            background
        }
    });
    Ok(frame.to_image(0))
}
