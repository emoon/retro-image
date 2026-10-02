//! Graffiti (Interlude, Free Game Blot) pictures: MAP files named by
//! display mode, each with a text palette file.
//!
//! Sources:
//! - Extensions (`.MAP`+`.DST` 40 columns, `.M16`+`.D16` bitmap 16,
//!   `.M04`+`.D04` bitmap 4, `.M02`+`.D02` 80 columns) and the palette
//!   file, read in BASIC as `INPUT #1,A,B` then 16 times
//!   `INPUT #1,B,BR,V,VR,R,PR: PALETTE I,B+V+R`: Prehisto, "Les fichiers
//!   graphiques Thomson" (ContacThoms bulletin article, Collection
//!   Thomson),
//!   <http://web.archive.org/web/20251005163132/http://collection.thomson.free.fr/code/articles/prehisto_bulletin/page.php?XI=0&XJ=13>.
//! - The numbers are blank-separated ASCII; the first two are the picture
//!   width and height - 1; in each colour B, V (green) and R are the
//!   channel already shifted into place (`B` a multiple of 256, `V` of 16):
//!   observed in `GARDEN.D16` on the Graffiti disk,
//!   <http://dcmoto.free.fr/programmes/graffiti/index.html>.

use super::map::{self, Screen};
use super::palette::DEFAULT_PALETTE;
use crate::{Companions, DecodeError, Image};

const COLORS: usize = 16;
const FIELDS_PER_COLOR: usize = 6;

/// The palette in a Graffiti palette file, if it is one.
pub(super) fn parse_palette(text: &[u8]) -> Option<[u16; 16]> {
    let text = core::str::from_utf8(text).ok()?;
    let mut numbers = text.split_ascii_whitespace().map(|n| n.parse::<u16>().ok());
    // Width and height - 1.
    numbers.next()??;
    numbers.next()??;
    let mut palette = [0; COLORS];
    for entry in &mut palette {
        let mut fields = [0; FIELDS_PER_COLOR];
        for field in &mut fields {
            *field = numbers.next()??;
        }
        let [blue, _, green, _, red, _] = fields;
        let valid = blue % 0x100 == 0 && blue <= 0xf00 && green % 0x10 == 0 && green <= 0xf0;
        if !valid || red > 0xf {
            return None;
        }
        *entry = blue | green | red;
    }
    numbers.next().is_none().then_some(palette)
}

/// The palette in the companion file with `extension`, if it parses.
pub(super) fn companion_palette(companions: &dyn Companions, extension: &str) -> Option<[u16; 16]> {
    parse_palette(&companions.get(extension)?)
}

/// A `.M16` bitmap 16 picture.
pub(super) fn decode_m16(data: &[u8], companions: &dyn Companions) -> Result<Image, DecodeError> {
    decode(data, companions, "d16", |screen| {
        matches!(screen, Screen::Bitmap16(_)).then_some(screen)
    })
}

/// A `.M04` bitmap 4 picture: its mode byte says 40 columns, which shares
/// the byte with bitmap 4.
pub(super) fn decode_m04(data: &[u8], companions: &dyn Companions) -> Result<Image, DecodeError> {
    decode(data, companions, "d04", |screen| match screen {
        Screen::Columns40 { rama, ramb } | Screen::Bitmap4 { rama, ramb } => {
            Some(Screen::Bitmap4 { rama, ramb })
        }
        _ => None,
    })
}

/// A `.M02` 80-column picture.
pub(super) fn decode_m02(data: &[u8], companions: &dyn Companions) -> Result<Image, DecodeError> {
    decode(data, companions, "d02", |screen| {
        matches!(screen, Screen::Columns80(_)).then_some(screen)
    })
}

/// A MAP file whose screen `expect` accepts (and may reinterpret), with the
/// palette of its trailer, else of the companion, else the default one.
fn decode(
    data: &[u8],
    companions: &dyn Companions,
    palette_extension: &str,
    expect: impl FnOnce(Screen) -> Option<Screen>,
) -> Result<Image, DecodeError> {
    let mut map = map::parse(data)?;
    map.screen = expect(map.screen).ok_or(DecodeError::Unrecognized)?;
    let palette = map
        .palette
        .or_else(|| companion_palette(companions, palette_extension))
        .unwrap_or(DEFAULT_PALETTE);
    Ok(map.render(&palette))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;
    use alloc::string::String;
    use alloc::vec::Vec;

    fn palette_text(colors: &[(u16, u16, u16)]) -> String {
        let mut text = String::from(" 159           199 \r\n");
        for &(blue, green, red) in colors {
            text += &format!(" {}  256  {}  16  {}  1 \r\n", blue * 256, green * 16, red);
        }
        text
    }

    struct Palette(Vec<u8>);

    impl Companions for Palette {
        fn get(&self, extension: &str) -> Option<Vec<u8>> {
            (extension == "d16").then(|| self.0.clone())
        }
    }

    #[test]
    fn palette_is_blue_plus_green_plus_red() {
        let mut colors = [(0, 0, 0); 16];
        colors[1] = (15, 1, 0);
        colors[2] = (0, 7, 15);
        let palette = parse_palette(palette_text(&colors).as_bytes()).unwrap();
        assert_eq!(palette[..3], [0x000, 0xf10, 0x07f]);
    }

    #[test]
    fn palette_rejects_bad_files() {
        let colors = [(0, 0, 0); 16];
        let good = palette_text(&colors);
        assert!(parse_palette(good.as_bytes()).is_some());
        assert!(parse_palette(palette_text(&colors[..15]).as_bytes()).is_none());
        assert!(parse_palette(format!("{good} 1").as_bytes()).is_none());
        assert!(parse_palette(good.replace(" 256 ", " x ").as_bytes()).is_none());
        // A channel out of place.
        assert!(parse_palette(good.replacen(" 0  256", " 16  256", 1).as_bytes()).is_none());
    }

    #[test]
    fn m16_uses_companion_palette_and_checks_mode() {
        // Bitmap 16, 2 columns of colour 1 pixels.
        let data = map::file(&[0x40, 0x01, 0x00, 0x10, 0x11, 0, 0, 0, 0]);
        let mut colors = [(0, 0, 0); 16];
        colors[1] = (15, 0, 0);
        let image = decode_m16(&data, &Palette(palette_text(&colors).into_bytes())).unwrap();
        assert_eq!(image.get(0, 0), 0x0000ff);
        let alone = decode_m16(&data, &crate::NoCompanions).unwrap();
        assert_eq!(alone.get(0, 0), 0xff0000); // default colour 1: red
        assert!(decode_m02(&data, &crate::NoCompanions).is_err());
        assert!(decode_m04(&data, &crate::NoCompanions).is_err());
    }

    #[test]
    fn m04_reads_40_column_data_as_bitmap4() {
        // 1 column, 8 lines: RAMA 0x80, RAMB 0x40.
        let data = map::file(&[0x00, 0x00, 0x00, 0x08, 0x80, 0, 0, 0x08, 0x40, 0, 0]);
        let image = decode_m04(&data, &crate::NoCompanions).unwrap();
        assert_eq!(image.get(0, 0), 0x00ff00); // colour 2
        assert_eq!(image.get(1, 0), 0xff0000); // colour 1
        assert_eq!(image.get(2, 0), 0x000000);
    }
}
