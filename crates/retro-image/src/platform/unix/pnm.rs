//! Netpbm portable anymaps: PBM, PGM, PPM (`P1` to `P6`, plain and raw) and
//! PAM (`P7`).
//!
//! Sources:
//! - The Netpbm format pages, prose only: PBM
//!   <https://netpbm.sourceforge.net/doc/pbm.html>, PGM
//!   <https://netpbm.sourceforge.net/doc/pgm.html>, PPM
//!   <https://netpbm.sourceforge.net/doc/ppm.html> and PAM
//!   <https://netpbm.sourceforge.net/doc/pam.html> (magic number, whitespace
//!   and `#` comments, maxval, raw samples of one byte below 256 and two
//!   bytes, most significant first, otherwise; plain samples as decimal text;
//!   PBM 1 is black but PAM 0 is black; PAM header lines `WIDTH`, `HEIGHT`,
//!   `DEPTH`, `MAXVAL`, `TUPLTYPE`, `ENDHDR`). No Netpbm source was read.
//!
//! Samples are scaled to 8 bits by `value * 255 / maxval`, rounded. A PAM
//! tuple type of `GRAYSCALE`, `BLACKANDWHITE` or `RGB`, with or without the
//! `_ALPHA` suffix, picks the channels; a PAM without a tuple type is read by
//! depth (1 gray, 2 gray and alpha, 3 RGB, 4 RGB and alpha). Alpha is
//! composited onto the shared transparent-fill gray. Only the first image of
//! a file is decoded, and anything after it is ignored. The XV thumbnail
//! variant of `P7` is not accepted.
//!
//! Verification: no RECOIL oracle. Output matches Pillow's PPM reader pixel
//! for pixel on the PBM, PGM and PPM samples (one 16-bit PGM is out of its
//! reach), and Deark's `pnm` module on the PAM files and that PGM. Deark takes
//! the high byte of 16-bit samples where Pillow and we round, which differs by
//! one level on `lighthouse_rgb48.ppm`. See the divergence file
//! `unix-rasters.tsv`.

use alloc::vec::Vec;

use super::{over_fill, to_byte};
use crate::image::check_size;
use crate::{BitOrder, DecodeError, Image};

const FAIL: DecodeError = DecodeError::Unrecognized;
const WHITE: u32 = 0xff_ffff;
const BLACK: u32 = 0;

/// The channels of each pixel of a gray, RGB or PAM image.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Layout {
    Gray,
    GrayAlpha,
    Rgb,
    RgbAlpha,
}

impl Layout {
    fn depth(self) -> usize {
        match self {
            Self::Gray => 1,
            Self::GrayAlpha => 2,
            Self::Rgb => 3,
            Self::RgbAlpha => 4,
        }
    }
}

/// How the samples of the raster are written.
#[derive(Clone, Copy)]
enum Encoding {
    /// Decimal text separated by whitespace (`P2`, `P3`).
    Text,
    /// One byte each (raw, maxval below 256).
    Bytes,
    /// Two bytes each, most significant first (raw, maxval 256 and above).
    Words,
}

struct Header {
    width: usize,
    height: usize,
    maxval: u32,
    layout: Layout,
    encoding: Encoding,
    /// Offset of the raster in the file.
    raster: usize,
}

pub(super) fn decode_pnm(data: &[u8]) -> Result<Image, DecodeError> {
    let (&kind, after_magic) = match data {
        [b'P', kind @ b'1'..=b'7', rest @ ..] => (kind, rest),
        _ => return Err(FAIL),
    };
    // `P7` must be followed by a newline: XV thumbnails say `P7 332` instead.
    let separator = if kind == b'7' {
        after_magic.first() == Some(&b'\n')
    } else {
        after_magic.first().is_some_and(|&b| is_space(b))
    };
    if !separator {
        return Err(FAIL);
    }
    match kind {
        b'1' | b'4' => decode_bitmap(data, kind == b'4'),
        _ => {
            let header = if kind == b'7' {
                pam_header(data)?
            } else {
                pnm_header(data, kind)?
            };
            decode_samples(data, &header)
        }
    }
}

fn is_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

/// A cursor over the text parts of a file.
struct Text<'a> {
    data: &'a [u8],
    pos: usize,
}

impl Text<'_> {
    /// Skips whitespace and `#` comments (to the end of the line).
    fn skip_blank(&mut self) {
        while let Some(&byte) = self.data.get(self.pos) {
            if byte == b'#' {
                while self.data.get(self.pos).is_some_and(|&b| b != b'\n') {
                    self.pos += 1;
                }
            } else if is_space(byte) {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    /// The next decimal number; `None` if there is none or it overflows.
    fn number(&mut self) -> Option<u32> {
        self.skip_blank();
        let start = self.pos;
        let mut value = 0u32;
        while let Some(digit) = self.data.get(self.pos).filter(|b| b.is_ascii_digit()) {
            value = value
                .checked_mul(10)?
                .checked_add(u32::from(digit - b'0'))?;
            self.pos += 1;
        }
        (self.pos > start).then_some(value)
    }
}

/// Width and height after the magic number.
fn dimensions(text: &mut Text) -> Result<(usize, usize), DecodeError> {
    let width = text.number().ok_or(FAIL)? as usize;
    let height = text.number().ok_or(FAIL)? as usize;
    check_size(width, height)?;
    Ok((width, height))
}

fn pnm_header(data: &[u8], kind: u8) -> Result<Header, DecodeError> {
    let mut text = Text { data, pos: 2 };
    let (width, height) = dimensions(&mut text)?;
    let maxval = text.number().ok_or(FAIL)?;
    if !(1..=0xffff).contains(&maxval) {
        return Err(FAIL);
    }
    let (layout, raw) = match kind {
        b'2' => (Layout::Gray, false),
        b'3' => (Layout::Rgb, false),
        b'5' => (Layout::Gray, true),
        _ => (Layout::Rgb, true),
    };
    let encoding = match (raw, maxval) {
        (false, _) => Encoding::Text,
        (true, 0..=255) => Encoding::Bytes,
        (true, _) => Encoding::Words,
    };
    // A raw raster follows exactly one whitespace byte; a plain one skips
    // blanks by itself.
    let raster = if raw {
        if !data.get(text.pos).is_some_and(|&b| is_space(b)) {
            return Err(FAIL);
        }
        text.pos + 1
    } else {
        text.pos
    };
    Ok(Header {
        width,
        height,
        maxval,
        layout,
        encoding,
        raster,
    })
}

/// The `P7` header: lines up to `ENDHDR`, each starting with a keyword.
fn pam_header(data: &[u8]) -> Result<Header, DecodeError> {
    let (mut width, mut height, mut depth, mut maxval) = (None, None, None, None);
    let mut tuple_type: Option<Vec<u8>> = None;
    let mut pos = 3;
    loop {
        let rest = data.get(pos..).filter(|r| !r.is_empty()).ok_or(FAIL)?;
        let end = rest.iter().position(|&b| b == b'\n').ok_or(FAIL)?;
        let line = &rest[..end];
        pos += end + 1;
        if line.first() == Some(&b'#') {
            continue;
        }
        let mut tokens = line
            .split(|&b| is_space(b))
            .filter(|token| !token.is_empty());
        let Some(keyword) = tokens.next() else {
            continue;
        };
        if keyword == b"ENDHDR" {
            break;
        }
        if keyword == b"TUPLTYPE" {
            // The rest of the line, joined by one blank to earlier lines.
            let value: Vec<&[u8]> = tokens.collect();
            if value.is_empty() {
                return Err(FAIL);
            }
            let tuple_type = tuple_type.get_or_insert_with(Vec::new);
            if !tuple_type.is_empty() {
                tuple_type.push(b' ');
            }
            tuple_type.extend_from_slice(&value.join(&b' '));
            continue;
        }
        let slot = match keyword {
            b"WIDTH" => &mut width,
            b"HEIGHT" => &mut height,
            b"DEPTH" => &mut depth,
            b"MAXVAL" => &mut maxval,
            _ => return Err(FAIL),
        };
        let number = tokens
            .next()
            .and_then(|t| core::str::from_utf8(t).ok())
            .and_then(|t| t.parse::<u32>().ok())
            .ok_or(FAIL)?;
        if slot.replace(number).is_some() {
            return Err(FAIL);
        }
    }
    let (Some(width), Some(height), Some(depth), Some(maxval)) = (width, height, depth, maxval)
    else {
        return Err(FAIL);
    };
    let (width, height) = (width as usize, height as usize);
    check_size(width, height)?;
    if !(1..=0xffff).contains(&maxval) {
        return Err(FAIL);
    }
    let layout = match (tuple_type.as_deref(), depth) {
        (Some(b"BLACKANDWHITE" | b"GRAYSCALE"), 1) | (None, 1) => Layout::Gray,
        (Some(b"BLACKANDWHITE_ALPHA" | b"GRAYSCALE_ALPHA"), 2) | (None, 2) => Layout::GrayAlpha,
        (Some(b"RGB"), 3) | (None, 3) => Layout::Rgb,
        (Some(b"RGB_ALPHA"), 4) | (None, 4) => Layout::RgbAlpha,
        _ => return Err(FAIL),
    };
    Ok(Header {
        width,
        height,
        maxval,
        layout,
        encoding: if maxval < 256 {
            Encoding::Bytes
        } else {
            Encoding::Words
        },
        raster: pos,
    })
}

/// `P1` and `P4`: one bit per pixel, a set bit is black.
fn decode_bitmap(data: &[u8], raw: bool) -> Result<Image, DecodeError> {
    let mut text = Text { data, pos: 2 };
    let (width, height) = dimensions(&mut text)?;
    let colors = [WHITE, BLACK];
    if raw {
        // One whitespace byte, then rows padded to whole bytes.
        if !data.get(text.pos).is_some_and(|&b| is_space(b)) {
            return Err(FAIL);
        }
        let raster = &data[text.pos + 1..];
        return Image::from_bits(
            width as u32,
            height as u32,
            raster,
            width.div_ceil(8),
            BitOrder::MsbFirst,
            colors,
        );
    }
    // Plain: one '0' or '1' character per pixel, whitespace optional.
    if width * height > data.len() {
        return Err(FAIL);
    }
    let mut indices = Vec::with_capacity(width * height);
    for _ in 0..width * height {
        text.skip_blank();
        let bit = match data.get(text.pos) {
            Some(b'0') => 0,
            Some(b'1') => 1,
            _ => return Err(FAIL),
        };
        text.pos += 1;
        indices.push(bit);
    }
    Image::from_indexed(width as u32, height as u32, &indices, &colors)
}

/// The next sample of a raster.
struct Samples<'a> {
    text: Text<'a>,
    encoding: Encoding,
}

impl Samples<'_> {
    fn next(&mut self) -> Option<u32> {
        let text = &mut self.text;
        match self.encoding {
            Encoding::Text => text.number(),
            Encoding::Bytes => {
                let byte = *text.data.get(text.pos)?;
                text.pos += 1;
                Some(u32::from(byte))
            }
            Encoding::Words => {
                let word = crate::bytes::be16(text.data, text.pos)?;
                text.pos += 2;
                Some(u32::from(word))
            }
        }
    }
}

/// Gray, RGB and PAM rasters: `depth` samples per pixel.
fn decode_samples(data: &[u8], header: &Header) -> Result<Image, DecodeError> {
    let Header {
        width,
        height,
        maxval,
        layout,
        encoding,
        raster,
    } = *header;
    // Every sample takes at least one byte: refuse before allocating.
    let samples = width * height * layout.depth();
    if samples > data.len().saturating_sub(raster) {
        return Err(FAIL);
    }
    let mut source = Samples {
        text: Text { data, pos: raster },
        encoding,
    };
    let mut image = Image::new(width as u32, height as u32);
    let channel = |source: &mut Samples| source.next().map(|v| u32::from(to_byte(v, maxval)));
    for y in 0..height as u32 {
        for x in 0..width as u32 {
            let color = match layout {
                Layout::Gray => channel(&mut source).ok_or(FAIL)? * 0x01_0101,
                Layout::GrayAlpha => {
                    let gray = channel(&mut source).ok_or(FAIL)? * 0x01_0101;
                    over_fill(gray, channel(&mut source).ok_or(FAIL)? as u8)
                }
                Layout::Rgb | Layout::RgbAlpha => {
                    let r = channel(&mut source).ok_or(FAIL)?;
                    let g = channel(&mut source).ok_or(FAIL)?;
                    let b = channel(&mut source).ok_or(FAIL)?;
                    let color = r << 16 | g << 8 | b;
                    if layout == Layout::RgbAlpha {
                        over_fill(color, channel(&mut source).ok_or(FAIL)? as u8)
                    } else {
                        color
                    }
                }
            };
            image.set(x, y, color);
        }
    }
    Ok(image)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::TRANSPARENT_FILL;

    #[test]
    fn plain_bitmap_pixels_need_no_separators_and_one_is_black() {
        let image = decode_pnm(b"P1\n# comment\n3 2\n101\n0 1 0").unwrap();
        let row = |y| [0, 1, 2].map(|x| image.get(x, y));
        assert_eq!(row(0), [BLACK, WHITE, BLACK]);
        assert_eq!(row(1), [WHITE, BLACK, WHITE]);
    }

    #[test]
    fn raw_bitmap_rows_are_padded_to_bytes() {
        let image = decode_pnm(b"P4 10 2\n\x80\x40\x00\xc0").unwrap();
        assert_eq!((image.get(0, 0), image.get(9, 0)), (BLACK, BLACK));
        assert_eq!((image.get(1, 0), image.get(8, 0)), (WHITE, WHITE));
        let tail = [7, 8, 9].map(|x| image.get(x, 1));
        assert_eq!(tail, [WHITE, BLACK, BLACK]);
        assert!(decode_pnm(b"P4 10 2\n\x80\x40\x00").is_err());
    }

    #[test]
    fn samples_scale_to_eight_bits() {
        let image = decode_pnm(b"P5 2 1 65535\n\x80\x00\xff\xff").unwrap();
        assert_eq!(image.rgb(), &[128, 128, 128, 255, 255, 255]);
        // Plain text, a comment inside the raster, maxval 15.
        let image = decode_pnm(b"P3 1 1 15\n15 # red\n0 5").unwrap();
        assert_eq!(image.get(0, 0), 0xff0055);
    }

    #[test]
    fn pam_alpha_is_composited_and_the_tuple_type_is_checked() {
        let pam = |tuple: &[u8], depth: &[u8], raster: &[u8]| {
            let mut file = b"P7\n# c\nWIDTH 2\nHEIGHT 1\nDEPTH ".to_vec();
            file.extend_from_slice(depth);
            file.extend_from_slice(b"\nMAXVAL 255\nTUPLTYPE ");
            file.extend_from_slice(tuple);
            file.extend_from_slice(b"\nENDHDR\n");
            file.extend_from_slice(raster);
            decode_pnm(&file)
        };
        let image = pam(b"RGB_ALPHA", b"4", &[9, 8, 7, 255, 1, 2, 3, 0]).unwrap();
        assert_eq!(
            (image.get(0, 0), image.get(1, 0)),
            (0x090807, TRANSPARENT_FILL)
        );
        assert!(pam(b"RGB", b"4", &[0; 8]).is_err());
        assert!(pam(b"CMYK", b"4", &[0; 8]).is_err());
    }

    #[test]
    fn rejects_truncated_and_foreign_headers() {
        assert!(decode_pnm(b"P6 2 1 255\n\0\0\0\0\0").is_err());
        assert!(decode_pnm(b"P6 2 1 255 \0\0\0\0\0\0").is_ok());
        assert!(decode_pnm(b"P6\n2 1\n0\n").is_err());
        assert!(decode_pnm(b"P7 332\n#XVVERSION\n").is_err());
        assert!(decode_pnm(b"P65 1 1 255\n\0\0\0").is_err());
        // A tiny file must not make us allocate for a huge picture.
        assert!(decode_pnm(b"P6 8000 8000 255\n").is_err());
    }
}
