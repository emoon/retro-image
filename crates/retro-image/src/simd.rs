//! Pixel primitives shared by the decoders, behind safe slice APIs.
//!
//! No external format knowledge. Each function checks its arguments and
//! then runs the scalar reference implementation in `scalar`.

/// ORs bit `x` of `plane` (most significant bit first) into `out[x]` as
/// bit `bit`, for every `x` in `out`: one bitplane row of palette indices.
///
/// Panics if `bit > 7` or `plane` is shorter than `out.len()` bits.
pub(crate) fn expand_plane(plane: &[u8], bit: u32, out: &mut [u8]) {
    assert!(bit < 8, "plane bit out of range");
    let plane = &plane[..out.len().div_ceil(8)];
    scalar::expand_plane(plane, bit, out);
}

/// Writes `table[index]` (as `0xRRGGBB`) for each of `indices` to `out`,
/// 3 bytes per pixel.
///
/// Panics if `out` doesn't hold exactly 3 bytes per index.
pub(crate) fn palette_to_rgb(indices: &[u8], table: &[u32; 256], out: &mut [u8]) {
    assert_eq!(out.len(), indices.len() * 3, "RGB buffer size");
    scalar::palette_to_rgb(indices, table, out);
}

/// `out[i] = (a[i] + b[i]) / 2`, rounding down.
///
/// Panics if the lengths differ.
pub(crate) fn average_floor(a: &[u8], b: &[u8], out: &mut [u8]) {
    assert!(
        a.len() == out.len() && b.len() == out.len(),
        "slice lengths"
    );
    scalar::average_floor(a, b, out);
}

/// Reference implementations: always compiled, used where no SIMD
/// version applies and to check the SIMD versions.
mod scalar {
    pub(super) fn expand_plane(plane: &[u8], bit: u32, out: &mut [u8]) {
        for (pixels, &byte) in out.chunks_mut(8).zip(plane) {
            for (i, pixel) in pixels.iter_mut().enumerate() {
                *pixel |= (byte >> (7 - i) & 1) << bit;
            }
        }
    }

    pub(super) fn palette_to_rgb(indices: &[u8], table: &[u32; 256], out: &mut [u8]) {
        for (rgb, &index) in out.chunks_exact_mut(3).zip(indices) {
            let [_, r, g, b] = table[usize::from(index)].to_be_bytes();
            rgb.copy_from_slice(&[r, g, b]);
        }
    }

    pub(super) fn average_floor(a: &[u8], b: &[u8], out: &mut [u8]) {
        for ((o, &a), &b) in out.iter_mut().zip(a).zip(b) {
            *o = ((u16::from(a) + u16::from(b)) / 2) as u8;
        }
    }
}
