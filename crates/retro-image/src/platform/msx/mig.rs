//! MIG pictures (MIF package, SofaRun): a compressed log of the VDP writes
//! that put the picture on screen.
//!
//! Source: reverse engineered from the 14 RECOIL sample files (no MIG
//! documentation exists; the MIF package and MIGVIEW sources were not read),
//! checked against `recoil2png` output and black-box probes of it:
//! - `"MSXMIG"`, the LE32 length of the rest of the file (from offset 6), a machine byte (0
//!   MSX1, 1 MSX2, 2 MSX2+), then blocks of LE16 unpacked size, LE16 packed
//!   size and BitBuster data (MIG variant, see `bitbuster.rs`), ended by an
//!   unpacked size of 0.
//! - The unpacked blocks form one command stream: `00 n` then n triples of
//!   register, value and mask (the bits of the mask are replaced); `01 first n`
//!   then n palette entries in the V9938 register format; `02` then a LE24
//!   VRAM address, a LE24 length and the bytes; `FF` ends it.
//! - The picture is what the VDP then shows. The screen mode, 192 or 212
//!   lines, the displayed page and the even/odd page interlace come from the
//!   standard V9938/V9958 registers (MSX2 Technical Handbook, chapter 4,
//!   <https://konamiman.github.io/MSX2-Technical-Handbook/md/Chapter4a.html>;
//!   V9958 Technical Data Book for R#25,
//!   <https://map.grauw.nl/resources/video/yamaha_v9958.pdf>), the picture
//!   from the standard VRAM tables.

use alloc::vec;
use alloc::vec::Vec;

use super::bitbuster::{self, Variant};
use super::screen::{self, Bitmap, Tiled};
use super::vdp::{self, Palette, Vram};
use crate::bytes::{le16, le32};
use crate::{DecodeError, Image};

/// V9938 VRAM, 128 KiB.
const VRAM_SIZE: usize = 0x20000;

/// Largest unpacked command stream accepted: a full VRAM write plus
/// registers and palette.
const MAX_STREAM: usize = VRAM_SIZE + 0x1000;

pub(super) fn decode(data: &[u8]) -> Result<Image, DecodeError> {
    let stream = unpack(data).ok_or(DecodeError::Unrecognized)?;
    let vdp = Vdp::run(&stream).ok_or(DecodeError::Unrecognized)?;
    vdp.render().ok_or(DecodeError::Unrecognized)
}

/// The command stream held in the BitBuster blocks.
fn unpack(data: &[u8]) -> Option<Vec<u8>> {
    if !data.starts_with(b"MSXMIG") || le32(data, 6)? as usize != data.len() - 6 {
        return None;
    }
    let mut stream = Vec::new();
    let mut pos = 11;
    loop {
        let size = le16(data, pos)? as usize;
        if size == 0 {
            return Some(stream);
        }
        let packed = le16(data, pos + 2)? as usize;
        pos += 4;
        let expected = stream.len() + size;
        if expected > MAX_STREAM {
            return None;
        }
        let block = data.get(pos..pos + packed)?;
        bitbuster::unpack_block(Variant::Mig, block, &mut stream, expected)?;
        if stream.len() < expected {
            return None;
        }
        stream.truncate(expected);
        pos += packed;
    }
}

/// VDP state after the command stream.
struct Vdp {
    registers: [u8; 64],
    palette: Option<Palette>,
    vram: Vec<u8>,
    /// End of the highest VRAM write.
    written_end: usize,
}

impl Vdp {
    fn run(stream: &[u8]) -> Option<Self> {
        let mut vdp = Self {
            registers: [0; 64],
            palette: None,
            vram: vec![0; VRAM_SIZE],
            written_end: 0,
        };
        let mut pos = 0;
        loop {
            match *stream.get(pos)? {
                0x00 => {
                    let count = *stream.get(pos + 1)? as usize;
                    let writes = stream.get(pos + 2..pos + 2 + 3 * count)?;
                    for write in writes.chunks_exact(3) {
                        let register = vdp.registers.get_mut(write[0] as usize)?;
                        *register = *register & !write[2] | write[1] & write[2];
                    }
                    pos += 2 + 3 * count;
                }
                0x01 => {
                    let first = *stream.get(pos + 1)? as usize;
                    let count = *stream.get(pos + 2)? as usize;
                    let entries = stream.get(pos + 3..pos + 3 + 2 * count)?;
                    let palette = vdp.palette.get_or_insert([0; 16]);
                    for (i, entry) in entries.chunks_exact(2).enumerate() {
                        *palette.get_mut(first + i)? = vdp::palette_entry(entry[0], entry[1]);
                    }
                    pos += 3 + 2 * count;
                }
                0x02 => {
                    let address = le24(stream, pos + 1)?;
                    let length = le24(stream, pos + 4)?;
                    let bytes = stream.get(pos + 7..pos + 7 + length)?;
                    vdp.vram
                        .get_mut(address..address + length)?
                        .copy_from_slice(bytes);
                    vdp.written_end = vdp.written_end.max(address + length);
                    pos += 7 + length;
                }
                0xff => return Some(vdp),
                _ => return None,
            }
        }
    }

    /// Screen mode bits M1-M5 of R#0 and R#1, and YJK/YAE of R#25.
    fn render(&self) -> Option<Image> {
        let r = &self.registers;
        let mode = (r[0] >> 1 & 7) << 2 | (r[1] >> 3 & 1) << 1 | r[1] >> 4 & 1;
        let tiled = match mode {
            0b00100 => Some(Tiled::Graphic2),
            0b00010 => Some(Tiled::Multicolour),
            0b01000 => Some(Tiled::Graphic3),
            _ => None,
        };
        if let Some(tiled) = tiled {
            let mut vram = Vram::new(&self.vram[..Vram::SIZE]);
            if tiled == Tiled::Multicolour && self.written_end <= 0x800 {
                screen::set_basic_multicolour_names(&mut vram);
            }
            let palette = self.palette.unwrap_or(match tiled {
                Tiled::Graphic3 => vdp::MSX2_PALETTE,
                _ => vdp::MSX1_PALETTE,
            });
            return Some(screen::render_tiled(tiled, &vram, &palette, false));
        }
        let bitmap = match (mode, r[25] & 0x18) {
            (0b01100, _) => Bitmap::Graphic4,
            (0b10000, _) => Bitmap::Graphic5,
            (0b10100, _) => Bitmap::Graphic6,
            (0b11100, 0x08) => Bitmap::Yjk,
            (0b11100, 0x18) => Bitmap::Yae,
            (0b11100, _) => Bitmap::Graphic7,
            _ => return None,
        };
        let palette = self.palette.unwrap_or_else(|| bitmap.default_palette());
        let lines = if r[9] & 0x80 != 0 { 212 } else { 192 };
        let page_size = match bitmap {
            Bitmap::Graphic4 | Bitmap::Graphic5 => 0x8000,
            _ => 0x10000,
        };
        let pages = VRAM_SIZE / page_size;
        let page = (r[2] >> 5) as usize & (pages - 1);
        let page_bytes = |page: usize| {
            let start = page * page_size;
            &self.vram[start..start + lines * bitmap.bytes_per_line()]
        };
        // R#9 bit 2 (EO): the even field shows the page with its lowest page
        // bit clear, the odd field the one with it set.
        let (even, odd) = if r[9] & 0x04 != 0 {
            (page_bytes(page & !1), Some(page_bytes(page | 1)))
        } else {
            (page_bytes(page), None)
        };
        screen::render_bitmap(bitmap, even, odd, &palette).ok()
    }
}

fn le24(data: &[u8], at: usize) -> Option<usize> {
    Some(le16(data, at)? as usize | (*data.get(at + 2)? as usize) << 16)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A BitBuster block of literals only: a zero flag byte before every 8.
    fn literals(bytes: &[u8]) -> Vec<u8> {
        bytes
            .chunks(8)
            .flat_map(|chunk| [&[0][..], chunk].concat())
            .collect()
    }

    fn mig(stream: &[u8]) -> Vec<u8> {
        let block = literals(stream);
        let mut data = b"MSXMIG\0\0\0\0\x01".to_vec();
        data.extend((stream.len() as u16).to_le_bytes());
        data.extend((block.len() as u16).to_le_bytes());
        data.extend(block);
        data.extend([0, 0]);
        let rest = (data.len() - 6) as u32;
        data[6..10].copy_from_slice(&rest.to_le_bytes());
        data
    }

    #[test]
    fn screen5_from_registers_palette_and_vram() {
        let mut stream = vec![0, 2, 0, 0x06, 0xff, 9, 0x80, 0x80];
        stream.extend([1, 1, 1, 0x70, 0x00]);
        stream.extend([2, 0, 0, 0, 1, 0, 0, 0x10]);
        stream.push(0xff);
        let image = decode(&mig(&stream)).unwrap();
        assert_eq!((image.width(), image.height()), (256, 212));
        assert_eq!(image.get(0, 0), 0xff0000);
        assert_eq!(image.get(1, 0), 0);
    }

    #[test]
    fn register_write_keeps_bits_outside_mask() {
        let stream = [0, 2, 1, 0xff, 0xff, 1, 0x00, 0x0f, 0xff];
        let vdp = Vdp::run(&stream).unwrap();
        assert_eq!(vdp.registers[1], 0xf0);
    }

    #[test]
    fn rejects_wrong_length_and_unknown_command() {
        let mut data = mig(&[0, 1, 0, 0x06, 0xff, 0xff]);
        assert!(decode(&data).is_ok());
        data[6] ^= 1;
        assert!(decode(&data).is_err());
        assert!(decode(&mig(&[3, 0xff])).is_err());
        // A VRAM write past 128 KiB.
        assert!(decode(&mig(&[2, 0, 0, 2, 1, 0, 0, 0, 0xff])).is_err());
    }
}
