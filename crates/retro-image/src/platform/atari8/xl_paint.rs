//! XL-Paint pictures: XLP, MAX and RAW (XL-Paint MAX).
//!
//! Sources:
//! - XL-Paint 1.9 MaX info (Polish),
//!   <http://ftp.pigwa.net/stuff/collections/atari_forever/Tools%20-%20atr/XL-Paint%201.9Max.txt>:
//!   the program draws 160x192 pictures in 16-colour interlace (two frames
//!   shown alternately). RAW is `XLPB` plus 7680 + 7680 bytes of frame data,
//!   XLP is `XLPC` (or no header), 4 colour bytes and compressed data, MAX
//!   is `XLPM`, per-line colour tables and compressed data.
//! - Just Solve "XL-Paint" (<http://fileformats.archiveteam.org/wiki/XL-Paint>).
//! - The rest was reverse engineered from the corpus and `recoil2png` output
//!   (black box, including hand-made files):
//!   - RAW: after the frames come two colour sets (playfield 0-2, then the
//!     background). Lines alternate: the first frame uses the first set on
//!     even lines and the second set on odd lines, the second frame the
//!     reverse.
//!   - MAX: nine tables of 192 bytes (one value per line) follow the header:
//!     the background and playfield 0-2 colours of the second stored frame
//!     (tables 0-3), then those of the first stored frame (tables 4-7), and
//!     one that has no effect. BIRD.MAX and BIRD.RAW (and the GOLDENB pair)
//!     render identically, which gave the layout: RAW's two colour sets are
//!     tables 4-7 and 0-3 (as playfield 0-2, then background), and line `y`
//!     of RAW frame `f` is stored frame `(f + y) % 2`.
//!   - The packer, shared by XLP and MAX, is described at [`command`]. The
//!     BIRD and GOLDENB streams unpack to their RAW frames exactly. The
//!     unpacked data is column-major: for each of the 40 byte columns the
//!     192 (or 200) lines of the first stored frame, then those of the
//!     second. Which commands are valid, how short data is treated and the
//!     XLP line count rules (see [`decode_xlp`]) were probed with hand-made
//!     files whose colour tables make every pair of stored pixel values show
//!     as a distinct colour.

use super::antic::Bitmap;
use super::palette::register_rgb;
use crate::{DecodeError, Image};
use alloc::vec::Vec;

const FRAME: usize = 7680;
const LINES: usize = 192;
const TABLE: usize = 192;

fn frame(data: &[u8]) -> Bitmap<'_> {
    Bitmap {
        data: &data[..FRAME],
        bytes_per_line: 40,
        lines: LINES,
        bits: 2,
    }
}

/// RAW: `XLPB`, two frames, then two sets of playfield 0-2 and background.
/// The first frame uses the first set on even lines and the second on odd
/// lines; the second frame the other way round.
pub(super) fn decode_raw(data: &[u8]) -> Result<Image, DecodeError> {
    let rest = data
        .strip_prefix(b"XLPB")
        .filter(|rest| rest.len() == 2 * FRAME + 8)
        .ok_or(DecodeError::Unrecognized)?;
    let (frames, sets) = rest.split_at(2 * FRAME);
    let draw = |data: &[u8], f: usize| {
        frame(data).render(2, 1, |line, value| {
            let half = (f + line) % 2;
            register_rgb(set_color(&sets[4 * half..4 * half + 4], value))
        })
    };
    let (first, second) = frames.split_at(FRAME);
    Ok(Image::blend(&[&draw(first, 0)?, &draw(second, 1)?]))
}

/// A colour set stored as playfield 0-2, then the background.
fn set_color(set: &[u8], value: u8) -> u8 {
    match value {
        0 => set[3],
        v => set[usize::from(v) - 1],
    }
}

/// MAX: `XLPM`, nine tables of 192 per-line colours, packed frames.
pub(super) fn decode_max(data: &[u8]) -> Result<Image, DecodeError> {
    let rest = data
        .strip_prefix(b"XLPM")
        .ok_or(DecodeError::Unrecognized)?;
    let (tables, packed) = rest
        .split_at_checked(9 * TABLE)
        .ok_or(DecodeError::Unrecognized)?;
    let stored = Stored::new(packed, LINES, false)?;
    // Stored frame 0 is drawn with tables 4-7, frame 1 with tables 0-3, each
    // listing the background first.
    let draw = |s: usize, first_table: usize| {
        stored.render(s, |line, value| {
            let table = first_table + usize::from(value);
            register_rgb(tables[table * TABLE + line])
        })
    };
    Ok(Image::blend(&[&draw(0, 4)?, &draw(1, 0)?]))
}

/// XLP: an optional `XLPC`, playfield 0-2 and background, then packed
/// frames. With `XLPC` the picture is 192 lines, and bad or missing data
/// just ends the unpacking. Without it the data must unpack to at least
/// 15360 bytes (192 lines), or to 16000 and more for 200 lines.
pub(super) fn decode_xlp(data: &[u8]) -> Result<Image, DecodeError> {
    let (compact, data) = match data.strip_prefix(b"XLPC") {
        Some(rest) => (true, rest),
        None => (false, data),
    };
    let (colors, packed) = data.split_at_checked(4).ok_or(DecodeError::Unrecognized)?;
    let lines = if compact {
        LINES
    } else if packed_len(packed).ok_or(DecodeError::Unrecognized)? >= 80 * 200 {
        200
    } else {
        LINES
    };
    let stored = Stored::new(packed, lines, compact)?;
    let draw = |s: usize| stored.render(s, |_, value| register_rgb(set_color(colors, value)));
    Ok(Image::blend(&[&draw(0)?, &draw(1)?]))
}

/// Two frames of 40 byte columns, as unpacked.
struct Stored {
    data: Vec<u8>,
    lines: usize,
}

impl Stored {
    fn new(packed: &[u8], lines: usize, lenient: bool) -> Result<Self, DecodeError> {
        let data = unpack(packed, 80 * lines, lenient).ok_or(DecodeError::Unrecognized)?;
        Ok(Self { data, lines })
    }

    /// Stored frame `s`, 2 bits per pixel, as a 160-pixel-wide picture of
    /// double-width pixels; `color(line, value)` is the RGB of a pixel.
    fn render(&self, s: usize, color: impl Fn(usize, u8) -> u32) -> Result<Image, DecodeError> {
        let mut row_major = Vec::with_capacity(40 * self.lines);
        for line in 0..self.lines {
            for column in 0..40 {
                row_major.push(self.data[(column * 2 + s) * self.lines + line]);
            }
        }
        Bitmap {
            data: &row_major,
            bytes_per_line: 40,
            lines: self.lines,
            bits: 2,
        }
        .render(2, 1, color)
    }
}

/// One command of the packed stream.
enum Command<'a> {
    /// The bytes present and the length the command asks for.
    Literal(&'a [u8], usize),
    Run(usize, u8),
}

impl Command<'_> {
    fn len(&self) -> usize {
        match *self {
            Command::Literal(_, len) | Command::Run(len, _) => len,
        }
    }
}

/// Reads the command at `pos` and returns it with the position after it.
/// `None` for an invalid command or one cut short, except that a literal's
/// data may end early (the missing bytes read as zeros).
///
/// The packer: a byte up to `0x3f` copies that many bytes; `0x40` copies the
/// number of bytes in the next byte; `0x80` plus a count (0-63) repeats the
/// next byte; `0xc0` plus a 14-bit count (the low 6 bits of the first byte,
/// then a byte) repeats the byte after it. The bytes `0x41`-`0x7f` are
/// invalid.
fn command(data: &[u8], pos: usize) -> Option<(Command<'_>, usize)> {
    let first = *data.get(pos)?;
    let literal = |start: usize, count: usize| {
        let end = start + count;
        let bytes = data.get(start..end.min(data.len()))?;
        Some((Command::Literal(bytes, count), end))
    };
    match first {
        0..=0x3f => literal(pos + 1, usize::from(first)),
        0x40 => literal(pos + 2, usize::from(*data.get(pos + 1)?)),
        0x41..=0x7f => None,
        0x80..=0xbf => Some((
            Command::Run(usize::from(first & 0x3f), *data.get(pos + 1)?),
            pos + 2,
        )),
        _ => {
            let count = usize::from(first & 0x3f) << 8 | usize::from(*data.get(pos + 1)?);
            Some((Command::Run(count, *data.get(pos + 2)?), pos + 3))
        }
    }
}

/// How many bytes the whole stream unpacks to.
fn packed_len(data: &[u8]) -> Option<usize> {
    let mut total = 0;
    let mut pos = 0;
    while pos < data.len() {
        let (command, next) = command(data, pos)?;
        total += command.len();
        pos = next;
    }
    Some(total)
}

/// Unpacks `len` bytes (the overshoot of the last command is dropped). The
/// stream must hold that much, unless `lenient`: then an invalid or missing
/// command ends it and the rest is zeros, as long as something was read.
fn unpack(data: &[u8], len: usize, lenient: bool) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(len);
    let mut pos = 0;
    while out.len() < len {
        let Some((command, next)) = command(data, pos) else {
            if lenient && pos > 0 {
                break;
            }
            return None;
        };
        match command {
            Command::Literal(bytes, count) => {
                out.extend_from_slice(bytes);
                out.resize(out.len() + count - bytes.len(), 0);
            }
            Command::Run(count, value) => out.resize(out.len() + count, value),
        }
        pos = next;
    }
    out.resize(len, 0);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unpacks_every_command() {
        // Literal, extended literal, short run, long run.
        let mut packed = alloc::vec![2, 1, 2, 0x40, 1, 3, 0x82, 4, 0xc0, 3, 5];
        packed.extend_from_slice(&[0xc0, 0x10, 0]);
        let out = unpack(&packed, 9, false).unwrap();
        assert_eq!(out, [1, 2, 3, 4, 4, 5, 5, 5, 0]);
    }

    #[test]
    fn invalid_commands_end_lenient_streams_only() {
        let packed = [1, 9, 0x41, 7];
        assert_eq!(unpack(&packed, 3, true).unwrap(), [9, 0, 0]);
        assert!(unpack(&packed, 3, false).is_none());
        assert!(unpack(&[], 3, true).is_none());
    }

    #[test]
    fn frames_are_stored_by_column() {
        // Stored frame 0, column 1, line 2: column 0 holds both frames first.
        let mut stored = alloc::vec![0u8; 80 * LINES];
        stored[2 * LINES + 2] = 0b1100_0000;
        let stored = Stored {
            data: stored,
            lines: LINES,
        };
        let image = stored.render(0, |_, value| u32::from(value)).unwrap();
        assert_eq!(image.get(8, 2), 3);
        assert_eq!(image.get(8, 3), 0);
    }
}
