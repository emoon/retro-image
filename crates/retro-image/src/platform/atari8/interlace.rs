//! Pictures of two bitmap frames shown alternately (interlace).
//!
//! Sources:
//! - INP: Just Solve "InterPainter"
//!   (<http://fileformats.archiveteam.org/wiki/InterPainter>), XL-Paint 1.9
//!   MaX doc, 16004 bytes
//!   (<http://ftp.pigwa.net/stuff/collections/atari_forever/Tools%20-%20atr/XL-Paint%201.9Max.txt>),
//!   atari-owner.com "Atari Software Graphic Modes", frames flipped per VBI
//!   (<https://atari-owner.com/club/articles/atari-software-graphic-modes.17/>).
//! - INT: Just Solve "INT95a" (<http://fileformats.archiveteam.org/wiki/INT95a>;
//!   signature, up to 160x239, 2 frames); the header fields and frame
//!   layout observed from `recoil2png` output.
//! - MCP: Just Solve "McPainter"
//!   (<http://fileformats.archiveteam.org/wiki/McPainter>; 16008 bytes,
//!   160x200, 2 frames).
//! - MCPP: Just Solve "Paradox"
//!   (<http://fileformats.archiveteam.org/wiki/Paradox_(graphics)>; 8008
//!   bytes, 160x100).
//! - SHC: Just Solve "SAMAR Hires Interlace"
//!   (<http://fileformats.archiveteam.org/wiki/SAMAR_Hires_Interlace>;
//!   exactly 17920 bytes, 320x192, 2 frames with a color map). The layout
//!   was reverse engineered by black-box probing of `recoil2png` with
//!   modified and hand-made files: two 7680-byte Graphics 8 frames, then
//!   per frame 1280 bytes holding 6 colors for each of the 192 scanlines
//!   (the last 128 unused). Each color is the background (COLPF2) of a
//!   fixed span of the scanline, a mid-line register change: frame 1
//!   switches at pixels 94, 166, 214, 262 and 306, frame 2 at 46, 142, 190,
//!   238 and 286. Set pixels show the background hue at luminance 0.
//! - IST: Just Solve "Atari Interlaced Studio"
//!   (<http://fileformats.archiveteam.org/wiki/Atari_Interlaced_Studio>;
//!   exactly 17184 bytes, 160x200, 2 frames); the frame offsets and
//!   per-line color tables are observed from `recoil2png` output.
//! - HCI: Just Solve "HCI" (<http://fileformats.archiveteam.org/wiki/HCI>;
//!   exactly 16006 bytes, 2 frames); the frame modes (Graphics 8 and 15)
//!   and color layout observed from `recoil2png` output.
//! - Observed from `recoil2png` output: the frames are shown as the average
//!   of their colors; INP keeps 4 colors after the frames (and RECOIL
//!   accepts trailing data); MCP and MCPP store two color sets (playfield
//!   0-2, background) that swap between the frames on every line; MCPP's
//!   frames are 100-line halves shown on alternate lines, not mixed.

use super::antic::Bitmap;
use super::palette::register_rgb;
use crate::{DecodeError, Image};

const FRAME: usize = 8000;

fn frame(data: &[u8], lines: usize) -> Bitmap<'_> {
    Bitmap {
        data: &data[..40 * lines],
        bytes_per_line: 40,
        lines,
        bits: 2,
    }
}

/// InterPainter: two 160x200 frames, then background and playfield 0-2.
pub(super) fn decode_inp(data: &[u8]) -> Result<Image, DecodeError> {
    let colors: [u8; 4] = data
        .get(2 * FRAME..2 * FRAME + 4)
        .and_then(|c| c.try_into().ok())
        .ok_or(DecodeError::Invalid)?;
    let color = |_, value: u8| register_rgb(colors[usize::from(value)]);
    Ok(Image::blend(&[
        &frame(data, 200).render(2, 1, color)?,
        &frame(&data[FRAME..], 200).render(2, 1, color)?,
    ]))
}

/// INT95a: `INT95a`, width in bytes, height, `0F 2B`, the color sets
/// (background, playfield 0-2) of both frames, then both frames. (`.INT`
/// files without the signature are InterPainter pictures.)
pub(super) fn decode_int(data: &[u8]) -> Result<Image, DecodeError> {
    let rest = data.strip_prefix(b"INT95a").ok_or(DecodeError::Invalid)?;
    let [width, height, 0x0f, 0x2b, ref rest @ ..] = *rest else {
        return Err(DecodeError::Invalid);
    };
    let (width, height) = (usize::from(width), usize::from(height));
    let frame_len = width * height;
    if width == 0 || !(1..=239).contains(&height) || rest.len() != 8 + 2 * frame_len {
        return Err(DecodeError::Invalid);
    }
    let (sets, frames) = rest.split_at(8);
    let frame_image = |frame: &[u8], set: &[u8]| {
        let bitmap = Bitmap {
            data: frame,
            bytes_per_line: width,
            lines: height,
            bits: 2,
        };
        bitmap.render(2, 1, |_, value| register_rgb(set[usize::from(value)]))
    };
    let (first, second) = frames.split_at(frame_len);
    Ok(Image::blend(&[
        &frame_image(first, &sets[..4])?,
        &frame_image(second, &sets[4..])?,
    ]))
}

/// HCI: a 320x200 Graphics 8 frame and a 160x200 Graphics 15 frame, then
/// the Graphics 8 background and foreground luminance, then the Graphics 15
/// background and playfield 0-2.
pub(super) fn decode_hci(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 2 * FRAME + 6 {
        return Err(DecodeError::Invalid);
    }
    let colors = &data[2 * FRAME..];
    let background = register_rgb(colors[0]);
    let foreground = register_rgb(colors[0] & 0xf0 | colors[1] & 0x0f);
    let hires = Bitmap {
        bits: 1,
        ..frame(data, 200)
    }
    .render(
        1,
        1,
        |_, value| if value == 0 { background } else { foreground },
    );
    let multicolor = frame(&data[FRAME..], 200).render(2, 1, |_, value| {
        register_rgb(colors[2 + usize::from(value)])
    });
    Ok(Image::blend(&[&hires?, &multicolor?]))
}

/// Atari Interlace Studio: two 160x200 frames at offsets 16 and 8208, then
/// at 16384 four tables of 200 per-line colors: background, playfield 0-2.
pub(super) fn decode_ist(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 17184 {
        return Err(DecodeError::Invalid);
    }
    let tables = &data[16384..];
    let color = |line: usize, value: u8| register_rgb(tables[usize::from(value) * 200 + line]);
    Ok(Image::blend(&[
        &frame(&data[16..], 200).render(2, 1, color)?,
        &frame(&data[8208..], 200).render(2, 1, color)?,
    ]))
}

/// McPainter: two 160x200 frames, then two color sets. On even lines the
/// first frame uses the first set, on odd lines the second.
pub(super) fn decode_mcp(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != 2 * FRAME + 8 {
        return Err(DecodeError::Invalid);
    }
    mcp_picture(data, 200)
}

/// Two `lines`-line frames and two color sets, which swap between the
/// frames on alternate lines. `data` must hold exactly that.
pub(super) fn mcp_picture(data: &[u8], lines: usize) -> Result<Image, DecodeError> {
    let frame_len = 40 * lines;
    let sets = color_sets(&data[2 * frame_len..]);
    let frame_image = |offset: usize, first: usize| {
        frame(&data[offset..], lines).render(2, 1, |line, value| {
            register_rgb(sets[(line + first) % 2][usize::from(value)])
        })
    };
    Ok(Image::blend(&[
        &frame_image(0, 0)?,
        &frame_image(frame_len, 1)?,
    ]))
}

/// Paradox: two 160x100 halves, then two color sets. Even output lines
/// come from the first half in the first set, odd lines from the second
/// half in the second set; nothing is mixed.
pub(super) fn decode_mcpp(data: &[u8]) -> Result<Image, DecodeError> {
    const HALF: usize = FRAME / 2;
    if data.len() != 2 * HALF + 8 {
        return Err(DecodeError::Invalid);
    }
    let sets = color_sets(&data[2 * HALF..]);
    let mut lines = [0u8; 2 * HALF];
    for (i, line) in lines.as_chunks_mut::<40>().0.iter_mut().enumerate() {
        let source = (i % 2) * HALF + (i / 2) * 40;
        line.copy_from_slice(&data[source..source + 40]);
    }
    frame(&lines, 200).render(2, 1, |line, value| {
        register_rgb(sets[line % 2][usize::from(value)])
    })
}

/// Two sets of playfield 0-2 and background, as background-first palettes.
fn color_sets(data: &[u8]) -> [[u8; 4]; 2] {
    let set = |s: &[u8]| [s[3], s[0], s[1], s[2]];
    [set(&data[..4]), set(&data[4..8])]
}

/// SAMAR Hires Interlace: two Graphics 8 frames, then each frame's 6
/// background colors per scanline, changed at fixed pixels mid-line.
pub(super) fn decode_shc(data: &[u8]) -> Result<Image, DecodeError> {
    const SCREEN: usize = 7680;
    const SPLITS: [[usize; 5]; 2] = [[94, 166, 214, 262, 306], [46, 142, 190, 238, 286]];
    if data.len() != 17920 {
        return Err(DecodeError::Invalid);
    }
    let frame_image = |f: usize| -> Result<Image, DecodeError> {
        let bitmap = &data[f * SCREEN..(f + 1) * SCREEN];
        let colors = &data[2 * SCREEN + 1280 * f..];
        let mut image = Image::new(320, 192)?;
        for y in 0..192 {
            for x in 0..320 {
                let span = SPLITS[f].iter().filter(|&&split| x >= split).count();
                let background = colors[6 * y + span];
                let set = bitmap[40 * y + x / 8] >> (7 - x % 8) & 1 != 0;
                let color = if set { background & 0xf0 } else { background };
                image.set(x as u32, y as u32, register_rgb(color));
            }
        }
        Ok(image)
    };
    Ok(Image::blend(&[&frame_image(0)?, &frame_image(1)?]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn inp_mixes_frames() {
        let mut data = vec![0u8; 2 * FRAME + 4];
        data[0] = 0xc0; // frame 1: playfield 2 at pixel 0
        data[2 * FRAME..].copy_from_slice(&[0x00, 0x00, 0x00, 0x0e]);
        let image = decode_inp(&data).unwrap();
        assert_eq!(&image.rgb()[..3], &[0x77, 0x77, 0x77]);
        assert!(decode_inp(&data[..2 * FRAME + 3]).is_err());
    }

    #[test]
    fn shc_changes_colour_mid_line() {
        let mut data = vec![0u8; 17920];
        data[0] = 0x80; // frame 1: pixel 0 set
        for f in 0..2 {
            let colors = 15360 + 1280 * f;
            data[colors..colors + 6].copy_from_slice(&[0x0e, 0x0e, 0x0e, 0x0e, 0x0e, 0x0e]);
        }
        data[15360 + 1] = 0x00; // frame 1, second span (pixels 94-165): black
        let image = decode_shc(&data).unwrap();
        // Pixel 0: frame 1 hue 0 at luminance 0, frame 2 white.
        assert_eq!(image.get(0, 0), 0x777777);
        assert_eq!(image.get(93, 0), 0xeeeeee);
        assert_eq!(image.get(94, 0), 0x777777);
        assert!(decode_shc(&data[1..]).is_err());
    }

    #[test]
    fn mcpp_alternates_halves() {
        let mut data = vec![0u8; FRAME + 8];
        data[FRAME / 2] = 0x40; // second half, line 0: playfield 0
        data[FRAME..].copy_from_slice(&[0, 0, 0, 0, 0x0e, 0, 0, 0]);
        let image = decode_mcpp(&data).unwrap();
        assert_eq!(&image.rgb()[..3], &[0, 0, 0]);
        let line1 = 320 * 3;
        assert_eq!(&image.rgb()[line1..line1 + 3], &[0xee, 0xee, 0xee]);
    }
}
