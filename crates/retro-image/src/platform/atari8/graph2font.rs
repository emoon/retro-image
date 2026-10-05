//! Graph2Font pictures (MCH, G2F): the picture model and its renderer.
//!
//! Graph2Font builds pictures from character rows (ANTIC 2 or 4, or GTIA
//! modes 9-11 on ANTIC 2 data), per-scanline color and PRIOR changes, and
//! players/missiles. The file layouts are in [`mch`] and [`g2f`].
//!
//! Sources:
//! - G2F manual (<https://g2f.atari8.info/instrukcja_eng.html>) and Just
//!   Solve "Graph2Font" (<http://fileformats.archiveteam.org/wiki/Graph2Font>)
//!   for the program's model; De Re Atari ch. 2-4
//!   (<https://www.atariarchives.org/dere/chapt02.php>,
//!   <https://www.atariarchives.org/dere/chapt03.php>,
//!   <https://www.atariarchives.org/dere/chapt04.php>) for the hardware.
//! - Rendering reverse engineered by black-box probing of `recoil2png` with
//!   hand-made MCH and G2F files, and confirmed on random synthetic MCH
//!   files, which render identically: the picture is 336x240; 40-column
//!   screens sit between 8-pixel borders, 48-column screens show columns
//!   3-44. A code byte's bit 7 inverts its cell (ANTIC 2 and GTIA: the
//!   bits; ANTIC 4: `11` pixels use COLPF3); in split mode bit 7 only
//!   covers the top 4 scanlines and bit 6 the bottom 4. A scanline's GTIA
//!   mode is its row's ORed with PRIOR bits 6-7. On GTIA mode 10 rows the 9
//!   color tables are COLPM0-3, COLPF0-3, COLBK in that order instead of
//!   COLBK, COLPF0-3, COLPM0-3. Where no player (or missile counted as one)
//!   is, GTIA mode 9 ORs the pixel into the winning color (COLBK, or COLPF3
//!   for fifth-player missiles) and mode 11 ORs `pixel << 4` (pixel 0
//!   clears the luminance instead). Mode 10 picks registers through the
//!   priority logic as the hardware does (0-3 players, 4-7 playfield, 8-11
//!   COLBK, 12-15 playfield) and is 2 pixels to the right; pixels outside
//!   the screen data are 0. In ANTIC 2 set pixels take COLPF1's luminance
//!   over whatever color wins. Luminance bit 0 only shows in mode 9 pixels.

mod g2f;
mod mch;
mod vsc;

pub(super) use g2f::{decode_g2f, decode_g2f_vbxe};
pub(super) use mch::decode_mch;
pub(super) use vsc::decode_vsc;

use super::gtia::{self, Colors, Pmg, WIDTH};
use super::palette::rgb;
use crate::Image;
use alloc::vec::Vec;

const LINES: usize = 240;
const ROWS: usize = 30;

/// A Graph2Font picture, independent of the file it came from.
struct Picture<'a> {
    columns: usize,
    rows: [Row; ROWS],
    /// Code byte of each cell, row by row: bit 7 inverse (top half in split
    /// mode), bit 6 inverse of the bottom half in split mode.
    codes: Vec<u8>,
    /// The 8 bytes of each cell.
    glyphs: Vec<&'a [u8]>,
    split: bool,
    /// Whether inverse ANTIC 4 cells show `11` pixels in COLPF3.
    antic4_inverse: bool,
    /// VBXE color attributes, which replace COLPF0-2 (see [`g2f`]).
    vbxe: Option<Vbxe<'a>>,
    /// Per-scanline registers.
    lines: Vec<Line>,
}

/// VBXE color attributes: 12-byte records, 240 for each of 48 screen
/// columns (a 40-column screen uses the middle 40), the record of a scanline
/// being its number divided by `height`. A record's bytes 4, 6 and 8 are the
/// colors standing for COLPF0-2. The first two bytes of the array are 8 and
/// `height`.
struct Vbxe<'a> {
    records: &'a [u8],
    /// Screen columns before the first of the picture.
    first_column: usize,
    /// Scanlines per record.
    height: usize,
}

const VBXE_RECORD: usize = 12;
const VBXE_COLUMN: usize = LINES * VBXE_RECORD;

impl Vbxe<'_> {
    fn colors(&self, column: usize, y: usize) -> [u8; 3] {
        let record = (self.first_column + column) * VBXE_COLUMN + y / self.height * VBXE_RECORD;
        [4, 6, 8].map(|field| self.records[record + field])
    }
}

/// How a character row is displayed.
#[derive(Clone, Copy)]
struct Row {
    /// ANTIC 4 (multicolor) instead of ANTIC 2 data.
    antic4: bool,
    /// GTIA mode: 0 none, 1-3 modes 9-11.
    gtia: u8,
    /// No playfield, only background and players.
    blank: bool,
}

struct Line {
    /// The 9 color tables in file order.
    colors: [u8; 9],
    prior: u8,
    pmg: Pmg,
}

impl Picture<'_> {
    fn render(&self) -> Image {
        let mut image = Image::new(WIDTH as u32, LINES as u32);
        for (y, line) in self.lines.iter().enumerate() {
            let row = self.rows[y / 8];
            let t = line.colors;
            let colors = if row.gtia == 2 {
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
            let mode = row.gtia | line.prior >> 6;
            let objects = line.pmg.draw();
            for x in 0..WIDTH {
                let pixel = self.playfield(x, y, row, mode, &colors);
                // A VBXE cell supplies its own playfield colors 0-2, which
                // keep their luminance bit 0 where the registers lose it.
                let colors = pixel.vbxe.map_or(colors, |[pf0, pf1, pf2]| Colors {
                    player: colors.player.map(|color| color & 0xfe),
                    playfield: [pf0, pf1, pf2, colors.playfield[3] & 0xfe],
                    background: colors.background & 0xfe,
                });
                let (players, playfield) = gtia::add_objects(
                    line.prior,
                    objects.pixels[x],
                    pixel.players,
                    pixel.playfield,
                );
                let mut color = gtia::resolve(line.prior, players, playfield, &colors);
                if let Some(luminance) = pixel.luminance {
                    color = color & 0xf0 | luminance;
                }
                if let Some(ink) = pixel.ink {
                    color = ink;
                }
                // Luminance bit 0 only shows in GTIA mode 9 pixels, which
                // like mode 11 pixels apply only where no player is.
                color = match (mode, pixel.nibble) {
                    _ if pixel.vbxe.is_some() => color,
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
    fn playfield(&self, x: usize, y: usize, row: Row, mode: u8, colors: &Colors) -> Pixel {
        // Pixel position from the left edge of the screen data; GTIA mode
        // 10 is delayed by 2 pixels.
        let left = 8 * (self.columns as isize / 2 - 21);
        let position = x as isize + left - if mode == 2 { 2 } else { 0 };
        let byte = if row.blank {
            None
        } else {
            self.byte(position, y, row.antic4)
        };
        let bit = position.rem_euclid(8) as u32;
        // GTIA modes take 4-bit pixels, 0 outside the screen data.
        let nibble = byte.map_or(0, |(byte, _)| (byte >> (4 - bit / 4 * 4)) & 0x0f);
        // VBXE colors replace the playfield colors of ANTIC modes 2 and 4
        // and GTIA mode 10; modes 9 and 11 don't show them.
        let vbxe = if matches!(mode, 0 | 2) {
            self.vbxe_colors(position, y)
        } else {
            None
        };
        let empty = Pixel {
            players: 0,
            playfield: 0,
            luminance: None,
            nibble,
            vbxe,
            ink: None,
        };
        match (mode, byte) {
            (0, None) | (1 | 3, _) => empty,
            (0, Some((byte, inverse))) if row.antic4 => {
                let value = (byte >> (6 - bit / 2 * 2)) & 3;
                let playfield = gtia::playfield_bit(gtia::antic4_register(
                    value,
                    inverse && self.antic4_inverse,
                ));
                Pixel { playfield, ..empty }
            }
            (0, Some((byte, _))) => {
                let set = byte >> (7 - bit) & 1 != 0;
                if let Some([_, pf1, _]) = vbxe {
                    // Set pixels show COLPF1's VBXE color whatever is
                    // above them; the rest is COLPF2 for the priorities.
                    Pixel {
                        playfield: 4,
                        ink: set.then_some(pf1),
                        ..empty
                    }
                } else {
                    Pixel {
                        playfield: 4,
                        luminance: set.then_some(colors.playfield[1] & 0x0f),
                        ..empty
                    }
                }
            }
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

    /// The VBXE colors of the cell at pixel `position` of scanline `y`.
    fn vbxe_colors(&self, position: isize, y: usize) -> Option<[u8; 3]> {
        let column = usize::try_from(position.div_euclid(8)).ok()?;
        if column >= self.columns {
            return None;
        }
        Some(self.vbxe.as_ref()?.colors(column, y))
    }

    /// The byte at pixel `position` of the screen data on scanline `y`
    /// (inverted where ANTIC 2 data is inverted), and whether its cell half
    /// is inverse; `None` outside the screen data.
    fn byte(&self, position: isize, y: usize, antic4: bool) -> Option<(u8, bool)> {
        let column = usize::try_from(position.div_euclid(8)).ok()?;
        if column >= self.columns {
            return None;
        }
        let cell = (y / 8) * self.columns + column;
        let code = self.codes[cell];
        let inverse = if self.split && y % 8 >= 4 {
            code & 0x40 != 0
        } else {
            code & 0x80 != 0
        };
        let byte = self.glyphs[cell][y % 8];
        let byte = if inverse && !antic4 { !byte } else { byte };
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
    /// The VBXE colors of the pixel's cell, if it has any.
    vbxe: Option<[u8; 3]>,
    /// An ANTIC 2 set pixel's VBXE color, which replaces the result.
    ink: Option<u8>,
}
