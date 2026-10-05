//! IFF bitmaps: ILBM (incl. HAM6, HAM8, EHB, 24-bit), PBM and ACBM.
//!
//! Sources:
//! - ILBM: <https://wiki.amigaos.net/wiki/ILBM_IFF_Interleaved_Bitmap>
//!   (BMHD, CMAP, CAMG, BODY, masking, ByteRun1, HAM and EHB). Masking 1 is a
//!   mask plane after the bitplanes of every row, where a set bit is opaque;
//!   masking 2 makes the pixels of one palette index (BMHD `transparentColor`)
//!   transparent, which is only read for plain and extra-half-brite pictures,
//!   whose values are palette indices. Masking 3 (lasso) is not read.
//! - ACBM: <https://wiki.amigaos.net/wiki/ACBM_IFF_Amiga_Continuous_Bitmap>
//!   (ABIT holds whole planes one after another).
//! - HAM8 control bits: <https://en.wikipedia.org/wiki/Hold-And-Modify>.
//! - PBM: chunky 8-bit BODY with rows padded to even length (reverse engineered
//!   from samples).
//! - Pixel doubling for interlaced low-res and non-interlaced high-res
//!   screens: observed from `recoil2png` output.
//! - Color mode without a CAMG chunk (6 planes: 16 colors is HAM6, 32 is
//!   EHB, anything else indexed) and the HAM flag on 5 or 7 planes (HAM6 or
//!   HAM8, missing top plane read as 0): measured against `recoil2png` on
//!   real files with the chunk removed or the flag set. Deark and the Just
//!   Solve ILBM page describe the same rules,
//!   <http://justsolve.archiveteam.org/wiki/ILBM>.
//! - Super-hires (`SUPERHIRES` 0x20 with `HIRES` 0x8000 in `graphics/view.h`)
//!   is only 1/4-lores-wide pixels on the native PAL/NTSC/default monitors
//!   (`graphics/modeid.h`: `SUPER_KEY` 0x8020 under monitor 0, 0x11000, 0x21000).
//!   Other monitors reuse the bit for different timings: `VGAPRODUCT_KEY`
//!   0x39024 is a 640x480 square-pixel mode, and Super72/DblPAL modes are
//!   likewise square. `recoil2png` probing of one super-hires PAL ILBM with
//!   the CAMG rewritten confirmed the split (see `scale_factors`).

use alloc::vec::Vec;

use super::iff::find;
use super::multi_palette::LinePalettes;
use super::vdat;
use crate::bytes::{be16, be32};
use crate::codec::packbits;
use crate::image::{check_size, planar_pixels, widen_channel};
use crate::{DecodeError, Image};

pub(super) const CAMG_LACE: u32 = 0x4;
const CAMG_EHB: u32 = 0x80;
pub(super) const CAMG_HAM: u32 = 0x800;
pub(super) const CAMG_HIRES: u32 = 0x8000;
const CAMG_SUPER: u32 = 0x20;

/// Fields of the BMHD chunk we use.
pub(super) struct Header {
    pub width: usize,
    pub height: usize,
    pub planes: usize,
    masking: u8,
    pub compression: u8,
    /// The palette index that masking 2 makes transparent.
    transparent: u16,
}

impl Header {
    pub(super) fn parse(contents: &[u8]) -> Option<Self> {
        let bmhd = find(contents, b"BMHD")?;
        if bmhd.len() < 20 {
            return None;
        }
        let (width, height) = (be16(bmhd, 0)? as usize, be16(bmhd, 2)? as usize);
        check_size(width, height).ok()?;
        let header = Self {
            width,
            height,
            planes: bmhd[8] as usize,
            masking: bmhd[9],
            compression: bmhd[10],
            transparent: be16(bmhd, 12)?,
        };
        (header.width > 0 && header.height > 0).then_some(header)
    }

    /// Bytes per row of one plane, padded to 16 bits.
    fn plane_row_len(&self) -> usize {
        self.width.div_ceil(16) * 2
    }
}

/// How BODY bytes are arranged.
#[derive(Clone, Copy, PartialEq)]
enum Layout {
    /// ILBM: for each row, one row per plane (and mask).
    Interleaved,
    /// ACBM: whole planes one after another.
    Contiguous,
    /// PBM: one byte per pixel.
    Chunky,
}

pub(super) fn decode_ilbm(contents: &[u8]) -> Result<Image, DecodeError> {
    decode_bitmap(contents, b"BODY", Layout::Interleaved)
}

pub(super) fn decode_pbm(contents: &[u8]) -> Result<Image, DecodeError> {
    decode_bitmap(contents, b"BODY", Layout::Chunky)
}

pub(super) fn decode_acbm(contents: &[u8]) -> Result<Image, DecodeError> {
    decode_bitmap(contents, b"ABIT", Layout::Contiguous)
}

fn decode_bitmap(contents: &[u8], body_id: &[u8; 4], layout: Layout) -> Result<Image, DecodeError> {
    let bitmap = read_bitmap(contents, body_id, layout)?;
    if bitmap.is_dctv() || bitmap.is_ham_e() {
        return Err(DecodeError::Unrecognized);
    }
    let Bitmap {
        header,
        camg,
        indices,
        mask,
    } = bitmap;
    let mut palette = find(contents, b"CMAP")
        .map(Palette::from_cmap)
        .unwrap_or_default();
    let line_palettes = LinePalettes::parse(contents, header.height);
    let mode = Mode::detect(&header, camg, palette.len(), layout)?;
    let mut image = Image::new(header.width as u32, header.height as u32);
    for (y, row) in indices.chunks_exact(header.width).enumerate() {
        if let Some(line_palettes) = &line_palettes {
            line_palettes.apply(y, &mut palette);
        }
        mode.render_row(row, &palette, image.row_mut(y as u32));
    }
    let indexed = matches!(mode, Mode::Indexed | Mode::ExtraHalfBrite);
    let alpha = mask.or_else(|| {
        (header.masking == 2 && indexed).then(|| {
            let clear = u32::from(header.transparent);
            indices
                .iter()
                .map(|&v| if v == clear { 0 } else { 255 })
                .collect()
        })
    });
    if let Some(alpha) = alpha {
        image = image.with_alpha(alpha);
    }
    scale(image, camg.unwrap_or(0))
}

/// The BMHD, CAMG and raw pixel values (palette indices or packed RGB,
/// row-major) of a bitmap FORM, before any colour interpretation.
pub(super) struct Bitmap {
    pub header: Header,
    pub camg: Option<u32>,
    pub indices: Vec<u32>,
    /// Alpha from the mask plane (255 where it is set), if there is one.
    mask: Option<Vec<u8>>,
}

/// Reads the ILBM at `contents` without interpreting its pixel values.
pub(super) fn read_ilbm(contents: &[u8]) -> Result<Bitmap, DecodeError> {
    read_bitmap(contents, b"BODY", Layout::Interleaved)
}

fn read_bitmap(contents: &[u8], body_id: &[u8; 4], layout: Layout) -> Result<Bitmap, DecodeError> {
    let header = Header::parse(contents).ok_or(DecodeError::Unrecognized)?;
    let body = find(contents, body_id).ok_or(DecodeError::Unrecognized)?;
    let camg = find(contents, b"CAMG").and_then(|c| be32(c, 0));
    let (indices, mask) = match layout {
        Layout::Chunky => (read_chunky(&header, body)?, None),
        _ => read_planar(&header, body, layout)?,
    };
    Ok(Bitmap {
        header,
        camg,
        indices,
        mask,
    })
}

impl Bitmap {
    /// Pixel row `y`.
    pub fn row(&self, y: usize) -> Option<&[u32]> {
        let width = self.header.width;
        self.indices.get(y * width..(y + 1) * width)
    }

    /// Whether the first row carries the DCTV signature.
    fn is_dctv(&self) -> bool {
        self.row(0)
            .is_some_and(|row| super::dctv::has_signature(self.header.planes, row))
    }

    /// Whether the first row starts with the HAM-E cookie. Both this and DCTV
    /// encode colours the plain bitmap does not show, so such files are left
    /// to decoders for those devices.
    fn is_ham_e(&self) -> bool {
        self.header.planes == 4 && self.row(0).is_some_and(super::ham_e::is_palette_line)
    }
}

/// Colour registers, as `0xRRGGBB`.
#[derive(Clone, Default)]
pub(super) struct Palette(Vec<u32>);

impl Palette {
    /// Reads a CMAP chunk.
    ///
    /// At most 32 colours (the OCS register count) whose components all have
    /// a zero low nibble were written by a 4-bit program, so the high nibble
    /// is repeated as the ILBM spec recommends. The 32-colour limit is
    /// observed from `recoil2png` output.
    pub(super) fn from_cmap(cmap: &[u8]) -> Self {
        let cmap = &cmap[..cmap.len() / 3 * 3];
        let four_bit = cmap.len() <= 32 * 3 && cmap.iter().all(|&c| c & 0x0f == 0);
        Self(
            cmap.as_chunks::<3>()
                .0
                .iter()
                .map(|c| {
                    c.iter().fold(0, |rgb, &v| {
                        let v = if four_bit { v | v >> 4 } else { v };
                        rgb << 8 | u32::from(v)
                    })
                })
                .collect(),
        )
    }

    pub(super) fn len(&self) -> usize {
        self.0.len()
    }

    /// Colour `index`; black if the CMAP is too short.
    pub(super) fn color(&self, index: u32) -> u32 {
        self.0.get(index as usize).copied().unwrap_or(0)
    }

    pub(super) fn set(&mut self, index: usize, color: u32) {
        if index >= self.0.len() {
            self.0.resize(index + 1, 0);
        }
        self.0[index] = color;
    }

    /// Extra-half-brite colour: colour `index` at half brightness.
    fn half(&self, index: u32) -> u32 {
        half_brite(self.color(index))
    }
}

/// Extra-half-brite colour: each 8-bit component halved.
pub(super) fn half_brite(rgb: u32) -> u32 {
    (rgb >> 1) & 0x7f7f7f
}

/// Unpacks BODY into exactly `len` bytes.
fn unpack_body(header: &Header, body: &[u8], len: usize) -> Result<Vec<u8>, DecodeError> {
    match header.compression {
        0 => body
            .get(..len)
            .map(<[u8]>::to_vec)
            .ok_or(DecodeError::Unrecognized),
        1 => packbits::unpack(body, len)
            .map(|(out, _)| out)
            .ok_or(DecodeError::Unrecognized),
        _ => Err(DecodeError::Unrecognized),
    }
}

/// Pixel values (palette indices or packed RGB) of a planar body, row-major,
/// and the alpha of its mask plane if an interleaved body has one.
fn read_planar(
    header: &Header,
    body: &[u8],
    layout: Layout,
) -> Result<(Vec<u32>, Option<Vec<u8>>), DecodeError> {
    if header.planes == 0 || header.planes > 32 {
        return Err(DecodeError::Unrecognized);
    }
    let stored_planes = header.planes + usize::from(header.masking == 1);
    let row_len = header.plane_row_len();
    let len = row_len * stored_planes * header.height;
    // No compression expands a byte to more than 128 bytes (VDAT word runs
    // can, but not in real files); this keeps corrupt sizes from allocating
    // huge buffers.
    if len > body.len().saturating_mul(128) {
        return Err(DecodeError::Unrecognized);
    }
    let data = match layout {
        // ABIT is never compressed, whatever BMHD says.
        Layout::Contiguous => body.get(..len).ok_or(DecodeError::Unrecognized)?.to_vec(),
        _ if header.compression == 2 => vdat::unpack(body, stored_planes, row_len, header.height)
            .ok_or(DecodeError::Unrecognized)?,
        _ => unpack_body(header, body, len)?,
    };
    let values = planar_pixels(
        &data,
        header.width,
        header.height,
        row_len,
        header.planes,
        |plane, y| match layout {
            Layout::Contiguous => (plane * header.height + y) * row_len,
            _ => (y * stored_planes + plane) * row_len,
        },
    );
    let mask = (header.masking == 1 && layout == Layout::Interleaved).then(|| {
        // The mask plane follows the bitplanes of each row.
        let bits = planar_pixels(&data, header.width, header.height, row_len, 1, |_, y| {
            (y * stored_planes + header.planes) * row_len
        });
        bits.iter()
            .map(|&bit| if bit != 0 { 255 } else { 0 })
            .collect()
    });
    Ok((values, mask))
}

fn read_chunky(header: &Header, body: &[u8]) -> Result<Vec<u32>, DecodeError> {
    if header.planes != 8 {
        return Err(DecodeError::Unrecognized);
    }
    let row_len = header.width + (header.width & 1);
    if row_len * header.height > body.len().saturating_mul(128) {
        return Err(DecodeError::Unrecognized);
    }
    let data = unpack_body(header, body, row_len * header.height)?;
    Ok(data
        .chunks_exact(row_len)
        .flat_map(|row| row[..header.width].iter().map(|&b| u32::from(b)))
        .collect())
}

/// How pixel values map to colours.
enum Mode {
    Indexed,
    ExtraHalfBrite,
    Ham6,
    Ham8,
    /// 24 planes: red in planes 0-7, green 8-15, blue 16-23.
    TrueColor,
}

impl Mode {
    fn detect(
        header: &Header,
        camg: Option<u32>,
        colors: usize,
        layout: Layout,
    ) -> Result<Self, DecodeError> {
        let camg_bits = camg.unwrap_or(0);
        let ham = camg_bits & CAMG_HAM != 0;
        Ok(match header.planes {
            _ if layout == Layout::Chunky => Self::Indexed,
            // A HAM picture one plane short reads the missing top plane as 0.
            5 | 6 if ham => Self::Ham6,
            7 | 8 if ham => Self::Ham8,
            6 if camg_bits & CAMG_EHB != 0 => Self::ExtraHalfBrite,
            // No CAMG: 16 colors is HAM6, 32 is EHB, anything else indexed.
            6 if camg.is_none() && colors == 32 => Self::ExtraHalfBrite,
            6 if camg.is_none() && colors == 16 => Self::Ham6,
            1..=8 => Self::Indexed,
            24 => Self::TrueColor,
            _ => return Err(DecodeError::Unrecognized),
        })
    }

    /// Colours the pixel values `row` into `out`, 3 bytes per pixel.
    fn render_row(&self, row: &[u32], palette: &Palette, out: &mut [u8]) {
        let lookup = |i: u32| palette.color(i);
        let mut held = lookup(0);
        for (&v, out) in row.iter().zip(out.as_chunks_mut::<3>().0) {
            let color = match self {
                Self::Indexed => lookup(v),
                Self::ExtraHalfBrite if v >= 32 => palette.half(v - 32),
                Self::ExtraHalfBrite => lookup(v),
                Self::Ham6 => ham(held, v >> 4, widen_channel(v & 15, 4), lookup(v & 15)),
                Self::Ham8 => {
                    let data = v & 63;
                    ham(held, v >> 6, widen_channel(data, 6), lookup(data))
                }
                Self::TrueColor => (v & 0xff) << 16 | (v & 0xff00) | (v >> 16 & 0xff),
            };
            held = color;
            let [_, r, g, b] = color.to_be_bytes();
            *out = [r, g, b];
        }
    }
}

/// Hold-and-modify: control 0 takes `base`, 1 sets blue, 2 red, 3 green.
pub(super) fn ham(held: u32, control: u32, component: u32, base: u32) -> u32 {
    match control {
        0 => base,
        1 => (held & 0xffff00) | component,
        2 => (held & 0x00ffff) | component << 16,
        _ => (held & 0xff00ff) | component << 8,
    }
}

/// Pixel doubling that keeps the picture's aspect ratio, as (x, y) factors.
///
/// A lores pixel is the unit; hires halves its width, super-hires quarters
/// it and interlace halves its height. Super-hires only counts on the native
/// monitors (monitor ID in the upper CAMG word is 0, NTSC or PAL).
pub(super) fn scale_factors(camg: u32) -> (u32, u32) {
    let lace = camg & CAMG_LACE != 0;
    let hires = camg & CAMG_HIRES != 0;
    let native_monitor = matches!(camg >> 16, 0..=2);
    let super_hires = hires && camg & CAMG_SUPER != 0 && native_monitor;
    match (super_hires, hires, lace) {
        (true, _, true) => (1, 2),
        (true, _, false) => (1, 4),
        (_, false, true) => (2, 1),
        (_, true, false) => (1, 2),
        _ => (1, 1),
    }
}

fn scale(image: Image, camg: u32) -> Result<Image, DecodeError> {
    let (sx, sy) = scale_factors(camg);
    image.scaled(sx, sy)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bmhd(width: u16, height: u16) -> Vec<u8> {
        let mut chunk = alloc::vec![0u8; 28];
        chunk[..4].copy_from_slice(b"BMHD");
        chunk[4..8].copy_from_slice(&20u32.to_be_bytes());
        chunk[8..10].copy_from_slice(&width.to_be_bytes());
        chunk[10..12].copy_from_slice(&height.to_be_bytes());
        chunk[16] = 1;
        chunk
    }

    fn mode(planes: usize, camg: Option<u32>, colors: usize) -> Mode {
        let header = Header {
            width: 1,
            height: 1,
            planes,
            masking: 0,
            compression: 0,
            transparent: 0,
        };
        Mode::detect(&header, camg, colors, Layout::Interleaved).unwrap()
    }

    #[test]
    fn six_planes_without_camg_are_guessed_from_the_palette_size() {
        assert!(matches!(mode(6, None, 16), Mode::Ham6));
        assert!(matches!(mode(6, None, 32), Mode::ExtraHalfBrite));
        for colors in [0, 17, 24, 33, 64] {
            assert!(matches!(mode(6, None, colors), Mode::Indexed));
        }
        // With a CAMG chunk, its flags decide.
        assert!(matches!(mode(6, Some(CAMG_EHB), 64), Mode::ExtraHalfBrite));
        assert!(matches!(mode(6, Some(0), 32), Mode::Indexed));
    }

    #[test]
    fn ham_flag_also_applies_to_five_and_seven_planes() {
        assert!(matches!(mode(5, Some(CAMG_HAM), 32), Mode::Ham6));
        assert!(matches!(mode(7, Some(CAMG_HAM), 128), Mode::Ham8));
        assert!(matches!(mode(5, None, 32), Mode::Indexed));
        assert!(matches!(mode(4, Some(CAMG_HAM), 16), Mode::Indexed));
    }

    #[test]
    fn header_rejects_pictures_over_the_pixel_cap() {
        assert!(Header::parse(&bmhd(100, 100)).is_some());
        assert!(Header::parse(&bmhd(65535, 65535)).is_none());
    }

    /// A 3 x 1 picture of one plane (black and white palette), uncompressed.
    fn one_plane(masking: u8, transparent: u16, body: &[u8]) -> Vec<u8> {
        let mut bmhd = alloc::vec![0u8; 20];
        bmhd[..2].copy_from_slice(&3u16.to_be_bytes());
        bmhd[2..4].copy_from_slice(&1u16.to_be_bytes());
        bmhd[8] = 1;
        bmhd[9] = masking;
        bmhd[12..14].copy_from_slice(&transparent.to_be_bytes());
        let mut contents = Vec::new();
        for (id, data) in [
            (b"BMHD", bmhd),
            (b"CMAP", alloc::vec![0, 0, 0, 255, 255, 255]),
            (b"BODY", body.to_vec()),
        ] {
            contents.extend_from_slice(id);
            contents.extend_from_slice(&(data.len() as u32).to_be_bytes());
            contents.extend_from_slice(&data);
        }
        contents
    }

    #[test]
    fn a_mask_plane_makes_unset_pixels_clear() {
        // Pixels 1, 0, 1; the mask row follows the plane row: set, set, unset.
        let image = decode_ilbm(&one_plane(1, 0, &[0b1010_0000, 0, 0b1100_0000, 0])).unwrap();
        assert_eq!(image.get_argb(0, 0), 0xffff_ffff);
        assert_eq!(image.get_argb(1, 0), 0xff00_0000);
        assert_eq!(image.get_argb(2, 0), crate::image::CLEAR);
    }

    #[test]
    fn the_transparent_color_index_makes_its_pixels_clear() {
        let image = decode_ilbm(&one_plane(2, 0, &[0b1010_0000, 0])).unwrap();
        assert_eq!(image.get_argb(0, 0), 0xffff_ffff);
        assert_eq!(image.get_argb(1, 0), crate::image::CLEAR);
        assert_eq!(image.get_argb(2, 0), 0xffff_ffff);
        // Without masking 2 the index is an ordinary color.
        assert!(
            !decode_ilbm(&one_plane(0, 0, &[0b1010_0000, 0]))
                .unwrap()
                .has_alpha()
        );
    }

    #[test]
    fn super_hires_only_scales_on_native_monitors() {
        assert_eq!(scale_factors(0x29824), (1, 2)); // PAL super-hires lace
        assert_eq!(scale_factors(0x29820), (1, 4));
        assert_eq!(scale_factors(0x39024), (1, 1)); // VGAPRODUCT_KEY
        assert_eq!(scale_factors(0x89824), (1, 1)); // Super72
        assert_eq!(scale_factors(0x29804), (1, 1)); // hires lace
    }

    #[test]
    fn interlace_doubling_over_the_pixel_cap_is_rejected() {
        let (width, height) = (50000u16, 700u16);
        let mut bmhd = bmhd(width, height)[8..].to_vec();
        bmhd[10] = 1; // ByteRun1
        let mut body = Vec::new();
        for _ in 0..36000 {
            body.extend_from_slice(&[0x81, 0]);
        }
        let mut contents = Vec::new();
        for (id, data) in [
            (b"BMHD", bmhd),
            (b"CAMG", alloc::vec![0, 0, 0, 4]),
            (b"BODY", body),
        ] {
            contents.extend_from_slice(id);
            contents.extend_from_slice(&(data.len() as u32).to_be_bytes());
            contents.extend_from_slice(&data);
        }
        assert!(matches!(
            decode_ilbm(&contents),
            Err(DecodeError::Unrecognized)
        ));
    }
}
