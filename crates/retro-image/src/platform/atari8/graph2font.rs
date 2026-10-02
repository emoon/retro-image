//! Graph2Font pictures: MCH.
//!
//! Graph2Font builds pictures from character modes (ANTIC 2 or 4, optionally
//! with GTIA modes 9-11), per-scanline colour and PRIOR changes, and
//! players/missiles. Its documentation (the G2F manual) describes the
//! exported pieces, not its own files.
//!
//! Sources:
//! - G2F manual (<https://g2f.atari8.info/instrukcja_eng.html>) for the
//!   program's model; De Re Atari and Mapping the Atari for the hardware.
//! - MCH layout reverse engineered from samples and by black-box probing of
//!   `recoil2png` with hand-made files (offsets for 40 columns; 48-column
//!   files have 1440 cells, so everything after them moves by 2160):
//!
//!   ```text
//!   0      1200 cells of 9 bytes: a code byte, then the 8 bytes shown
//!          on the cell's 8 scanlines. The first code byte's low 6 bits
//!          are the mode: 01 ANTIC 2, 05 ANTIC 4, 09/19/29 ANTIC 2 with
//!          GTIA mode 9/10/11.
//!   10800  20 tables of 240 per-scanline values: COLBK, COLPF0-3,
//!          COLPM0-3, HPOSP0-3, HPOSM0-3, SIZEP0-3 (two bits each,
//!          player 0 lowest), SIZEM, PRIOR
//!   15600  GRAFM for each scanline
//!   15840  player 0-3 memory, 256 bytes each, scanline y at byte 16 + y
//!   16864  ignored by RECOIL (13856 bytes, then a 113-byte tail)
//!   ```
//!
//! - Observed from `recoil2png` output: the picture is 336x240; 40-column
//!   screens sit between 8-pixel borders, 48-column screens show columns
//!   3-44. A code byte's bit 7 inverts the cell (ANTIC 2: the bits; ANTIC
//!   4: `11` pixels use COLPF3); if any code byte has bit 6 set, bit 7 only
//!   applies to the top 4 scanlines and bit 6 to the bottom 4. A
//!   scanline's GTIA mode is the file's ORed with PRIOR bits 6-7. With
//!   GTIA mode 10 in the header, the 9 colour tables are COLPM0-3,
//!   COLPF0-3, COLBK in that order instead. Where no player (or missile
//!   counted as one) is, GTIA mode 9 ORs the pixel into the winning colour
//!   (COLBK, or COLPF3 for fifth-player missiles) and mode 11 ORs
//!   `pixel << 4` (pixel 0 clears the luminance instead). Mode 10 picks
//!   registers through the priority logic as the hardware does (0-3
//!   players, 4-7 playfield, 8-11 COLBK, 12-15 playfield) and is 2 pixels
//!   to the right; pixels outside the screen data are 0. In ANTIC 2 set
//!   pixels take COLPF1's luminance over whatever colour wins. Luminance
//!   bit 0 only shows in mode 9 pixels. All of this was confirmed on
//!   random synthetic files, which render identically to `recoil2png`.

use super::gtia::{self, Colors, Pmg, WIDTH};
use super::palette::rgb;
use crate::{DecodeError, Image};
use alloc::vec::Vec;

const LINES: usize = 240;
const ROWS: usize = 30;

/// A Graph2Font picture, independent of the file it came from.
struct Picture<'a> {
    columns: usize,
    /// ANTIC 4 (multicolour) characters instead of ANTIC 2.
    antic4: bool,
    /// GTIA mode from the header: 0 none, 1-3 modes 9-11.
    gtia: u8,
    /// Code byte of each cell, row by row.
    codes: Vec<u8>,
    /// The 8 bytes of each cell.
    glyphs: Vec<&'a [u8]>,
    /// Per-scanline registers.
    lines: Vec<Line>,
}

struct Line {
    /// The 9 colour tables in file order.
    colors: [u8; 9],
    prior: u8,
    pmg: Pmg,
}

impl Picture<'_> {
    fn render(&self) -> Image {
        let mut image = Image::new(WIDTH as u32, LINES as u32);
        let split = self.codes.iter().any(|code| code & 0x40 != 0);
        let hardware_order = self.gtia == 2;
        for (y, line) in self.lines.iter().enumerate() {
            let t = line.colors;
            let colors = if hardware_order {
                Colors {
                    player: [t[0], t[1], t[2], t[3]],
                    playfield: [t[4], t[5], t[6], t[7]],
                    background: t[8],
                }
            } else {
                Colors {
                    player: [t[5], t[6], t[7], t[8]],
                    playfield: [t[1], t[2], t[3], t[4]],
                    background: t[0],
                }
            };
            let mode = self.gtia | line.prior >> 6;
            let objects = line.pmg.draw();
            let fifth = line.prior & 0x10 != 0;
            for x in 0..WIDTH {
                let pixel = self.playfield(x, y, mode, split, &colors);
                let objs = objects.pixels[x];
                let mut players = pixel.players | objs & 0x0f;
                let mut playfield = pixel.playfield;
                if fifth {
                    if objs & 0xf0 != 0 {
                        playfield |= 8;
                    }
                } else {
                    players |= objs >> 4;
                }
                let mut color = gtia::resolve(line.prior, players, playfield, &colors);
                if let Some(luminance) = pixel.luminance {
                    color = color & 0xf0 | luminance;
                }
                // Luminance bit 0 only shows in GTIA mode 9 pixels, which
                // like mode 11 pixels apply only where no player is.
                color = match (mode, pixel.nibble) {
                    (1, value) if players == 0 => color & 0xfe | value,
                    (3, 0) if players == 0 => color & 0xf0,
                    (3, value) if players == 0 => (color | value << 4) & 0xfe,
                    _ => color & 0xfe,
                };
                image.set(x as u32, y as u32, rgb(color));
            }
        }
        image
    }

    /// The playfield at output pixel `x` of scanline `y`.
    fn playfield(&self, x: usize, y: usize, mode: u8, split: bool, colors: &Colors) -> Pixel {
        // Pixel position from the left edge of the screen data; GTIA mode
        // 10 is delayed by 2 pixels.
        let left = 8 * (self.columns as isize / 2 - 21);
        let position = x as isize + left - if mode == 2 { 2 } else { 0 };
        let byte = self.byte(position, y, split);
        let bit = position.rem_euclid(8) as u32;
        // GTIA modes take 4-bit pixels, 0 outside the screen data.
        let nibble = byte.map_or(0, |(byte, _)| (byte >> (4 - bit / 4 * 4)) & 0x0f);
        let empty = Pixel {
            players: 0,
            playfield: 0,
            luminance: None,
            nibble,
        };
        match (mode, byte) {
            (0, None) | (1 | 3, _) => empty,
            (0, Some((byte, inverse))) if self.antic4 => {
                let value = (byte >> (6 - bit / 2 * 2)) & 3;
                let playfield = match value {
                    0 => 0,
                    3 if inverse => 8,
                    _ => 1 << (value - 1),
                };
                Pixel { playfield, ..empty }
            }
            (0, Some((byte, _))) => Pixel {
                playfield: 4,
                luminance: (byte >> (7 - bit) & 1 != 0).then_some(colors.playfield[1] & 0x0f),
                ..empty
            },
            // GTIA mode 10 selects registers through the priority logic.
            _ => match nibble {
                0..=3 => Pixel {
                    players: 1 << nibble,
                    ..empty
                },
                4..=7 | 12..=15 => Pixel {
                    playfield: 1 << (nibble & 3),
                    ..empty
                },
                _ => empty,
            },
        }
    }

    /// The byte at pixel `position` of the screen data on scanline `y`
    /// (inverted where ANTIC 2 inverts it), and whether its cell half is
    /// inverse; `None` outside the screen data.
    fn byte(&self, position: isize, y: usize, split: bool) -> Option<(u8, bool)> {
        let column = usize::try_from(position.div_euclid(8)).ok()?;
        if column >= self.columns {
            return None;
        }
        let cell = (y / 8) * self.columns + column;
        let code = self.codes[cell];
        let inverse = if split && y % 8 >= 4 {
            code & 0x40 != 0
        } else {
            code & 0x80 != 0
        };
        let byte = self.glyphs[cell][y % 8];
        let byte = if inverse && !self.antic4 { !byte } else { byte };
        Some((byte, inverse))
    }
}

/// The playfield's contribution to one pixel.
struct Pixel {
    /// Player and playfield signals for the priority logic.
    players: u8,
    playfield: u8,
    /// ANTIC 2 set pixel: COLPF1's luminance.
    luminance: Option<u8>,
    /// The GTIA mode 9-11 pixel.
    nibble: u8,
}

/// Graph2Font MCH: cells with their character data inline, then
/// per-scanline registers and player/missile graphics.
pub(super) fn decode_mch(data: &[u8]) -> Result<Image, DecodeError> {
    let columns = match data.len() {
        30833 => 40,
        32993 => 48,
        _ => return Err(DecodeError::Unrecognized),
    };
    let (antic4, gtia) = match data[0] & 0x3f {
        0x01 => (false, 0),
        0x05 => (true, 0),
        0x09 => (false, 1),
        0x19 => (false, 2),
        0x29 => (false, 3),
        _ => return Err(DecodeError::Unrecognized),
    };
    let cells = &data[..9 * columns * ROWS];
    let tables = &data[cells.len()..];
    let table = |index: usize, y: usize| tables[index * LINES + y];
    let lines = (0..LINES)
        .map(|y| Line {
            colors: core::array::from_fn(|i| table(i, y)),
            prior: table(19, y),
            pmg: Pmg {
                hpos_player: core::array::from_fn(|k| table(9 + k, y)),
                hpos_missile: core::array::from_fn(|k| table(13 + k, y)),
                size_player: table(17, y),
                size_missile: table(18, y),
                graf_missile: table(20, y),
                graf_player: core::array::from_fn(|k| tables[21 * LINES + 256 * k + 16 + y]),
            },
        })
        .collect();
    let picture = Picture {
        columns,
        antic4,
        gtia,
        codes: cells.chunks_exact(9).map(|cell| cell[0]).collect(),
        glyphs: cells.chunks_exact(9).map(|cell| &cell[1..]).collect(),
        lines,
    };
    Ok(picture.render())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn mch(mode: u8) -> Vec<u8> {
        let mut data = vec![0; 30833];
        data[0] = mode;
        data
    }

    #[test]
    fn antic4_inverse_uses_colpf4() {
        let mut data = mch(0x05);
        data[9] = 0x80; // cell 1 inverse
        data[10] = 0xff; // cell 1, scanline 0: four `11` pixels
        data[1] = 0xff; // cell 0, scanline 0
        for (table, color) in [(3, 0x46), (4, 0x88)] {
            data[10800 + 240 * table] = color;
        }
        let image = decode_mch(&data).unwrap();
        assert_eq!(image.get(8, 0), rgb(0x46));
        assert_eq!(image.get(16, 0), rgb(0x88));
    }

    #[test]
    fn gtia9_ors_into_background() {
        let mut data = mch(0x09);
        data[1] = 0x5a;
        data[10800] = 0x20;
        let image = decode_mch(&data).unwrap();
        assert_eq!(image.get(0, 0), rgb(0x20));
        assert_eq!(image.get(8, 0), rgb(0x25));
        assert_eq!(image.get(12, 0), rgb(0x2a));
    }

    #[test]
    fn rejects_other_modes_and_sizes() {
        assert!(decode_mch(&mch(0x07)).is_err());
        assert!(decode_mch(&[1; 30832]).is_err());
    }
}
