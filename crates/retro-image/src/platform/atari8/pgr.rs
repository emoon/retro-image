//! PowerGraphics pictures (PGR): an Atari memory image with a display list,
//! a register image and a per-scanline stream of register writes.
//!
//! Sources:
//! - Just Solve, "Atari graphics formats"
//!   (<http://fileformats.archiveteam.org/wiki/Atari_graphics_formats>) lists
//!   the format without a layout. The hardware (ANTIC display lists, GTIA
//!   registers and modes) is as in De Re Atari ch. 2-4
//!   (<https://www.atariarchives.org/dere/chapt02.php>,
//!   <https://www.atariarchives.org/dere/chapt04.php>), and the rendering
//!   shares [`super::gtia`] and the Graph2Font renderer's pixel rules.
//! - The layout is reverse engineered from the corpus samples and checked
//!   against `recoil2png` output (black box), including acceptance probes
//!   with hand-made event streams; notes in
//!   `docs/research/gaps-corpus-atari8.md` section 6.
//!
//! ```text
//! file    FF FF, start (8206), end; length = end - start + 7
//! 8206    address of the event stream
//! 8208    "PowerGFX"
//! 8210    ANTIC display list
//! 83F8    HPOSP0-3, HPOSM0-3, SIZEP0-3, SIZEM, GRAFP0 (14 bytes)
//! 84F8    GRAFP1-3, GRAFM, COLPM0-3
//! 8500    COLPF0-3, COLBK, PRIOR, DMACTL
//! ```
//!
//! The register image sits in the unused rows of the player/missile memory
//! (PMBASE 8000). The display list gives one scanline per entry: blank
//! lines, and Graphics 7.5 (mode E, 2 bits) or 8 (mode F) lines whose
//! screen address comes from the last LMS and advances by 32, 40 or 48
//! bytes (DMACTL bits 0-1) within a 4K block. Scanlines after the list are
//! blank. Mode F lines draw COLPF2 with COLPF1's luminance on set pixels;
//! with PRIOR bit 6 they are Graphics 9. A 40-byte screen is centred in the
//! 336-pixel picture, a 32-byte one has wider borders, a 48-byte one is
//! cropped.
//!
//! The event stream has one entry list per scanline, 240 lines. Each event
//! byte `b` writes register `b & 0x1f` (see [`Registers`]; 0x1c is a no-op)
//! with the byte that follows if `b & 0x20` is set, else with the last value
//! byte seen. The event ends the scanline if `b & 0x80` is set; so does a
//! bare `1C` or `3C`. Writes apply from the start of their scanline and
//! persist. Events on registers 0x1d-0x1f (which move later writes into the
//! middle of the scanline by ANTIC DMA timing) and player/missile DMA are not
//! understood, and such files are rejected.

use super::gtia::{self, Colors, Pmg, WIDTH};
use super::palette::rgb;
use crate::{DecodeError, Image};
use alloc::vec::Vec;

const LINES: usize = 240;
const START: usize = 0x8206;
const SIGNATURE: &[u8; 8] = b"PowerGFX";
const DISPLAY_LIST: usize = 0x8210;

/// Memory address of the initial value of event register `register`. The 28
/// registers are stored in event order, split across player/missile memory
/// rows: 0-13 from 83F8, 14-21 from 84F8, 22-27 from 8500.
const fn initial_address(register: usize) -> usize {
    match register {
        0..=13 => 0x83f8 + register,
        14..=21 => 0x84f8 + register - 14,
        _ => 0x8500 + register - 22,
    }
}

const DMACTL: usize = 0x8506;

/// The registers written by events, in event order: HPOSP0-3, HPOSM0-3,
/// SIZEP0-3, SIZEM, GRAFP0-3, GRAFM, COLPM0-3, COLPF0-3, COLBK, PRIOR.
type Registers = [u8; 0x1c];

/// PGR: 336x240.
pub(super) fn decode_pgr(data: &[u8]) -> Result<Image, DecodeError> {
    let memory = Memory::parse(data).ok_or(DecodeError::Unrecognized)?;
    let dmactl = memory.byte(DMACTL);
    // Player and missile DMA are not implemented.
    if dmactl & 0x0c != 0 {
        return Err(DecodeError::Unrecognized);
    }
    let bytes_per_line = match dmactl & 3 {
        1 => 32,
        2 => 40,
        3 => 48,
        _ => return Err(DecodeError::Unrecognized),
    };
    let sources = display_list(&memory, bytes_per_line).ok_or(DecodeError::Unrecognized)?;
    let registers = scanline_registers(&memory).ok_or(DecodeError::Unrecognized)?;

    let mut image = Image::new(WIDTH as u32, LINES as u32);
    for (y, (source, registers)) in sources.iter().zip(&registers).enumerate() {
        render_line(&mut image, y, &memory, source, registers, bytes_per_line)?;
    }
    Ok(image)
}

/// The memory image, addressed as on the Atari.
struct Memory<'a> {
    image: &'a [u8],
}

impl<'a> Memory<'a> {
    fn parse(data: &'a [u8]) -> Option<Self> {
        let image = data.strip_prefix(&[0xff, 0xff])?.get(4..)?;
        let start = usize::from(crate::bytes::le16(data, 2)?);
        let end = usize::from(crate::bytes::le16(data, 4)?);
        if start != START || end < start || image.len() != end - start + 1 {
            return None;
        }
        let memory = Memory { image };
        (memory.image.get(2..10)? == SIGNATURE).then_some(memory)
    }

    /// The byte at `address`; 0 outside the file.
    fn byte(&self, address: usize) -> u8 {
        address
            .checked_sub(START)
            .and_then(|offset| self.image.get(offset))
            .copied()
            .unwrap_or(0)
    }

    fn word(&self, address: usize) -> usize {
        usize::from(self.byte(address)) | usize::from(self.byte(address + 1)) << 8
    }
}

/// What ANTIC shows on a scanline.
#[derive(Clone, Copy)]
enum Source {
    Blank,
    /// Graphics 7.5 (2 bits per pixel) from the address.
    Four(usize),
    /// Graphics 8 (1 bit per pixel) from the address.
    Hires(usize),
}

/// Runs the display list, one source per scanline.
fn display_list(memory: &Memory<'_>, bytes_per_line: usize) -> Option<Vec<Source>> {
    let mut lines = Vec::with_capacity(LINES);
    let mut address = 0;
    let mut pc = DISPLAY_LIST;
    while lines.len() < LINES {
        let instruction = memory.byte(pc);
        pc += 1;
        let mode = instruction & 0x0f;
        match mode {
            0 => {
                let blank = usize::from(instruction >> 4 & 7) + 1;
                lines.resize(lines.len() + blank, Source::Blank);
            }
            1 if instruction & 0x40 != 0 => break,
            0x0e | 0x0f => {
                if instruction & 0x40 != 0 {
                    address = memory.word(pc);
                    pc += 2;
                }
                lines.push(if mode == 0x0e {
                    Source::Four(address)
                } else {
                    Source::Hires(address)
                });
                // ANTIC's address counter wraps within 4K blocks.
                address = address & 0xf000 | (address + bytes_per_line) & 0x0fff;
            }
            _ => return None,
        }
    }
    lines.resize(LINES, Source::Blank);
    Some(lines)
}

/// The registers in effect on each of the 240 scanlines.
fn scanline_registers(memory: &Memory<'_>) -> Option<Vec<Registers>> {
    let mut registers: Registers = core::array::from_fn(|n| memory.byte(initial_address(n)));
    let mut latch = 0;
    let mut pos = memory.word(START);
    let mut lines = Vec::with_capacity(LINES);
    while lines.len() < LINES {
        let event = memory.byte(pos);
        let value = memory.byte(pos + 1);
        let register = usize::from(event & 0x1f);
        pos += 1;
        if event & 0x20 != 0 {
            latch = value;
            pos += 1;
        }
        let ends = match event {
            0x1c | 0x3c => true,
            _ if register >= registers.len() => return None,
            _ => event & 0x80 != 0,
        };
        if let Some(target) = registers.get_mut(register) {
            *target = latch;
        }
        if ends {
            lines.push(registers);
        }
        if pos > 0xffff {
            return None;
        }
    }
    Some(lines)
}

/// Draws scanline `y`.
fn render_line(
    image: &mut Image,
    y: usize,
    memory: &Memory<'_>,
    source: &Source,
    registers: &Registers,
    bytes_per_line: usize,
) -> Result<(), DecodeError> {
    let r = registers;
    let prior = r[0x1b];
    let gtia9 = match (prior >> 6, source) {
        (0, _) => false,
        (1, Source::Hires(_) | Source::Blank) => true,
        _ => return Err(DecodeError::Unrecognized),
    };
    let colors = Colors {
        player: [r[0x12], r[0x13], r[0x14], r[0x15]],
        playfield: [r[0x16], r[0x17], r[0x18], r[0x19]],
        background: r[0x1a],
    };
    let pmg = Pmg {
        hpos_player: [r[0], r[1], r[2], r[3]],
        hpos_missile: [r[4], r[5], r[6], r[7]],
        size_player: r[8] & 3 | (r[9] & 3) << 2 | (r[10] & 3) << 4 | (r[11] & 3) << 6,
        size_missile: r[12],
        graf_player: [r[0x0d], r[0x0e], r[0x0f], r[0x10]],
        graf_missile: r[0x11],
    };
    let objects = pmg.draw();
    let fifth = prior & 0x10 != 0;
    // Output pixel of the first screen byte, so that 40 bytes are centred.
    let left = 8 * (bytes_per_line as isize / 2 - 21);
    for (x, &objs) in objects.pixels.iter().enumerate() {
        let position = x as isize + left;
        let column = usize::try_from(position.div_euclid(8))
            .ok()
            .filter(|&column| column < bytes_per_line);
        let bit = position.rem_euclid(8) as u32;
        let cell = |address: usize| column.map(|column| memory.byte(address + column));
        let on_screen = (8..WIDTH - 8).contains(&x);
        let (mut playfield, luminance, nibble) = match *source {
            Source::Blank => (0, None, 0),
            Source::Four(address) => {
                let value = cell(address).map_or(0, |byte| byte >> (6 - bit / 2 * 2) & 3);
                (if value == 0 { 0 } else { 1 << (value - 1) }, None, 0)
            }
            Source::Hires(address) => {
                // Outside the screen data only the background shows.
                match cell(address) {
                    None => (0, None, 0),
                    Some(byte) => (
                        4,
                        (byte >> (7 - bit) & 1 != 0).then_some(colors.playfield[1] & 0x0f),
                        byte >> (4 - bit / 4 * 4) & 0x0f,
                    ),
                }
            }
        };
        if gtia9 {
            playfield = 0;
        }
        let mut players = objs & 0x0f;
        if fifth {
            if objs & 0xf0 != 0 {
                playfield |= 8;
            }
        } else {
            players |= objs >> 4;
        }
        let mut color = gtia::resolve(prior, players, playfield, &colors);
        if let (false, Some(luminance)) = (gtia9, luminance) {
            color = color & 0xf0 | luminance;
        }
        color = if gtia9 && players == 0 {
            // Graphics 9 is black outside the 320-pixel window, whatever
            // COLBK's hue, also on blank lines.
            if on_screen { color & 0xfe | nibble } else { 0 }
        } else {
            color & 0xfe
        };
        image.set(x as u32, y as u32, rgb(color));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A PGR with one Graphics 8 line at 8520 and `events` as the stream.
    fn file(events: &[u8]) -> Vec<u8> {
        let end = 0x8600 + events.len() - 1;
        let mut memory = alloc::vec![0; end - START + 1];
        let mut put = |address: usize, bytes: &[u8]| {
            memory[address - START..][..bytes.len()].copy_from_slice(bytes);
        };
        put(START, &[0x00, 0x86]);
        put(START + 2, SIGNATURE);
        put(DISPLAY_LIST, &[0x4f, 0x20, 0x85, 0x41, 0x10, 0x82]);
        put(0x8520, &[0x80]);
        // COLPF1 luminance 0xe, COLPF2 black, 40 bytes per line.
        put(initial_address(0x17), &[0x0e, 0x00]);
        put(DMACTL, &[0x32]);
        put(0x8600, events);
        let mut data = alloc::vec![0xff, 0xff];
        data.extend_from_slice(&(START as u16).to_le_bytes());
        data.extend_from_slice(&(end as u16).to_le_bytes());
        data.extend_from_slice(&memory);
        data
    }

    fn idle() -> Vec<u8> {
        alloc::vec![0x1c; LINES]
    }

    #[test]
    fn draws_hires_and_applies_events() {
        // COLBK = 0x4a at the start of line 0, which ends it.
        let mut events = idle();
        events.splice(0..1, [0xba, 0x4a]);
        let image = decode_pgr(&file(&events)).unwrap();
        assert_eq!((image.width(), image.height()), (336, 240));
        assert_eq!(image.get(8, 0), rgb(0x0e));
        assert_eq!(image.get(9, 0), 0);
        // Outside the screen data the background shows, and it persists.
        assert_eq!(image.get(0, 0), rgb(0x4a));
        assert_eq!(image.get(100, 100), rgb(0x4a));
    }

    #[test]
    fn rejects_bad_files() {
        let good = file(&idle());
        assert!(decode_pgr(&good).is_ok());
        // Fewer than 240 scanlines of events.
        assert!(decode_pgr(&file(&idle()[..239])).is_err());
        // Unknown event register.
        let mut events = idle();
        events[5] = 0x1e;
        assert!(decode_pgr(&file(&events)).is_err());
        // Wrong signature, length and load address.
        let mut bad = good.clone();
        bad[14] = b'X';
        assert!(decode_pgr(&bad).is_err());
        assert!(decode_pgr(&good[..good.len() - 1]).is_err());
        let mut bad = good;
        bad[2] = 0x07;
        assert!(decode_pgr(&bad).is_err());
    }
}
