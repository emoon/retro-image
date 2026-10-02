//! TMS9918 / V9938 / V9958 video model shared by the MSX decoders: a VRAM
//! snapshot, palettes, colour encodings and sprite rendering.
//!
//! Sources:
//! - VRAM tables, palette register format (`0RRR0BBB`, `00000GGG`) and the
//!   Graphic 7 `GGGRRRBB` pixel format: MSX2 Technical Handbook, chapter 4 and
//!   appendix 5 (<https://konamiman.github.io/MSX2-Technical-Handbook/>).
//! - V9938 power-on palette and Graphic 7 sprite colours: V9938 Technical Data
//!   Book (<https://map.grauw.nl/resources/video/yamaha_v9938.pdf>).
//! - YJK conversion: grauw, "The YJK screen modes"
//!   (<https://map.grauw.nl/articles/yjk/>).
//! - Sprite attribute layout, early clock, end markers (208 / 216), the
//!   per-line sprite limits and the CC (colour combine) bit: TMS9918 data sheet
//!   (<https://map.grauw.nl/resources/video/texasinstruments_tms9918.pdf>)
//!   and the V9938 data book above.
//! - Observed from `recoil2png` output: the RGB values of the MSX1 (TMS9918)
//!   palette, 3-bit to 8-bit scaling by bit replication, 2-bit Graphic 7 blue
//!   levels (0, 0x49, 0x92, 0xff), the default Graphic 5 palette, sprites always
//!   being 16x16 unmagnified, colour-0 sprites being opaque in sprite mode 2, and
//!   a CC sprite being shown only after a non-CC sprite on the same line.

use alloc::vec;
use alloc::vec::Vec;

/// 64 KiB of VRAM plus how many bytes, from address 0, were actually loaded.
pub(super) struct Vram {
    data: Vec<u8>,
    loaded: usize,
}

impl Vram {
    pub(super) const SIZE: usize = 0x10000;

    /// VRAM with `bytes` loaded at address 0 (truncated to 64 KiB).
    pub(super) fn new(bytes: &[u8]) -> Self {
        let mut data = vec![0; Self::SIZE];
        let loaded = bytes.len().min(Self::SIZE);
        data[..loaded].copy_from_slice(&bytes[..loaded]);
        Self { data, loaded }
    }

    pub(super) fn loaded(&self) -> usize {
        self.loaded
    }

    /// All 64 KiB.
    pub(super) fn bytes(&self) -> &[u8] {
        &self.data
    }

    /// Byte at `address`, wrapping inside 64 KiB.
    pub(super) fn get(&self, address: usize) -> u8 {
        self.data[address & (Self::SIZE - 1)]
    }

    pub(super) fn set(&mut self, address: usize, value: u8) {
        self.data[address & (Self::SIZE - 1)] = value;
    }

    /// Whether the 16-entry palette table at `address` is set: not all zero
    /// and every entry in the `0RRR0BBB 00000GGG` format.
    pub(super) fn is_valid_palette(&self, address: usize) -> bool {
        let table = (0..16).map(|i| (self.get(address + 2 * i), self.get(address + 2 * i + 1)));
        let mut set = false;
        for (rb, g) in table {
            if rb & 0x88 != 0 || g & 0xf8 != 0 {
                return false;
            }
            set |= rb != 0 || g != 0;
        }
        set
    }

    /// The `count`-entry palette table at `address`, or `None` if the dump
    /// stops before its end.
    pub(super) fn palette(&self, address: usize, count: usize) -> Option<Palette> {
        if address + count * 2 > self.loaded {
            return None;
        }
        let mut palette = [0; 16];
        for (i, entry) in palette.iter_mut().enumerate().take(count) {
            *entry = palette_entry(self.get(address + 2 * i), self.get(address + 2 * i + 1));
        }
        Some(palette)
    }
}

/// 16 colours as `0xRRGGBB`.
pub(in crate::platform) type Palette = [u32; 16];

/// Scales a 3-bit level to 8 bits.
pub(super) const fn level3(v: u8) -> u32 {
    let v = (v & 7) as u32;
    (v << 5 | v << 2 | v >> 1) & 0xff
}

/// Scales a 5-bit level to 8 bits.
pub(in crate::platform) const fn level5(v: u8) -> u32 {
    let v = (v & 31) as u32;
    v << 3 | v >> 2
}

pub(super) const fn rgb3(r: u8, g: u8, b: u8) -> u32 {
    level3(r) << 16 | level3(g) << 8 | level3(b)
}

/// One V9938 palette register: `0RRR0BBB`, `00000GGG`.
pub(super) const fn palette_entry(rb: u8, g: u8) -> u32 {
    rgb3(rb >> 4, g, rb)
}

const fn palette3(levels: [(u8, u8, u8); 16]) -> Palette {
    let mut palette = [0; 16];
    let mut i = 0;
    while i < 16 {
        let (r, g, b) = levels[i];
        palette[i] = rgb3(r, g, b);
        i += 1;
    }
    palette
}

/// V9938 power-on palette.
pub(super) const MSX2_PALETTE: Palette = palette3([
    (0, 0, 0),
    (0, 0, 0),
    (1, 6, 1),
    (3, 7, 3),
    (1, 1, 7),
    (2, 3, 7),
    (5, 1, 1),
    (2, 6, 7),
    (7, 1, 1),
    (7, 3, 3),
    (6, 6, 1),
    (6, 6, 4),
    (1, 4, 1),
    (6, 2, 5),
    (5, 5, 5),
    (7, 7, 7),
]);

/// Palette used for Graphic 5 (Screen 6) dumps without one.
pub(super) const GRAPHIC5_PALETTE: Palette = {
    let mut palette = [0; 16];
    palette[1] = rgb3(1, 4, 1);
    palette[2] = rgb3(1, 6, 1);
    palette[3] = rgb3(3, 7, 3);
    palette
};

/// TMS9918 colours for MSX1 dumps without a palette.
pub(super) const MSX1_PALETTE: Palette = [
    0x000800, 0x000400, 0x3abb43, 0x70d377, 0x5459d7, 0x7b7be8, 0xb3634b, 0x61dfe7, 0xd46a53,
    0xf88e77, 0xc7c759, 0xd9d481, 0x36a53b, 0xb06bae, 0xc7d0c5, 0xfafff8,
];

/// Fixed sprite colours in Graphic 7 (Screen 8).
pub(super) const GRAPHIC7_SPRITE_PALETTE: Palette = palette3([
    (0, 0, 0),
    (0, 0, 2),
    (3, 0, 0),
    (3, 0, 2),
    (0, 3, 0),
    (0, 3, 2),
    (3, 3, 0),
    (3, 3, 2),
    (7, 4, 2),
    (0, 0, 7),
    (7, 0, 0),
    (7, 0, 7),
    (0, 7, 0),
    (0, 7, 7),
    (7, 7, 0),
    (7, 7, 7),
]);

/// Graphic 7 pixel `GGGRRRBB`.
pub(super) const fn graphic7(b: u8) -> u32 {
    const BLUE: [u32; 4] = [0, 0x49, 0x92, 0xff];
    level3(b >> 2) << 16 | level3(b >> 5) << 8 | BLUE[(b & 3) as usize]
}

/// Sign-extends a 6-bit two's complement value.
const fn signed6(v: u8) -> i32 {
    ((v as i32) << 26) >> 26
}

/// Converts four YJK bytes (Y in bits 7-3, K low/high and J low/high in bits
/// 2-0) to RGB. With `yae`, bytes with bit 3 set are palette indices in bits 7-4.
pub(in crate::platform) fn yjk_group(bytes: [u8; 4], yae: bool, palette: &Palette) -> [u32; 4] {
    let k = signed6((bytes[0] & 7) | (bytes[1] & 7) << 3);
    let j = signed6((bytes[2] & 7) | (bytes[3] & 7) << 3);
    bytes.map(|b| {
        if yae && b & 8 != 0 {
            palette[(b >> 4) as usize]
        } else {
            yjk_rgb((b >> 3) as i32, j, k)
        }
    })
}

fn yjk_rgb(y: i32, j: i32, k: i32) -> u32 {
    let clamp = |v: i32| v.clamp(0, 31) as u8;
    let r = clamp(y + j);
    let g = clamp(y + k);
    let b = clamp((5 * y - 2 * j - k + 2).div_euclid(4));
    level5(r) << 16 | level5(g) << 8 | level5(b)
}

/// Where the sprite tables live and which sprite mode is used.
#[derive(Clone, Copy)]
pub(super) struct SpriteTables {
    pub attributes: usize,
    pub patterns: usize,
    /// Sprite mode 2 (V9938 modes from Screen 4 up): per-line colour table.
    pub colours: Option<usize>,
}

/// Colour indices of the sprite pixels on screen line `y`, 256 wide.
pub(super) fn sprite_line(vram: &Vram, tables: SpriteTables, y: i32) -> [Option<u8>; 256] {
    let mut line = [None; 256];
    let mode2 = tables.colours.is_some();
    let (end_marker, limit) = if mode2 { (216, 8) } else { (208, 4) };
    let mut count = 0;
    let mut seen_plain = false;
    for sprite in 0..32 {
        let attribute = tables.attributes + 4 * sprite;
        let sprite_y = vram.get(attribute);
        if sprite_y == end_marker {
            break;
        }
        let mut top = (sprite_y as i32 + 1) & 0xff;
        if top > 0xe0 {
            top -= 256;
        }
        let row = y - top;
        if !(0..16).contains(&row) {
            continue;
        }
        count += 1;
        if count > limit {
            break;
        }
        let row = row as usize;
        let colour = match tables.colours {
            Some(colours) => vram.get(colours + 16 * sprite + row),
            None => vram.get(attribute + 3),
        };
        let combine = mode2 && colour & 0x40 != 0;
        if combine && !seen_plain {
            continue;
        }
        seen_plain |= !combine;
        let mut left = vram.get(attribute + 1) as i32;
        if colour & 0x80 != 0 {
            left -= 32;
        }
        let index = colour & 15;
        let pattern = tables.patterns + (vram.get(attribute + 2) & 0xfc) as usize * 8;
        for column in 0..16 {
            let x = left + column;
            if !(0..256).contains(&x) {
                continue;
            }
            let bits = vram.get(pattern + (column as usize / 8) * 16 + row);
            if bits & (0x80 >> (column & 7)) == 0 {
                continue;
            }
            let pixel = &mut line[x as usize];
            match (*pixel, combine) {
                (Some(old), true) => *pixel = Some(old | index),
                (None, _) => *pixel = Some(index),
                (Some(_), false) => {}
            }
        }
    }
    if !mode2 {
        // Colour 0 is transparent in sprite mode 1.
        for pixel in &mut line {
            if *pixel == Some(0) {
                *pixel = None;
            }
        }
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_replicate_bits() {
        assert_eq!(level3(1), 0x24);
        assert_eq!(level3(6), 0xdb);
        assert_eq!(level3(7), 0xff);
        assert_eq!(level5(31), 0xff);
        assert_eq!(level5(16), 0x84);
    }

    #[test]
    fn graphic7_uses_ggg_rrr_bb() {
        assert_eq!(graphic7(0xff), 0xffffff);
        assert_eq!(graphic7(0b1110_0000), 0x00ff00);
        assert_eq!(graphic7(0b0001_1100), 0xff0000);
        assert_eq!(graphic7(0b0000_0010), 0x000092);
    }

    #[test]
    fn yjk_grey_and_yae_palette() {
        let grey = yjk_group([0xf8; 4], false, &MSX2_PALETTE);
        assert_eq!(grey, [0xffffff; 4]);
        let yae = yjk_group([0xf8 | 8, 0, 0, 0], true, &MSX2_PALETTE);
        assert_eq!(yae[0], MSX2_PALETTE[15]);
    }

    #[test]
    fn mode1_sprite_draws_pattern() {
        let mut vram = Vram::new(&[]);
        vram.set(0x1b00, 9); // y + 1 = 10
        vram.set(0x1b01, 20);
        vram.set(0x1b03, 5);
        vram.set(0x1b04, 208);
        vram.set(0x3800, 0x80);
        let tables = SpriteTables {
            attributes: 0x1b00,
            patterns: 0x3800,
            colours: None,
        };
        let line = sprite_line(&vram, tables, 10);
        assert_eq!(line[20], Some(5));
        assert_eq!(line[21], None);
        assert_eq!(sprite_line(&vram, tables, 9)[20], None);
    }
}
