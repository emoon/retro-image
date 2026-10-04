//! Raw screen dumps of the OS bitmap graphics modes.
//!
//! Sources:
//! - Modes: De Re Atari ch. 2 (<https://www.atariarchives.org/dere/chapt02.php>)
//!   and App. E (<https://www.atariarchives.org/dere/chaptE.php>); Mapping
//!   the Atari App. 15, colour registers 704-712
//!   (<https://www.atariarchives.org/mapping/appendix15.php>).
//! - GR7, GR8, GR9: Just Solve "GR*" (<http://fileformats.archiveteam.org/wiki/GR*>)
//!   and AtariWiki File Suffix
//!   (<https://atariwiki.org/wiki/Wiki.jsp?page=File+Suffix>): sizes.
//! - G10: Just Solve "GR*" (7689 bytes = screen + registers 704-712).
//! - G11: De Re Atari App. E (raw GTIA mode 11 dump).
//! - TXE (96 doubled GR9 lines) and ZM4 (64x64 greys drawn 4x4): sizes
//!   and layouts observed from `recoil2png` output.
//! - TX0 (Just Solve "Texture Maker0",
//!   <http://fileformats.archiveteam.org/wiki/Texture_Maker0>: 16x16, 16
//!   colours) and WND (Blazing Paddles window, up to 160x192, 4 colours;
//!   manual: <https://archive.org/details/BlazingPaddlesAtariSupplementManualBaudville>):
//!   header, layout, the hue OR and the window colours observed from
//!   `recoil2png` output.
//! - G09: sizes (7680, 15360) and the two screens side by side: observed
//!   from `recoil2png` output.
//! - MIC: Graph2Font manual (<https://g2f.atari8.info/instrukcja_eng.html>;
//!   screen + colours 712, 708, 709, 710; COL = 5 x 256 per-line colours).
//!   Which MIC sizes and COL sizes RECOIL pairs, and the table order:
//!   observed from `recoil2png` output.
//! - DRG: Just Solve "AtariCAD" (<http://fileformats.archiveteam.org/wiki/AtariCAD>;
//!   6400 bytes = 320x160 GR8).
//! - MBG: Just Solve "Mad Designer"
//!   (<http://fileformats.archiveteam.org/wiki/Mad_Designer>; 16384 bytes =
//!   512x256 mono).
//! - SKP: Sketch-PadDles page (<https://www.vitoco.cl/atari/10liner/SKETCH/>;
//!   raw 7680-byte GR15 screen).
//! - DIT: Just Solve "DrawIt" (<http://fileformats.archiveteam.org/wiki/DrawIt_(Atari)>;
//!   3845 bytes = GR7 screen + 5 colours).
//! - BKG: Just Solve "Movie Maker" (<http://fileformats.archiveteam.org/wiki/Movie_Maker>;
//!   3856 bytes = GR7 screen + 16 bytes).
//! - MGP: Just Solve "Magic Painter"
//!   (<http://fileformats.archiveteam.org/wiki/Magic_Painter>; 3845 bytes,
//!   starts `F4 0E 36 00`). The `.PIC` variant without the rainbow flag
//!   (screen at offset 5) was reverse engineered from a sample (dexvert
//!   `crumble.pic`).
//! - GR3: Mad Studio file formats PDF
//!   (<https://raw.githubusercontent.com/Gury8/Mad-Studio/master/docs/mad-studio-file-formats.pdf>;
//!   240 bytes + COLOR4, COLOR0-2).
//! - SG3: Just Solve "Standard Graphics 3"
//!   (<http://fileformats.archiveteam.org/wiki/Standard_Graphics_3_(Atari)>;
//!   40x24, 4 colours).
//! - AGP: Just Solve "AtariTools-800"
//!   (<http://fileformats.archiveteam.org/wiki/AtariTools-800>; exactly 7690
//!   bytes).
//! - Visualizer PIC: ANTIC "Rapid Graphics Converter" article
//!   (<https://www.atarimagazines.com/v4n7/rapidgraphicsconverter.html>;
//!   about 31 sectors, 160x79, 4 colours); size and layout observed from
//!   `recoil2png` output.
//! - PSF: AtariAge "Print Shop graphics" thread
//!   (<https://forums.atariage.com/topic/324752-print-shop-atari-related-graphics/>)
//!   and Just Solve "The Print Shop"
//!   (<http://justsolve.archiveteam.org/wiki/The_Print_Shop>; 572-byte raw
//!   88x52 bitmap).
//! - RAP: Just Solve "Vidig Paint" (<http://fileformats.archiveteam.org/wiki/Vidig_Paint>;
//!   7681 bytes = 7680 + 1).
//! - Observed from `recoil2png` output: accepted sizes, the 5-byte MIC tail,
//!   the GR8 colour tail, default colours, the fixed GR9/G11 luminances, the
//!   tail orders of DIT, BKG and MGP, the AGP header (mode, then
//!   registers 704-712), and GTIA mode 9 ORing pixels into the background.

use super::antic::Bitmap;
use super::palette::{register_rgb, rgb};
use crate::{Companions, DecodeError, Image};

const LINE: usize = 40;
const MAX_LINES: usize = 240;

/// The OS power-up colours: background, playfield 0-2.
pub(super) const OS_COLORS: [u8; 4] = [0x00, 0x28, 0xca, 0x94];
/// Grey ramp used when a GR15 file has no colours.
pub(super) const GREY_COLORS: [u8; 4] = [0x00, 0x04, 0x08, 0x0c];

/// Splits a dump of whole 40-byte lines (1 to 240) from the bytes after them.
pub(super) fn lines(data: &[u8]) -> Result<(Bitmap<'_>, &[u8]), DecodeError> {
    let lines = data.len() / LINE;
    if !(1..=MAX_LINES).contains(&lines) {
        return Err(DecodeError::Unrecognized);
    }
    let (screen, tail) = data.split_at(lines * LINE);
    Ok((bitmap(screen, LINE, 1), tail))
}

/// A bitmap of whole `bytes_per_line` lines; trailing bytes are ignored.
pub(super) fn bitmap(data: &[u8], bytes_per_line: usize, bits: u8) -> Bitmap<'_> {
    Bitmap {
        data,
        bytes_per_line,
        lines: data.len() / bytes_per_line,
        bits,
    }
}

/// `data` if it is exactly `len` bytes long.
pub(super) fn exactly(data: &[u8], len: usize) -> Result<&[u8], DecodeError> {
    if data.len() == len {
        Ok(data)
    } else {
        Err(DecodeError::Unrecognized)
    }
}

/// ANTIC mode F (Graphics 8): 1 bit per pixel, background and foreground.
pub(super) fn hires(bitmap: Bitmap<'_>, background: u32, foreground: u32) -> Image {
    bitmap.render(
        1,
        1,
        |_, value| {
            if value == 0 { background } else { foreground }
        },
    )
}

/// 2 bits per pixel (Graphics 3, 7, 15) indexing background, playfield 0-2.
pub(super) fn four_color(
    bitmap: Bitmap<'_>,
    pixel_width: u32,
    pixel_height: u32,
    colors: [u8; 4],
) -> Image {
    Bitmap { bits: 2, ..bitmap }.render(pixel_width, pixel_height, |_, value| {
        register_rgb(colors[usize::from(value)])
    })
}

/// GTIA mode 9: each pixel's luminance ORed into the background register
/// (whose luminance bit 0 is ignored).
pub(super) fn gtia9(bitmap: Bitmap<'_>, background: u8) -> Image {
    Bitmap { bits: 4, ..bitmap }.render(4, 1, |_, value| rgb(background & 0xfe | value))
}

/// GTIA mode 10: values index registers 704-712.
pub(super) fn gtia10(bitmap: Bitmap<'_>, registers: &[u8; 9]) -> Image {
    Bitmap { bits: 4, ..bitmap }.render(4, 1, |_, value| {
        register_rgb(registers[gtia10_register(value)])
    })
}

/// Values 9-11 show the background, 12-15 playfield 0-3.
pub(super) fn gtia10_register(value: u8) -> usize {
    match value {
        0..=8 => usize::from(value),
        9..=11 => 8,
        _ => usize::from(value) - 8,
    }
}

/// GTIA mode 11: 16 hues at the background luminance; hue 0 is black.
fn gtia11(bitmap: Bitmap<'_>, background: u8) -> Image {
    Bitmap { bits: 4, ..bitmap }.render(4, 1, |_, value| {
        if value == 0 {
            0
        } else {
            register_rgb(value << 4 | background & 0x0f)
        }
    })
}

/// Graphics 8: 320 pixels per line. Only a 7682-byte file stores colours:
/// background and foreground luminance after 192 lines.
pub(super) fn decode_gr8(data: &[u8]) -> Result<Image, DecodeError> {
    let (bitmap, tail) = lines(data)?;
    let (background, foreground) = match (data.len(), tail) {
        (7682, &[background, foreground]) => (background & 0x0e, foreground & 0x0e),
        _ => (0x00, 0x0e),
    };
    Ok(hires(bitmap, rgb(background), rgb(foreground)))
}

/// AtariCAD drawing: 160 lines of Graphics 8.
pub(super) fn decode_drg(data: &[u8]) -> Result<Image, DecodeError> {
    let (bitmap, _) = lines(exactly(data, 6400)?)?;
    Ok(hires(bitmap, rgb(0x00), rgb(0x0e)))
}

/// Mad Designer: 512x256 mono, 64 bytes per line.
pub(super) fn decode_mbg(data: &[u8]) -> Result<Image, DecodeError> {
    let screen = exactly(data, 16384)?;
    Ok(hires(bitmap(screen, 64, 1), rgb(0x00), rgb(0x0e)))
}

/// Vidig Paint: 192 lines of Graphics 9, then the background colour.
pub(super) fn decode_rap(data: &[u8]) -> Result<Image, DecodeError> {
    let (bitmap, tail) = lines(exactly(data, 7681)?)?;
    Ok(gtia9(bitmap, tail[0]))
}

/// Print Shop graphic: 88x52 mono, 11 bytes per line, black on white.
/// RECOIL accepts up to 68 trailing bytes.
pub(super) fn decode_psf(data: &[u8]) -> Result<Image, DecodeError> {
    if !(572..=640).contains(&data.len()) {
        return Err(DecodeError::Unrecognized);
    }
    Ok(hires(bitmap(&data[..572], 11, 1), rgb(0x0e), rgb(0x00)))
}

/// Graphics 9: 80 pixels of 16 grey luminances.
pub(super) fn decode_gr9(data: &[u8]) -> Result<Image, DecodeError> {
    let (bitmap, _) = lines(data)?;
    Ok(gtia9(bitmap, 0x00))
}

/// Graphics 9 from a G09 file: one 192-line screen, or two shown side by
/// side (left first).
pub(super) fn decode_g09(data: &[u8]) -> Result<Image, DecodeError> {
    match data.len() {
        7680 => Ok(gtia9(bitmap(data, LINE, 4), 0x00)),
        15360 => {
            let (left, right) = data.split_at(7680);
            let mut wide = alloc::vec::Vec::with_capacity(data.len());
            for (l, r) in left
                .as_chunks::<LINE>()
                .0
                .iter()
                .zip(right.as_chunks::<LINE>().0)
            {
                wide.extend_from_slice(l);
                wide.extend_from_slice(r);
            }
            Ok(gtia9(bitmap(&wide, 2 * LINE, 4), 0x00))
        }
        _ => Err(DecodeError::Unrecognized),
    }
}

/// TXE: 96 lines of Graphics 9 greys, each shown twice.
pub(super) fn decode_txe(data: &[u8]) -> Result<Image, DecodeError> {
    let screen = exactly(data, 3840)?;
    Ok(bitmap(screen, LINE, 4).render(4, 2, |_, value| rgb(value)))
}

/// Zoom 4: 64x64 greys, one nibble per pixel, drawn 4x4.
pub(super) fn decode_zm4(data: &[u8]) -> Result<Image, DecodeError> {
    let screen = exactly(data, 2048)?;
    Ok(bitmap(screen, 32, 4).render(4, 4, |_, value| rgb(value)))
}

/// Texture Maker0: 16x16 luminances (0-15), then the hue byte ORed into
/// each, drawn 4x4.
pub(super) fn decode_tx0(data: &[u8]) -> Result<Image, DecodeError> {
    let data = exactly(data, 257)?;
    let (pixels, &[hue]) = data.split_at(256) else {
        return Err(DecodeError::Unrecognized);
    };
    if pixels.iter().any(|&value| value > 15) {
        return Err(DecodeError::Unrecognized);
    }
    let mut image = Image::new(16, 16);
    for (i, &value) in pixels.iter().enumerate() {
        image.set(i as u32 % 16, i as u32 / 16, rgb(hue | value));
    }
    Ok(image.scaled(4, 4))
}

/// Blazing Paddles window: width - 1 and height, then Graphics 15 lines
/// of (width + 3) / 4 bytes, in a 3072-byte file.
pub(super) fn decode_wnd(data: &[u8]) -> Result<Image, DecodeError> {
    let data = exactly(data, 3072)?;
    let width = usize::from(data[0]) + 1;
    let height = usize::from(data[1]);
    let bytes_per_line = width.div_ceil(4);
    let screen = data
        .get(2..2 + bytes_per_line * height)
        .filter(|_| height > 0)
        .ok_or(DecodeError::Unrecognized)?;
    let colors = [0x00, 0x46, 0x88, 0x0e];
    let mut image = Image::new(width as u32, height as u32);
    let bitmap = bitmap(screen, bytes_per_line, 2);
    for y in 0..height {
        for x in 0..width {
            let color = register_rgb(colors[usize::from(bitmap.pixel(x, y))]);
            image.set(x as u32, y as u32, color);
        }
    }
    Ok(image.scaled(2, 1))
}

/// Graphics 10: screen, then the 9 registers 704-712.
pub(super) fn decode_g10(data: &[u8]) -> Result<Image, DecodeError> {
    let screen_len = data.len().checked_sub(9).ok_or(DecodeError::Unrecognized)?;
    if screen_len % LINE != 0 {
        return Err(DecodeError::Unrecognized);
    }
    let (screen, registers) = data.split_at(screen_len);
    let (bitmap, _) = lines(screen)?;
    let registers = registers
        .try_into()
        .map_err(|_| DecodeError::Unrecognized)?;
    Ok(gtia10(bitmap, registers))
}

/// Graphics 11 at luminance 6.
pub(super) fn decode_g11(data: &[u8]) -> Result<Image, DecodeError> {
    let (bitmap, _) = lines(data)?;
    Ok(gtia11(bitmap, 0x06))
}

/// Graphics 7: 160x96, then background and playfield 0-2.
pub(super) fn decode_gr7(data: &[u8]) -> Result<Image, DecodeError> {
    let (bitmap, tail) = lines(exactly(data, 3844)?)?;
    Ok(four_color(
        bitmap,
        2,
        2,
        [tail[0], tail[1], tail[2], tail[3]],
    ))
}

/// DrawIt: Graphics 7, then playfield 0-3 and background.
pub(super) fn decode_dit(data: &[u8]) -> Result<Image, DecodeError> {
    let (bitmap, tail) = lines(exactly(data, 3845)?)?;
    Ok(four_color(
        bitmap,
        2,
        2,
        [tail[4], tail[0], tail[1], tail[2]],
    ))
}

/// Movie Maker background: Graphics 7, then background and playfield 0-2,
/// then 12 unused bytes.
pub(super) fn decode_bkg(data: &[u8]) -> Result<Image, DecodeError> {
    let (bitmap, tail) = lines(exactly(data, 3856)?)?;
    Ok(four_color(
        bitmap,
        2,
        2,
        [tail[0], tail[1], tail[2], tail[3]],
    ))
}

/// Magic Painter: playfield 0-2, background, an unknown byte, a rainbow
/// flag, then a Graphics 7 screen lacking its last byte. The flag is 4 for
/// fixed colours or 3 when playfield 2 cycles through all colours, starting
/// at 0x10 and stepping once per line.
pub(super) fn decode_mgp(data: &[u8]) -> Result<Image, DecodeError> {
    let data = exactly(data, 3845)?;
    let rainbow = match data[5] {
        3 => true,
        4 => false,
        _ => return Err(DecodeError::Unrecognized),
    };
    let mut screen = [0; 3840];
    screen[..3839].copy_from_slice(&data[6..]);
    let (bitmap, _) = lines(&screen)?;
    let colors = [data[3], data[0], data[1], data[2]];
    Ok(
        Bitmap { bits: 2, ..bitmap }.render(2, 2, |line, value| match value {
            3 if rainbow => register_rgb((0x10 + line) as u8),
            _ => register_rgb(colors[usize::from(value)]),
        }),
    )
}

/// Magic Painter picture saved as `.PIC`: playfield 0-2, background, an
/// unused zero byte, then the whole Graphics 7 screen (no rainbow flag).
pub(super) fn decode_mgp_pic(data: &[u8]) -> Result<Image, DecodeError> {
    let data = exactly(data, 3845)?;
    if data[4] != 0 {
        return Err(DecodeError::Unrecognized);
    }
    let (bitmap, _) = lines(&data[5..])?;
    Ok(four_color(
        bitmap,
        2,
        2,
        [data[3], data[0], data[1], data[2]],
    ))
}

/// Visualizer: playfield 0-3 and background, then 79 Graphics 7 lines and
/// 160 unused bytes.
pub(super) fn decode_visualizer(data: &[u8]) -> Result<Image, DecodeError> {
    let data = exactly(data, 3325)?;
    let colors = [data[4], data[0], data[1], data[2]];
    Ok(four_color(
        bitmap(&data[5..5 + 79 * LINE], LINE, 2),
        2,
        2,
        colors,
    ))
}

/// Graphics 3: 40x24 pixels, then background and playfield 0-2.
pub(super) fn decode_gr3(data: &[u8]) -> Result<Image, DecodeError> {
    let data = exactly(data, 244)?;
    let colors = [data[240], data[241], data[242], data[243]];
    Ok(four_color(bitmap(&data[..240], 10, 2), 8, 8, colors))
}

/// Standard Graphics 3: 40x24 pixels in the OS colours.
pub(super) fn decode_sg3(data: &[u8]) -> Result<Image, DecodeError> {
    Ok(four_color(
        bitmap(exactly(data, 240)?, 10, 2),
        8,
        8,
        OS_COLORS,
    ))
}

/// Micro Illustrator / Graphics 15: 160 pixels, 4 colours. A 4-byte tail is
/// background and playfield 0-2; a 5-byte tail is playfield 0-2, background
/// and an unused byte; otherwise grey defaults apply. A 240-line picture
/// takes per-line colours from a Graph2Font `.COL` file of 1024 or 1280
/// bytes when present: table `value` (background, playfield 0-2), entry
/// `line`.
pub(super) fn decode_mic(data: &[u8], companions: &dyn Companions) -> Result<Image, DecodeError> {
    let (bitmap, tail) = lines(data)?;
    let colors = match *tail {
        [] | [_, _, _] => GREY_COLORS,
        [background, pf0, pf1, pf2] | [pf0, pf1, pf2, background, _] => [background, pf0, pf1, pf2],
        _ => return Err(DecodeError::Unrecognized),
    };
    let tables = (bitmap.lines == 240)
        .then(|| companions.get("col"))
        .flatten()
        .filter(|col| matches!(col.len(), 1024 | 1280));
    if let Some(tables) = tables {
        return Ok(Bitmap { bits: 2, ..bitmap }.render(2, 1, |line, value| {
            register_rgb(tables[usize::from(value) * 256 + line])
        }));
    }
    Ok(four_color(bitmap, 2, 1, colors))
}

/// Sketch-PadDles: a bare Graphics 15 screen in the program's colours.
pub(super) fn decode_skp(data: &[u8]) -> Result<Image, DecodeError> {
    let (bitmap, _) = lines(exactly(data, 7680)?)?;
    Ok(four_color(bitmap, 2, 1, [0x26, 0x28, 0x00, 0x0c]))
}

/// AtariTools-800 graphic: OS graphics mode, registers 704-712, then 7680
/// bytes of screen.
pub(super) fn decode_agp(data: &[u8]) -> Result<Image, DecodeError> {
    let data = exactly(data, 7690)?;
    let (header, screen) = data.split_at(10);
    let registers: &[u8; 9] = header[1..]
        .try_into()
        .map_err(|_| DecodeError::Unrecognized)?;
    let [.., pf0, pf1, pf2, _, background] = *registers;
    let (bitmap, _) = lines(screen)?;
    match header[0] {
        8 => Ok(hires(
            bitmap,
            register_rgb(pf2),
            register_rgb(pf2 & 0xf0 | pf1 & 0x0f),
        )),
        9 => Ok(gtia9(bitmap, background)),
        10 => Ok(gtia10(bitmap, registers)),
        11 => Ok(gtia11(bitmap, background)),
        15 => Ok(four_color(bitmap, 2, 1, [background, pf0, pf1, pf2])),
        _ => Err(DecodeError::Unrecognized),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gtia10_folds_registers() {
        let mapped: [usize; 16] = core::array::from_fn(|v| gtia10_register(v as u8));
        assert_eq!(mapped, [0, 1, 2, 3, 4, 5, 6, 7, 8, 8, 8, 8, 4, 5, 6, 7]);
    }

    #[test]
    fn mic_tail_orders() {
        let mut data = [0u8; 44];
        data[0] = 0b0001_1011;
        data[40..].copy_from_slice(&[0x02, 0x14, 0x26, 0x38]);
        let four = decode_mic(&data, &crate::NoCompanions).unwrap();
        assert_eq!(&four.rgb()[..3], &[0x22, 0x22, 0x22]);
        let mut five = [0u8; 45];
        five[..40].copy_from_slice(&data[..40]);
        five[40..].copy_from_slice(&[0x14, 0x26, 0x38, 0x02, 0xff]);
        assert_eq!(decode_mic(&five, &crate::NoCompanions).unwrap(), four);
        assert_eq!(
            decode_mic(&[0; 41], &crate::NoCompanions),
            Err(DecodeError::Unrecognized)
        );
    }

    struct Col(usize);

    impl Companions for Col {
        fn get_named(&self, _file_name: &str) -> Option<alloc::vec::Vec<u8>> {
            None
        }
        fn get(&self, extension: &str) -> Option<alloc::vec::Vec<u8>> {
            // Table t, line y holds hue t + 1, luminance y.
            let col = (0..self.0).map(|i| (((i / 256 + 1) << 4) | (i % 16)) as u8);
            (extension == "col").then(|| col.collect())
        }
    }

    #[test]
    fn mic_col_gives_per_line_colours_to_240_lines() {
        let mut data = [0u8; 9600];
        data[2 * 40] = 0b1000_0000; // line 2, pixel 0: playfield 1
        let image = decode_mic(&data, &Col(1280)).unwrap();
        assert_eq!(image.get(0, 2), register_rgb(0x32));
        assert_eq!(image.get(2, 2), register_rgb(0x12));
        let greys = decode_mic(&data, &crate::NoCompanions).unwrap();
        assert_eq!(decode_mic(&data, &Col(1000)).unwrap(), greys);
        let short = decode_mic(&data[..7680], &Col(1024)).unwrap();
        assert_eq!(short.get(2, 2), register_rgb(GREY_COLORS[0]));
    }

    #[test]
    fn wnd_window_must_fit_the_file() {
        let mut data = [0u8; 3072];
        data[..2].copy_from_slice(&[0x59, 133]);
        assert_eq!(decode_wnd(&data).unwrap().height(), 133);
        data[1] = 134;
        assert!(decode_wnd(&data).is_err());
        data[1] = 0;
        assert!(decode_wnd(&data).is_err());
    }

    #[test]
    fn tx0_ors_hue_into_luminance() {
        let mut data = [0u8; 257];
        data[0] = 0x05;
        data[256] = 0xa3;
        assert_eq!(decode_tx0(&data).unwrap().get(3, 3), rgb(0xa7));
        data[1] = 0x10;
        assert!(decode_tx0(&data).is_err());
    }

    #[test]
    fn rejects_bad_sizes() {
        assert!(decode_gr8(&[0; 39]).is_err());
        assert!(decode_gr8(&[0; 9640]).is_err());
        assert!(decode_g10(&[0; 7690]).is_err());
        assert!(decode_g10(&[0; 9]).is_err());
        assert!(decode_gr7(&[0; 3840]).is_err());
        assert!(decode_agp(&[0; 7690]).is_err());
    }
}
