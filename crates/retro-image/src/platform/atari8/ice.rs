//! Character-set interlace pictures (the ICE family): ICN, IMN, IPC, IP2,
//! IRG, IR2 and DIN.
//!
//! Sources:
//! - Just Solve "ICE (Atari)"
//!   (<http://fileformats.archiveteam.org/wiki/ICE_(Atari)>) and "Super IRG"
//!   (<http://fileformats.archiveteam.org/wiki/Super_IRG>) name the formats
//!   and their color counts; atari-owner.com "Atari Software Graphic Modes"
//!   (<https://atari-owner.com/club/articles/atari-software-graphic-modes.17/>)
//!   and the AtariAge "Super IRG modes" thread
//!   (<https://forums.atariage.com/topic/186653-super-irg-modes-using-graphics-1/>)
//!   describe the technique: a text screen whose character set changes
//!   every 3 text rows and every video frame.
//! - ANTIC modes 2 and 4 and the GTIA modes 9, 10 and 11: De Re Atari
//!   ch. 2 and App. E (<https://www.atariarchives.org/dere/chaptE.php>).
//! - The file layouts are reverse engineered from the corpus samples
//!   (`AMSTERDM.IMN`, `EFOREST.ICN`, `HEMAN.IPC`, `WATER.IP2`,
//!   `XPHOENIX.IRG`, `ODDIE.IR2`, `CASTLE.DIN`) and by black-box probing of
//!   `recoil2png` with hand-made files (one glyph byte or one header byte
//!   changed at a time, the output colors read back). Findings:
//!   - Every file is a header, then 16 character sets of 1024 bytes, then
//!     one 40x24 screen of character codes (two screens in IRG and IR2).
//!     The 24 text rows form 8 bands of 3 rows; band `b` shows character
//!     set `2b` in the first frame and `2b + 1` in the second. The screen
//!     holds 120 codes per band (the glyphs a band shows), so a picture is
//!     160x192 in the 4-pixel-wide glyphs of ANTIC mode 4 (drawn 320 wide).
//!   - The two frames are blended by averaging (rounded down), as for the
//!     other interlace formats.
//!   - The header starts with a version byte (1, or 3 for DIN) followed by
//!     color registers whose meaning depends on the format (see the
//!     decoders). A mode 4 frame takes 4 colors from the header plus the
//!     background; code bit 7 selects playfield 3 for pixel value 3. In
//!     DIN's hires frame bit 7 is ignored.
//!   - GTIA frames: mode 9 (IMN) shows the luminance given by the pixel
//!     ORed into the background; mode 11 (ICN) shows hue = pixel with the
//!     background's luminance, pixel 0 keeping only the background hue;
//!     mode 10 (IPC, IP2) shows the 9 colors of the header.
//!   - Color registers ignore luminance bit 0, also in the GTIA frames.

use super::gtia;
use super::palette::{average, register_rgb, rgb};
use crate::{DecodeError, Image};

const CHARSET: usize = 1024;
const CHARSETS: usize = 16;
const SCREEN: usize = 40 * 24;
/// Text rows that share a character set.
const BAND_ROWS: usize = 3;

/// How one frame turns the bytes of a glyph into 8 output pixels.
pub(super) enum Frame {
    /// ANTIC mode 4: 4 pixels of 2 bits, 2 output pixels wide. The colors
    /// are the background and playfield 0-3; playfield 3 shows for pixel
    /// value 3 when bit 7 of the character code is set.
    Antic4([u32; 5]),
    /// ANTIC mode 2: 8 pixels of 1 bit; the code's bit 7 is ignored.
    Hires { foreground: u32, background: u32 },
    /// GTIA modes 9-11: 2 pixels of 4 bits, 4 output pixels wide.
    Gtia([u32; 16]),
}

impl Frame {
    /// Mode 4 frame from color registers: background, playfield 0-3.
    pub(super) fn antic4(registers: [u8; 5]) -> Self {
        Self::Antic4(registers.map(register_rgb))
    }

    /// Hires frame: `background` register, text in its hue at `luminance`.
    fn hires(background: u8, luminance: u8) -> Self {
        Self::Hires {
            foreground: register_rgb(background & 0xf0 | luminance & 0x0f),
            background: register_rgb(background),
        }
    }

    /// GTIA mode 9: the pixel value is the luminance on the background hue.
    fn mode9(background: u8) -> Self {
        Self::Gtia(core::array::from_fn(|v| rgb(background & 0xfe | v as u8)))
    }

    /// Like [`Frame::pixels`] for the inverse-video variant (`inverse`):
    /// mode 4 selects playfield 3 as if bit 7 of the code were set, the
    /// other modes invert the glyph bits.
    fn pixels_inverse(&self, code: u8, bits: u8, inverse: bool) -> [u32; 8] {
        match (self, inverse) {
            (Self::Antic4(_), true) => self.pixels(code | 0x80, bits),
            (_, true) => self.pixels(code, !bits),
            _ => self.pixels(code, bits),
        }
    }

    /// The pixels of one glyph row `bits` for the character `code`.
    pub(super) fn pixels(&self, code: u8, bits: u8) -> [u32; 8] {
        let mut out = [0; 8];
        match self {
            Self::Antic4(colors) => {
                for (x, pair) in out.as_chunks_mut::<2>().0.iter_mut().enumerate() {
                    let value = bits >> (6 - 2 * x) & 3;
                    pair.fill(colors[gtia::antic4_register(value, code & 0x80 != 0)]);
                }
            }
            Self::Hires {
                foreground,
                background,
            } => {
                for (x, pixel) in out.iter_mut().enumerate() {
                    *pixel = if bits >> (7 - x) & 1 != 0 {
                        *foreground
                    } else {
                        *background
                    };
                }
            }
            Self::Gtia(colors) => {
                for (x, quad) in out.as_chunks_mut::<4>().0.iter_mut().enumerate() {
                    quad.fill(colors[usize::from(bits >> (4 - 4 * x) & 15)]);
                }
            }
        }
        out
    }

    /// Draws the whole screen with one character set per band.
    fn render(&self, charsets: &[&[u8]], screen: &[u8]) -> Result<Image, DecodeError> {
        let mut image = Image::new(320, 192)?;
        for (index, &code) in screen.iter().enumerate() {
            let (row, column) = (index / 40, index % 40);
            let glyph_start = usize::from(code & 0x7f) * 8;
            let glyph = &charsets[row / BAND_ROWS][glyph_start..glyph_start + 8];
            for (line, &bits) in glyph.iter().enumerate() {
                for (x, color) in self.pixels(code, bits).into_iter().enumerate() {
                    image.set((column * 8 + x) as u32, (row * 8 + line) as u32, color);
                }
            }
        }
        Ok(image)
    }
}

/// Splits `data` into the `header`, the 16 character sets (as two groups of
/// 8, one per frame) and the screens. `screens` is 1 or 2.
struct Parts<'a> {
    header: &'a [u8],
    sets: [[&'a [u8]; 8]; 2],
    screens: [&'a [u8]; 2],
}

impl<'a> Parts<'a> {
    /// Fails unless `data` is exactly the header (starting with `version`),
    /// the character sets and `screens` screens.
    fn new(
        data: &'a [u8],
        header_len: usize,
        version: u8,
        screens: usize,
    ) -> Result<Self, DecodeError> {
        if data.len() != header_len + CHARSETS * CHARSET + screens * SCREEN || data[0] != version {
            return Err(DecodeError::Unrecognized);
        }
        let (header, rest) = data.split_at(header_len);
        let (sets, rest) = rest.split_at(CHARSETS * CHARSET);
        let set = |frame: usize| {
            core::array::from_fn(|band| {
                let index = 2 * band + frame;
                &sets[index * CHARSET..(index + 1) * CHARSET]
            })
        };
        let screen = |frame: usize| &rest[frame.min(screens - 1) * SCREEN..][..SCREEN];
        Ok(Self {
            header,
            sets: [set(0), set(1)],
            screens: [screen(0), screen(1)],
        })
    }

    /// The blend of the two frames.
    fn render(&self, frames: [Frame; 2]) -> Result<Image, DecodeError> {
        let [first, second] = frames;
        Ok(Image::blend(&[
            &first.render(&self.sets[0], self.screens[0])?,
            &second.render(&self.sets[1], self.screens[1])?,
        ]))
    }
}

/// ICE MIN (`.IMN`): version 1, background, playfield 0-3. The first frame
/// is mode 4, the second GTIA mode 9 (luminance on the background's hue).
pub(super) fn decode_imn(data: &[u8]) -> Result<Image, DecodeError> {
    let parts = Parts::new(data, 6, 1, 1)?;
    let h = parts.header;
    parts.render([
        Frame::antic4([h[1], h[2], h[3], h[4], h[5]]),
        Frame::mode9(h[1]),
    ])
}

/// ICE CIN (`.ICN`): version 1, background, playfield 0-3. The first frame
/// is mode 4 on a black background, the second GTIA mode 11 (hue by pixel
/// value, luminance of the background; pixel 0 shows the background hue
/// only).
pub(super) fn decode_icn(data: &[u8]) -> Result<Image, DecodeError> {
    let parts = Parts::new(data, 6, 1, 1)?;
    let h = parts.header;
    parts.render([
        Frame::antic4([0, h[2], h[3], h[4], h[5]]),
        Frame::Gtia(core::array::from_fn(|v| {
            register_rgb(if v == 0 {
                h[1] & 0xf0
            } else {
                h[1] | (v as u8) << 4
            })
        })),
    ])
}

/// ICE PCIN (`.IPC`): version 1, then the registers COLPM0-3, COLPF0-3 and
/// COLBK. The first frame is mode 4 with COLPM0 as background, the second
/// GTIA mode 10.
pub(super) fn decode_ipc(data: &[u8]) -> Result<Image, DecodeError> {
    let parts = Parts::new(data, 10, 1, 1)?;
    let h = parts.header;
    parts.render(pcin_frames(
        [h[1], h[2], h[3], h[4]],
        [h[5], h[6], h[7], h[8]],
        [h[5], h[6], h[7], h[8]],
        h[9],
    ))
}

/// ICE PCIN+ (`.IP2`): like PCIN, but the playfield colors of the two
/// frames are stored interleaved (frame 1 playfield 0, frame 2 playfield 0,
/// ...) so they can differ; COLBK comes last.
pub(super) fn decode_ip2(data: &[u8]) -> Result<Image, DecodeError> {
    let parts = Parts::new(data, 14, 1, 1)?;
    let h = parts.header;
    parts.render(pcin_frames(
        [h[1], h[2], h[3], h[4]],
        [h[5], h[7], h[9], h[11]],
        [h[6], h[8], h[10], h[12]],
        h[13],
    ))
}

/// Mode 4 with playfield `first`, then mode 10 with `second`. In mode 10
/// the pixel values 0-3 show COLPM0-3, 4-7 and 12-15 playfield 0-3 and
/// 8-11 the background.
fn pcin_frames(player: [u8; 4], first: [u8; 4], second: [u8; 4], background: u8) -> [Frame; 2] {
    let registers = |v: usize| match v {
        0..=3 => player[v],
        4..=7 => second[v - 4],
        8..=11 => background,
        _ => second[v - 12],
    };
    [
        Frame::antic4([player[0], first[0], first[1], first[2], first[3]]),
        Frame::Gtia(core::array::from_fn(|v| register_rgb(registers(v)))),
    ]
}

/// Super IRG (`.IRG`): version 1, background, playfield 0-3. Both frames
/// are mode 4 with their own screen.
pub(super) fn decode_irg(data: &[u8]) -> Result<Image, DecodeError> {
    let parts = Parts::new(data, 6, 1, 2)?;
    let h = parts.header;
    parts.render([
        Frame::antic4([h[1], h[2], h[3], h[4], h[5]]),
        Frame::antic4([h[1], h[2], h[3], h[4], h[5]]),
    ])
}

/// Super IRG 2 (`.IR2`): like IRG, but the playfield colors of the two
/// frames are stored interleaved (frame 1 playfield 0, frame 2 playfield 0,
/// ...).
pub(super) fn decode_ir2(data: &[u8]) -> Result<Image, DecodeError> {
    let parts = Parts::new(data, 10, 1, 2)?;
    let h = parts.header;
    parts.render([
        Frame::antic4([h[1], h[2], h[4], h[6], h[8]]),
        Frame::antic4([h[1], h[3], h[5], h[7], h[9]]),
    ])
}

/// DIN: version 3, background, the text luminance of the first (mode 2)
/// frame, then playfield 0-3 of the second (mode 4) frame.
pub(super) fn decode_din(data: &[u8]) -> Result<Image, DecodeError> {
    let parts = Parts::new(data, 7, 3, 1)?;
    let h = parts.header;
    parts.render(din_frames(h))
}

/// The hires frame (text on the background's hue) and the mode 4 frame of
/// a DIN header.
pub(super) fn din_frames(header: &[u8]) -> [Frame; 2] {
    let h = header;
    [
        Frame::hires(h[1], h[2]),
        Frame::antic4([h[1], h[3], h[4], h[5], h[6]]),
    ]
}

/// Builds the two frames from a header.
type FrameMaker = fn(&[u8]) -> [Frame; 2];

/// ICE character-set files (`.ICE`): a mode byte, color registers (the
/// header length depends on the mode), then the two character sets of the
/// mode's two frames. The first byte picks how the sets are drawn:
/// 0 two hires frames (header: luminance 1, luminance 2, hue/background 1
/// and 2), 1 Super IRG, 3 DIN and 12 ICE MIN's hires and mode 9 frames.
/// The sheet shows the 128 characters 32 to a row in ATASCII order, in four
/// 32-pixel blocks: the normal frames, then variants with the first and/or
/// second frame in inverse video (see `VARIANTS`). Other modes (the
/// single-set Graphics 9/11/APAC, HIP and IRG 2.0 fonts) are not decoded.
pub(super) fn decode_ice(data: &[u8]) -> Result<Image, DecodeError> {
    let (header_len, frames): (usize, FrameMaker) = match data.first() {
        Some(0) => (5, |h| [Frame::hires(h[3], h[1]), Frame::hires(h[4], h[2])]),
        Some(1) => (6, |h| {
            [
                Frame::antic4([h[1], h[2], h[3], h[4], h[5]]),
                Frame::antic4([h[1], h[2], h[3], h[4], h[5]]),
            ]
        }),
        Some(3) => (7, din_frames),
        Some(12) => (3, |h| [Frame::hires(h[1], h[2]), Frame::mode9(h[1])]),
        _ => return Err(DecodeError::Unrecognized),
    };
    if data.len() != header_len + 2 * CHARSET {
        return Err(DecodeError::Unrecognized);
    }
    let (header, sets) = data.split_at(header_len);
    let frames = frames(header);
    let mut image = Image::new(32 * 8, 4 * 32)?;
    for (block, (first, second)) in VARIANTS.into_iter().enumerate() {
        for (row, first_code) in ROW_CODES.into_iter().enumerate() {
            for column in 0..32 {
                let code = first_code + column as u8;
                let start = usize::from(code) * 8;
                for line in 0..8 {
                    let bits = |frame: usize| sets[frame * CHARSET + start + line];
                    let one = frames[0].pixels_inverse(code, bits(0), first);
                    let two = frames[1].pixels_inverse(code, bits(1), second);
                    for x in 0..8 {
                        image.set(
                            (column * 8 + x) as u32,
                            (block * 32 + row * 8 + line) as u32,
                            average([one[x], two[x]]),
                        );
                    }
                }
            }
        }
    }
    Ok(image)
}

/// Whether the (first, second) frame is drawn inverse in each block.
const VARIANTS: [(bool, bool); 4] = [(false, false), (true, true), (false, true), (true, false)];
/// First screen code of each sheet row: ATASCII order (control characters
/// are screen codes 64-95).
const ROW_CODES: [u8; 4] = [64, 0, 32, 96];

#[cfg(test)]
mod tests {
    use super::*;

    /// A file of `header_len` header bytes, zeroed sets and `screens` screens.
    fn blank(header: &[u8], screens: usize) -> alloc::vec::Vec<u8> {
        let mut data = header.to_vec();
        data.resize(header.len() + CHARSETS * CHARSET + screens * SCREEN, 0);
        data
    }

    #[test]
    fn bands_use_their_own_charset_pair() {
        let mut data = blank(&[1, 0, 0x74, 0x30, 0x66, 0x18], 2);
        // Band 1 (rows 3-5), frame 1 is set 2: character 1, line 0 = all 3s.
        data[6 + 2 * CHARSET + 8] = 0xff;
        let screens = 6 + CHARSETS * CHARSET;
        data[screens + 3 * 40] = 1;
        let image = decode_irg(&data).unwrap();
        // Averaged with the black second frame.
        let expected = register_rgb(0x66).to_be_bytes();
        let line = 24 * 320 * 3;
        assert_eq!(
            image.rgb()[line..line + 3],
            [expected[1] / 2, expected[2] / 2, expected[3] / 2]
        );
        assert_eq!(image.rgb()[..3], [0, 0, 0]);
    }

    #[test]
    fn code_bit_7_selects_playfield_3() {
        let frame = Frame::antic4([0, 0x0e, 0x0e, 0x0e, 0x00]);
        assert_eq!(frame.pixels(0x01, 0xc0)[0], register_rgb(0x0e));
        assert_eq!(frame.pixels(0x81, 0xc0)[0], register_rgb(0x00));
    }

    #[test]
    fn hires_ignores_code_bit_7() {
        let frame = Frame::Hires {
            foreground: 1,
            background: 2,
        };
        assert_eq!(frame.pixels(0x80, 0x80), [1, 2, 2, 2, 2, 2, 2, 2]);
    }

    #[test]
    fn ice_sheet_orders_rows_and_inverts_variants() {
        // Mode 3 (DIN), hires set: character 64 line 0 all set.
        let mut data = alloc::vec![3, 0x0a, 0x00, 0, 0, 0, 0];
        data.resize(7 + 2 * CHARSET, 0);
        data[7 + 64 * 8] = 0xff;
        let image = decode_ice(&data).unwrap();
        assert_eq!((image.width(), image.height()), (256, 128));
        // Character 64 is the first of the sheet; blocks 0 and 1 are
        // mirrored (inverse hires), averaged with the black second frame.
        assert_eq!(image.get(0, 0), 0x555555);
        assert_eq!(image.get(0, 32), 0xaaaaaa);
        assert!(decode_ice(&data[..data.len() - 1]).is_err());
        data[0] = 2;
        assert!(decode_ice(&data).is_err());
    }

    #[test]
    fn rejects_wrong_sizes_and_versions() {
        let data = blank(&[1, 0, 0, 0, 0, 0], 1);
        assert!(decode_imn(&data).is_ok());
        assert!(decode_irg(&data).is_err());
        assert!(decode_imn(&data[1..]).is_err());
        let mut wrong = data.clone();
        wrong[0] = 2;
        assert!(decode_imn(&wrong).is_err());
        assert!(decode_imn(&[]).is_err());
    }
}
