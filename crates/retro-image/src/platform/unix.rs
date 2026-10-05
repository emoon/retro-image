//! Unix and workstation raster formats.
//!
//! Each format cites its own sources in its submodule: `unix/pnm.rs`, `unix/sun.rs`,
//! `unix/sgi.rs`, `unix/xbm.rs`.
//! Survey: `docs/research/gaps-computers-extra.md` (candidate C4). These are
//! general-purpose formats, like GIF and BMP on the PC, so no machine of the
//! RECOIL list owns them; the platform is named `Unix`.
//!
//! The helpers below hold the two pieces of arithmetic every format of the
//! family needs: sample widths above 8 bits and an alpha channel, neither of
//! which `Image` represents.

mod c_source;
mod pnm;
mod sgi;
mod sun;
mod xbm;

use crate::Format;
use crate::image::TRANSPARENT_FILL;

pub(super) static FORMATS: &[Format] = &[
    Format::new(
        "Unix",
        "Netpbm PBM, PGM, PPM and PAM",
        &["pbm", "pgm", "ppm", "pnm", "pam"],
        pnm::decode_pnm,
    )
    .signature(),
    Format::new(
        "Unix",
        "Sun raster",
        &["ras", "sun", "im1", "im8", "im24", "im32"],
        sun::decode_sun,
    )
    .signature(),
    Format::new(
        "Unix",
        "SGI image",
        &["sgi", "rgb", "rgba", "bw"],
        sgi::decode_sgi,
    )
    .signature(),
    Format::new("Unix", "X bitmap", &["xbm"], xbm::decode_xbm).signature(),
];

/// Scales a sample of `0..=max` to `0..=255`, rounding to nearest. Values
/// above `max` count as `max`.
pub(super) fn to_byte(value: u32, max: u32) -> u8 {
    let value = u64::from(value.min(max));
    ((value * 255 + u64::from(max) / 2) / u64::from(max)) as u8
}

/// A color (`0xRRGGBB`) at opacity `alpha` (0 to 255) over
/// [`TRANSPARENT_FILL`], rounding to nearest.
pub(super) fn over_fill(color: u32, alpha: u8) -> u32 {
    let alpha = u32::from(alpha);
    let channel = |shift: u32| {
        let front = color >> shift & 0xff;
        let back = TRANSPARENT_FILL >> shift & 0xff;
        (front * alpha + back * (255 - alpha) + 127) / 255
    };
    channel(16) << 16 | channel(8) << 8 | channel(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_byte_rounds_and_clamps() {
        assert_eq!(to_byte(0, 1), 0);
        assert_eq!(to_byte(1, 1), 255);
        assert_eq!(to_byte(5, 15), 85);
        assert_eq!(to_byte(0x8000, 0xffff), 128);
        assert_eq!(to_byte(300, 255), 255);
        assert_eq!(to_byte(200, 255), 200);
    }

    #[test]
    fn over_fill_blends_towards_the_fill() {
        assert_eq!(over_fill(0x123456, 255), 0x123456);
        assert_eq!(over_fill(0x123456, 0), TRANSPARENT_FILL);
        assert_eq!(over_fill(0xffffff, 128) >> 16 & 0xff, 0xe0);
    }
}
