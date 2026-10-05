//! Palette precision of Japanese computers, used by the MAG, MKI and Pi
//! decoders to turn 8-bit palette components into the displayed colors.
//!
//! Sources: the significant-bit conventions are described in Kirinn Bunnylin's
//! Maki-chan and Pi pages (<https://mooncore.eu/bunny/txt/makichan.htm>); the
//! exact rounding (bit replication, R5 G6 B5, the X68000 intensity bit) was
//! observed from `recoil2png` output.

/// How 8-bit components are reduced to the machine's precision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Precision {
    /// Top `n` bits of each component, replicated.
    Bits(u32),
    /// 5 bits red and blue, 6 bits green.
    Rgb565,
    /// X68000 `GGGGGRRRRRBBBBBI`: 5 bits per component plus an intensity bit
    /// taken from bit 2 of green.
    X68000,
}

/// Repeats the top `bits` bits of `v` to fill a byte.
fn scale(v: u8, bits: u32) -> u32 {
    let top = (v >> (8 - bits)) as u32;
    let mut out = 0;
    let mut filled = 0;
    while filled < 8 {
        out = out << bits | top;
        filled += bits;
    }
    (out >> (filled - 8)) & 0xff
}

impl Precision {
    /// `0xRRGGBB` for 8-bit components.
    pub(super) fn rgb(self, r: u8, g: u8, b: u8) -> u32 {
        let (r, g, b) = match self {
            Self::Bits(n) => (scale(r, n), scale(g, n), scale(b, n)),
            Self::Rgb565 => (scale(r, 5), scale(g, 6), scale(b, 5)),
            Self::X68000 => {
                let intensity = (g >> 2) & 1;
                let six = |c: u8| scale((c & 0xf8) | intensity << 2, 6);
                (six(r), six(g), six(b))
            }
        };
        r << 16 | g << 8 | b
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaling() {
        assert_eq!(Precision::Bits(3).rgb(0x37, 0x37, 0x37), 0x242424);
        assert_eq!(Precision::Bits(4).rgb(0x4f, 0x4f, 0x4f), 0x444444);
        assert_eq!(Precision::Bits(8).rgb(0x13, 0x37, 0x4f), 0x13374f);
        assert_eq!(Precision::Rgb565.rgb(0x37, 0x37, 0x37), 0x313431);
        assert_eq!(Precision::X68000.rgb(0xf8, 0x04, 0x00), 0xff0404);
        assert_eq!(Precision::X68000.rgb(0xff, 0x00, 0x7f), 0xfb0079);
    }
}
