//! Game Boy Printer packet captures: text logs of what a Game Boy sent to
//! the printer, as the Arduino and WebUSB printer emulators write them. The
//! picture is rebuilt from the packets.
//!
//! Sources:
//! - Pan Docs, "Game Boy Printer"
//!   (<https://gbdev.io/pandocs/Gameboy_Printer.html>, CC0): a packet is the
//!   sync bytes `0x88 0x33`, a command (1 initialize, 2 print, 4 data, 0xF
//!   inquiry), a compression flag, a little-endian `u16` length, the data and a
//!   little-endian `u16` checksum, the sum of every byte after the sync bytes
//!   up to the checksum; the printer answers with `0x81` and a status byte. A
//!   data packet holds up to 0x280 bytes of 2 bpp tiles in the Game Boy's
//!   tile encoding, 20 tiles to a row. A print packet has four bytes: the
//!   number of sheets, the margins, the palette (the Game Boy's `BGP` layout,
//!   usually `0xE4`) and the exposure.
//! - Reverse engineered from the capture files and expected pictures in the
//!   `mofosyne/arduino-gameboy-printer-emulator` repository (GPL-3; only its
//!   data files were used, none of its code was read), in
//!   `corpus/extra/nintendo-rom-icons/gb-printer`:
//!   - The text form is bytes as hex, either `88 33 01 ...` or `0x88, 0x33,
//!     ...`, with `//` and `/* */` comments. The printer's answer is part of
//!     the stream in the first form (after every packet) and sits in a comment
//!     in the second.
//!   - The compression is a run-length code over the data: a control byte
//!     with bit 7 set repeats the next byte `(n & 0x7F) + 2` times, otherwise
//!     it is followed by `n + 1` bytes to copy. All three prints of the
//!     compression test capture come out identical to the reference PNG.
//!   - Each print packet turns the data sent since the previous print or
//!     initialize into a strip 160 pixels wide, drawn through the palette byte
//!     with the four shades 255, 170, 85 and 0. A capture with several prints
//!     is one picture with the strips in order; margins and sheet counts are
//!     ignored.
//!
//! Detection is by content: the whole file must be hex bytes and comments,
//! hold at least one packet with a correct checksum, and print something.
//! Packets with a wrong checksum are skipped, as the printer would refuse them.
//! The capture `test1.txt` of that repository has a print packet whose
//! checksum is wrong, so it draws nothing; its reference PNG came from a
//! decoder that does not check. The older `!DATA` text form of the emulator
//! is not read.

use alloc::vec::Vec;

use super::TILE;
use crate::image::check_size;
use crate::{DecodeError, Image};

const SYNC: [u8; 2] = [0x88, 0x33];
const INITIALIZE: u8 = 1;
const PRINT: u8 = 2;
const DATA: u8 = 4;
const ACKNOWLEDGE: u8 = 0x81;
/// Bytes of a packet before its data, and of its checksum.
const HEADER_LEN: usize = 6;
const CHECKSUM_LEN: usize = 2;
/// Most data bytes a packet carries.
const MAX_DATA: usize = 0x280;
const TILES_PER_ROW: usize = 20;
const WIDTH: usize = TILES_PER_ROW * 8;
/// Most tile bytes one capture may hold in all: 4 million pixels.
const MAX_TILE_BYTES: usize = 1 << 20;
/// The shades of the printer's four levels, lightest first.
const SHADES: [u32; 4] = [0xff_ffff, 0xaa_aaaa, 0x55_5555, 0x00_0000];

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    let fail = DecodeError::Unrecognized;
    let bytes = capture_bytes(data).ok_or(fail)?;
    let mut strips: Vec<Image> = Vec::new();
    let mut tiles: Vec<u8> = Vec::new();
    let mut total = 0;
    let mut at = 0;
    while at + 1 < bytes.len() {
        let Some(packet) = Packet::at(&bytes[at..]) else {
            at += 1;
            continue;
        };
        at += packet.len;
        // The answer of the printer, when the capture holds it.
        if bytes.get(at) == Some(&ACKNOWLEDGE) {
            at += 2;
        }
        match packet.command {
            INITIALIZE => tiles.clear(),
            DATA => {
                let before = tiles.len();
                if packet.compressed {
                    unrle(packet.data, &mut tiles).ok_or(fail)?;
                } else {
                    tiles.extend_from_slice(packet.data);
                }
                total += tiles.len() - before;
                if total > MAX_TILE_BYTES {
                    return Err(fail);
                }
            }
            PRINT if packet.data.len() == 4 && !tiles.is_empty() => {
                strips.push(strip(&tiles, packet.data[2])?);
                tiles.clear();
            }
            _ => {}
        }
    }
    stack(&strips)
}

/// One packet of a byte stream.
struct Packet<'a> {
    command: u8,
    compressed: bool,
    data: &'a [u8],
    /// Bytes from the sync to the end of the checksum.
    len: usize,
}

impl<'a> Packet<'a> {
    /// The packet that starts `bytes`, if there is one with a correct
    /// checksum.
    fn at(bytes: &'a [u8]) -> Option<Self> {
        let header = bytes.strip_prefix(&SYNC)?.get(..HEADER_LEN - SYNC.len())?;
        let (command, compression) = (header[0], header[1]);
        let data_len = usize::from(u16::from_le_bytes([header[2], header[3]]));
        if data_len > MAX_DATA {
            return None;
        }
        let end = HEADER_LEN + data_len;
        let body = bytes.get(SYNC.len()..end)?;
        let checksum = bytes.get(end..end + CHECKSUM_LEN)?;
        let sum = body
            .iter()
            .fold(0u16, |sum, &b| sum.wrapping_add(u16::from(b)));
        (sum.to_le_bytes() == checksum).then_some(Self {
            command,
            compressed: compression != 0,
            data: &bytes[HEADER_LEN..end],
            len: end + CHECKSUM_LEN,
        })
    }
}

/// The bytes a capture text writes, or `None` if the text holds anything
/// else than hex bytes (`FF` or `0xFF`), separators and comments.
fn capture_bytes(text: &[u8]) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut at = 0;
    while let Some(&c) = text.get(at) {
        match c {
            b' ' | b'\t' | b'\r' | b'\n' | b',' => at += 1,
            b'/' => {
                at = match text.get(at + 1)? {
                    b'/' => text[at..]
                        .iter()
                        .position(|&b| b == b'\n')
                        .map_or(text.len(), |n| at + n),
                    b'*' => at + 2 + text[at + 2..].windows(2).position(|w| w == b"*/")? + 2,
                    _ => return None,
                };
            }
            _ => {
                let digits_at = if c == b'0' && matches!(text.get(at + 1), Some(b'x' | b'X')) {
                    at + 2
                } else {
                    at
                };
                let digit = |i: usize| char::from(*text.get(i)?).to_digit(16);
                bytes.push((digit(digits_at)? << 4 | digit(digits_at + 1)?) as u8);
                at = digits_at + 2;
                if text.get(at).is_some_and(u8::is_ascii_alphanumeric) {
                    return None;
                }
            }
        }
    }
    Some(bytes)
}

/// Appends the bytes `data` run-length decodes to.
fn unrle(data: &[u8], out: &mut Vec<u8>) -> Option<()> {
    let mut at = 0;
    while let Some(&control) = data.get(at) {
        at += 1;
        if control & 0x80 != 0 {
            let value = *data.get(at)?;
            at += 1;
            out.resize(out.len() + usize::from(control & 0x7f) + 2, value);
        } else {
            let count = usize::from(control) + 1;
            out.extend_from_slice(data.get(at..at + count)?);
            at += count;
        }
    }
    Some(())
}

/// The strip that `tiles` print as with `palette`.
fn strip(tiles: &[u8], palette: u8) -> Result<Image, DecodeError> {
    let shades: [u32; 4] =
        core::array::from_fn(|color| SHADES[usize::from(palette >> (2 * color) & 3)]);
    TILE.sheet(tiles, TILES_PER_ROW, &shades)
}

/// The strips one below the other.
fn stack(strips: &[Image]) -> Result<Image, DecodeError> {
    let height: usize = strips.iter().map(|s| s.height() as usize).sum();
    check_size(WIDTH, height)?;
    let mut image = Image::new(WIDTH as u32, height as u32);
    let mut y = 0u32;
    for strip in strips {
        for row in strip.rgb().as_chunks::<{ WIDTH * 3 }>().0 {
            image.row_mut(y).copy_from_slice(row);
            y += 1;
        }
    }
    Ok(image)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;
    use alloc::string::String;

    fn packet(command: u8, compression: u8, data: &[u8]) -> Vec<u8> {
        let mut packet = alloc::vec![0x88, 0x33, command, compression];
        packet.extend((data.len() as u16).to_le_bytes());
        packet.extend_from_slice(data);
        let sum = packet[2..]
            .iter()
            .fold(0u16, |s, &b| s.wrapping_add(u16::from(b)));
        packet.extend(sum.to_le_bytes());
        packet
    }

    /// The capture as one line per packet, with the printer's answer in the
    /// stream, and as a C array with the answer in a comment.
    fn texts(packets: &[Vec<u8>]) -> (String, String) {
        let hex = |bytes: &[u8], prefix: &str| {
            bytes
                .iter()
                .map(|b| format!("{prefix}{b:02X}"))
                .collect::<Vec<_>>()
                .join(if prefix.is_empty() { " " } else { ", " })
        };
        let mut lines = String::from("// capture\n");
        let mut array = String::from("/* capture */\n");
        for (i, packet) in packets.iter().enumerate() {
            lines += &format!("// {i} : PACKET\n{} 81 00\n", hex(packet, ""));
            array += &format!(
                "/* {i} */\n{}, /*(*/ 0x81, 0x00, /*)*/\n",
                hex(packet, "0x")
            );
        }
        (lines, array)
    }

    /// One tile row of 20 tiles: tile 0 has pixel (0, 0) in color 3.
    fn tile_row() -> Vec<u8> {
        let mut row = alloc::vec![0; 320];
        row[0] = 0x80;
        row[1] = 0x80;
        row
    }

    #[test]
    fn both_text_forms_draw_the_strips_in_order_through_the_palette() {
        let first = tile_row();
        let packets = [
            packet(INITIALIZE, 0, &[]),
            packet(DATA, 0, &first),
            packet(DATA, 0, &[]),
            packet(PRINT, 0, &[1, 0x13, 0xe4, 0x40]),
            packet(0x0f, 0, &[]),
            // The second strip prints the same tiles through another palette.
            packet(DATA, 0, &first),
            packet(PRINT, 0, &[1, 0x13, 0x1b, 0x40]),
        ];
        let (lines, array) = texts(&packets);
        let image = decode(lines.as_bytes()).unwrap();
        assert_eq!(image, decode(array.as_bytes()).unwrap());
        assert_eq!((image.width(), image.height()), (160, 16));
        // Color 3 is black through 0xE4 and white through 0x1B; 0 swaps.
        assert_eq!(image.get(0, 0), 0x00_0000);
        assert_eq!(image.get(1, 0), 0xff_ffff);
        assert_eq!(image.get(0, 8), 0xff_ffff);
        assert_eq!(image.get(1, 8), 0x00_0000);
    }

    #[test]
    fn a_compression_flag_run_length_decodes_the_data() {
        // 4 literal bytes, then runs of 127, 127 and 62 zeros: the run
        // length is the low 7 bits plus 2.
        let mut packed = alloc::vec![3, 0x80, 0x80, 0, 0];
        packed.extend([0x80 | 125, 0, 0x80 | 125, 0, 0x80 | 60, 0]);
        let mut unpacked = Vec::new();
        unrle(&packed, &mut unpacked).unwrap();
        assert_eq!(unpacked.len(), 320);
        assert_eq!(unpacked[..4], [0x80, 0x80, 0, 0]);
        let packets = [
            packet(DATA, 1, &packed),
            packet(PRINT, 0, &[1, 0, 0xe4, 0x40]),
        ];
        let (text, _) = texts(&packets);
        assert_eq!(decode(text.as_bytes()).unwrap().get(0, 0), 0x00_0000);
        // A control byte that promises more bytes than are there.
        assert!(unrle(&[5, 1, 2], &mut Vec::new()).is_none());
        assert!(unrle(&[0x81], &mut Vec::new()).is_none());
    }

    #[test]
    fn packets_with_a_wrong_checksum_are_skipped() {
        let mut bad_print = packet(PRINT, 0, &[1, 0, 0xe4, 0x40]);
        *bad_print.last_mut().unwrap() ^= 1;
        let init = packet(INITIALIZE, 0, &[]);
        let data = packet(DATA, 0, &tile_row());
        let (text, _) = texts(&[init.clone(), data.clone(), bad_print.clone()]);
        assert!(decode(text.as_bytes()).is_err());
        // A resent print makes the picture.
        let good = packet(PRINT, 0, &[1, 0, 0xe4, 0x40]);
        let (text, _) = texts(&[init, data, bad_print, good]);
        assert!(decode(text.as_bytes()).is_ok());
    }

    #[test]
    fn anything_but_hex_and_comments_is_rejected() {
        let (text, _) = texts(&[
            packet(DATA, 0, &tile_row()),
            packet(PRINT, 0, &[1, 0, 0xe4, 0x40]),
        ]);
        assert!(decode(text.as_bytes()).is_ok());
        assert!(decode(format!("{text}\nstray").as_bytes()).is_err());
        assert!(decode(format!("{text}\n/* open").as_bytes()).is_err());
        assert!(decode(format!("{text}\n123").as_bytes()).is_err());
        assert!(decode(b"").is_err());
        assert!(decode(&[0xff, 0x00, 0x88, 0x33]).is_err());
        // Packets without any print.
        let (text, _) = texts(&[packet(DATA, 0, &tile_row())]);
        assert!(decode(text.as_bytes()).is_err());
    }
}
