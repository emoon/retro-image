//! Chunky bitmaps in memory order, shared by SGX/SVG, IFF-RGFX, YAFA and CDXL.
//!
//! These formats store a picture as rows of `bytes_per_line` bytes holding
//! one pixel after another, instead of bitplanes. Only the pixel kinds that a
//! sample file or a reference tool has confirmed are drawn:
//! - palette indices, one byte per pixel, and 24-bit RGB (SGX, RGFX);
//! - 32-bit red, green, blue, alpha (SuperView's `abydos.rlen.svg`);
//! - 6- and 8-bit hold-and-modify values (CDXL, checked against ffmpeg), whose
//!   control bits sit above the data bits as in ILBM (see `ilbm.rs`).
//!
//! The pixel layouts are described in the specs of the modules that use them:
//! - SGX: Andreas R. Kleinert, "The SGX Graphics File Format" v4.2
//!   (<https://aminet.net/docs/misc/SGX-Specs.lha>, `FormatSpecs`).
//! - RGFX: `rgfx.h` in <https://aminet.net/dev/misc/IFF-RGFX.zip>.
//!
//! The view-mode check that tells HAM and extra-half-brite data from plain
//! indices is the one both specs ask for (`HAM_KEY`, `EXTRA_HALF_BRITE_KEY` in
//! AmigaOS `graphics/modeid.h`).

use alloc::borrow::Cow;
use alloc::vec::Vec;

use super::ilbm::ham;
use crate::bytes::be32;
use crate::codec::{inflate, powerpacker, xpk};
use crate::image::{check_size, over_fill, widen_channel};
use crate::{DecodeError, Image};

/// `ViewMode` bit that says indexed data is really HAM, at any depth. This
/// module does not draw it from plain indices.
const HAM_KEY: u32 = 0x800;
/// `ViewMode` bit that says 6-bit indexed data is really extra-half-brite.
const EXTRA_HALF_BRITE_KEY: u32 = 0x80;

/// How one pixel is stored.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Pixels {
    /// One palette index per byte.
    Indexed8,
    /// 6-bit hold-and-modify values.
    Ham6,
    /// 8-bit hold-and-modify values.
    Ham8,
    /// Red, green, blue bytes.
    Rgb24,
    /// Red, green, blue, alpha bytes (0 transparent, 255 opaque).
    Rgba32,
}

impl Pixels {
    /// One byte per pixel holding a palette index of `depth` bits. `None` if
    /// the screen mode `view_mode` says the data is HAM (at any depth) or
    /// 6-bit extra-half-brite: plain indices would draw those pictures
    /// wrongly.
    pub fn indexed(depth: u32, view_mode: u32) -> Option<Self> {
        let hold_and_modify = view_mode & HAM_KEY != 0;
        let half_brite = view_mode & EXTRA_HALF_BRITE_KEY != 0 && depth == 6;
        (!hold_and_modify && !half_brite).then_some(Self::Indexed8)
    }

    /// The fewest bytes a row of `width` pixels can take.
    fn min_row_len(self, width: usize) -> usize {
        match self {
            Self::Indexed8 | Self::Ham6 | Self::Ham8 => width,
            Self::Rgb24 => width * 3,
            Self::Rgba32 => width * 4,
        }
    }
}

/// How a bitmap's bytes are stored in the file.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Packing {
    Stored,
    /// The uncompressed size as a big-endian long, then a zlib stream.
    Zlib,
    Xpk,
    PowerPacker,
}

/// The `len` bytes of bitmap data held in `payload`.
pub(super) fn bitmap_bytes(
    packing: Packing,
    payload: &[u8],
    len: usize,
) -> Result<Cow<'_, [u8]>, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let unpacked: Option<Vec<u8>> = match packing {
        Packing::Stored => return payload.get(..len).map(Cow::Borrowed).ok_or(fail),
        Packing::Zlib => {
            if be32(payload, 0).ok_or(fail)? as usize != len {
                return Err(fail);
            }
            inflate::zlib(&payload[4..], len)
        }
        Packing::Xpk => xpk::unpack(payload, len),
        Packing::PowerPacker => powerpacker::unpack(payload),
    };
    unpacked
        .filter(|bytes| bytes.len() >= len)
        .map(Cow::Owned)
        .ok_or(fail)
}

/// Where the rows are and how big they are.
#[derive(Clone, Copy)]
pub(super) struct Rows {
    pub width: usize,
    pub height: usize,
    pub bytes_per_line: usize,
}

impl Rows {
    /// Bytes the rows take for `pixels`. A row may be padded, but not by more
    /// than its pixel bytes plus 64: sizes read from a file must not be able
    /// to demand a gigabyte for a picture of one pixel.
    pub fn len(&self, pixels: Pixels) -> Result<usize, DecodeError> {
        check_size(self.width, self.height)?;
        let fewest = pixels.min_row_len(self.width);
        if self.bytes_per_line < fewest || self.bytes_per_line > fewest * 2 + 64 {
            return Err(DecodeError::Unrecognized);
        }
        Ok(self.bytes_per_line * self.height)
    }
}

/// Draws `data` as a picture. `palette` is read for the indexed kinds.
pub(super) fn render(
    pixels: Pixels,
    rows: Rows,
    data: &[u8],
    palette: &[u32; 256],
) -> Result<Image, DecodeError> {
    let len = rows.len(pixels)?;
    if data.len() < len {
        return Err(DecodeError::Unrecognized);
    }
    let mut image = Image::new(rows.width as u32, rows.height as u32);
    for (y, row) in data[..len].chunks_exact(rows.bytes_per_line).enumerate() {
        let out = image.row_mut(y as u32).as_chunks_mut::<3>().0;
        draw_row(pixels, row, palette, out);
    }
    Ok(image)
}

fn draw_row(pixels: Pixels, row: &[u8], palette: &[u32; 256], out: &mut [[u8; 3]]) {
    let mut held = palette[0];
    for (x, out) in out.iter_mut().enumerate() {
        let color = match pixels {
            Pixels::Indexed8 => palette[usize::from(row[x])],
            Pixels::Ham6 => {
                let v = u32::from(row[x]);
                ham(
                    held,
                    v >> 4 & 3,
                    widen_channel(v & 15, 4),
                    palette[(v & 15) as usize],
                )
            }
            Pixels::Ham8 => {
                let v = u32::from(row[x]);
                let data = v & 63;
                ham(held, v >> 6, widen_channel(data, 6), palette[data as usize])
            }
            Pixels::Rgb24 => rgb(&row[x * 3..]),
            Pixels::Rgba32 => over_fill(rgb(&row[x * 4..]), row[x * 4 + 3]),
        };
        held = color;
        let [_, r, g, b] = color.to_be_bytes();
        *out = [r, g, b];
    }
}

/// The color of the first three bytes of `bytes`.
fn rgb(bytes: &[u8]) -> u32 {
    u32::from(bytes[0]) << 16 | u32::from(bytes[1]) << 8 | u32::from(bytes[2])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::TRANSPARENT_FILL;

    fn grays() -> [u32; 256] {
        core::array::from_fn(|i| u32::from(i as u8) * 0x01_0101)
    }

    #[test]
    fn rows_skip_their_padding() {
        // 2x2 pixels in rows of 4 bytes; the padding must not show.
        let data = [1, 2, 99, 99, 3, 4, 99, 99];
        let rows = Rows {
            width: 2,
            height: 2,
            bytes_per_line: 4,
        };
        let image = render(Pixels::Indexed8, rows, &data, &grays()).unwrap();
        assert_eq!(image.rgb(), &[1, 1, 1, 2, 2, 2, 3, 3, 3, 4, 4, 4]);
    }

    #[test]
    fn row_lengths_far_beyond_the_pixels_are_rejected() {
        // A 1x1 picture claiming 1 GiB per row must not be sized from that.
        let rows = Rows {
            width: 1,
            height: 1,
            bytes_per_line: 0x4000_0000,
        };
        assert!(rows.len(Pixels::Indexed8).is_err());
        assert!(render(Pixels::Indexed8, rows, &[0], &grays()).is_err());
        // Some padding is fine: up to twice the pixel bytes plus 64.
        let padded = Rows {
            bytes_per_line: 64 + 2,
            ..rows
        };
        assert_eq!(padded.len(Pixels::Indexed8).unwrap(), 66);
        let too_padded = Rows {
            bytes_per_line: 64 + 3,
            ..rows
        };
        assert!(too_padded.len(Pixels::Indexed8).is_err());
    }

    #[test]
    fn alpha_blends_over_the_transparent_fill() {
        let rows = Rows {
            width: 3,
            height: 1,
            bytes_per_line: 12,
        };
        // Red at alpha 255, 0 and 128.
        let data = [255, 0, 0, 255, 255, 0, 0, 0, 255, 0, 0, 128];
        let image = render(Pixels::Rgba32, rows, &data, &grays()).unwrap();
        let fill = TRANSPARENT_FILL.to_be_bytes();
        assert_eq!(&image.rgb()[..3], &[255, 0, 0]);
        assert_eq!(&image.rgb()[3..6], &fill[1..]);
        assert_eq!(&image.rgb()[6..], &[224, 96, 96]);
    }

    #[test]
    fn short_data_and_short_rows_are_rejected() {
        let rows = Rows {
            width: 2,
            height: 2,
            bytes_per_line: 5,
        };
        assert!(render(Pixels::Rgb24, rows, &[0; 10], &grays()).is_err());
        assert!(render(Pixels::Rgb24, rows, &[0; 9], &grays()).is_err());
        let rows = Rows {
            bytes_per_line: 6,
            ..rows
        };
        assert!(render(Pixels::Rgb24, rows, &[0; 12], &grays()).is_ok());
    }

    #[test]
    fn hold_and_modify_views_are_not_drawn_from_plain_indices() {
        assert_eq!(Pixels::indexed(8, 0), Some(Pixels::Indexed8));
        assert_eq!(Pixels::indexed(4, HAM_KEY), None);
        assert_eq!(Pixels::indexed(6, HAM_KEY), None);
        assert_eq!(Pixels::indexed(8, HAM_KEY | 4), None);
        assert_eq!(Pixels::indexed(6, EXTRA_HALF_BRITE_KEY), None);
        assert_eq!(
            Pixels::indexed(8, EXTRA_HALF_BRITE_KEY),
            Some(Pixels::Indexed8)
        );
    }

    #[test]
    fn ham6_holds_and_modifies() {
        let rows = Rows {
            width: 3,
            height: 1,
            bytes_per_line: 3,
        };
        let mut palette = grays();
        palette[1] = 0x102030;
        // Palette color 1, then red set to 0xf, then blue set to 0x5.
        let image = render(Pixels::Ham6, rows, &[0x01, 0x2f, 0x15], &palette).unwrap();
        assert_eq!(
            image.rgb(),
            &[0x10, 0x20, 0x30, 0xff, 0x20, 0x30, 0xff, 0x20, 0x55]
        );
    }
}
