//! Unix and workstation raster formats.
//!
//! These are general-purpose formats, like GIF and BMP on the PC, so no
//! machine of the RECOIL list owns them; the platform is named `Unix`.
//! Survey: `docs/research/gaps-computers-extra.md` (candidate C4).
//!
//! Each submodule (`pnm.rs`, `sun.rs`, `sgi.rs`, `xbm.rs`, `xpm.rs`, `xwd.rs`,
//! `farbfeld.rs`, `utah_rle.rs`) cites its own sources; `c_source.rs` is the
//! tokenizer that XBM and XPM share. All of them check a signature.
//!
//! Extensions shared with other platforms are tried in registry order. For
//! `.rgb` that is Atari 8-bit ColorViewSquash (magic `RGB1`), Atari ST RGB
//! Intermediate (any file of exactly 96102 bytes), SGI, then ZX Spectrum
//! Tricolor (an exact size); for `.rle` the TRS-80 CompuServe RLE (magic
//! `ESC G`) comes before Utah RLE. So an SGI file that is exactly 96102 bytes
//! long, such as a 1-channel picture of 95590 pixels, is taken for an RGB
//! Intermediate picture.
//!
//! The helpers below hold the two pieces of arithmetic every format of the
//! family needs: sample widths above 8 bits and an alpha channel, neither of
//! which `Image` represents.

mod c_source;
mod farbfeld;
mod pnm;
mod sgi;
mod sun;
mod utah_rle;
mod xbm;
mod xpm;
mod xwd;

use crate::Format;

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
    Format::new("Unix", "X pixmap", &["xpm"], xpm::decode_xpm).signature(),
    Format::new("Unix", "X window dump", &["xwd"], xwd::decode_xwd).signature(),
    Format::new("Unix", "farbfeld", &["ff"], farbfeld::decode_farbfeld).signature(),
    Format::new("Unix", "Utah RLE", &["rle"], utah_rle::decode_utah_rle).signature(),
];

/// Scales a sample of `0..=max` to `0..=255`, rounding to nearest. Values
/// above `max` count as `max`.
///
/// Panics if `max` is 0 (a division by zero); every caller has a maximum of
/// at least 1 from its header or its mask.
pub(super) fn to_byte(value: u32, max: u32) -> u8 {
    let value = u64::from(value.min(max));
    ((value * 255 + u64::from(max) / 2) / u64::from(max)) as u8
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
}
