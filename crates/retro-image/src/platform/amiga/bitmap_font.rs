//! Amiga bitmap fonts: the font descriptor file of one size (the numbered
//! file in a font's directory), shown as a sheet of its glyphs.
//!
//! Sources:
//! - AmigaOS wiki, "Graphics Library and Text", sections "How is an Amiga
//!   Font Structured in Memory?", "But What About Color Fonts?" and
//!   "Composition of a Bitmap Font on Disk"
//!   (<https://wiki.amigaos.net/wiki/Graphics_Library_and_Text>): the
//!   `DiskFontHeader` inside a loadable hunk, `TextFont` (`tf_YSize`,
//!   `tf_LoChar`, `tf_HiChar`, `tf_CharData`, `tf_Modulo`, `tf_CharLoc`),
//!   `ColorTextFont` and `ColorFontColors`.
//! - The hunk file layout (`HUNK_HEADER` 0x3f3, `HUNK_CODE` 0x3e9,
//!   `HUNK_DATA` 0x3ea, `HUNK_RELOC32`, `HUNK_END`): AmigaDOS technical
//!   reference, "Amiga Hunk" format, as summarized at
//!   <https://wiki.amigaos.net/wiki/Amiga_Hunk_File_Format>.
//! - Checked on Sembiance's `font/amigaBitmapFont` and
//!   `amigaBitmapFontContent` samples (29 size files, 19 of them in the 13
//!   font directories next to their `.font` files; 5 color fonts): the font data is one hunk whose first
//!   longword is the `moveq #0,d0; rts` return code, followed by the
//!   `DiskFontHeader` (so the `TextFont` is at hunk offset 58). The pointers
//!   in it are offsets from the start of the hunk data, which is what the
//!   `HUNK_RELOC32` block would add the load address to, so the block is not
//!   read.
//!
//! Output: a glyph sheet of 16 glyphs per row in character code order, the
//! row of a character being its code divided by 16 (the first row holds
//! `tf_LoChar` rounded down to a multiple of 16), like a code chart. Every
//! cell is as wide as the widest glyph plus a pixel of margin on each side, and
//! as high as the font plus the same margin. Monochrome glyphs are black on the
//! shared transparent-fill gray; a color font's pixels use its color table,
//! value 0 staying transparent. The spacing and kerning tables are not used,
//! since glyphs sit in a grid instead of a line of text. The default glyph
//! after `tf_HiChar` is not shown. The `.font` file that lists a font's sizes
//! holds no pixels and is not decoded.
//!
//! RECOIL has no Amiga font support.

use alloc::vec::Vec;

use crate::bytes::{be16, be32};
use crate::image::{TRANSPARENT_FILL, check_size, rgb444};
use crate::{DecodeError, Image};

const HUNK_HEADER: u32 = 0x3f3;
const HUNK_CODE: u32 = 0x3e9;
const HUNK_DATA: u32 = 0x3ea;
/// Memory type flags in the top bits of a hunk's size word.
const HUNK_SIZE_MASK: u32 = 0x3fff_ffff;
/// Offset of the `DiskFontHeader` in the hunk: after the return code.
const HEADER_AT: usize = 4;
/// `DFH_ID`, and the `NT_FONT` node type.
const DISK_FONT_ID: u16 = 0x0f80;
const NODE_FONT: u8 = 12;
/// `TextFont` starts at this offset in a `DiskFontHeader`.
const TEXT_FONT_AT: usize = HEADER_AT + 54;
/// A `TextFont` is this long; a `ColorTextFont` extension follows it.
const TEXT_FONT_LEN: usize = 52;
const FSF_COLORFONT: u8 = 0x40;
const COLUMNS: usize = 16;
const MARGIN: usize = 1;

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    let hunk = font_hunk(data).ok_or(DecodeError::Unrecognized)?;
    let font = Font::parse(hunk).ok_or(DecodeError::Unrecognized)?;
    font.sheet(hunk)
}

/// The data of the only hunk of a font file.
fn font_hunk(data: &[u8]) -> Option<&[u8]> {
    // HUNK_HEADER, no resident libraries, one hunk numbered 0 to 0.
    if (be32(data, 0)?, be32(data, 4)?, be32(data, 8)?) != (HUNK_HEADER, 0, 1)
        || (be32(data, 12)?, be32(data, 16)?) != (0, 0)
    {
        return None;
    }
    let size = ((be32(data, 20)? & HUNK_SIZE_MASK) as usize).checked_mul(4)?;
    let kind = be32(data, 24)?;
    if (kind != HUNK_CODE && kind != HUNK_DATA)
        || (be32(data, 28)? as usize).checked_mul(4)? != size
    {
        return None;
    }
    data.get(32..32usize.checked_add(size)?)
}

/// Where a color font keeps its planes and colors.
struct Colors {
    depth: usize,
    pick: u8,
    on_off: u8,
    planes: Vec<usize>,
    table: Vec<u32>,
}

struct Font {
    height: usize,
    low: usize,
    high: usize,
    chars: usize,
    modulo: usize,
    /// Offsets in the hunk of the strike (the first plane of a color font)
    /// and the glyph location table.
    chardata: usize,
    locations: usize,
    color: Option<Colors>,
}

impl Font {
    fn parse(hunk: &[u8]) -> Option<Self> {
        let at = |offset: usize| TEXT_FONT_AT + offset;
        if be16(hunk, HEADER_AT + 14)? != DISK_FONT_ID || *hunk.get(HEADER_AT + 8)? != NODE_FONT {
            return None;
        }
        let height = usize::from(be16(hunk, at(20))?);
        let style = *hunk.get(at(22))?;
        let (low, high) = (*hunk.get(at(32))?, *hunk.get(at(33))?);
        let font = Self {
            height,
            low: usize::from(low),
            high: usize::from(high),
            chars: usize::from(high).checked_sub(usize::from(low))? + 1,
            modulo: usize::from(be16(hunk, at(38))?),
            chardata: be32(hunk, at(34))? as usize,
            locations: be32(hunk, at(40))? as usize,
            color: None,
        };
        let plane_len = font.height * font.modulo;
        if height == 0 || font.modulo == 0 {
            return None;
        }
        if style & FSF_COLORFONT == 0 {
            hunk.get(font.chardata..font.chardata.checked_add(plane_len)?)?;
            return Some(font);
        }
        let ext = at(TEXT_FONT_LEN);
        let depth = usize::from(*hunk.get(ext + 2)?);
        let planes: Vec<usize> = (0..depth.min(8))
            .map(|i| be32(hunk, ext + 12 + i * 4).map(|p| p as usize))
            .collect::<Option<_>>()?;
        for &plane in &planes {
            hunk.get(plane..plane.checked_add(plane_len)?)?;
        }
        let colors = be32(hunk, ext + 8)? as usize;
        let count = usize::from(be16(hunk, colors.checked_add(2)?)?);
        let table_at = be32(hunk, colors.checked_add(4)?)? as usize;
        let table = hunk
            .get(table_at..table_at.checked_add(count * 2)?)?
            .as_chunks::<2>()
            .0
            .iter()
            .map(|word| rgb444(u16::from_be_bytes(*word)))
            .collect();
        Some(Self {
            color: Some(Colors {
                depth: planes.len(),
                pick: *hunk.get(ext + 6)?,
                on_off: *hunk.get(ext + 7)?,
                planes,
                table,
            }),
            ..font
        })
    }

    /// The bit offset and width of every glyph from `tf_LoChar` to `tf_HiChar`.
    fn glyphs(&self, hunk: &[u8]) -> Option<Vec<(usize, usize)>> {
        let table = hunk.get(self.locations..self.locations.checked_add(self.chars * 4)?)?;
        let glyphs: Vec<(usize, usize)> = table
            .as_chunks::<4>()
            .0
            .iter()
            .map(|e| {
                (
                    usize::from(u16::from_be_bytes([e[0], e[1]])),
                    usize::from(u16::from_be_bytes([e[2], e[3]])),
                )
            })
            .collect();
        // Every glyph must lie within a row of the strike.
        glyphs
            .iter()
            .all(|&(offset, width)| offset + width <= self.modulo * 8)
            .then_some(glyphs)
    }

    /// The value (0 for blank) of bit `bit` of row `y` in the plane at `plane`.
    fn bit(&self, hunk: &[u8], plane: usize, y: usize, bit: usize) -> u8 {
        hunk[plane + y * self.modulo + bit / 8] >> (7 - bit % 8) & 1
    }

    /// The color of bit `bit` of row `y` of the strike, `None` where nothing
    /// is drawn.
    fn pixel(&self, hunk: &[u8], y: usize, bit: usize) -> Option<u32> {
        let Some(colors) = &self.color else {
            return (self.bit(hunk, self.chardata, y, bit) == 1).then_some(0);
        };
        // Plane `i` of the value comes from the next data plane if the
        // pick mask has bit `i`, else from the on/off mask.
        let mut next = colors.planes.iter();
        let mut value = 0;
        for i in 0..colors.depth {
            let plane_bit = if colors.pick >> i & 1 == 1 {
                self.bit(hunk, *next.next()?, y, bit)
            } else {
                colors.on_off >> i & 1
            };
            value |= usize::from(plane_bit) << i;
        }
        (value != 0).then(|| colors.table.get(value).copied().unwrap_or(0))
    }

    fn sheet(&self, hunk: &[u8]) -> Result<Image, DecodeError> {
        let fail = DecodeError::Unrecognized;
        let glyphs = self.glyphs(hunk).ok_or(fail)?;
        let widest = glyphs.iter().map(|&(_, width)| width).max().ok_or(fail)?;
        let (cell_w, cell_h) = (widest + 2 * MARGIN, self.height + 2 * MARGIN);
        let rows = self.high / COLUMNS - self.low / COLUMNS + 1;
        check_size(COLUMNS * cell_w, rows * cell_h)?;
        let (width, height) = ((COLUMNS * cell_w) as u32, (rows * cell_h) as u32);
        let mut image = Image::from_colors(width, height, core::iter::repeat(TRANSPARENT_FILL));
        for (i, &(offset, width)) in glyphs.iter().enumerate() {
            let code = self.low + i;
            let left = (code % COLUMNS) * cell_w + MARGIN;
            let top = (code / COLUMNS - self.low / COLUMNS) * cell_h + MARGIN;
            for y in 0..self.height {
                for x in 0..width {
                    if let Some(color) = self.pixel(hunk, y, offset + x) {
                        image.set((left + x) as u32, (top + y) as u32, color);
                    }
                }
            }
        }
        Ok(image)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put16(hunk: &mut [u8], at: usize, value: u16) {
        hunk[at..at + 2].copy_from_slice(&value.to_be_bytes());
    }

    fn put32(hunk: &mut [u8], at: usize, value: usize) {
        hunk[at..at + 4].copy_from_slice(&(value as u32).to_be_bytes());
    }

    /// A font file with the glyphs 'A' (4 wide) and 'B' (3 wide), 2 rows
    /// high, in strikes of 2 bytes per row. `planes` holds the 4 bytes of each
    /// plane; more than one makes it a color font with the given color table
    /// and pick mask. The hunk is laid out as header, glyph locations,
    /// planes, then the color table.
    fn font_file(planes: &[[u8; 4]], colors: &[u16], pick: u8) -> Vec<u8> {
        let color = planes.len() > 1;
        let locations = TEXT_FONT_AT + TEXT_FONT_LEN + if color { 44 } else { 0 };
        let strike = locations + 8;
        let color_table = strike + 4 * planes.len();
        let mut hunk = alloc::vec![0u8; color_table + 8 + colors.len() * 2];
        hunk[0..4].copy_from_slice(&[0x70, 0x00, 0x4e, 0x75]);
        hunk[HEADER_AT + 8] = NODE_FONT;
        put16(&mut hunk, HEADER_AT + 14, DISK_FONT_ID);
        let tf = TEXT_FONT_AT;
        put16(&mut hunk, tf + 20, 2); // height
        hunk[tf + 22] = if color { FSF_COLORFONT } else { 0 };
        hunk[tf + 32] = b'A';
        hunk[tf + 33] = b'B';
        put32(&mut hunk, tf + 34, strike);
        put16(&mut hunk, tf + 38, 2); // modulo
        put32(&mut hunk, tf + 40, locations);
        if color {
            let ext = tf + TEXT_FONT_LEN;
            hunk[ext + 2] = planes.len() as u8;
            hunk[ext + 6] = pick;
            put32(&mut hunk, ext + 8, color_table);
            for i in 0..planes.len() {
                put32(&mut hunk, ext + 12 + i * 4, strike + i * 4);
            }
            put16(&mut hunk, color_table + 2, colors.len() as u16);
            put32(&mut hunk, color_table + 4, color_table + 8);
            for (i, word) in colors.iter().enumerate() {
                put16(&mut hunk, color_table + 8 + i * 2, *word);
            }
        }
        // Glyph locations: A at bit 0, 4 wide; B at bit 4, 3 wide.
        hunk[locations..locations + 8].copy_from_slice(&[0, 0, 0, 4, 0, 4, 0, 3]);
        for (i, plane) in planes.iter().enumerate() {
            hunk[strike + i * 4..][..4].copy_from_slice(plane);
        }
        let longs = hunk.len().div_ceil(4);
        hunk.resize(longs * 4, 0);
        let mut file = alloc::vec![0u8; 32];
        put32(&mut file, 0, HUNK_HEADER as usize);
        put32(&mut file, 8, 1);
        put32(&mut file, 20, longs);
        put32(&mut file, 24, HUNK_CODE as usize);
        put32(&mut file, 28, longs);
        file.extend_from_slice(&hunk);
        file.extend_from_slice(&[0, 0, 0x03, 0xf2]); // HUNK_END
        file
    }

    /// Row 0 of the strike: A 1111, B 101. Row 1: A 1001, B 010.
    const STRIKE: [u8; 4] = [0xfa, 0x00, 0x94, 0x00];

    /// The ink mask of the `w` x 2 glyph of character `code` in a one-row sheet
    /// with `cell` pixel wide cells.
    fn glyph(image: &Image, code: u32, w: u32, cell: u32) -> Vec<bool> {
        (0..2)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .map(|(x, y)| image.get((code % 16) * cell + 1 + x, 1 + y) != TRANSPARENT_FILL)
            .collect()
    }

    #[test]
    fn glyphs_sit_in_a_sixteen_column_code_chart() {
        let image = decode(&font_file(&[STRIKE], &[], 0)).unwrap();
        // Cells are 4 + 2 wide and 2 + 2 high, and the chart has one row.
        assert_eq!((image.width(), image.height()), (16 * 6, 4));
        assert_eq!(
            glyph(&image, 65, 4, 6),
            [true, true, true, true, true, false, false, true]
        );
        assert_eq!(
            glyph(&image, 66, 3, 6),
            [true, false, true, false, true, false]
        );
        assert_eq!(image.get(7, 1), 0, "ink is black");
        assert_eq!(image.get(0, 0), TRANSPARENT_FILL);
    }

    #[test]
    fn color_fonts_combine_their_planes_through_the_color_table() {
        // Plane 1 is the strike of 'A' and 'B' shifted by a row: its row 1
        // is the strike's row 0 and the other way round.
        let swapped = [0x94, 0x00, 0xfa, 0x00];
        let colors = [0x0000, 0x0f00, 0x00f0, 0x000f];
        let image = decode(&font_file(&[STRIKE, swapped], &colors, 3)).unwrap();
        // 'A' row 0: plane 0 = 1111, plane 1 = 1001: values 3, 1, 1, 3.
        let pixel = |x: u32, y: u32| image.get(6 + 1 + x, 1 + y);
        assert_eq!(
            [pixel(0, 0), pixel(1, 0), pixel(3, 0)],
            [0x0000ff, 0xff0000, 0x0000ff]
        );
        // Row 1: plane 0 = 1001, plane 1 = 1111: values 3, 2, 2, 3.
        assert_eq!(pixel(1, 1), 0x00ff00);
        // With only plane 0 picked and the on/off mask clear, plane 1 reads 0
        // and the color is red wherever plane 0 is set.
        let flat = decode(&font_file(&[STRIKE, swapped], &colors, 1)).unwrap();
        assert_eq!(flat.get(6 + 1 + 1, 1), 0xff0000);
        assert_eq!(flat.get(6 + 1 + 1, 2), TRANSPARENT_FILL);
    }

    #[test]
    fn pointers_at_the_end_of_the_address_space_are_rejected() {
        // Offsets are added to others: on a 32-bit target 0xffff_fffe + 4
        // wraps, which is a panic in a debug build.
        let swapped = [0x94, 0x00, 0xfa, 0x00];
        let colors = [0x0000, 0x0f00, 0x00f0, 0x000f];
        let file = font_file(&[STRIKE, swapped], &colors, 3);
        let colors_at = 32 + TEXT_FONT_AT + TEXT_FONT_LEN + 8;
        let mut far = file.clone();
        put32(&mut far, colors_at, 0xffff_fffe);
        assert!(decode(&far).is_err());
        let mut huge_hunk = file;
        put32(&mut huge_hunk, 28, 0xffff_ffff);
        assert!(decode(&huge_hunk).is_err());
    }

    #[test]
    fn rejects_other_hunk_files_and_damaged_fonts() {
        let file = font_file(&[STRIKE], &[], 0);
        assert!(decode(&file[..60]).is_err(), "truncated");
        let mut wrong_id = file.clone();
        wrong_id[32 + HEADER_AT + 14] = 0;
        assert!(decode(&wrong_id).is_err());
        let mut far_strike = file.clone();
        put32(&mut far_strike, 32 + TEXT_FONT_AT + 34, 5000);
        assert!(decode(&far_strike).is_err());
        let mut wide_glyph = file;
        wide_glyph[32 + TEXT_FONT_AT + TEXT_FONT_LEN + 3] = 40;
        assert!(decode(&wide_glyph).is_err());
    }
}
