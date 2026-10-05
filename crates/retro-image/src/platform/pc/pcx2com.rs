//! PCX2COM self-displaying pictures (`.COM`), a DOS utility by "Dr.Destiny".
//!
//! Sources:
//! - Deark `pcx.c` (<https://github.com/jsummers/deark>, MIT license): a COM
//!   file of 922 to 65280 bytes that starts `B8 13 00 CD` (`mov ax, 13h`,
//!   `int 10h`: VGA mode 13h), has the string "Self PCX" with every byte
//!   xor 80h at offset 104, and ends with a `0C` byte. The 6-bit VGA palette
//!   (768 bytes) sits at 152, and the PCX run-length data of the 320x200
//!   picture starts at 920.
//! - Disassembling the stub confirms the offsets: it loads the palette from
//!   `mov si, 198h`, the pixels from `mov si, 498h` (a COM file loads at
//!   `100h`, so 152 and 920) and fills `FA00h` = 64000 bytes of video memory.
//! - Checked on 2 sample files (Sembiance's `pcx2com` folder).
//!
//! No extension is claimed: `.com` and `.exe` belong to every DOS program, and
//! the shared MIME package would send them all to the image viewer. The check
//! is strict enough to find these files by content under any name.
//!
//! Verification: no RECOIL oracle for this format; output matches Deark's
//! `pcx2com` module (which writes a PCX and converts that) on both files.

// Parts of this file follow Deark's modules/pcx.c
// (Deark, https://github.com/jsummers/deark):
//
// Copyright (C) 2016 Jason Summers
// <jason1@pobox.com>
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

use alloc::vec::Vec;

use super::dac_rounded;
use super::pcx::unpack;
use crate::{DecodeError, Image};

const FAIL: DecodeError = DecodeError::Unrecognized;
const STUB: [u8; 4] = [0xb8, 0x13, 0x00, 0xcd];
/// "Self PCX" with the top bit of each byte set.
const SIGNATURE: [u8; 8] = [0xd3, 0xe5, 0xec, 0xe6, 0xa0, 0xd0, 0xc3, 0xd8];
const SIGNATURE_AT: usize = 104;
const PALETTE_AT: usize = 152;
const PIXELS_AT: usize = 920;
const MAX_COM_LEN: usize = 65280;
const WIDTH: usize = 320;
const HEIGHT: usize = 200;

pub(super) fn decode_pcx2com(data: &[u8]) -> Result<Image, DecodeError> {
    if !(PIXELS_AT + 2..=MAX_COM_LEN).contains(&data.len())
        || !data.starts_with(&STUB)
        || data.last() != Some(&0x0c)
        || data.get(SIGNATURE_AT..SIGNATURE_AT + SIGNATURE.len()) != Some(&SIGNATURE)
    {
        return Err(FAIL);
    }
    let palette: Vec<u32> = data[PALETTE_AT..PIXELS_AT]
        .as_chunks::<3>()
        .0
        .iter()
        .map(|c| dac_rounded(c[0]) << 16 | dac_rounded(c[1]) << 8 | dac_rounded(c[2]))
        .collect();
    let (pixels, _) = unpack(&data[PIXELS_AT..], WIDTH * HEIGHT)?;
    Image::from_indexed(WIDTH as u32, HEIGHT as u32, &pixels, &palette)
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;

    #[test]
    fn needs_the_stub_the_signature_and_the_palette_marker() {
        let mut com = vec![0u8; PIXELS_AT];
        com[..STUB.len()].copy_from_slice(&STUB);
        com[SIGNATURE_AT..SIGNATURE_AT + 8].copy_from_slice(&SIGNATURE);
        com[PALETTE_AT + 3..PALETTE_AT + 6].copy_from_slice(&[63, 0, 32]);
        // 64000 pixels of color 1: 1015 runs of 63, then one of 55.
        for _ in 0..1015 {
            com.extend_from_slice(&[0xc0 | 63, 1]);
        }
        com.extend_from_slice(&[0xc0 | 55, 1, 0x0c]);
        let image = decode_pcx2com(&com).unwrap();
        assert_eq!((image.width(), image.height()), (320, 200));
        assert_eq!(image.get(319, 199), 0xff0082);
        assert_eq!(crate::decode("picture.com", &com), Ok(image));
        let last = com.len() - 1;
        com[last] = 0;
        assert!(decode_pcx2com(&com).is_err());
    }
}
