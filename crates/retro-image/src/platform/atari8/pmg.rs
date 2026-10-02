//! Player/missile sprite sheets: 4MI, 4PL, 4PM (AtariTools-800), APL (Atari
//! Player Editor), PMD (PMG Designer) and LDM (Ludek Maker).
//!
//! Sources:
//! - Player/missile geometry (players 8 bits wide, missiles 2 bits, a pixel is
//!   2 high-resolution pixels, overlapping players OR their colours): Atari
//!   Player-Missile Graphics in BASIC, ch. 2
//!   (<https://www.atariarchives.org/pmgraphics/chapter2.php>) and De Re Atari
//!   App. E (<https://www.atariarchives.org/dere/chaptE.php>).
//! - File names and sizes: Just Solve "AtariTools-800"
//!   (<http://fileformats.archiveteam.org/wiki/AtariTools-800>), "PMG Designer"
//!   (<http://fileformats.archiveteam.org/wiki/PMG_Designer>; starts `F0 ED E4`)
//!   and the Atari graphics formats list
//!   (<http://fileformats.archiveteam.org/wiki/Atari_graphics_formats>).
//! - LDM: Just Solve "Ludek Maker"
//!   (<http://fileformats.archiveteam.org/wiki/Ludek_Maker>; starts with the
//!   inverse-ATASCII text "Ludek Maker data file", 4 colours).
//! - The layouts are reverse engineered from the corpus samples (TEST_4M.4MI,
//!   4ALIENS.4PL, 4PM_TEST2.4PM, MWALK.APL, FAIRY.PMD, PMG Designer's CTR0,
//!   CTR1, HELI, and GO/SPYGO/ROTATE.LDM) and by `recoil2png` probing: one byte of a zero-filled file
//!   at a time was set and the pixel that lit up read back, which gives the
//!   placement of every data block, the colour byte of every frame, the
//!   accepted header ranges and the exact file lengths.

use super::antic::fill;
use super::palette::{register_rgb, rgb};
use crate::{DecodeError, Image};
use alloc::vec;
use alloc::vec::Vec;

/// A player pixel is 2 high-resolution pixels wide.
const PIXEL: u32 = 2;
/// Spacing of sprites on a sheet in player pixels: 8 and a gap of 2.
const CELL: usize = 10;
/// Lines of a full PMG memory area at double-line resolution.
const PM_LINES: usize = 240;

/// A sheet of player pixels where overlapping players OR their colour
/// register values, shown when the sheet is finished.
struct Sheet {
    width: usize,
    height: usize,
    colors: Vec<u8>,
}

impl Sheet {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            colors: vec![0; width * height],
        }
    }

    /// ORs `color` into the pixels where `bits` (bit 7 leftmost) is set.
    fn draw_player(&mut self, x: usize, y: usize, bits: u8, color: u8) {
        for bit in 0..8 {
            if bits & (0x80 >> bit) != 0 {
                self.colors[y * self.width + x + bit] |= color;
            }
        }
    }

    /// Scales every pixel to `PIXEL` wide; unset pixels stay black.
    fn finish(&self) -> Image {
        let mut image = Image::new(self.width as u32 * PIXEL, self.height as u32);
        for y in 0..self.height {
            for x in 0..self.width {
                let color = self.colors[y * self.width + x];
                if color != 0 {
                    fill(
                        &mut image,
                        x as u32 * PIXEL,
                        y as u32,
                        PIXEL,
                        1,
                        register_rgb(color),
                    );
                }
            }
        }
        image
    }
}

/// Draws the four missiles of one line byte (missile 0 in the low bits, the
/// left pixel in the high bit of each pair) at `x`, 8 image pixels apart,
/// each in its own full-resolution colour.
fn draw_missiles(image: &mut Image, x: u32, y: u32, line: u8, colors: &[u8]) {
    for (missile, &color) in colors.iter().enumerate().take(4) {
        for pixel in 0..2 {
            if line & (2 >> pixel) << (2 * missile) != 0 {
                let left = x + missile as u32 * 8 + pixel * PIXEL;
                fill(image, left, y, PIXEL, 1, rgb(color));
            }
        }
    }
}

/// Four missiles: colours of missiles 0-3, then 240 lines.
pub(super) fn decode_4mi(data: &[u8]) -> Result<Image, DecodeError> {
    let (colors, lines) = data.split_at_checked(4).ok_or(DecodeError::Unrecognized)?;
    if lines.len() != PM_LINES {
        return Err(DecodeError::Unrecognized);
    }
    let mut image = Image::new(32, PM_LINES as u32);
    for (y, &line) in lines.iter().enumerate() {
        draw_missiles(&mut image, 0, y as u32, line, colors);
    }
    Ok(image)
}

/// Draws players 0-3 side by side, one `CELL` apart, in the colours of
/// `colors`; each has `PM_LINES` lines in `players`.
fn draw_players(sheet: &mut Sheet, colors: &[u8], players: &[u8]) {
    for (player, memory) in players.chunks_exact(PM_LINES).enumerate() {
        for (y, &bits) in memory.iter().enumerate() {
            sheet.draw_player(player * CELL, y, bits, colors[player]);
        }
    }
}

/// Four players: colours of players 0-3, then 4 x 240 lines.
pub(super) fn decode_4pl(data: &[u8]) -> Result<Image, DecodeError> {
    let (colors, players) = data.split_at_checked(4).ok_or(DecodeError::Unrecognized)?;
    if players.len() != 4 * PM_LINES {
        return Err(DecodeError::Unrecognized);
    }
    let mut sheet = Sheet::new(4 * CELL, PM_LINES);
    draw_players(&mut sheet, colors, players);
    Ok(sheet.finish())
}

/// Four players and four missiles: colours of 0-3, 4 x 240 player lines,
/// then 240 missile lines. The missiles sit right of the players.
pub(super) fn decode_4pm(data: &[u8]) -> Result<Image, DecodeError> {
    let (colors, rest) = data.split_at_checked(4).ok_or(DecodeError::Unrecognized)?;
    let (players, missiles) = rest
        .split_at_checked(4 * PM_LINES)
        .ok_or(DecodeError::Unrecognized)?;
    if missiles.len() != PM_LINES {
        return Err(DecodeError::Unrecognized);
    }
    let mut sheet = Sheet::new(4 * CELL, PM_LINES);
    draw_players(&mut sheet, colors, players);
    let players = sheet.finish();
    let mut image = Image::new(players.width() + 32, PM_LINES as u32);
    for y in 0..PM_LINES as u32 {
        for x in 0..players.width() {
            image.set(x, y, players.get(x, y));
        }
        draw_missiles(&mut image, players.width(), y, missiles[y as usize], colors);
    }
    Ok(image)
}

/// Atari Player Editor animation: `9A F8 39 21`, frame count (1-16), height
/// (1-48), the X offset of player 1 from player 0 in player pixels (0-8),
/// then per player 16 frame colours plus a spare byte (the first player's at
/// 7, the second's at 24), a spare byte, and per player 17 frame slots of
/// 48 lines (from 42 and 858). Exactly 1677 bytes, the last 3 unused.
pub(super) fn decode_apl(data: &[u8]) -> Result<Image, DecodeError> {
    const COLORS: usize = 7;
    const COLOR_STRIDE: usize = 17;
    const LINES: usize = 42;
    const SLOT: usize = 48;
    const PLAYER_STRIDE: usize = 17 * SLOT;
    let [
        0x9a,
        0xf8,
        0x39,
        0x21,
        frames @ 1..=16,
        height @ 1..=48,
        offset @ 0..=8,
        ..,
    ] = *data
    else {
        return Err(DecodeError::Unrecognized);
    };
    if data.len() != 1677 {
        return Err(DecodeError::Unrecognized);
    }
    let (frames, height, offset) = (
        usize::from(frames),
        usize::from(height),
        usize::from(offset),
    );
    let cell = CELL + offset;
    let mut sheet = Sheet::new(frames * cell, height);
    for frame in 0..frames {
        for player in 0..2 {
            let color = data[COLORS + player * COLOR_STRIDE + frame];
            let slot = LINES + player * PLAYER_STRIDE + frame * SLOT;
            for (y, &bits) in data[slot..slot + height].iter().enumerate() {
                sheet.draw_player(frame * cell + player * offset, y, bits, color);
            }
        }
    }
    Ok(sheet.finish())
}

/// PMG Designer: `F0 ED E4`, the colours of players 0-3, three counts
/// (players per frame 2 or 4, then two factors of the frame count, at most
/// 160 frames), the height (1-48), then `players x frames` blocks of `height`
/// lines, player-major. Players 0 and 1 form one sprite, 2 and 3 a second;
/// the sprites of a sheet go 16 to a row.
pub(super) fn decode_pmd(data: &[u8]) -> Result<Image, DecodeError> {
    let [
        0xf0,
        0xed,
        0xe4,
        c0,
        c1,
        c2,
        c3,
        players @ (2 | 4),
        a,
        b,
        height @ 1..=48,
        ref blocks @ ..,
    ] = *data
    else {
        return Err(DecodeError::Unrecognized);
    };
    let colors = [c0, c1, c2, c3];
    let frames = usize::from(a) * usize::from(b);
    let (players, height) = (usize::from(players), usize::from(height));
    if frames == 0 || frames > 160 || blocks.len() != players * frames * height {
        return Err(DecodeError::Unrecognized);
    }
    let sprites = players / 2 * frames;
    let per_row = sprites.min(16);
    let rows = sprites.div_ceil(16);
    let row_height = height + 2;
    let mut sheet = Sheet::new(per_row * CELL, rows * row_height - 2);
    for player in 0..players {
        for frame in 0..frames {
            let sprite = player / 2 * frames + frame;
            let (x, y) = (sprite % 16 * CELL, sprite / 16 * row_height);
            let block = &blocks[(player * frames + frame) * height..][..height];
            for (line, &bits) in block.iter().enumerate() {
                sheet.draw_player(x, y + line, bits, colors[player]);
            }
        }
    }
    Ok(sheet.finish())
}

/// Ludek Maker animation: the text "Ludek Maker data file" in inverse
/// ATASCII, the colours of players 0/2 and 1/3, a number of unshown trailing
/// frames, the frame count (1-100, the shown ones are the first `count - skip`),
/// a 256-byte script that RECOIL ignores, then at least `count` frames of
/// 120 bytes: players 0-3, 30 lines each. Players 0/1 are the left 8 pixels
/// of a frame, 2/3 the right 8; frames go 8 to a row.
pub(super) fn decode_ldm(data: &[u8]) -> Result<Image, DecodeError> {
    const TEXT: [u8; 21] = [
        0xcc, 0xf5, 0xe4, 0xe5, 0xeb, 0xa0, 0xcd, 0xe1, 0xeb, 0xe5, 0xf2, 0xa0, 0xe4, 0xe1, 0xf4,
        0xe1, 0xa0, 0xe6, 0xe9, 0xec, 0xe5,
    ];
    const LINES: usize = 30;
    const CELL: usize = 20;
    let Some(([a, b, skip, count], frames)) = data
        .strip_prefix(&TEXT)
        .and_then(|rest| rest.split_first_chunk::<4>())
    else {
        return Err(DecodeError::Unrecognized);
    };
    let frames = frames
        .get(256..)
        .filter(|frames| frames.len() >= usize::from(*count) * 4 * LINES)
        .ok_or(DecodeError::Unrecognized)?;
    let (count, skip) = (usize::from(*count), usize::from(*skip));
    if !(1..=100).contains(&count) || skip >= count {
        return Err(DecodeError::Unrecognized);
    }
    let shown = count - skip;
    let per_row = shown.min(8);
    let rows = shown.div_ceil(8);
    let mut sheet = Sheet::new(per_row * CELL, rows * (LINES + 2) - 2);
    for frame in 0..shown {
        let (x, y) = (frame % 8 * CELL, frame / 8 * (LINES + 2));
        for player in 0..4 {
            let color = if player % 2 == 0 { *a } else { *b };
            let block = &frames[(frame * 4 + player) * LINES..][..LINES];
            for (line, &bits) in block.iter().enumerate() {
                sheet.draw_player(x + player / 2 * 8, y + line, bits, color);
            }
        }
    }
    Ok(sheet.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missile_pairs_take_low_bits_first() {
        let mut data = [0u8; 244];
        data[..4].copy_from_slice(&[0x0f; 4]);
        data[4] = 0b0100_0010; // missile 0 left pixel, missile 3 right pixel
        let image = decode_4mi(&data).unwrap();
        assert_eq!((image.width(), image.height()), (32, 240));
        assert_ne!(image.get(0, 0), 0);
        assert_eq!(image.get(2, 0), 0);
        assert_ne!(image.get(26, 0), 0);
    }

    #[test]
    fn pmd_sheet_wraps_after_16_sprites() {
        let mut data = alloc::vec![0xf0, 0xed, 0xe4, 0x34, 0x84, 0xb4, 0xea, 2, 17, 1, 3];
        data.extend_from_slice(&[0; 2 * 17 * 3]);
        let image = decode_pmd(&data).unwrap();
        assert_eq!((image.width(), image.height()), (320, 3 + 2 + 3));
        data.pop();
        assert!(decode_pmd(&data).is_err());
    }

    #[test]
    fn ludek_maker_shows_count_minus_skip_frames() {
        let mut data = vec![0u8; 21 + 4 + 256 + 3 * 120];
        data[..21].copy_from_slice(&[
            0xcc, 0xf5, 0xe4, 0xe5, 0xeb, 0xa0, 0xcd, 0xe1, 0xeb, 0xe5, 0xf2, 0xa0, 0xe4, 0xe1,
            0xf4, 0xe1, 0xa0, 0xe6, 0xe9, 0xec, 0xe5,
        ]);
        data[21..25].copy_from_slice(&[0x0a, 0x0e, 1, 3]);
        let image = decode_ldm(&data).unwrap();
        assert_eq!((image.width(), image.height()), (2 * 20 * 2, 30));
        data[24] = 4; // needs a fourth frame
        assert!(decode_ldm(&data).is_err());
    }

    #[test]
    fn overlapping_players_or_their_colours() {
        let mut sheet = Sheet::new(8, 1);
        sheet.draw_player(0, 0, 0x80, 0x14);
        sheet.draw_player(0, 0, 0x80, 0x28);
        assert_eq!(sheet.finish().get(0, 0), register_rgb(0x3c));
    }
}
