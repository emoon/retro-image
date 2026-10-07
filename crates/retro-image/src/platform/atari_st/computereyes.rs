//! ComputerEyes raw digitizer data (`CE1`-`CE3`).
//!
//! Sources:
//! - <https://temlib.org/AtariForumWiki/index.php/ComputerEyes_Raw_Data_file_format>
//! - <http://fileformats.archiveteam.org/wiki/ComputerEyes>
//! - Observed from `recoil2png` output: components are scaled by bit
//!   replication, the high-resolution sum (0-191) as `sum * 4 / 3`, and
//!   medium resolution lines are doubled.

use crate::bytes::{be16, be32};
use crate::image::widen_channel;
use crate::{DecodeError, Image};

const HEADER_LEN: usize = 22;

pub(super) fn decode_ce(data: &[u8]) -> Result<Image, DecodeError> {
    decode(data).ok_or(DecodeError::Invalid)
}

fn decode(data: &[u8]) -> Option<Image> {
    if be32(data, 0)? != u32::from_be_bytes(*b"EYES") {
        return None;
    }
    let body = &data[HEADER_LEN.min(data.len())..];
    match be16(data, 4)? {
        0 if body.len() == 3 * 64000 => {
            let mut image = Image::new(320, 200).ok()?;
            for x in 0..320 {
                for y in 0..200 {
                    let i = x * 200 + y;
                    let level =
                        |plane: usize| widen_channel(u32::from(body[plane * 64000 + i] & 0x3f), 6);
                    image.set(
                        x as u32,
                        y as u32,
                        level(0) << 16 | level(1) << 8 | level(2),
                    );
                }
            }
            Some(image)
        }
        1 if body.len() == 256000 => {
            let mut image = Image::new(640, 400).ok()?;
            for x in 0..640 {
                for y in 0..200 {
                    let i = (x * 200 + y) * 2;
                    let word = u32::from(u16::from_be_bytes([body[i], body[i + 1]]));
                    let level = |shift: u32| widen_channel(word >> shift & 0x1f, 5);
                    let color = level(10) << 16 | level(5) << 8 | level(0);
                    image.set(x as u32, y as u32 * 2, color);
                    image.set(x as u32, y as u32 * 2 + 1, color);
                }
            }
            Some(image)
        }
        2 if body.len() == 256000 => {
            let mut image = Image::new(640, 400).ok()?;
            for x in 0..640 {
                for i in 0..400 {
                    // Even lines first, then odd lines, per column.
                    let y = if i < 200 { i * 2 } else { (i - 200) * 2 + 1 };
                    let grey = u32::from(body[x * 400 + i].min(191)) * 4 / 3;
                    image.set(x as u32, y as u32, grey * 0x010101);
                }
            }
            Some(image)
        }
        _ => None,
    }
}
