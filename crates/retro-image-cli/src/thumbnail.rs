//! Downscaling for thumbnails.

use retro_image::Image;

/// `image` scaled down to fit within `size` x `size`, keeping its aspect
/// ratio, as (width, height, RGB). Smaller images are returned unchanged.
///
/// Each output pixel is the average of the source pixels it covers (box
/// filter), so dithered and interlaced pictures don't alias.
pub fn fit(image: &Image, size: u32) -> (u32, u32, Vec<u8>) {
    let (w, h) = (image.width(), image.height());
    if w <= size && h <= size {
        return (w, h, image.rgb().to_vec());
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
    let src = image.rgb();
    let mut out = Vec::with_capacity(tw as usize * th as usize * 3);
    for ty in 0..th {
        let (y0, y1) = span(ty, th, h);
        for tx in 0..tw {
            let (x0, x1) = span(tx, tw, w);
            let mut sum = [0u64; 3];
            for y in y0..y1 {
                for x in x0..x1 {
                    let i = (y as usize * w as usize + x as usize) * 3;
                    for (s, &v) in sum.iter_mut().zip(&src[i..i + 3]) {
                        *s += u64::from(v);
                    }
                }
            }
            let count = u64::from((y1 - y0) * (x1 - x0));
            out.extend(sum.map(|s| (s / count) as u8));
        }
    }
    (tw, th, out)
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
    fn spans_cover_the_source_without_gaps() {
        let spans: Vec<_> = (0..3).map(|i| span(i, 3, 10)).collect();
        assert_eq!(spans, [(0, 3), (3, 6), (6, 10)]);
    }
}
