//! SprEd sprite sheets (`Spr!`, `.SPR`): player/missile animations for the
//! Atari 8-bit.
//!
//! The AtariAge thread about SprEd
//! (<https://forums.atariage.com/topic/330217-spred-new-atari-sprite-editor/>)
//! publishes no layout. Reverse engineered from `corpus/running-cat.spr`,
//! `cinema-counter.spr` and `vial.spr`, by flipping one byte of each at a time
//! and reading back which `recoil2png` pixels changed, and by probing the
//! accepted header values and file lengths with synthetic files:
//!
//! - `Spr!`, a four-byte version (ignored), then `00 01`-style flags. Byte 9
//!   is 1 if every line is shown twice, 0 if once. Byte 10 bit 2 gives a
//!   second column of sprites, bit 0 a missile pair beside each column. Byte
//!   14 bit 0 says colors are given per line. Bytes 12 and 13 widen the
//!   canvas for the second column (`2 * byte 13` pixels, the width of that
//!   column). With byte 16 zero, bytes 17 and 18 are the frame and line
//!   counts; otherwise bytes 16 and 17 are (and byte 18 widens a one-column
//!   canvas). The data starts at byte 19, which is where all offsets below are
//!   counted from.
//! - Per frame, one color byte for each player (two players per column): all
//!   frames of player 0, then of player 1, and so on.
//! - Then one plane per player, again player after player, each holding one
//!   byte per line per frame (frame after frame). With missiles, one more plane
//!   per player follows, whose low two bits are the missile pixels (bit 1
//!   left).
//! - With per-line colors, five more planes of one byte per line per frame
//!   follow: the background and the four players.
//! - Players are 8 bits wide, one bit being 2 screen pixels. The two players of
//!   a column overlap and OR their color registers (the Atari's multi-color
//!   players); a column is 16 pixels wide, 20 with its missiles. Frames are
//!   spread 4 pixels apart, the background color of a frame filling the gap
//!   after it. Colors are Atari color registers.
//! - Bytes after the last plane are ignored.

use super::antic::fill;
use super::palette::register_rgb;
use crate::{DecodeError, Image};

const HEADER_LEN: usize = 19;
const PIXEL: u32 = 2;
const GAP: usize = 4;

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let header = data.get(..HEADER_LEN).ok_or(fail)?;
    if &header[..4] != b"Spr!" || header[9] > 1 {
        return Err(fail);
    }
    let layout = Layout::parse(header).ok_or(fail)?;
    let mut body = data.get(HEADER_LEN..).ok_or(fail)?;
    let mut take = |len: usize| -> Result<&[u8], DecodeError> {
        let (head, rest) = body.split_at_checked(len).ok_or(fail)?;
        body = rest;
        Ok(head)
    };
    let per_plane = layout.frames * layout.lines;
    let colors = take(layout.players() * layout.frames)?;
    let planes = take(layout.planes() * per_plane)?;
    let line_colors = if layout.per_line {
        take(5 * per_plane)?
    } else {
        &[]
    };
    let frame_width = layout.canvas_width() + GAP;
    let repeat = if header[9] == 1 { 2 } else { 1 };
    let mut image = Image::new(
        (layout.frames * frame_width - GAP) as u32,
        (layout.lines * repeat) as u32,
    )?;
    // Color register of `player` on `line` of `frame`.
    let color = |player: usize, frame: usize, line: usize| {
        if layout.per_line {
            line_colors[(1 + player) * per_plane + frame * layout.lines + line]
        } else {
            colors[player * layout.frames + frame]
        }
    };
    for frame in 0..layout.frames {
        for line in 0..layout.lines {
            let background = if layout.per_line {
                line_colors[frame * layout.lines + line]
            } else {
                0
            };
            let plane = |index: usize| planes[index * per_plane + frame * layout.lines + line];
            let y = (line * repeat) as u32;
            let origin = frame * frame_width;
            // The background runs on into the gap after the frame.
            let band = (layout.canvas_width() + GAP).min(image.width() as usize - origin);
            fill(
                &mut image,
                origin as u32,
                y,
                band as u32,
                repeat as u32,
                register_rgb(background),
            );
            for column in 0..layout.columns {
                let x0 = origin + column * layout.column_width();
                for bit in 0..8 {
                    let mask = 0x80 >> bit;
                    let mut value = 0;
                    for player in 0..2 {
                        let index = 2 * column + player;
                        if plane(index) & mask != 0 {
                            value |= color(index, frame, line);
                        }
                    }
                    if value != 0 {
                        let x = (x0 + bit * 2) as u32;
                        fill(&mut image, x, y, PIXEL, repeat as u32, register_rgb(value));
                    }
                }
                if layout.missiles {
                    for pixel in 0..2 {
                        let mut value = 0;
                        for player in 0..2 {
                            let index = 2 * column + player;
                            if plane(layout.players() + index) & (2 >> pixel) != 0 {
                                value |= color(index, frame, line);
                            }
                        }
                        if value != 0 {
                            let x = (x0 + 16 + pixel * 2) as u32;
                            fill(&mut image, x, y, PIXEL, repeat as u32, register_rgb(value));
                        }
                    }
                }
            }
        }
    }
    Ok(image)
}

/// What the header says about the sheet.
struct Layout {
    frames: usize,
    lines: usize,
    columns: usize,
    missiles: bool,
    per_line: bool,
}

impl Layout {
    fn parse(header: &[u8]) -> Option<Self> {
        let flags = header[10];
        let (frames, lines, extra_width) = if header[16] == 0 {
            (header[17], header[18], 0)
        } else {
            (header[16], header[17], header[18])
        };
        let layout = Self {
            frames: usize::from(frames),
            lines: usize::from(lines),
            columns: if flags & 4 != 0 { 2 } else { 1 },
            missiles: flags & 1 != 0,
            per_line: header[14] & 1 != 0,
        };
        // Only the canvases of the samples are understood: a second column
        // exactly as wide as the first and no extra width.
        let known_width = if layout.columns == 2 {
            header[16] == 0
                && header[12] == 0
                && usize::from(header[13]) * 2 == layout.column_width()
        } else {
            extra_width == 0
        };
        (layout.frames > 0 && layout.lines > 0 && known_width).then_some(layout)
    }

    /// A player's 16 pixels plus 4 for its missiles.
    fn column_width(&self) -> usize {
        if self.missiles { 20 } else { 16 }
    }

    fn canvas_width(&self) -> usize {
        self.columns * self.column_width()
    }

    /// Players with color bytes.
    fn players(&self) -> usize {
        2 * self.columns
    }

    /// Planes of sprite data: one per player and, with missiles, one more.
    fn planes(&self) -> usize {
        if self.missiles {
            2 * self.players()
        } else {
            self.players()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One frame of one line: players 0 and 1 in colors `c0` and `c1`.
    fn sheet(bits0: u8, bits1: u8, c0: u8, c1: u8) -> alloc::vec::Vec<u8> {
        let mut data = b"Spr!\0\0\0\x03\0\0".to_vec();
        data.resize(16, 0);
        data.extend([0, 1, 1, c0, c1, bits0, bits1]);
        data
    }

    #[test]
    fn overlapping_players_or_their_colours() {
        let image = decode(&sheet(0b1100_0000, 0b0110_0000, 0x40, 0x0a)).unwrap();
        assert_eq!((image.width(), image.height()), (16, 1));
        assert_eq!(image.get(0, 0), register_rgb(0x40));
        assert_eq!(image.get(2, 0), register_rgb(0x4a));
        assert_eq!(image.get(4, 0), register_rgb(0x0a));
        assert_eq!(image.get(6, 0), 0);
    }

    #[test]
    fn rejects_other_data() {
        assert!(decode(&sheet(0, 0, 0, 0)[..20]).is_err(), "truncated");
        let mut data = sheet(0, 0, 0, 0);
        data[0] = b'X';
        assert!(decode(&data).is_err());
    }
}
