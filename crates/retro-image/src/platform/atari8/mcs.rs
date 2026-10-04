//! MCS pictures: 160x192, nine colours, a multicolour character screen with
//! players and missiles.
//!
//! Sources:
//! - Just Solve "MCS" (<http://fileformats.archiveteam.org/wiki/MCS>):
//!   exactly 10185 bytes, 160x192, 9 colours.
//! - ANTIC mode 4 (multicolour text) and the player/missile hardware: De Re
//!   Atari ch. 2 and 4 (<https://www.atariarchives.org/dere/chapt02.php>,
//!   <https://www.atariarchives.org/dere/chapt04.php>); priority rules in
//!   [`gtia`].
//! - The layout was reverse engineered from the corpus samples and
//!   `recoil2png` output on hand-made files. The file is nine colour
//!   registers (COLPM0-3, COLPF0-3, COLBK), eight 1024-byte character sets
//!   of 128 8-byte characters, a 40x24 screen of character codes (960 bytes
//!   at offset 8201), and 128 bytes each for the missiles and players 0-3 (at
//!   9161; the first and last 16 bytes of each have no effect), then 400
//!   bytes that are never read. Character rows 0-2 use the first character
//!   set, rows 3-5 the second and so on (the character base changes every
//!   third row, which gives 960 distinct characters). Characters are
//!   ANTIC mode 4: 2 bits per pixel, values 1-3 are playfield 0-2, and a
//!   code with bit 7 set shows `11` pixels in playfield 3. Each player is 64
//!   pixels wide (8 bits of 8 output pixels, bit 7 leftmost) and starts at
//!   output pixel 80 * n; the missile of the same number follows it at
//!   80 * n + 64 with two bits (bit 1 left) of 8 pixels, in the player's
//!   colour. Their bytes cover two scanlines each, from byte 16 on. The
//!   playfield hides every player and missile, which show only over the
//!   background (PRIOR 4).

use super::gtia::{self, Colors};
use super::palette::register_rgb;
use crate::{DecodeError, Image};

const LEN: usize = 10185;
const FIRST_CHARSET: usize = 9;
const CHARSET_LEN: usize = 1024;
const SCREEN: usize = FIRST_CHARSET + 8 * CHARSET_LEN;
const OBJECTS: usize = SCREEN + 960;
const OBJECT_LEN: usize = 128;
const LINES: usize = 192;
/// Players and missiles are drawn behind the playfield.
const PRIOR: u8 = 0x04;

pub(super) fn decode_mcs(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != LEN {
        return Err(DecodeError::Unrecognized);
    }
    let colors = Colors {
        player: [data[0], data[1], data[2], data[3]],
        playfield: [data[4], data[5], data[6], data[7]],
        background: data[8],
    };
    let mut image = Image::new(320, LINES as u32);
    for y in 0..LINES {
        let objects = line_objects(data, y);
        for (x, &object) in objects.iter().enumerate() {
            let playfield = playfield_pixel(data, x, y);
            let color = gtia::resolve(PRIOR, object, playfield, &colors);
            image.set(x as u32, y as u32, register_rgb(color));
        }
    }
    Ok(image)
}

/// Which playfield registers (bit 0 = playfield 0 ... bit 3 = playfield 3)
/// cover output pixel `x` of scanline `y`.
fn playfield_pixel(data: &[u8], x: usize, y: usize) -> u8 {
    let (row, line) = (y / 8, y % 8);
    let cell = row * 40 + x / 8;
    let code = data[SCREEN + cell];
    let charset = FIRST_CHARSET + CHARSET_LEN * (row / 3);
    let glyph = data[charset + 8 * usize::from(code & 0x7f) + line];
    let value = glyph >> (6 - 2 * (x % 8 / 2)) & 3;
    gtia::playfield_bit(gtia::antic4_register(value, code & 0x80 != 0))
}

/// The players (bit n) and missiles (also bit n, for the player of the same
/// number) covering each output pixel of scanline `y`.
fn line_objects(data: &[u8], y: usize) -> [u8; 320] {
    let mut pixels = [0u8; 320];
    let byte = |object: usize| data[OBJECTS + OBJECT_LEN * object + 16 + y / 2];
    for n in 0..4 {
        let player = byte(1 + n);
        let missile = byte(0) >> (2 * n);
        let start = 80 * n;
        for bit in 0..8 {
            if player >> (7 - bit) & 1 != 0 {
                pixels[start + 8 * bit..start + 8 * bit + 8].fill(1 << n);
            }
        }
        for bit in 0..2 {
            if missile >> (1 - bit) & 1 != 0 {
                let left = start + 64 + 8 * bit;
                pixels[left..left + 8].fill(1 << n);
            }
        }
    }
    pixels
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blank() -> alloc::vec::Vec<u8> {
        let mut data = alloc::vec![0u8; LEN];
        data[..9].copy_from_slice(&[0x12, 0x24, 0x38, 0x46, 0x5a, 0x6c, 0x7e, 0x8a, 0x9c]);
        data
    }

    #[test]
    fn character_sets_change_every_third_row() {
        let mut data = blank();
        // Character 1 of set 1 (rows 3-5) is solid playfield 1.
        data[FIRST_CHARSET + CHARSET_LEN + 8..FIRST_CHARSET + CHARSET_LEN + 16].fill(0xaa);
        data[SCREEN + 3 * 40] = 1;
        data[SCREEN] = 1;
        let image = decode_mcs(&data).unwrap();
        assert_eq!(image.get(0, 3 * 8), register_rgb(0x6c));
        assert_eq!(image.get(0, 0), register_rgb(0x9c));
    }

    #[test]
    fn objects_sit_behind_the_playfield() {
        let mut data = blank();
        data[OBJECTS + OBJECT_LEN + 16] = 0x80; // player 0, leftmost bit
        data[OBJECTS + 16] = 0b0100; // missile 1: bit 0 of its pair is the right half
        let image = decode_mcs(&data).unwrap();
        assert_eq!(image.get(0, 0), register_rgb(0x12));
        assert_eq!(image.get(8, 0), register_rgb(0x9c));
        assert_eq!(image.get(80 + 64 + 8, 0), register_rgb(0x24));
        assert!(decode_mcs(&data[1..]).is_err());
    }
}
