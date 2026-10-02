//! The 256-colour GTIA palette (PAL).
//!
//! Source: reverse engineered by black-box probing of `recoil2png` (its
//! default PAL palette); no palette document or table was copied. The
//! hue/luminance model (high nibble hue, low nibble luminance) is from De Re
//! Atari ch. 3 (<https://www.atariarchives.org/dere/chapt03.php>). Synthetic
//! MIC files with every colour-register value were rendered and the even
//! luminances read back. Every channel fits `clamp(base[hue] + 0x11 * luminance)`
//! exactly, so the table below stores only the 16 per-hue bases. Odd luminances
//! (used by GTIA mode 9) follow the same formula; for hue 0 they were confirmed
//! with a 16-grey GR9 file (`0x111111 * luminance`).

/// Per-hue RGB value at luminance 0 before clamping.
const HUE_BASE: [[i16; 3]; 16] = [
    [0, 0, 0],
    [63, -12, -101],
    [80, -31, -48],
    [84, -43, 3],
    [79, -50, 53],
    [61, -51, 104],
    [32, -43, 139],
    [-35, -9, 137],
    [-63, 12, 101],
    [-80, 31, 48],
    [-84, 43, -3],
    [-61, 51, -104],
    [-32, 43, -139],
    [1, 28, -150],
    [35, 9, -137],
    [63, -12, -101],
];

/// `0xRRGGBB` of an Atari colour value: hue in the high nibble, luminance in the low.
pub(super) fn rgb(color: u8) -> u32 {
    let base = HUE_BASE[usize::from(color >> 4)];
    let step = 0x11 * i16::from(color & 0x0f);
    base.iter().fold(0, |acc, &channel| {
        (acc << 8) | (channel + step).clamp(0, 0xff) as u32
    })
}

/// Colour of a register outside GTIA mode 9: luminance bit 0 is ignored.
pub(super) fn register_rgb(color: u8) -> u32 {
    rgb(color & 0xfe)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greys_step_by_0x11() {
        assert_eq!(rgb(0x00), 0x000000);
        assert_eq!(rgb(0x01), 0x111111);
        assert_eq!(rgb(0x0f), 0xffffff);
    }

    #[test]
    fn matches_observed_colours() {
        assert_eq!(rgb(0x10), 0x3f0000);
        assert_eq!(rgb(0x26), 0xb64736);
        assert_eq!(rgb(0x78), 0x657fff);
        assert_eq!(rgb(0xb4), 0x077700);
        assert_eq!(rgb(0xfe), 0xffe289);
    }

    #[test]
    fn registers_ignore_luminance_bit_0() {
        assert_eq!(register_rgb(0x11), rgb(0x10));
    }
}
