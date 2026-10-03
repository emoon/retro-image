//! C64 text-mode (PETSCII) pictures: C64 OS screenshots (`.pet` versions
//! 0-2), PETSCII Editor screens (`.pet`, and `.scr` with a `.col`
//! companion) and PETSCII BOTs (`.pbot`).
//!
//! Sources:
//! - Greg Naçu, "Image File Formats", <https://c64os.com/post/imageformats>:
//!   C64 OS `.pet` (`PET` in PETSCII and a version digit, three 17-byte
//!   strings, 1000 screen codes, 1000 colours, border, background; version
//!   0 uses the ROM upper case/graphics set, version 1 the lower/upper case
//!   set, version 2 carries a 2048-byte set) and PETSCII BOT (70 or 384
//!   bytes: colours then screen codes for 5×7 or 12×16 characters, ROM upper
//!   case/graphics set).
//! - Text mode semantics (set character pixels in the colour RAM colour,
//!   clear ones in the background): <https://www.cebix.net/VIC-Article.txt>.
//! - PETSCII Editor `.pet`: reverse engineered from samples. A memory dump
//!   of `$3000-$37E7`: screen codes at `$3000`, the background colour at
//!   `$33E9`, colour RAM at `$3400`. `.scr`/`.col`: the screen codes and the
//!   colours as two 1002-byte files. Character set (upper case/graphics),
//!   black background for `.scr`+`.col` and PETSCII BOT, and the accepted
//!   sizes were checked against `recoil2png` output.
//! - PetDraw64 `.pdr` (2029 bytes): reverse engineered from five CSDb
//!   samples and the pictures on the PetDraw disk by black-box probing of
//!   `recoil2png`. Two load address bytes, three header bytes of which only
//!   the second (background colour, low nibble) matters, 1000 screen codes,
//!   24 ignored bytes, 1000 colours. Upper case/graphics set.
//! - Self-displaying PETSCII PRG (`SYS 2061` stub, 2098 bytes): reverse
//!   engineered from 1,882 CSDb samples by reading the stub's operands (a
//!   copy loop from `$0861` to screen RAM and from `$0C49` to colour RAM,
//!   `$D018`, `$D020` and `$D021` immediates); no decoder code was used.
//! - Character glyphs: the C64 character ROM, see [`CHARGEN`].

use super::vic2::{self, SCREEN_LEN};
use crate::{Companions, DecodeError, Image};
use alloc::vec::Vec;

/// The C64 character ROM (part 901225-01, 4096 bytes; MD5
/// `12a4202f5331d45af846af6c58fba946`): the upper case/graphics set, then
/// the lower/upper case set, each with its reversed characters in the upper
/// half. Taken from the ROM dump `chargen` in `1.4.0/Firmware.zip` of the
/// archive.org item `bizhawk-firmware-files_20250516`
/// (<https://archive.org/details/bizhawk-firmware-files_20250516>).
///
/// The reversed `@` (screen code `$80`) is not an exact inverse of `@` in
/// this ROM (row 5 is `$99`, not `$9D`); the VIC-II shows the ROM as is.
pub(super) static CHARGEN: &[u8; 4096] = include_bytes!("chargen-901225-01.bin");

/// Which ROM character set a picture uses.
#[derive(Clone, Copy)]
enum RomCharset {
    UpperGraphics,
    LowerUpper,
}

impl RomCharset {
    fn glyphs(self) -> &'static [u8] {
        match self {
            Self::UpperGraphics => &CHARGEN[..2048],
            Self::LowerUpper => &CHARGEN[2048..],
        }
    }
}

/// A character screen: `columns`×`rows` screen codes and colours.
struct TextScreen<'a> {
    columns: usize,
    rows: usize,
    screen: &'a [u8],
    colors: &'a [u8],
    background: u8,
    /// 8 bytes per character, 256 characters.
    charset: &'a [u8],
}

impl TextScreen<'_> {
    fn render(&self) -> Result<Image, DecodeError> {
        let cells = self.columns * self.rows;
        if self.screen.len() < cells || self.colors.len() < cells || self.charset.len() < 2048 {
            return Err(DecodeError::Unrecognized);
        }
        let (width, height) = (self.columns * 8, self.rows * 8);
        let mut pixels = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                let cell = y / 8 * self.columns + x / 8;
                let glyph = usize::from(self.screen[cell]);
                let set = self.charset[glyph * 8 + y % 8] & (0x80 >> (x % 8)) != 0;
                pixels.push(if set {
                    self.colors[cell]
                } else {
                    self.background
                });
            }
        }
        Ok(vic2::image(width, height, pixels))
    }
}

/// A full 40×25 screen.
fn full_screen<'a>(
    screen: &'a [u8],
    colors: &'a [u8],
    background: u8,
    charset: &'a [u8],
) -> TextScreen<'a> {
    TextScreen {
        columns: 40,
        rows: 25,
        screen,
        colors,
        background,
        charset,
    }
}

const C64OS_HEADER: usize = 4 + 3 * 17;
const C64OS_COLORS: usize = C64OS_HEADER + SCREEN_LEN;
const C64OS_BACKGROUND: usize = C64OS_COLORS + SCREEN_LEN + 1;
const C64OS_CHARSET: usize = C64OS_BACKGROUND + 1;

/// C64 OS screenshot, versions 0 and 1 (ROM character set) and 2 (own set).
pub(super) fn decode_c64os(data: &[u8]) -> Result<Image, DecodeError> {
    let (magic, version) = (data.get(..3), data.get(3));
    if magic != Some(&[0xd0, 0xc5, 0xd4]) {
        return Err(DecodeError::Unrecognized);
    }
    let charset = match (version, data.len()) {
        (Some(b'0'), C64OS_CHARSET) => RomCharset::UpperGraphics.glyphs(),
        (Some(b'1'), C64OS_CHARSET) => RomCharset::LowerUpper.glyphs(),
        (Some(b'2'), len) if len == C64OS_CHARSET + 2048 => &data[C64OS_CHARSET..],
        _ => return Err(DecodeError::Unrecognized),
    };
    full_screen(
        &data[C64OS_HEADER..],
        &data[C64OS_COLORS..],
        data[C64OS_BACKGROUND],
        charset,
    )
    .render()
}

/// PETSCII Editor: load address `$3000`, screen codes, 24 bytes holding
/// the border and background colours, colour RAM from `$3400`.
pub(super) fn decode_petscii_editor(data: &[u8]) -> Result<Image, DecodeError> {
    const COLORS: usize = 2 + 0x400;
    if data.len() != COLORS + SCREEN_LEN {
        return Err(DecodeError::Unrecognized);
    }
    full_screen(
        &data[2..],
        &data[COLORS..],
        data[2 + 0x3e9],
        RomCharset::UpperGraphics.glyphs(),
    )
    .render()
}

/// PETSCII Editor screen codes (`.scr`) with their colours in the `.col`
/// companion; both have a load address. Without the colours there is no
/// picture to show.
pub(super) fn decode_scr_col(
    data: &[u8],
    companions: &dyn Companions,
) -> Result<Image, DecodeError> {
    const LEN: usize = 2 + SCREEN_LEN;
    if data.len() != LEN {
        return Err(DecodeError::Unrecognized);
    }
    let colors = companions
        .get("col")
        .filter(|colors| colors.len() == LEN)
        .ok_or(DecodeError::Unrecognized)?;
    full_screen(
        &data[2..],
        &colors[2..],
        0,
        RomCharset::UpperGraphics.glyphs(),
    )
    .render()
}

/// PetDraw64: load address, three header bytes (background colour second),
/// screen codes, 24 unused bytes, colour RAM.
pub(super) fn decode_petdraw(data: &[u8]) -> Result<Image, DecodeError> {
    const SCREEN: usize = 5;
    const COLORS: usize = SCREEN + SCREEN_LEN + 24;
    if data.len() != COLORS + SCREEN_LEN {
        return Err(DecodeError::Unrecognized);
    }
    full_screen(
        &data[SCREEN..],
        &data[COLORS..],
        data[3],
        RomCharset::UpperGraphics.glyphs(),
    )
    .render()
}

/// The part of the PETSCII PRG stub after the background colour: the loop
/// that copies the screen and colour data and starts the program.
const PRG_STUB_TAIL: [u8; 67] = [
    0x8d, 0x21, 0xd0, 0xa2, 0x00, 0xa0, 0xfa, 0xbd, 0x61, 0x08, 0x9d, 0x00, 0x04, 0xbd, 0x5b, 0x09,
    0x9d, 0xfa, 0x04, 0xbd, 0x55, 0x0a, 0x9d, 0xf4, 0x05, 0xbd, 0x4f, 0x0b, 0x9d, 0xee, 0x06, 0xbd,
    0x49, 0x0c, 0x9d, 0x00, 0xd8, 0xbd, 0x43, 0x0d, 0x9d, 0xfa, 0xd8, 0xbd, 0x3d, 0x0e, 0x9d, 0xf4,
    0xd9, 0xbd, 0x37, 0x0f, 0x9d, 0xee, 0xda, 0xe8, 0x88, 0xd0, 0xcc, 0xa9, 0x1b, 0x8d, 0x11, 0xd0,
    0x4c, 0x5e, 0x08,
];

/// Self-displaying PETSCII program (as exported by PETSCII editors): a
/// BASIC `SYS 2061` line, a 98-byte machine code stub that sets `$D018`
/// (character set), `$D020` and `$D021` and copies the data, then 1000
/// screen codes and 1000 colour RAM bytes. The BASIC line number varies.
pub(super) fn decode_petscii_prg(data: &[u8]) -> Result<Image, DecodeError> {
    const SCREEN: usize = 98;
    const COLORS: usize = SCREEN + SCREEN_LEN;
    const D018: usize = 20;
    const BACKGROUND: usize = 30;
    let stub_ok = data.len() == COLORS + SCREEN_LEN
        && data[..4] == [0x01, 0x08, 0x0b, 0x08]
        && data[6..14] == *b"\x9e2061\0\0\0"
        && (data[14], data[19]) == (0xa9, 0xa9)
        && data[21..25] == [0x8d, 0x18, 0xd0, 0xa9]
        && data[26..30] == [0x8d, 0x20, 0xd0, 0xa9]
        && data[31..SCREEN] == PRG_STUB_TAIL;
    if !stub_ok || data[COLORS..].iter().any(|&c| c > 15) || data[BACKGROUND] > 15 {
        return Err(DecodeError::Unrecognized);
    }
    // `$D018` selects the character ROM: `$1000` upper case/graphics,
    // `$1800` lower/upper case.
    let charset = match data[D018] {
        0x14 => RomCharset::UpperGraphics,
        0x17 => RomCharset::LowerUpper,
        _ => return Err(DecodeError::Unrecognized),
    };
    full_screen(
        &data[SCREEN..],
        &data[COLORS..],
        data[BACKGROUND],
        charset.glyphs(),
    )
    .render()
}

/// PETSCII BOT: 5×7 or 12×16 colours, then as many screen codes.
pub(super) fn decode_pbot(data: &[u8]) -> Result<Image, DecodeError> {
    let (columns, rows) = match data.len() {
        70 => (5, 7),
        384 => (12, 16),
        _ => return Err(DecodeError::Unrecognized),
    };
    let (colors, screen) = data.split_at(columns * rows);
    TextScreen {
        columns,
        rows,
        screen,
        colors,
        background: 0,
        charset: RomCharset::UpperGraphics.glyphs(),
    }
    .render()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NoCompanions;

    #[test]
    fn rom_has_known_glyphs() {
        // `@` and its reversed form with the ROM's row-5 quirk.
        assert_eq!(
            &CHARGEN[..8],
            &[0x3c, 0x66, 0x6e, 0x6e, 0x60, 0x62, 0x3c, 0x00]
        );
        assert_eq!(CHARGEN[0x80 * 8 + 5], 0x99);
        // Lower case `a` (screen code 1 of the second set).
        assert_eq!(
            &CHARGEN[2048 + 8..2048 + 16],
            &[0x00, 0x00, 0x3c, 0x06, 0x3e, 0x66, 0x3e, 0x00]
        );
    }

    #[test]
    fn pbot_draws_colours_then_screen_codes() {
        let mut data = [0u8; 70];
        data[0] = 1; // white
        data[35] = 0xa0; // reversed space: all pixels set
        let image = decode_pbot(&data).unwrap();
        assert_eq!((image.width(), image.height()), (40, 56));
        assert_eq!(image.get(0, 0), 0xffffff);
        assert_eq!(image.get(8, 0), 0x000000);
    }

    struct Colors(Vec<u8>);

    impl Companions for Colors {
        fn get(&self, extension: &str) -> Option<Vec<u8>> {
            (extension == "col").then(|| self.0.clone())
        }
    }

    #[test]
    fn scr_needs_its_colours() {
        let mut scr = alloc::vec![0x20u8; 1002];
        scr[2] = 0xa0;
        assert!(decode_scr_col(&scr, &NoCompanions).is_err());
        let mut col = alloc::vec![0u8; 1002];
        col[2] = 2;
        let image = decode_scr_col(&scr, &Colors(col)).unwrap();
        assert_eq!(image.get(0, 0), 0x68372b);
        assert!(decode_scr_col(&scr, &Colors(alloc::vec![0; 1000])).is_err());
    }

    #[test]
    fn prg_stub_is_checked_and_charset_selected() {
        let mut data = alloc::vec![0u8; 2098];
        data[..14].copy_from_slice(&[
            0x01, 0x08, 0x0b, 0x08, 0xef, 0x00, 0x9e, b'2', b'0', b'6', b'1', 0, 0, 0,
        ]);
        data[14] = 0xa9;
        data[19] = 0xa9;
        data[20] = 0x17;
        data[21..25].copy_from_slice(&[0x8d, 0x18, 0xd0, 0xa9]);
        data[26..30].copy_from_slice(&[0x8d, 0x20, 0xd0, 0xa9]);
        data[31..98].copy_from_slice(&PRG_STUB_TAIL);
        data[98] = 1; // `a` in the lower case set
        data[1098] = 1;
        let image = decode_petscii_prg(&data).unwrap();
        assert_eq!((image.width(), image.height()), (320, 200));
        assert_eq!(image.get(2, 2), 0xffffff);
        data[20] = 0x14;
        assert_eq!(decode_petscii_prg(&data).unwrap().get(1, 2), 0xffffff);
        data[20] = 0x15;
        assert!(decode_petscii_prg(&data).is_err());
        data[20] = 0x14;
        data[1098] = 16;
        assert!(decode_petscii_prg(&data).is_err());
    }

    #[test]
    fn c64os_versions_pick_charsets() {
        let mut data = alloc::vec![0u8; C64OS_CHARSET];
        data[..4].copy_from_slice(&[0xd0, 0xc5, 0xd4, b'1']);
        data[C64OS_HEADER] = 1; // `a` in the lower case set
        data[C64OS_COLORS] = 1;
        let image = decode_c64os(&data).unwrap();
        // Row 2 of lower case `a` is $3C: pixels 2-5 set.
        assert_eq!(image.get(2, 2), 0xffffff);
        assert_eq!(image.get(1, 2), 0x000000);
        data[3] = b'0';
        // Upper case `A` row 2 is $66.
        assert_eq!(decode_c64os(&data).unwrap().get(1, 2), 0xffffff);
        data[3] = b'2';
        assert!(decode_c64os(&data).is_err());
    }
}
