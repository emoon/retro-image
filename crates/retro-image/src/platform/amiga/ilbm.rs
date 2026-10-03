//! IFF bitmaps: ILBM (incl. HAM6, HAM8, EHB, 24-bit), PBM and ACBM.
//!
//! Sources:
//! - ILBM: <https://wiki.amigaos.net/wiki/ILBM_IFF_Interleaved_Bitmap>
//!   (BMHD, CMAP, CAMG, BODY, masking, ByteRun1, HAM and EHB).
//! - ACBM: <https://wiki.amigaos.net/wiki/ACBM_IFF_Amiga_Continuous_Bitmap>
//!   (ABIT holds whole planes one after another).
//! - HAM8 control bits: <https://en.wikipedia.org/wiki/Hold-And-Modify>.
//! - PBM: chunky 8-bit BODY with rows padded to even length (reverse engineered
//!   from samples).
//! - Pixel doubling for interlaced low-res and non-interlaced high-res
//!   screens: observed from `recoil2png` output.

use alloc::vec::Vec;

use super::iff::find;
use super::multi_palette::LinePalettes;
use super::vdat;
use crate::bytes::{be16, be32};
use crate::codec::packbits;
use crate::image::{check_size, planar_pixels};
use crate::{DecodeError, Image};

pub(super) const CAMG_LACE: u32 = 0x4;
const CAMG_EHB: u32 = 0x80;
pub(super) const CAMG_HAM: u32 = 0x800;
pub(super) const CAMG_HIRES: u32 = 0x8000;

/// Fields of the BMHD chunk we use.
pub(super) struct Header {
    pub width: usize,
    pub height: usize,
    pub planes: usize,
    masking: u8,
    compression: u8,
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
        mode.render_row(row, &palette, |x, color| {
            image.set(x as u32, y as u32, color)
        });
    }
    Ok(scale(image, camg.unwrap_or(0)))
}

/// The BMHD, CAMG and raw pixel values (palette indices or packed RGB,
/// row-major) of a bitmap FORM, before any colour interpretation.
pub(super) struct Bitmap {
    pub header: Header,
    pub camg: Option<u32>,
    pub indices: Vec<u32>,
}

/// Reads the ILBM at `contents` without interpreting its pixel values.
pub(super) fn read_ilbm(contents: &[u8]) -> Result<Bitmap, DecodeError> {
    read_bitmap(contents, b"BODY", Layout::Interleaved)
}

fn read_bitmap(contents: &[u8], body_id: &[u8; 4], layout: Layout) -> Result<Bitmap, DecodeError> {
    let header = Header::parse(contents).ok_or(DecodeError::Unrecognized)?;
    let body = find(contents, body_id).ok_or(DecodeError::Unrecognized)?;
    let camg = find(contents, b"CAMG").and_then(|c| be32(c, 0));
    let indices = match layout {
        Layout::Chunky => read_chunky(&header, body)?,
        _ => read_planar(&header, body, layout)?,
    };
    Ok(Bitmap {
        header,
        camg,
        indices,
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
        (self.color(index) >> 1) & 0x7f7f7f
    }
}

/// Expands a 12-bit `0RGB` colour word.
pub(super) fn rgb12(word: u16) -> u32 {
    let word = u32::from(word);
    ((word & 0xf00) << 8 | (word & 0xf0) << 4 | (word & 0xf)) * 0x11
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

/// Pixel values (palette indices or packed RGB) of a planar body, row-major.
fn read_planar(header: &Header, body: &[u8], layout: Layout) -> Result<Vec<u32>, DecodeError> {
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
    Ok(planar_pixels(
        &data,
        header.width,
        header.height,
        row_len,
        header.planes,
        |plane, y| match layout {
            Layout::Contiguous => (plane * header.height + y) * row_len,
            _ => (y * stored_planes + plane) * row_len,
        },
    ))
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
        Ok(match header.planes {
            _ if layout == Layout::Chunky => Self::Indexed,
            6 if camg_bits & CAMG_HAM != 0 => Self::Ham6,
            8 if camg_bits & CAMG_HAM != 0 => Self::Ham8,
            6 if camg_bits & CAMG_EHB != 0 || (camg.is_none() && colors <= 32) => {
                Self::ExtraHalfBrite
            }
            6 if camg.is_none() => Self::Ham6,
            1..=8 => Self::Indexed,
            24 => Self::TrueColor,
            _ => return Err(DecodeError::Unrecognized),
        })
    }

    fn render_row(&self, row: &[u32], palette: &Palette, mut put: impl FnMut(usize, u32)) {
        let lookup = |i: u32| palette.color(i);
        let mut held = lookup(0);
        for (x, &v) in row.iter().enumerate() {
            let color = match self {
                Self::Indexed => lookup(v),
                Self::ExtraHalfBrite if v >= 32 => palette.half(v - 32),
                Self::ExtraHalfBrite => lookup(v),
                Self::Ham6 => ham(held, v >> 4, (v & 15) * 0x11, lookup(v & 15)),
                Self::Ham8 => {
                    let data = v & 63;
                    ham(held, v >> 6, data << 2 | data >> 4, lookup(data))
                }
                Self::TrueColor => (v & 0xff) << 16 | (v & 0xff00) | (v >> 16 & 0xff),
            };
            held = color;
            put(x, color);
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

/// Doubles pixels so the picture keeps its aspect ratio.
fn scale(image: Image, camg: u32) -> Image {
    let lace = camg & CAMG_LACE != 0;
    let hires = camg & CAMG_HIRES != 0;
    match (hires, lace) {
        (false, true) => image.scaled(2, 1),
        (true, false) => image.scaled(1, 2),
        _ => image,
    }
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

    #[test]
    fn header_rejects_pictures_over_the_pixel_cap() {
        assert!(Header::parse(&bmhd(100, 100)).is_some());
        assert!(Header::parse(&bmhd(65535, 65535)).is_none());
    }
}
