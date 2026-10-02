//! TIP (Taquart Interlace Picture): HIP's GTIA mode 9 and 10 frames for
//! luminance, each scanline pair completed by a GTIA mode 11 hue line.
//!
//! Sources:
//! - Just Solve "Taquart Interlace Picture" (`TIP`, version byte 1, up to
//!   160x119, 2 frames, GR9/10/11); Mad Team TIP/HIP article (mode idea).
//! - Observed from `recoil2png` output: the header (`TIP`, 1, 0, width in
//!   half-pixels, height, frame length) followed by the mode 9, mode 10 and
//!   mode 11 frames; output of 2 x width by 2 x height; each line shown as a
//!   hue scanline (hue, with the average luminance of this and the previous
//!   line) and then a luminance scanline (hue and this line's luminance);
//!   mode 10 values 0-7 meaning luminance 0-14 (values 8-11 = 0, 12-15 as
//!   4-7); mode 9 and hue pixels drawn 1 output pixel left of the grid,
//!   mode 10 pixels 1 to the right; and the two frames mixed by averaging.

use super::palette::{average, rgb};
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
    let mut image = Image::new(out_width as u32, 2 * height as u32);
    for line in 0..height {
        for x in 0..out_width {
            let left = Some(x + 1);
            let right = x.checked_sub(1);
            let hue = pixel(gtia11, line, left);
            let luminances = |line: usize| {
                let lum9 = pixel(gtia9, line, left).unwrap_or(0);
                let lum10 = pixel(gtia10, line, right).map_or(0, gtia10_luminance);
                [lum9, lum10]
            };
            let current = luminances(line);
            let previous = line.checked_sub(1).map_or([0, 0], luminances);
            let hue_scanline = current
                .iter()
                .zip(previous)
                .map(|(&now, before)| match hue {
                    Some(hue) => rgb((hue << 4) | ((now + before) / 2)),
                    None => 0,
                });
            let hue = hue.unwrap_or(0);
            let luminance_scanline = current.iter().map(|&now| rgb((hue << 4) | now));
            let y = 2 * line as u32;
            image.set(x as u32, y, hue_scanline.reduce(average).unwrap_or(0));
            image.set(
                x as u32,
                y + 1,
                luminance_scanline.reduce(average).unwrap_or(0),
            );
        }
    }
    Ok(image)
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
        let at = |x: usize, y: usize| {
            let i = (y * 320 + x) * 3;
            [image.rgb()[i], image.rgb()[i + 1], image.rgb()[i + 2]]
        };
        // Hue scanline: luminance (9 + 0) / 2 and (8 + 0) / 2 of hue 15.
        assert_eq!(at(2, 0), [0x83, 0x38, 0x00]);
        // Luminance scanline: hue 15 at luminances 9 and 8.
        assert_eq!(at(2, 1), [0xcf, 0x84, 0x2b]);
        // The rightmost pixel has no hue pixel on its hue scanline.
        assert_eq!(at(319, 0), [0, 0, 0]);
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
