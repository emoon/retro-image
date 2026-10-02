//! Commodore 16/116/Plus4 (TED) Botticelli and Multi Botticelli pictures
//! (`.p4i`).
//!
//! Sources:
//! - GoDot Botticelli loader page, <https://www.godot64.de/german/l_botticelli.htm>,
//!   and <http://plus4world.powweb.com/software/Multi_Botticelli>: luminance
//!   (1024 bytes), colour (1024 bytes) and bitmap (8000 bytes); `MULT` and
//!   the nibble-swapped `$FF16`/`$FF15` colours at the end of the luminance
//!   block.
//! - Which nibbles colour which pixel values (set/`01`: high colour nibble
//!   with low luminance nibble; clear/`10`: low colour nibble with high
//!   luminance nibble; `00` = `$FF15`, `11` = `$FF16`): worked out from the
//!   sample files against `recoil2png` output.
//! - Palette: observed from `recoil2png` output, by rendering a synthetic
//!   picture with every hue/luminance pair.

use crate::{DecodeError, Image};

/// RGB of TED colour `luminance * 16 + hue` (hue 0 is black at any luminance).
const PALETTE: [u32; 128] = [
    0x030303, 0x2f2f2f, 0x681010, 0x004242, 0x58006d, 0x004e00, 0x191c94, 0x383800, 0x562000,
    0x4b2800, 0x164800, 0x69072f, 0x004626, 0x062a80, 0x2a149b, 0x0b4900, //
    0x030303, 0x3d3d3d, 0x751e20, 0x00504f, 0x6a1078, 0x045c00, 0x2a2aa3, 0x4c4700, 0x692f00,
    0x593800, 0x265600, 0x751541, 0x00583d, 0x153d8f, 0x3922ae, 0x195900, //
    0x030303, 0x424242, 0x7b2820, 0x025659, 0x6f1a82, 0x0a6509, 0x3034a7, 0x505100, 0x6e3600,
    0x654000, 0x2c5c00, 0x7d1e45, 0x016145, 0x1c4599, 0x422dad, 0x1d6200, //
    0x030303, 0x56555a, 0x903c3b, 0x176d72, 0x872d99, 0x1f7b15, 0x4649c1, 0x666300, 0x844c0d,
    0x735500, 0x407200, 0x91335e, 0x19745c, 0x3259ae, 0x593fc3, 0x327600, //
    0x030303, 0x847e85, 0xbb6768, 0x459696, 0xaf58c3, 0x4aa73e, 0x7373ec, 0x928d11, 0xaf7832,
    0xa18020, 0x6c9e12, 0xba5f89, 0x469f83, 0x6185dd, 0x846cef, 0x5da329, //
    0x030303, 0xb2acb3, 0xe99292, 0x6cc3c1, 0xd986f0, 0x79d176, 0x9da1ff, 0xbdbe40, 0xdca261,
    0xd1a94c, 0x93c83d, 0xe98ab1, 0x6fcdab, 0x8ab4ff, 0xb29aff, 0x88cb59, //
    0x030303, 0xcacaca, 0xffacac, 0x85d8e0, 0xf39cff, 0x92ea8a, 0xb7baff, 0xd6d35b, 0xf3be79,
    0xe6c565, 0xb0e057, 0xffa4cf, 0x89e5c8, 0xa4caff, 0xc8b8ff, 0xa2e57a, //
    0x030303, 0xf9f9f9, 0xfff6f2, 0xd1ffff, 0xffe9ff, 0xdbffd3, 0xf0ffff, 0xffffa3, 0xffffc1,
    0xffffb2, 0xfcffa2, 0xffeeff, 0xd1ffff, 0xebffff, 0xfff8ff, 0xedffbc,
];

/// RGB of a TED colour register value (bits 6-4 luminance, bits 3-0 hue).
fn rgb(color: u8) -> u32 {
    PALETTE[usize::from(color & 0x7f)]
}

const LEN: usize = 2 + 1024 + 1024 + 8000;

/// Botticelli (hires) or Multi Botticelli (`MULT` tag) picture.
pub(super) fn decode_p4i(data: &[u8]) -> Result<Image, DecodeError> {
    if data.len() != LEN {
        return Err(DecodeError::Unrecognized);
    }
    let luma = &data[2..1026];
    let chroma = &data[1026..2050];
    let bitmap = &data[2050..];
    let multi = &luma[0x3fa..0x3fe] == b"MULT";
    let swap = |b: u8| b.rotate_left(4);
    let (color0, color3) = (swap(luma[0x3ff]), swap(luma[0x3fe]));
    let mut image = Image::new(320, 200);
    for y in 0..200 {
        for x in 0..320 {
            let cell = y / 8 * 40 + x / 8;
            let byte = bitmap[cell * 8 + y % 8];
            let set = (luma[cell] << 4 & 0x70) | chroma[cell] >> 4;
            let clear = (luma[cell] & 0x70) | chroma[cell] & 15;
            let color = if multi {
                match byte >> (6 - (x & 6)) & 3 {
                    0 => color0,
                    1 => set,
                    2 => clear,
                    _ => color3,
                }
            } else if byte & (0x80 >> (x % 8)) != 0 {
                set
            } else {
                clear
            };
            image.set(x as u32, y as u32, rgb(color));
        }
    }
    Ok(image)
}
