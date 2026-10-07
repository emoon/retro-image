//! MAP pictures: screen areas saved by BASIC 128/512 `SAVEP` (after `GET`)
//! and by drawing programs, compressed column by column.
//!
//! Sources:
//! - Layout: Prehisto, "Les fichiers graphiques Thomson" (ContacThoms
//!   bulletin article, Collection Thomson),
//!   <http://web.archive.org/web/20251005163132/http://collection.thomson.free.fr/code/articles/prehisto_bulletin/page.php?XI=0&XJ=13>:
//!   - DOS binary file record: `00`, 16-bit big-endian data length, load
//!     address `0000`, the data, then `FF 00 00 00 00`;
//!   - data: display mode (`00` 40 columns or bitmap 4, `40` bitmap 16,
//!     `80` 80 columns), columns - 1, character rows (8 lines) - 1;
//!   - each column is packed top to bottom, columns left to right, runs
//!     carrying on into the next column: `n v` repeats `v` n times
//!     (1-255), `00 n` is followed by n literal bytes. In 40 columns and
//!     bitmap 4 RAMA is packed, then RAMB, each closed by two zero bytes;
//!     in bitmap 16 and 80 columns the columns alternate RAMA, RAMB and the
//!     whole is closed by four zero bytes. The article's reference
//!     unpacker (DECMAP, 6809 assembly) stops as soon as the area is full
//!     and reads a literal count of 0 as 256;
//!   - optional trailers before the closing record: TO-SNAP (40 bytes:
//!     `SCRMOD` word (`0001` for bitmap 4), border color, BASIC `CONSOLE`
//!     mode, 16 palette words for colors 0-15, marker `A55A`) and "PPM"
//!     (36 bytes: 16 palette words for colors 15-0, `CONSOLE` mode,
//!     marker `HL`). A negative palette word `w` stands for `-(w + 1)`.
//! - One more byte may sit between the closing zeros and the record end:
//!   `SAVEP` saves an integer array, so the data length is even (observed
//!   in samples from the Teo-Drive disk magazines on dcmoto.free.fr).
//! - No TO-SNAP or PPM sample was found, so the trailer offsets follow the
//!   article's table unverified.

use alloc::vec::Vec;

use super::palette::{DEFAULT_PALETTE, to_rgb};
use super::video::{self, Columns};
use crate::bytes::be16;
use crate::{Companions, DecodeError, Image};

/// Bytes around the data: the opening and closing binary file records.
const RECORD_LEN: usize = 5;
const CLOSING: [u8; RECORD_LEN] = [0xff, 0, 0, 0, 0];
const TO_SNAP_LEN: usize = 40;
const PPM_LEN: usize = 36;
/// The even-length padding byte that may follow the packed screen.
const MAX_PAD: usize = 1;

/// An unpacked MAP picture: its screen banks, by display mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Screen {
    Columns40 {
        rama: Vec<u8>,
        ramb: Vec<u8>,
    },
    Bitmap4 {
        rama: Vec<u8>,
        ramb: Vec<u8>,
    },
    /// Columns alternating RAMA, RAMB.
    Bitmap16(Vec<u8>),
    /// Columns alternating RAMA, RAMB.
    Columns80(Vec<u8>),
}

/// A parsed MAP file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Map {
    pub(super) screen: Screen,
    pub(super) lines: usize,
    /// The palette from a TO-SNAP or PPM trailer.
    pub(super) palette: Option<[u16; 16]>,
}

impl Map {
    /// Draws the picture with `palette` (12-bit `0BGR` values).
    pub(super) fn render(&self, palette: &[u16; 16]) -> Result<Image, DecodeError> {
        let palette = to_rgb(palette);
        let lines = self.lines;
        let columns = |bytes| Columns { bytes, lines };
        match &self.screen {
            Screen::Columns40 { rama, ramb } => {
                Ok(video::columns40(columns(rama), columns(ramb), &palette)?)
            }
            Screen::Bitmap4 { rama, ramb } => {
                Ok(video::bitmap4(columns(rama), columns(ramb), &palette)?)
            }
            Screen::Bitmap16(bytes) => video::bitmap16(columns(bytes), &palette),
            Screen::Columns80(bytes) => video::columns80(columns(bytes), &palette),
        }
    }
}

/// A MAP file, with the palette of its trailer, else of a Graffiti `.DST`
/// companion, else the default one.
pub(super) fn decode_map(data: &[u8], companions: &dyn Companions) -> Result<Image, DecodeError> {
    let map = parse(data)?;
    let palette = map
        .palette
        .or_else(|| super::graffiti::companion_palette(companions, "dst"))
        .unwrap_or(DEFAULT_PALETTE);
    map.render(&palette)
}

/// Parses a MAP file, checking the whole structure.
pub(super) fn parse(data: &[u8]) -> Result<Map, DecodeError> {
    let body = binary_record(data).ok_or(DecodeError::Unrecognized)?;
    let [mode, columns, rows, packed @ ..] = body else {
        return Err(DecodeError::Unrecognized);
    };
    let (columns, rows) = (usize::from(*columns) + 1, usize::from(*rows) + 1);
    let max_columns = match mode {
        0x00 => 40,
        0x40 | 0x80 => 80,
        _ => return Err(DecodeError::Unrecognized),
    };
    if columns > max_columns || rows > 25 {
        return Err(DecodeError::Unrecognized);
    }
    let lines = rows * 8;
    let len = columns * lines;
    let mut pos = 0;
    let mut bank = |closing| {
        let bytes = unpack(packed, &mut pos, len)?;
        let zeros = packed.get(pos..pos + closing)?;
        pos += closing;
        zeros.iter().all(|&b| b == 0).then_some(bytes)
    };
    let screen = match mode {
        0x00 => {
            let rama = bank(2).ok_or(DecodeError::Unrecognized)?;
            let ramb = bank(2).ok_or(DecodeError::Unrecognized)?;
            Screen::Columns40 { rama, ramb }
        }
        0x40 => Screen::Bitmap16(bank(4).ok_or(DecodeError::Unrecognized)?),
        _ => Screen::Columns80(bank(4).ok_or(DecodeError::Unrecognized)?),
    };
    let trailer = trailer(&packed[pos..])?;
    let screen = match (screen, trailer) {
        (Screen::Columns40 { rama, ramb }, Some(Trailer { bitmap4: true, .. })) => {
            Screen::Bitmap4 { rama, ramb }
        }
        (screen, _) => screen,
    };
    Ok(Map {
        screen,
        lines,
        palette: trailer.map(|t| t.palette),
    })
}

/// The data of a DOS binary file holding one record at address 0.
fn binary_record(data: &[u8]) -> Option<&[u8]> {
    let len = usize::from(be16(data, 1)?);
    let valid = data.len() == len + 2 * RECORD_LEN
        && data[0] == 0
        && data[3..5] == [0, 0]
        && data[RECORD_LEN + len..] == CLOSING;
    valid.then(|| &data[RECORD_LEN..RECORD_LEN + len])
}

/// Unpacks `len` bytes of run-length code from `src` at `*pos`, stopping as
/// soon as they are complete (the rest of a run or literal is dropped).
fn unpack(src: &[u8], pos: &mut usize, len: usize) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(len);
    while out.len() < len {
        let count = *src.get(*pos)?;
        let operand = *src.get(*pos + 1)?;
        *pos += 2;
        if count == 0 {
            let literal = if operand == 0 {
                256
            } else {
                usize::from(operand)
            };
            let take = literal.min(len - out.len());
            out.extend_from_slice(src.get(*pos..*pos + take)?);
            *pos += take;
        } else {
            let take = usize::from(count).min(len - out.len());
            out.resize(out.len() + take, operand);
        }
    }
    Some(out)
}

#[derive(Debug, Clone, Copy)]
struct Trailer {
    palette: [u16; 16],
    bitmap4: bool,
}

/// The TO-SNAP or PPM trailer in what follows the packed screen; anything
/// else but a padding byte is an error.
fn trailer(tail: &[u8]) -> Result<Option<Trailer>, DecodeError> {
    let ending = |len: usize, marker: &[u8; 2]| {
        let extra = tail.len().checked_sub(len)?;
        let found = &tail[extra..];
        (extra <= MAX_PAD && found.ends_with(marker)).then_some(found)
    };
    let word = |bytes: &[u8], at: usize| be16(bytes, at).unwrap_or(0);
    if let Some(found) = ending(TO_SNAP_LEN, &[0xa5, 0x5a]) {
        return Ok(Some(Trailer {
            palette: core::array::from_fn(|i| palette_value(word(found, 6 + 2 * i))),
            bitmap4: word(found, 0) == 1,
        }));
    }
    if let Some(found) = ending(PPM_LEN, b"HL") {
        return Ok(Some(Trailer {
            palette: core::array::from_fn(|i| palette_value(word(found, 2 * (15 - i)))),
            bitmap4: false,
        }));
    }
    if tail.len() <= MAX_PAD {
        Ok(None)
    } else {
        Err(DecodeError::Unrecognized)
    }
}

/// A trailer palette word as a 12-bit value; negative `w` means `-(w + 1)`.
fn palette_value(word: u16) -> u16 {
    let value = word as i16;
    (if value < 0 { !value } else { value }) as u16 & 0xfff
}

/// Wraps MAP data in the binary file records.
#[cfg(test)]
pub(super) fn file(data: &[u8]) -> Vec<u8> {
    let mut out = alloc::vec![0, (data.len() >> 8) as u8, data.len() as u8, 0, 0];
    out.extend_from_slice(data);
    out.extend_from_slice(&CLOSING);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    /// The article's 40-column example: a 2x8 box outline, red on black.
    const EXAMPLE_40: [u8; 29] = [
        0x00, 0x01, 0x00, // mode, 2 columns, 1 row
        0x00, 0x02, 0x00, 0x7f, 0x04, 0x40, 0x00, 0x01, 0x7f, 0x02, 0x00, 0x00, 0x01, 0xfe, 0x04,
        0x02, 0x00, 0x02, 0xfe, 0x00, 0x00, 0x00, // RAMA
        0x10, 0xc8, 0x00, 0x00, // RAMB
    ];

    #[test]
    fn runs_cross_columns_and_literals_follow_zero() {
        let mut pos = 0;
        let packed = &EXAMPLE_40[3..];
        let rama = unpack(packed, &mut pos, 16).unwrap();
        assert_eq!(
            rama,
            [
                0x00, 0x7f, 0x40, 0x40, 0x40, 0x40, 0x7f, 0x00, // column 0
                0x00, 0xfe, 0x02, 0x02, 0x02, 0x02, 0xfe, 0x00, // column 1
            ]
        );
        assert_eq!(pos, 20);
    }

    #[test]
    fn literal_count_zero_means_256() {
        let mut src = vec![0, 0];
        src.extend(0..=255);
        let mut pos = 0;
        let out = unpack(&src, &mut pos, 256).unwrap();
        assert_eq!(out[255], 255);
        assert_eq!(pos, 258);
    }

    #[test]
    fn decodes_the_article_example() {
        let map = parse(&file(&EXAMPLE_40)).unwrap();
        assert_eq!(map.palette, None);
        let image = map.render(&DEFAULT_PALETTE).unwrap();
        assert_eq!((image.width(), image.height()), (16, 8));
        assert_eq!(image.get(0, 0), 0x000000);
        assert_eq!(image.get(1, 1), 0xff0000); // 0x7f: forme bit 1 at x = 1
        assert_eq!(image.get(0, 1), 0x000000);
        assert_eq!(image.get(14, 1), 0xff0000); // 0xfe in column 1
    }

    #[test]
    fn interleaved_modes_alternate_banks() {
        // Bitmap 16, 2 columns (RAMA, RAMB), 8 lines: RAMA 0x12, RAMB 0x34.
        let data = [0x40, 0x01, 0x00, 0x08, 0x12, 0x08, 0x34, 0, 0, 0, 0];
        let map = parse(&file(&data)).unwrap();
        assert_eq!(
            map.screen,
            Screen::Bitmap16([[0x12; 8], [0x34; 8]].concat())
        );
        let image = map.render(&DEFAULT_PALETTE).unwrap();
        assert_eq!((image.width(), image.height()), (8, 8));
        assert_eq!(image.get(2, 0), 0x00ff00); // color 2
        assert_eq!(image.get(6, 0), 0x0000ff); // color 4
    }

    #[test]
    fn to_snap_trailer_sets_palette_and_bitmap4() {
        let mut data = EXAMPLE_40.to_vec();
        data.push(0); // padding
        let mut trailer = vec![0x00, 0x01, 0x00, 0x00, 0x00, 0x02];
        for i in 0..16u16 {
            trailer.extend_from_slice(&(i * 0x111).to_be_bytes());
        }
        trailer[6 + 2 * 15..6 + 2 * 16].copy_from_slice(&(!0xfffu16).to_be_bytes());
        trailer.extend_from_slice(&[0xa5, 0x5a]);
        data.extend_from_slice(&trailer);
        let map = parse(&file(&data)).unwrap();
        let palette = map.palette.unwrap();
        assert_eq!(palette[1], 0x111);
        assert_eq!(palette[15], 0xfff);
        assert!(matches!(map.screen, Screen::Bitmap4 { .. }));
    }

    #[test]
    fn ppm_trailer_palette_is_descending() {
        let mut data = EXAMPLE_40.to_vec();
        for i in (0..16u16).rev() {
            data.extend_from_slice(&i.to_be_bytes());
        }
        data.extend_from_slice(&[0x00, 0x00, b'H', b'L']);
        let map = parse(&file(&data)).unwrap();
        assert_eq!(map.palette.unwrap()[3], 3);
        assert!(matches!(map.screen, Screen::Columns40 { .. }));
    }

    #[test]
    fn rejects_damaged_files() {
        let good = file(&EXAMPLE_40);
        assert!(parse(&good).is_ok());
        // Unknown trailing bytes.
        let mut data = EXAMPLE_40.to_vec();
        data.extend_from_slice(&[1, 2, 3]);
        assert!(parse(&file(&data)).is_err());
        // Length field disagreeing with the file.
        let mut bad = good.clone();
        bad[2] ^= 1;
        assert!(parse(&bad).is_err());
        // Unknown mode, oversized area.
        for (at, value) in [(5, 0x20), (6, 40), (7, 25)] {
            let mut bad = good.clone();
            bad[at] = value;
            assert!(parse(&bad).is_err(), "byte {at} = {value}");
        }
        // Missing closing zeros after RAMA.
        let mut bad = good.clone();
        bad[5 + 23] = 1;
        assert!(parse(&bad).is_err());
        for len in 0..good.len() {
            assert!(parse(&good[..len]).is_err());
        }
    }
}
