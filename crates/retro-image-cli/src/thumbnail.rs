//! Downscaling for thumbnails.
//!
//! No external document: a plain box filter (each output pixel averages
//! the source pixels it covers) that keeps the aspect ratio.

use retro_image::Image;

use crate::Raster;

/// `image` scaled down to fit within `size` x `size`, keeping its aspect
/// ratio. Smaller images are returned unchanged.
///
/// Each output pixel is the average of the source pixels it covers (box
/// filter), so dithered and interlaced pictures don't alias. With alpha the
/// colors are weighted by it, so transparent pixels don't tint their
/// neighbors.
pub fn fit(image: &Image, size: u32) -> Raster {
    let (w, h) = (image.width(), image.height());
    if (w <= size && h <= size) || w == 0 || h == 0 {
        return Raster::of(image);
    }
    let (tw, th) = if w >= h {
        (
            size,
            (u64::from(h) * u64::from(size) / u64::from(w)).max(1) as u32,
        )
    } else {
        (
            (u64::from(w) * u64::from(size) / u64::from(h)).max(1) as u32,
            size,
        )
    };
    let alpha = image.has_alpha();
    let data = if alpha {
        box_filter::<4>(&image.rgba(), (w, h), (tw, th))
    } else {
        box_filter::<3>(image.rgb(), (w, h), (tw, th))
    };
    Raster {
        width: tw,
        height: th,
        alpha,
        data,
    }
}

/// The `src` pixels (`N` bytes each, RGB or RGBA) of a `w` x `h` picture
/// averaged down to `tw` x `th`, which is no larger.
fn box_filter<const N: usize>(src: &[u8], (w, h): (u32, u32), (tw, th): (u32, u32)) -> Vec<u8> {
    let mut out = Vec::with_capacity(tw as usize * th as usize * N);
    for ty in 0..th {
        let (y0, y1) = span(ty, th, h);
        for tx in 0..tw {
            let (x0, x1) = span(tx, tw, w);
            // Colors weighted by alpha; without alpha every pixel weighs 1.
            let mut sum = [0u64; 3];
            let mut weight = 0u64;
            for y in y0..y1 {
                for x in x0..x1 {
                    let i = (y as usize * w as usize + x as usize) * N;
                    let pixel = &src[i..i + N];
                    let pixel_weight = if N == 4 { u64::from(pixel[3]) } else { 1 };
                    for (s, &v) in sum.iter_mut().zip(pixel) {
                        *s += u64::from(v) * pixel_weight;
                    }
                    weight += pixel_weight;
                }
            }
            out.extend(sum.map(|s| s.checked_div(weight).unwrap_or(0) as u8));
            if N == 4 {
                out.push((weight / u64::from((y1 - y0) * (x1 - x0))) as u8);
            }
        }
    }
    out
}

/// Source range `[start, end)` covered by output index `i` of `n`, for a
/// source of length `len >= n`; never empty.
fn span(i: u32, n: u32, len: u32) -> (u32, u32) {
    let start = (u64::from(i) * u64::from(len) / u64::from(n)) as u32;
    let end = (u64::from(i + 1) * u64::from(len) / u64::from(n)) as u32;
    (start, end.max(start + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transparent_pixels_add_no_color_to_the_average() {
        // Clear red next to opaque blue: half the alpha, but still pure blue.
        let image = crate::tests::pam_image(2, &[255, 0, 0, 0, 0, 0, 255, 255]);
        let thumb = fit(&image, 1);
        assert!(thumb.alpha);
        assert_eq!((thumb.width, thumb.height), (1, 1));
        assert_eq!(thumb.data, [0, 0, 255, 127]);
    }

    #[test]
    fn spans_cover_the_source_without_gaps() {
        let spans: Vec<_> = (0..3).map(|i| span(i, 3, 10)).collect();
        assert_eq!(spans, [(0, 3), (3, 6), (6, 10)]);
    }
}
