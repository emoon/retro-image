//! Chunky bitmaps in memory order, shared by SGX/SVG and IFF-RGFX.
//!
//! Both formats store a picture as rows of `bytes_per_line` bytes holding
//! one pixel after another, instead of bitplanes. How the bytes map to
//! colors is described in the format specs these modules serve:
//! - SGX pixel layouts: Andreas R. Kleinert, "The SGX Graphics File Format"
//!   v4.2 (<https://aminet.net/docs/misc/SGX-Specs.lha>, `FormatSpecs`).
//! - RGFX bitmap types and the HAM/EHB view-mode note: `rgfx.h` in
//!   <https://aminet.net/dev/misc/IFF-RGFX.zip>.
//! - HAM and EHB pixel values are the ILBM ones (see `ilbm.rs`): control
//!   bits above the data bits, colors 32-63 at half brightness.

use super::ilbm::{half_brite, ham};
use crate::image::{TRANSPARENT_FILL, check_size};
use crate::{DecodeError, Image};

/// How one pixel is stored.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Pixels {
    /// One bit per pixel, most significant bit first; colors 0 and 1.
    Mono,
    /// One palette index per byte.
    Indexed8,
    /// Indices of 6 bits, 32 colors, the upper 32 at half brightness.
    ExtraHalfBrite,
    /// 6-bit hold-and-modify values.
    Ham6,
    /// 8-bit hold-and-modify values.
    Ham8,
    /// Red, green, blue channels, possibly with a fourth.
    Direct(Direct),
}

/// Red, green and blue channels one after another per pixel.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct Direct {
    /// Channels are big-endian 16-bit words, of which the high byte is shown.
    pub wide: bool,
    /// A channel before or after the three colors, if any.
    pub fourth: Option<Fourth>,
}

/// The extra channel of a pixel: padding, or transparency.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct Fourth {
    /// Stored before red instead of after blue.
    pub first: bool,
    /// Transparency (0 is transparent, the maximum opaque) instead of padding.
    pub alpha: bool,
}

impl Pixels {
    /// The fewest bytes a row of `width` pixels can take.
    fn min_row_len(self, width: usize) -> usize {
        match self {
            Self::Mono => width.div_ceil(8),
            Self::Indexed8 | Self::ExtraHalfBrite | Self::Ham6 | Self::Ham8 => width,
            Self::Direct(direct) => width * direct.pixel_len(),
        }
    }
}

impl Direct {
    fn channel_len(self) -> usize {
        if self.wide { 2 } else { 1 }
    }

    fn pixel_len(self) -> usize {
        self.channel_len() * (3 + usize::from(self.fourth.is_some()))
    }

    /// Reads one pixel; transparent parts show [`TRANSPARENT_FILL`].
    fn pixel(self, bytes: &[u8]) -> u32 {
        let step = self.channel_len();
        let colors = usize::from(self.fourth.is_some_and(|f| f.first)) * step;
        let [r, g, b] = [0, 1, 2].map(|i| u32::from(bytes[colors + i * step]));
        let color = r << 16 | g << 8 | b;
        let Some(Fourth { first, alpha: true }) = self.fourth else {
            return color;
        };
        let opacity = u32::from(bytes[if first { 0 } else { 3 * step }]);
        over_fill(color, opacity)
    }
}

/// `color` at `opacity` (0-255) over [`TRANSPARENT_FILL`].
fn over_fill(color: u32, opacity: u32) -> u32 {
    let mix = |shift: u32| {
        let (c, f) = (color >> shift & 0xff, TRANSPARENT_FILL >> shift & 0xff);
        (c * opacity + f * (255 - opacity) + 127) / 255
    };
    mix(16) << 16 | mix(8) << 8 | mix(0)
}

/// Where the rows are and how big they are.
#[derive(Clone, Copy)]
pub(super) struct Rows {
    pub width: usize,
    pub height: usize,
    pub bytes_per_line: usize,
}

impl Rows {
    /// Bytes the rows take.
    pub fn len(&self) -> Result<usize, DecodeError> {
        check_size(self.width, self.height)?;
        if self.bytes_per_line == 0 {
            return Err(DecodeError::Unrecognized);
        }
        self.bytes_per_line
            .checked_mul(self.height)
            .ok_or(DecodeError::Unrecognized)
    }
}

/// Draws `data` as a picture. `palette` is read for the indexed kinds.
pub(super) fn render(
    pixels: Pixels,
    rows: Rows,
    data: &[u8],
    palette: &[u32; 256],
) -> Result<Image, DecodeError> {
    let len = rows.len()?;
    if rows.bytes_per_line < pixels.min_row_len(rows.width) || data.len() < len {
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
            Pixels::Mono => palette[usize::from(row[x / 8] >> (7 - x % 8) & 1)],
            Pixels::Indexed8 => palette[usize::from(row[x])],
            Pixels::ExtraHalfBrite => match row[x] & 63 {
                v @ 0..32 => palette[usize::from(v)],
                v => half_brite(palette[usize::from(v - 32)]),
            },
            Pixels::Ham6 => {
                let v = u32::from(row[x]);
                ham(
                    held,
                    v >> 4 & 3,
                    (v & 15) * 0x11,
                    palette[(v & 15) as usize],
                )
            }
            Pixels::Ham8 => {
                let v = u32::from(row[x]);
                let data = v & 63;
                ham(held, v >> 6, data << 2 | data >> 4, palette[data as usize])
            }
            Pixels::Direct(direct) => direct.pixel(&row[x * direct.pixel_len()..]),
        };
        held = color;
        let [_, r, g, b] = color.to_be_bytes();
        *out = [r, g, b];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grays() -> [u32; 256] {
        core::array::from_fn(|i| u32::from(i as u8) * 0x01_0101)
    }

    const RGB24: Pixels = Pixels::Direct(Direct {
        wide: false,
        fourth: None,
    });

    fn rgba(first: bool, wide: bool) -> Pixels {
        Pixels::Direct(Direct {
            wide,
            fourth: Some(Fourth { first, alpha: true }),
        })
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
    fn mono_reads_the_top_bit_first() {
        let rows = Rows {
            width: 3,
            height: 1,
            bytes_per_line: 1,
        };
        let mut palette = grays();
        palette[1] = 0xffffff;
        let image = render(Pixels::Mono, rows, &[0b1010_0000], &palette).unwrap();
        assert_eq!(image.rgb(), &[255, 255, 255, 0, 0, 0, 255, 255, 255]);
    }

    #[test]
    fn sixteen_bit_channels_show_their_high_byte() {
        let rows = Rows {
            width: 1,
            height: 1,
            bytes_per_line: 6,
        };
        let data = [0x12, 0xff, 0x34, 0x00, 0x56, 0x80];
        let wide = Pixels::Direct(Direct {
            wide: true,
            fourth: None,
        });
        let image = render(wide, rows, &data, &grays()).unwrap();
        assert_eq!(image.rgb(), &[0x12, 0x34, 0x56]);
    }

    #[test]
    fn alpha_blends_over_the_transparent_fill() {
        let rows = Rows {
            width: 4,
            height: 1,
            bytes_per_line: 16,
        };
        // Red at alpha 255, 0, 128, and a padding byte that must not show.
        let data = [
            255, 0, 0, 255, 255, 0, 0, 0, 255, 0, 0, 128, //
            0, 0, 255, 0,
        ];
        let image = render(rgba(false, false), rows, &data, &grays()).unwrap();
        let fill = TRANSPARENT_FILL.to_be_bytes();
        assert_eq!(&image.rgb()[..3], &[255, 0, 0]);
        assert_eq!(&image.rgb()[3..6], &fill[1..]);
        assert_eq!(&image.rgb()[6..9], &[224, 96, 96]);
        assert_eq!(&image.rgb()[9..], &fill[1..]);
    }

    #[test]
    fn alpha_may_come_first_and_channels_may_be_wide() {
        let rows = Rows {
            width: 1,
            height: 1,
            bytes_per_line: 4,
        };
        let data = [255, 10, 20, 30];
        let image = render(rgba(true, false), rows, &data, &grays()).unwrap();
        assert_eq!(image.rgb(), &[10, 20, 30]);
        // Sixteen-bit channels, alpha last: the high bytes count.
        let rows = Rows {
            bytes_per_line: 8,
            ..rows
        };
        let data = [10, 0, 20, 0, 30, 0, 255, 255];
        let image = render(rgba(false, true), rows, &data, &grays()).unwrap();
        assert_eq!(image.rgb(), &[10, 20, 30]);
    }

    #[test]
    fn short_data_and_short_rows_are_rejected() {
        let rows = Rows {
            width: 2,
            height: 2,
            bytes_per_line: 5,
        };
        assert!(render(RGB24, rows, &[0; 10], &grays()).is_err());
        assert!(render(RGB24, rows, &[0; 9], &grays()).is_err());
        let rows = Rows {
            bytes_per_line: 6,
            ..rows
        };
        assert!(render(RGB24, rows, &[0; 12], &grays()).is_ok());
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
