//! Mad Studio player/missile and tile formats: SPR, MPL, MSL, TL4.
//!
//! Sources:
//! - Mad Studio file formats PDF (MIT-licensed project),
//!   <https://raw.githubusercontent.com/Gury8/Mad-Studio/master/docs/mad-studio-file-formats.pdf>.
//! - Player/missile pixel widths: Atari Player-Missile Graphics in BASIC,
//!   ch. 2 (<https://www.atariarchives.org/pmgraphics/chapter2.php>).
//! - PLA/MIS (AtariTools-800 player and missile, Just Solve
//!   "AtariTools-800", <http://fileformats.archiveteam.org/wiki/AtariTools-800>):
//!   sizes, color byte and bit order observed from `recoil2png` output.
//! - Observed from `recoil2png` output: the fixed SPR/MSL heights, the MPL
//!   variant with a 9-byte header (height, 4 X positions, 4 colors) that the
//!   sample uses, the MPL canvas (from the leftmost to the rightmost player)
//!   and player priority, the 174-byte MPL variant (14-byte header, bytes 9-13
//!   ignored; mutation probing of `recoil2png`, six samples in
//!   `corpus/extra/atari8/madstudio`), and the TL4 colors (OS defaults, playfield 3 = 0x46).

use super::antic::{Bitmap, fill};
use super::font::draw_multicolor_glyph;
use super::palette::{register_rgb, rgb};
use super::screen::OS_COLORS;
use crate::{DecodeError, Image};

/// A player pixel is 2 high-resolution pixels wide.
const PLAYER_PIXEL: u32 = 2;

/// Single player: height (always 40), color, 40 lines of 8 pixels.
pub(super) fn decode_spr(data: &[u8]) -> Result<Image, DecodeError> {
    let [40, color, ref lines @ ..] = *data else {
        return Err(DecodeError::Invalid);
    };
    if lines.len() != 40 {
        return Err(DecodeError::Invalid);
    }
    let bitmap = Bitmap {
        data: lines,
        bytes_per_line: 1,
        lines: 40,
        bits: 1,
    };
    let color = register_rgb(color);
    bitmap.render(
        PLAYER_PIXEL,
        1,
        |_, value| {
            if value == 0 { 0 } else { color }
        },
    )
}

/// Single missile: height (always 34), color, 34 lines of 2 pixels.
pub(super) fn decode_msl(data: &[u8]) -> Result<Image, DecodeError> {
    let [34, color, ref lines @ ..] = *data else {
        return Err(DecodeError::Invalid);
    };
    if lines.len() != 34 || lines.iter().any(|&line| line > 3) {
        return Err(DecodeError::Invalid);
    }
    let color = register_rgb(color);
    let mut image = Image::new(2 * PLAYER_PIXEL, 34)?;
    for (y, &line) in lines.iter().enumerate() {
        for x in 0..2 {
            if line & (2 >> x) != 0 {
                fill(
                    &mut image,
                    x * PLAYER_PIXEL,
                    y as u32,
                    PLAYER_PIXEL,
                    1,
                    color,
                );
            }
        }
    }
    Ok(image)
}

/// AtariTools-800 player: color, then 240 lines of 8 pixels.
pub(super) fn decode_pla(data: &[u8]) -> Result<Image, DecodeError> {
    let [color, ref lines @ ..] = *data else {
        return Err(DecodeError::Invalid);
    };
    if lines.len() != 240 {
        return Err(DecodeError::Invalid);
    }
    let bitmap = Bitmap {
        data: lines,
        bytes_per_line: 1,
        lines: 240,
        bits: 1,
    };
    let color = register_rgb(color);
    bitmap.render(
        PLAYER_PIXEL,
        1,
        |_, value| if value == 0 { 0 } else { color },
    )
}

/// AtariTools-800 missile: color (all 8 bits used), then 240 lines of
/// 2 pixels, 4 lines to a byte, first line in the high bits.
pub(super) fn decode_mis(data: &[u8]) -> Result<Image, DecodeError> {
    let [color, ref lines @ ..] = *data else {
        return Err(DecodeError::Invalid);
    };
    if lines.len() != 60 {
        return Err(DecodeError::Invalid);
    }
    let bitmap = Bitmap {
        data: lines,
        bytes_per_line: 1,
        lines: 60,
        bits: 2,
    };
    let color = rgb(color);
    let mut image = Image::new(2 * PLAYER_PIXEL, 240)?;
    for y in 0..240 {
        let line = bitmap.pixel(y % 4, y / 4);
        for x in 0..2 {
            if line & (2 >> x) != 0 {
                fill(
                    &mut image,
                    x * PLAYER_PIXEL,
                    y as u32,
                    PLAYER_PIXEL,
                    1,
                    color,
                );
            }
        }
    }
    Ok(image)
}

/// Four overlapping players: height, X positions, colors, then the data of
/// players 0-3. Player 0 has the highest priority.
pub(super) fn decode_mpl(data: &[u8]) -> Result<Image, DecodeError> {
    // The 174-byte layout has five more header bytes (player sizes and the
    // third-color flag) that are not needed to draw the players.
    let header_len = if data.len() == 14 + 4 * 40 && data[0] == 40 {
        14
    } else {
        9
    };
    let (header, players) = data
        .split_at_checked(header_len)
        .ok_or(DecodeError::Invalid)?;
    let height = usize::from(header[0]);
    if height == 0 || players.len() != 4 * height {
        return Err(DecodeError::Invalid);
    }
    let positions = &header[1..5];
    let colors = &header[5..9];
    let left = u32::from(*positions.iter().min().unwrap_or(&0));
    let right = positions
        .iter()
        .map(|&x| u32::from(x) + 8)
        .max()
        .unwrap_or(8);
    let mut image = Image::new((right - left) * PLAYER_PIXEL, height as u32)?;
    for player in (0..4).rev() {
        let color = register_rgb(colors[player]);
        let origin = u32::from(positions[player]) - left;
        for (y, &line) in players[player * height..][..height].iter().enumerate() {
            for bit in 0..8 {
                if line & (0x80 >> bit) != 0 {
                    let x = (origin + bit) * PLAYER_PIXEL;
                    fill(&mut image, x, y as u32, PLAYER_PIXEL, 1, color);
                }
            }
        }
    }
    Ok(image)
}

/// ANTIC mode 4 tile: width (1-4) and height (1-5) in characters, then per
/// character 8 bytes of glyph and an inverse flag selecting playfield 3
/// instead of playfield 2.
pub(super) fn decode_tl4(data: &[u8]) -> Result<Image, DecodeError> {
    let [width @ 1..=4, height @ 1..=5, ref chars @ ..] = *data else {
        return Err(DecodeError::Invalid);
    };
    let (width, height) = (usize::from(width), usize::from(height));
    if chars.len() != 9 * width * height {
        return Err(DecodeError::Invalid);
    }
    let [background, pf0, pf1, pf2] = OS_COLORS;
    let pf3 = 0x46;
    let mut image = Image::new(width as u32 * 8, height as u32 * 8)?;
    for (index, char) in chars.as_chunks::<9>().0.iter().enumerate() {
        let colors = [background, pf0, pf1, if char[8] != 0 { pf3 } else { pf2 }];
        let x = (index % width) as u32 * 8;
        let y = (index / width) as u32 * 8;
        draw_multicolor_glyph(&mut image, x, y, &char[..8], colors);
    }
    Ok(image)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mpl_canvas_spans_players() {
        let mut data = alloc::vec![1, 4, 8, 0, 0, 0x28, 0x34, 0x16, 0x98];
        data.extend_from_slice(&[0x80; 4]);
        let image = decode_mpl(&data).unwrap();
        assert_eq!(image.width(), 32);
        assert!(decode_mpl(&data[..12]).is_err());
    }

    #[test]
    fn mpl_long_header() {
        let mut data = alloc::vec![40, 4, 8, 0, 0, 0x28, 0x34, 0x16, 0x98, 1, 2, 3, 4, 5];
        data.extend_from_slice(&[0x80; 160]);
        assert_eq!(decode_mpl(&data).unwrap().height(), 40);
        data[0] = 39;
        assert!(decode_mpl(&data).is_err());
    }

    #[test]
    fn missile_lines_start_in_high_bits() {
        let mut data = [0u8; 61];
        data[0] = 0x0f;
        data[1] = 0b0100_0010; // line 0: right pixel; line 3: left pixel
        let image = decode_mis(&data).unwrap();
        assert_eq!((image.width(), image.height()), (4, 240));
        assert_eq!(image.get(0, 0), 0);
        assert_ne!(image.get(2, 0), 0);
        assert_ne!(image.get(0, 3), 0);
    }

    #[test]
    fn rejects_bad_headers() {
        assert!(decode_spr(&[10; 42]).is_err());
        assert!(decode_msl(&[34; 36]).is_err());
        assert!(decode_tl4(&[5, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0]).is_err());
        assert!(decode_tl4(&[1, 1]).is_err());
    }
}
