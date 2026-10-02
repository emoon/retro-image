//! TIP (Taquart Interlace Picture): HIP's GTIA mode 9 and 10 frames for
//! luminance, each scanline pair completed by a GTIA mode 11 hue line.
//!
//! Sources:
//! - Just Solve "Taquart Interlace Picture"
//!   (<http://fileformats.archiveteam.org/wiki/Taquart_Interlace_Picture>;
//!   `TIP`, version byte 1, up to 160x119, 2 frames, GR9/10/11); Mad Team
//!   TIP/HIP article, the mode idea
//!   (<https://madteam.atari8.info/index.php?atarynka=tip>).
//! - Observed from `recoil2png` output: the header (`TIP`, 1, 0, width in
//!   half-pixels, height, frame length) followed by the mode 9, mode 10 and
//!   mode 11 frames; output of 2 x width by 2 x height; each line shown as a
//!   hue scanline (hue, with the average luminance of this and the previous
//!   line) and then a luminance scanline (hue and this line's luminance);
//!   mode 10 values 0-7 meaning luminance 0-14 (values 8-11 = 0, 12-15 as
//!   4-7); mode 9 and hue pixels drawn 1 output pixel left of the grid,
//!   mode 10 pixels 1 to the right; and the two frames mixed by averaging.

use super::palette::rgb;
use crate::{DecodeError, Image};

pub(super) fn decode_tip(data: &[u8]) -> Result<Image, DecodeError> {
    let header = data.strip_prefix(b"TIP\x01\x00");
    let Some(&[width, height, len_low, len_high, ref frames @ ..]) = header else {
        return Err(DecodeError::Unrecognized);
    };
    let (width, height) = (usize::from(width), usize::from(height));
    let line_len = width / 4;
    let frame_len = usize::from(u16::from_le_bytes([len_low, len_high]));
    if width == 0
        || width > 160
        || !width.is_multiple_of(4)
        || !(1..=119).contains(&height)
        || frame_len != line_len * height
        || frames.len() != 3 * frame_len
    {
        return Err(DecodeError::Unrecognized);
    }
    let (gtia9, rest) = frames.split_at(frame_len);
    let (gtia10, gtia11) = rest.split_at(frame_len);
    let out_width = 2 * width;
    // Pixel of `frame` covering output pixel `x` on `line`, if any.
    let pixel = |frame: &[u8], line: usize, x: Option<usize>| {
        let x = x.filter(|&x| x < out_width)? / 4;
        let byte = frame[line * line_len + x / 2];
        Some(if x.is_multiple_of(2) {
            byte >> 4
        } else {
            byte & 0x0f
        })
    };
    // Frame 0 takes its luminance from mode 9, frame 1 from mode 10.
    let luminance = |frame: usize, line: usize, x: usize| match frame {
        0 => pixel(gtia9, line, Some(x + 1)).unwrap_or(0),
        _ => pixel(gtia10, line, x.checked_sub(1)).map_or(0, gtia10_luminance),
    };
    let frames: [Image; 2] = core::array::from_fn(|frame| {
        let mut image = Image::new(out_width as u32, 2 * height as u32);
        for line in 0..height {
            for x in 0..out_width {
                let hue = pixel(gtia11, line, Some(x + 1));
                let now = luminance(frame, line, x);
                let before = line.checked_sub(1).map_or(0, |l| luminance(frame, l, x));
                let hue_scanline = hue.map_or(0, |hue| rgb((hue << 4) | ((now + before) / 2)));
                let luminance_scanline = rgb((hue.unwrap_or(0) << 4) | now);
                let y = 2 * line as u32;
                image.set(x as u32, y, hue_scanline);
                image.set(x as u32, y + 1, luminance_scanline);
            }
        }
        image
    });
    Ok(Image::blend(&[&frames[0], &frames[1]]))
}

/// Luminance of a GTIA mode 10 value with TIP's registers 0, 2, ..., 14, 0.
fn gtia10_luminance(value: u8) -> u8 {
    match value {
        0..=7 => 2 * value,
        8..=11 => 0,
        _ => 2 * (value - 8),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    fn tip(gtia9: u8, gtia10: u8, gtia11: u8) -> Vec<u8> {
        let mut data = Vec::from(*b"TIP\x01\x00\xa0\x01\x28\x00");
        for value in [gtia9, gtia10, gtia11] {
            data.extend_from_slice(&[value; 40]);
        }
        data
    }

    #[test]
    fn mixes_scanline_pairs() {
        let image = decode_tip(&tip(0x99, 0x44, 0xff)).unwrap();
        // Hue scanline: luminance (9 + 0) / 2 and (8 + 0) / 2 of hue 15.
        assert_eq!(image.get(2, 0), 0x833800);
        // Luminance scanline: hue 15 at luminances 9 and 8.
        assert_eq!(image.get(2, 1), 0xcf842b);
        // The rightmost pixel has no hue pixel on its hue scanline.
        assert_eq!(image.get(319, 0), 0);
    }

    #[test]
    fn detected_by_content() {
        let data = tip(0x99, 0x44, 0xff);
        assert_eq!(crate::decode("x.dat", &data), decode_tip(&data));
    }

    #[test]
    fn rejects_bad_headers() {
        let data = tip(0, 0, 0);
        assert!(decode_tip(&data[..data.len() - 1]).is_err());
        let mut wide = data.clone();
        wide[5] = 164;
        assert!(decode_tip(&wide).is_err());
    }
}
