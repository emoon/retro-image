//! C64 single-screen bitmap formats: one 320×200 bitmap with one screen RAM
//! (hires) or screen RAM, colour RAM and background colour (multicolour).
//!
//! Sources (memory maps):
//! - Codebase64 "C64 Graphics File Format Specs" (CB),
//!   <http://codebase.c64.org/doku.php?id=base:c64_grafix_files_specs_list_v0.03>
//! - Peter Schepers, "Standard C64 BITMAP files" (BT),
//!   <http://ist.uwaterloo.ca/~schepers/formats/BITMAP.TXT>
//! - GoDot loader pages (GD), <https://www.godot64.de/german/lstab.htm>
//!
//! | Format | Sources |
//! |---|---|
//! | Koala Painter, Koala compressed (GG), Run Paint, Interpaint lores, CDU-Paint, Create with Garfield | CB, BT, GD Koala/CDU |
//! | Amica Paint | CB, GD Amica |
//! | Wigmore Artist 64, Blazing Paddles, Vidcom 64, Image System multi | CB, BT, GD |
//! | Advanced Art Studio, Saracen Paint, Paint Magic, Drazpaint | CB, GD |
//! | Art Studio, Interpaint hires, Image System hires, Hi-Eddi, Doodle (DD, JJ) | CB, BT, GD |
//! | Hires-Bitmap (mono), Gigapaint hires, Giga-CAD, Mono Magic | <http://fileformats.archiveteam.org/wiki/Hires-Bitmap>, GD HiresBitmap, GD format table <https://www.godot64.de/german/formats.htm>; colours observed from `recoil2png` output |
//! | Micro Illustrator (uncompressed only) | GD MIllustr8or <https://www.godot64.de/german/l_millu.htm> |
//! | Picasso 64 | <http://fileformats.archiveteam.org/wiki/Picasso_64> (load `$1800`, size); Vidcom's layout at `$1800`, background at `$1FFF`: reverse engineered from 9 samples with different backgrounds and checked against `recoil2png` output |
//! | Cheese | <http://fileformats.archiveteam.org/wiki/Cheese> (size); load `$8000`, bitmap `$8000`, screen `$C200`, colour `$C800`, background `$CFFD`: reverse engineered from 3 samples and checked against `recoil2png` output (changing `$CFFD` changes its background) |
//! | Rainbow Painter | <http://fileformats.archiveteam.org/wiki/Rainbow_Painter> (size); load `$5C00`, screen `$5C00`, bitmap `$6000`, colour `$8000`: reverse engineered from 2 samples. No byte sets the background: `recoil2png` shows black whatever the unused bytes hold |
//! | Hi-Pic Creator | <http://fileformats.archiveteam.org/wiki/Hi-Pic_Creator> (size); bitmap-then-screen order checked against `recoil2png` output |

use super::prg::Prg;
use super::unpack::{Run, escape_rle};
use super::vic2::{BITMAP_LEN, Bitmap, Frame, SCREEN_LEN};
use crate::{DecodeError, Image};
use alloc::vec::Vec;

const HEIGHT: usize = 200;

/// Memory map of a multicolour picture.
pub(super) struct Multicolor {
    pub load: u16,
    /// Accepted file sizes, including the load address.
    pub sizes: &'static [usize],
    pub bitmap: u16,
    pub screen: u16,
    pub color: u16,
    pub background: u16,
}

impl Multicolor {
    pub(super) fn decode(&self, data: &[u8]) -> Result<Image, DecodeError> {
        if !self.sizes.contains(&data.len()) {
            return Err(DecodeError::Unrecognized);
        }
        self.decode_unchecked(data)
    }

    /// Decodes without checking the file size (for unpacked data).
    pub(super) fn decode_unchecked(&self, data: &[u8]) -> Result<Image, DecodeError> {
        let prg = Prg::new(data, self.load);
        self.frame(&prg)
            .map(|frame| frame.to_image(0))
            .ok_or(DecodeError::Unrecognized)
    }

    pub(super) fn frame(&self, prg: &Prg) -> Option<Frame> {
        let bitmap = Bitmap::multicolor(
            prg.at(self.bitmap, BITMAP_LEN)?,
            prg.at(self.screen, SCREEN_LEN)?,
            prg.at(self.color, SCREEN_LEN)?,
            prg.byte(self.background)?,
        );
        Frame::multicolor(&bitmap, HEIGHT)
    }
}

/// Memory map of a hires picture.
pub(super) struct Hires {
    pub load: u16,
    pub sizes: &'static [usize],
    pub bitmap: u16,
    pub screen: u16,
}

impl Hires {
    pub(super) fn decode(&self, data: &[u8]) -> Result<Image, DecodeError> {
        if !self.sizes.contains(&data.len()) {
            return Err(DecodeError::Unrecognized);
        }
        self.decode_unchecked(data)
    }

    pub(super) fn decode_unchecked(&self, data: &[u8]) -> Result<Image, DecodeError> {
        self.frame(&Prg::new(data, self.load))
            .map(|f| f.to_image(0))
            .ok_or(DecodeError::Unrecognized)
    }

    pub(super) fn frame(&self, prg: &Prg) -> Option<Frame> {
        let bitmap = Bitmap::hires(
            prg.at(self.bitmap, BITMAP_LEN)?,
            prg.at(self.screen, SCREEN_LEN)?,
        );
        Frame::hires(&bitmap, HEIGHT)
    }
}

/// Koala layout: bitmap, screen, colour, background, all consecutive.
const fn koala_at(load: u16, sizes: &'static [usize]) -> Multicolor {
    Multicolor {
        load,
        sizes,
        bitmap: load,
        screen: load + 8000,
        color: load + 9000,
        background: load + 10000,
    }
}

pub(super) const KOALA: Multicolor = koala_at(0x6000, &[10003]);
const RUN_PAINT: Multicolor = koala_at(0x6000, &[10003, 10006]);
const INTERPAINT_LORES: Multicolor = koala_at(0x4000, &[10003]);
const CREATE_WITH_GARFIELD: Multicolor = koala_at(0x8000, &[10007]);
const CDU_PAINT: Multicolor = Multicolor {
    load: 0x7eef,
    ..koala_at(0x8000, &[10277])
};
pub(super) const ARTIST_64: Multicolor = Multicolor {
    load: 0x4000,
    sizes: &[10242],
    bitmap: 0x4000,
    screen: 0x6000,
    color: 0x6400,
    background: 0x67ff,
};
pub(super) const BLAZING_PADDLES: Multicolor = Multicolor {
    load: 0xa000,
    sizes: &[10242],
    bitmap: 0xa000,
    screen: 0xc000,
    color: 0xc400,
    background: 0xbf80,
};
pub(super) const VIDCOM: Multicolor = Multicolor {
    load: 0x5800,
    sizes: &[10050],
    bitmap: 0x6000,
    screen: 0x5c00,
    color: 0x5800,
    background: 0x5fe8,
};
const PICASSO_64: Multicolor = Multicolor {
    load: 0x1800,
    sizes: &[10050],
    bitmap: 0x2000,
    screen: 0x1c00,
    color: 0x1800,
    background: 0x1fff,
};
const CHEESE: Multicolor = Multicolor {
    load: 0x8000,
    sizes: &[20482],
    bitmap: 0x8000,
    screen: 0xc200,
    color: 0xc800,
    background: 0xcffd,
};
const IMAGE_SYSTEM_MULTI: Multicolor = Multicolor {
    load: 0x3c00,
    sizes: &[10218],
    bitmap: 0x4000,
    screen: 0x6000,
    color: 0x3c00,
    background: 0x5fff,
};
pub(super) const ADVANCED_ART_STUDIO: Multicolor = Multicolor {
    load: 0x2000,
    sizes: &[10018],
    bitmap: 0x2000,
    screen: 0x3f40,
    color: 0x4338,
    background: 0x4329,
};
const SARACEN_PAINT: Multicolor = Multicolor {
    load: 0x7800,
    sizes: &[10219],
    bitmap: 0x7c00,
    screen: 0x7800,
    color: 0x9c00,
    background: 0x7bf0,
};
const DRAZPAINT: Multicolor = Multicolor {
    load: 0x5800,
    sizes: &[10051, 10241],
    bitmap: 0x6000,
    screen: 0x5c00,
    color: 0x5800,
    background: 0x7f40,
};

pub(super) const ART_STUDIO: Hires = Hires {
    load: 0x2000,
    sizes: &[9002, 9003, 9009],
    bitmap: 0x2000,
    screen: 0x3f40,
};
pub(super) const INTERPAINT_HIRES: Hires = Hires {
    load: 0x4000,
    sizes: &[9002],
    bitmap: 0x4000,
    screen: 0x5f40,
};
const IMAGE_SYSTEM_HIRES: Hires = Hires {
    load: 0x4000,
    sizes: &[9194],
    bitmap: 0x4000,
    screen: 0x6000,
};
pub(super) const HI_EDDI: Hires = Hires {
    load: 0x2000,
    sizes: &[9218],
    bitmap: 0x2000,
    screen: 0x4000,
};
const DOODLE: Hires = Hires {
    load: 0x5c00,
    sizes: &[9218],
    bitmap: 0x6000,
    screen: 0x5c00,
};

const HI_PIC_CREATOR: Hires = Hires {
    load: 0x6000,
    sizes: &[9003],
    bitmap: 0x6000,
    screen: 0x7f40,
};

pub(super) fn decode_hi_pic_creator(data: &[u8]) -> Result<Image, DecodeError> {
    HI_PIC_CREATOR.decode(data)
}

/// Paint Magic: display code, bitmap, background, one byte filling the
/// whole colour RAM, border, screen.
pub(super) fn decode_paint_magic(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 9332 {
        return Err(DecodeError::Unrecognized);
    }
    let prg = Prg::new(data, 0x3f8e);
    let frame = (|| {
        let color = [prg.byte(0x5f43)?; SCREEN_LEN];
        let bitmap = Bitmap::multicolor(
            prg.at(0x4000, BITMAP_LEN)?,
            prg.at(0x6000, SCREEN_LEN)?,
            &color,
            prg.byte(0x5f40)?,
        );
        Frame::multicolor(&bitmap, HEIGHT)
    })();
    frame
        .map(|f| f.to_image(0))
        .ok_or(DecodeError::Unrecognized)
}

/// Micro Illustrator, uncompressed: 22-byte header (load address, magic
/// `FF 80 69 67`, header length 20, compression 0, background, ...), then
/// screen, colour RAM and bitmap.
pub(super) fn decode_micro_illustrator(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 10022 || data[2..8] != [0xff, 0x80, 0x69, 0x67, 20, 0] {
        return Err(DecodeError::Unrecognized);
    }
    let bitmap = Bitmap::multicolor(&data[2022..], &data[22..], &data[1022..], data[8]);
    let frame = Frame::multicolor(&bitmap, HEIGHT).ok_or(DecodeError::Unrecognized)?;
    Ok(frame.to_image(0))
}

pub(super) fn decode_koala(data: &[u8]) -> Result<Image, DecodeError> {
    KOALA.decode(data)
}

pub(super) fn decode_run_paint(data: &[u8]) -> Result<Image, DecodeError> {
    RUN_PAINT.decode(data)
}

pub(super) fn decode_interpaint_lores(data: &[u8]) -> Result<Image, DecodeError> {
    INTERPAINT_LORES.decode(data)
}

pub(super) fn decode_create_with_garfield(data: &[u8]) -> Result<Image, DecodeError> {
    CREATE_WITH_GARFIELD.decode(data)
}

pub(super) fn decode_cdu_paint(data: &[u8]) -> Result<Image, DecodeError> {
    CDU_PAINT.decode(data)
}

pub(super) fn decode_artist_64(data: &[u8]) -> Result<Image, DecodeError> {
    ARTIST_64.decode(data)
}

pub(super) fn decode_blazing_paddles(data: &[u8]) -> Result<Image, DecodeError> {
    BLAZING_PADDLES.decode(data)
}

pub(super) fn decode_vidcom(data: &[u8]) -> Result<Image, DecodeError> {
    VIDCOM.decode(data)
}

pub(super) fn decode_picasso_64(data: &[u8]) -> Result<Image, DecodeError> {
    PICASSO_64.decode(data)
}

pub(super) fn decode_cheese(data: &[u8]) -> Result<Image, DecodeError> {
    CHEESE.decode(data)
}

/// Rainbow Painter: screen, bitmap and colour RAM from `$5C00`, on black.
pub(super) fn decode_rainbow_painter(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 10242 {
        return Err(DecodeError::Unrecognized);
    }
    let prg = Prg::new(data, 0x5c00);
    let bitmap = Bitmap::multicolor(
        prg.at(0x6000, BITMAP_LEN)
            .ok_or(DecodeError::Unrecognized)?,
        prg.at(0x5c00, SCREEN_LEN)
            .ok_or(DecodeError::Unrecognized)?,
        prg.at(0x8000, SCREEN_LEN)
            .ok_or(DecodeError::Unrecognized)?,
        0,
    );
    Frame::multicolor(&bitmap, HEIGHT)
        .map(|frame| frame.to_image(0))
        .ok_or(DecodeError::Unrecognized)
}

pub(super) fn decode_image_system_multi(data: &[u8]) -> Result<Image, DecodeError> {
    IMAGE_SYSTEM_MULTI.decode(data)
}

pub(super) fn decode_advanced_art_studio(data: &[u8]) -> Result<Image, DecodeError> {
    ADVANCED_ART_STUDIO.decode(data)
}

/// Saracen Paint: `$7800-$9FE8`. Some files stop at `$9F1F`, before the
/// last five rows of colour RAM; one such sample is a byte-exact prefix of
/// a full file whose missing tail is all zeros, so the tail is zero-filled.
pub(super) fn decode_saracen_paint(data: &[u8]) -> Result<Image, DecodeError> {
    const SHORT_LEN: usize = 2 + 0x9f20 - 0x7800;
    if data.len() == SHORT_LEN && data[..2] == [0x00, 0x78] {
        let mut full = data.to_vec();
        full.resize(SARACEN_PAINT.sizes[0], 0);
        return SARACEN_PAINT.decode(&full);
    }
    SARACEN_PAINT.decode(data)
}

pub(super) fn decode_drazpaint(data: &[u8]) -> Result<Image, DecodeError> {
    DRAZPAINT.decode(data)
}

/// Drazpaint packed: `DRAZPAINT 1.4` or `DRAZPAINT 2.0` header. Both
/// versions pack the same `$5800-$7F40` memory image (samples of each
/// unpack to exactly that length, using all their input).
pub(super) fn decode_drazpaint_packed(data: &[u8]) -> Result<Image, DecodeError> {
    let unpacked = draz_unpack(data, &[b"DRAZPAINT 1.4", b"DRAZPAINT 2.0"], 0x2741)?;
    DRAZPAINT.decode_unchecked(&unpacked)
}

/// Unpacks a Drazpaint/Drazlace file: load address, a 13-byte magic (one
/// of `magics`), escape byte, then `ESC count value` RLE. Returns the data
/// with a load address header, padded with zeros to `len` bytes.
pub(super) fn draz_unpack(
    data: &[u8],
    magics: &[&[u8; 13]],
    len: usize,
) -> Result<Vec<u8>, DecodeError> {
    let header = data.get(2..15).ok_or(DecodeError::Unrecognized)?;
    let (&escape, packed) = data
        .get(15..)
        .and_then(|d| d.split_first())
        .ok_or(DecodeError::Unrecognized)?;
    if !magics.iter().any(|magic| header == *magic) {
        return Err(DecodeError::Unrecognized);
    }
    let mut unpacked =
        escape_rle(packed, escape, Run::CountValue, len).ok_or(DecodeError::Unrecognized)?;
    unpacked.resize(len, 0);
    Ok(with_header(unpacked))
}

pub(super) fn decode_art_studio(data: &[u8]) -> Result<Image, DecodeError> {
    ART_STUDIO.decode(data)
}

pub(super) fn decode_interpaint_hires(data: &[u8]) -> Result<Image, DecodeError> {
    INTERPAINT_HIRES.decode(data)
}

pub(super) fn decode_image_system_hires(data: &[u8]) -> Result<Image, DecodeError> {
    IMAGE_SYSTEM_HIRES.decode(data)
}

pub(super) fn decode_hi_eddi(data: &[u8]) -> Result<Image, DecodeError> {
    HI_EDDI.decode(data)
}

pub(super) fn decode_doodle(data: &[u8]) -> Result<Image, DecodeError> {
    DOODLE.decode(data)
}

/// Prepends a dummy load address so unpacked data can be read as a [`Prg`].
pub(super) fn with_header(unpacked: Vec<u8>) -> Vec<u8> {
    let mut data = Vec::with_capacity(unpacked.len() + 2);
    data.extend_from_slice(&[0, 0]);
    data.extend(unpacked);
    data
}

/// Koala compressed (GG): load address, then `$FE value count` RLE.
pub(super) fn decode_koala_packed(data: &[u8]) -> Result<Image, DecodeError> {
    let packed = data.get(2..).ok_or(DecodeError::Unrecognized)?;
    let unpacked =
        escape_rle(packed, 0xfe, Run::ValueCount, 10001).ok_or(DecodeError::Unrecognized)?;
    koala_at(0, &[]).decode_unchecked(&with_header(unpacked))
}

/// Doodle compressed (JJ): load address, then `$FE value count` RLE.
pub(super) fn decode_doodle_packed(data: &[u8]) -> Result<Image, DecodeError> {
    let packed = data.get(2..).ok_or(DecodeError::Unrecognized)?;
    // Some files end right after the bitmap ($7F3F).
    let mut unpacked =
        escape_rle(packed, 0xfe, Run::ValueCount, 9216).ok_or(DecodeError::Unrecognized)?;
    if unpacked.len() < 9024 {
        return Err(DecodeError::Unrecognized);
    }
    unpacked.resize(9216, 0);
    DOODLE.decode_unchecked(&with_header(unpacked))
}

/// Amica Paint: load address, then `$C2 count value` RLE; Koala layout.
pub(super) fn decode_amica(data: &[u8]) -> Result<Image, DecodeError> {
    let packed = data.get(2..).ok_or(DecodeError::Unrecognized)?;
    let unpacked =
        escape_rle(packed, 0xc2, Run::CountValue, 10001).ok_or(DecodeError::Unrecognized)?;
    koala_at(0, &[]).decode_unchecked(&with_header(unpacked))
}

/// Hires-Bitmap: load address and a bare 8000-byte bitmap, white on black.
pub(super) fn decode_mono(data: &[u8]) -> Result<Image, DecodeError> {
    mono(data, 8002, 0x10)
}

/// Gigapaint hires: as Hires-Bitmap but black on white.
pub(super) fn decode_gigapaint_hires(data: &[u8]) -> Result<Image, DecodeError> {
    mono(data, 8002, 0x01)
}

/// Giga-CAD / Mono Magic: load address and an 8192-byte hires bitmap.
pub(super) fn decode_giga_cad(data: &[u8]) -> Result<Image, DecodeError> {
    mono(data, 8194, 0x10)
}

fn mono(data: &[u8], len: usize, colors: u8) -> Result<Image, DecodeError> {
    if data.len() != len {
        return Err(DecodeError::Unrecognized);
    }
    let screen = [colors; SCREEN_LEN];
    let frame = Frame::hires(&Bitmap::hires(&data[2..], &screen), HEIGHT)
        .ok_or(DecodeError::Unrecognized)?;
    Ok(frame.to_image(0))
}
