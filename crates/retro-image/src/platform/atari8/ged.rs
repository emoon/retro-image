//! GED pictures: 160x200 in four colors with players and missiles, driven
//! by display list interrupts that rewrite GTIA registers on every line.
//!
//! Sources:
//! - Just Solve "GED" (<http://fileformats.archiveteam.org/wiki/GED>):
//!   exactly 11302 bytes, starts `FF FF`, 160x200 with per-line colors.
//! - GTIA registers and player/missile graphics: De Re Atari ch. 4
//!   (<https://www.atariarchives.org/dere/chapt04.php>), Mapping the Atari
//!   App. 15 (<https://www.atariarchives.org/mapping/appendix15.php>); the
//!   priority rules are in [`gtia`], which also draws the objects.
//! - Everything else was reverse engineered by black-box probing of
//!   `recoil2png` with hand-made files (one byte or table changed at a
//!   time) and checked on the corpus samples. The file is the binary-load
//!   header `FF FF 30 53 4F 7F`, then:
//!   - two 200-byte tables at offsets 6 and 206 holding, for every line, the
//!     value and the number of a GTIA register (`$D000` + the low 5 bits;
//!     `$1E` and the registers above `$1B` do nothing) which the line's DLI
//!     writes before the line is drawn, e.g. a color or a player position;
//!     the register stays changed (the graphics registers only for the line);
//!   - eight 200-byte tables at 406, each line's color for playfield 0
//!     (tables 0, 3, 6), playfield 1 (1, 4, 7) and playfield 2 (2, 5): the
//!     first of each applies from the left edge, the others take over at
//!     pixels that depend on the DLI's timing byte below, see [`SWITCH_AT`];
//!   - 1280 bytes of player/missile data at 2006: the missiles, then
//!     players 0-3, 256 bytes each, line `y` at byte `y + 28`;
//!   - 16 initial registers at 3286: COLPM0-3, SIZEP (player 0 in the high
//!     bits, unlike the hardware), SIZEM, PRIOR, COLPF3, COLBK, HPOSP0-3,
//!     the HPOS of missile 0 (each other missile starts where the one before ends), the
//!     timing byte 0-7, and a byte that does nothing. Positions here are
//!     relative to the left edge of the picture, in color clocks; the
//!     writes use the hardware's (`$30` is the left edge);
//!   - the 200 lines of 40 bytes of Graphics 15 at 3302.
//!   - Pixel value 0 shows COLBK, 1-3 playfield 0-2.
//!
//! A PRIOR value with bits 6-7 (GTIA modes 9-11, which RECOIL draws
//! differently) is rejected, as no sample uses one.

use super::antic::Bitmap;
use super::gtia::{self, Colors, Pmg};
use super::palette::register_rgb;
use crate::{DecodeError, Image};

const HEADER: [u8; 6] = [0xff, 0xff, 0x30, 0x53, 0x4f, 0x7f];
const LEN: usize = 11302;
const WRITES: usize = 6;
const REGISTER_NUMBERS: usize = 206;
const TABLES: usize = 406;
const OBJECTS: usize = 2006;
const REGISTERS: usize = 3286;
const SCREEN: usize = 3302;
const LINES: usize = 200;
/// Byte of an object's data shown on the first line.
const FIRST_LINE: usize = 28;
/// The hardware position of the left edge.
const LEFT_EDGE: u8 = 0x30;
const GTIA_MODE: u8 = 0xc0;

/// The GTIA registers a DLI can change, as far as they show.
struct Gtia {
    hpos_player: [u8; 4],
    hpos_missile: [u8; 4],
    /// Two bits per player, player 0 lowest (the hardware's SIZEP0-3).
    size_player: u8,
    size_missile: u8,
    colors: Colors,
    prior: u8,
}

pub(super) fn decode_ged(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != LEN || !data.starts_with(&HEADER) || data[REGISTERS + 14] > 7 {
        return Err(DecodeError::Invalid);
    }
    // PRIOR bits 6-7 select GTIA modes 9-11, which are not drawn.
    if data[REGISTERS + 6] & GTIA_MODE != 0 {
        return Err(DecodeError::Invalid);
    }
    let regs = &data[REGISTERS..REGISTERS + 16];
    let hardware = |position: u8| position.wrapping_add(LEFT_EDGE);
    let mut gtia = Gtia {
        hpos_player: [regs[9], regs[10], regs[11], regs[12]].map(hardware),
        hpos_missile: missile_positions(hardware(regs[13]), regs[5]),
        size_player: (0..4).fold(0, |sizes, k| {
            sizes | (regs[4] >> (6 - 2 * k) & 3) << (2 * k)
        }),
        size_missile: regs[5],
        colors: Colors {
            player: [regs[0], regs[1], regs[2], regs[3]],
            playfield: [0, 0, 0, regs[7]],
            background: regs[8],
        },
        prior: regs[6],
    };
    let timing = usize::from(regs[14]);
    let bitmap = Bitmap {
        data: &data[SCREEN..SCREEN + 40 * LINES],
        bytes_per_line: 40,
        lines: LINES,
        bits: 2,
    };
    let mut image = Image::new(320, LINES as u32)?;
    for y in 0..LINES {
        let mut graphics = [None; 5];
        gtia.write(
            data[REGISTER_NUMBERS + y] & 0x1f,
            data[WRITES + y],
            &mut graphics,
        );
        if gtia.prior & GTIA_MODE != 0 {
            return Err(DecodeError::Invalid);
        }
        let object = |index: usize, override_: Option<u8>| {
            override_.unwrap_or(data[OBJECTS + 256 * index + FIRST_LINE + y])
        };
        let pmg = Pmg {
            hpos_player: gtia.hpos_player,
            hpos_missile: gtia.hpos_missile,
            size_player: gtia.size_player,
            size_missile: gtia.size_missile,
            graf_player: [1, 2, 3, 4].map(|k| object(k, graphics[k - 1])),
            graf_missile: object(0, graphics[4]),
        };
        let objects = pmg.draw();
        for x in 0..320 {
            let mut colors = gtia.colors;
            for register in 0..3 {
                let table = playfield_table(register, x, timing);
                colors.playfield[register] = data[TABLES + 200 * table + y];
            }
            let pixel = bitmap.pixel(x / 2, y);
            let (players, playfield) = gtia::add_objects(
                gtia.prior,
                objects.pixels[x + 8],
                0,
                gtia::playfield_bit(usize::from(pixel)),
            );
            let color = gtia::resolve(gtia.prior, players, playfield, &colors);
            image.set(x as u32, y as u32, register_rgb(color));
        }
    }
    Ok(image)
}

/// Output pixels where tables 3 to 7 take over, for each value of the timing
/// byte (measured with `recoil2png`: the DLI's writes happen later the
/// larger the byte, but the second write of playfield 1 is held back by the
/// one after it).
const SWITCH_AT: [[usize; 8]; 5] = [
    [30, 46, 62, 78, 94, 110, 126, 142],
    [94, 110, 126, 142, 150, 158, 166, 174],
    [150, 158, 166, 174, 182, 190, 198, 206],
    [198, 206, 214, 222, 230, 238, 246, 254],
    [246, 254, 262, 270, 278, 286, 294, 302],
];

/// Where the four missiles start: missile 0 at `first` and each next one
/// right after the one before, whose width (two bits of 1, 2, 1 or 4 color
/// clocks) its size sets. Writes to the position registers move them
/// individually later.
fn missile_positions(first: u8, sizes: u8) -> [u8; 4] {
    let mut positions = [first; 4];
    for k in 1..4 {
        let width = 2 * [1u8, 2, 1, 4][usize::from(sizes >> (2 * (k - 1)) & 3)];
        positions[k] = positions[k - 1].wrapping_add(width);
    }
    positions
}

/// The table giving playfield color `register` (0-2) at output pixel `x`:
/// table `register` from the left edge, then table `register + 3` and
/// (for playfield 0 and 1) table 6 or 7, or for playfield 2 only table 5.
fn playfield_table(register: usize, x: usize, timing: usize) -> usize {
    let switched = |table: usize| x >= SWITCH_AT[table - 3][timing];
    match register {
        0 if switched(6) => 6,
        0 if switched(3) => 3,
        1 if switched(7) => 7,
        1 if switched(4) => 4,
        2 if switched(5) => 5,
        register => register,
    }
}

impl Gtia {
    /// Writes `value` to register `$D000 + register`. Writes to the
    /// graphics registers set `graphics` (players 0-3 then the missiles)
    /// for this line only.
    fn write(&mut self, register: u8, value: u8, graphics: &mut [Option<u8>; 5]) {
        let index = usize::from(register);
        match register {
            0..=3 => self.hpos_player[index] = value,
            4..=7 => self.hpos_missile[index - 4] = value,
            8..=11 => {
                let shift = 2 * (index - 8);
                self.size_player = self.size_player & !(3 << shift) | (value & 3) << shift;
            }
            12 => self.size_missile = value,
            13..=17 => graphics[index - 13] = Some(value),
            18..=21 => self.colors.player[index - 18] = value,
            25 => self.colors.playfield[3] = value,
            26 => self.colors.background = value,
            27 => self.prior = value,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    fn blank() -> Vec<u8> {
        let mut data = alloc::vec![0u8; LEN];
        data[..6].copy_from_slice(&HEADER);
        // Register 30 (HITCLR) does nothing; 0 would move player 0 away.
        data[REGISTER_NUMBERS..TABLES].fill(30);
        data
    }

    #[test]
    fn colour_tables_switch_mid_line() {
        let mut data = blank();
        // Playfield 0 pixels on line 0: table 0 until pixel 30, table 3 after.
        data[SCREEN..SCREEN + 40].fill(0x55);
        data[TABLES] = 0x20;
        data[TABLES + 3 * 200] = 0x40;
        let image = decode_ged(&data).unwrap();
        assert_eq!(image.get(0, 0), register_rgb(0x20));
        assert_eq!(image.get(30, 0), register_rgb(0x40));
        // The timing byte delays the switch by 16 pixels per step.
        data[REGISTERS + 14] = 2;
        let image = decode_ged(&data).unwrap();
        assert_eq!(image.get(61, 0), register_rgb(0x20));
        assert_eq!(image.get(62, 0), register_rgb(0x40));
    }

    #[test]
    fn line_writes_move_players() {
        let mut data = blank();
        // Player 0 on lines 0 and 1, color 0x0e; line 1 writes HPOSP0 = 0x40,
        // 32 pixels from the left edge.
        data[OBJECTS + 256 + FIRST_LINE] = 0xff;
        data[OBJECTS + 256 + FIRST_LINE + 1] = 0xff;
        data[REGISTERS] = 0x0e;
        data[REGISTER_NUMBERS + 1] = 0;
        data[WRITES + 1] = 0x40;
        let image = decode_ged(&data).unwrap();
        assert_eq!(image.get(0, 0), register_rgb(0x0e));
        assert_eq!(image.get(0, 1), register_rgb(0x00));
        assert_eq!(image.get(32, 1), register_rgb(0x0e));
    }

    #[test]
    fn rejects_gtia_modes_and_bad_timing() {
        let mut data = blank();
        data[REGISTERS + 6] = 0x40;
        assert!(decode_ged(&data).is_err());
        data[REGISTERS + 6] = 0x14;
        data[REGISTERS + 14] = 8;
        assert!(decode_ged(&data).is_err());
        data[REGISTERS + 14] = 7;
        assert!(decode_ged(&data).is_ok());
    }
}
