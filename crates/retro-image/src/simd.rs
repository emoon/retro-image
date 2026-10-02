//! Pixel primitives shared by the decoders, behind safe slice APIs.
//!
//! No external format knowledge. Each public function checks its
//! arguments, then runs the best implementation the CPU supports: the
//! scalar reference in `scalar` (always compiled; the only one on other
//! architectures and under Miri) or a SIMD version in `x86_64`, the
//! only module allowed `unsafe`.
//!
//! The SIMD versions cite their sources in `x86_64`.

#[cfg(all(target_arch = "x86_64", not(miri)))]
#[allow(unsafe_code)]
mod x86_64;
// NEON: add an `aarch64` module with the same entry points and a `Neon`
// level, and dispatch to it next to `x86_64` in the `_at` functions.

/// Instruction set levels, lowest first; each includes those below. A
/// primitive uses its best kernel at or below the level and the scalar
/// reference where it has none (levels that gained nothing measurable
/// were left out).
// Only `x86_64` builds the SIMD levels.
#[cfg_attr(any(not(target_arch = "x86_64"), miri), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Level {
    Scalar,
    /// x86_64 baseline: `expand_plane`.
    Sse2,
    /// SSSE3 and SSE4.1: `palette_to_rgb`.
    Sse41,
    /// `expand_plane`, `average_floor`.
    Avx2,
    /// AVX-512 F and BW: `expand_plane`.
    Avx512,
}

#[cfg_attr(any(not(target_arch = "x86_64"), miri), allow(dead_code))]
impl Level {
    const ALL: [Self; 5] = [
        Self::Scalar,
        Self::Sse2,
        Self::Sse41,
        Self::Avx2,
        Self::Avx512,
    ];
}

/// The highest level this CPU (and OS) supports.
fn detected() -> Level {
    #[cfg(all(target_arch = "x86_64", not(miri)))]
    return x86_64::detected();
    #[allow(unreachable_code)]
    Level::Scalar
}

/// Every level this CPU supports, lowest first.
#[cfg(any(test, fuzzing))]
fn supported() -> impl Iterator<Item = Level> {
    Level::ALL.into_iter().filter(|&l| l <= detected())
}

/// ORs bit `x` of `plane` (most significant bit first) into `out[x]` as
/// bit `bit`, for every `x` in `out`: one bitplane row of palette indices.
///
/// Panics if `bit > 7` or `plane` is shorter than `out.len()` bits.
pub(crate) fn expand_plane(plane: &[u8], bit: u32, out: &mut [u8]) {
    expand_plane_at(detected(), plane, bit, out);
}

fn expand_plane_at(level: Level, plane: &[u8], bit: u32, out: &mut [u8]) {
    assert!(bit < 8, "plane bit out of range");
    let plane = &plane[..out.len().div_ceil(8)];
    #[cfg(all(target_arch = "x86_64", not(miri)))]
    if level > Level::Scalar {
        return x86_64::expand_plane(level, plane, bit, out);
    }
    let _ = level;
    scalar::expand_plane(plane, bit, out);
}

/// Writes `table[index]` (as `0xRRGGBB`) for each of `indices` to `out`,
/// 3 bytes per pixel.
///
/// Panics if `out` doesn't hold exactly 3 bytes per index.
pub(crate) fn palette_to_rgb(indices: &[u8], table: &[u32; 256], out: &mut [u8]) {
    palette_to_rgb_at(detected(), indices, table, out);
}

fn palette_to_rgb_at(level: Level, indices: &[u8], table: &[u32; 256], out: &mut [u8]) {
    assert_eq!(out.len(), indices.len() * 3, "RGB buffer size");
    #[cfg(all(target_arch = "x86_64", not(miri)))]
    if level > Level::Scalar {
        return x86_64::palette_to_rgb(level, indices, table, out);
    }
    let _ = level;
    scalar::palette_to_rgb(indices, table, out);
}

/// `out[i] = (a[i] + b[i]) / 2`, rounding down.
///
/// Panics if the lengths differ.
pub(crate) fn average_floor(a: &[u8], b: &[u8], out: &mut [u8]) {
    average_floor_at(detected(), a, b, out);
}

fn average_floor_at(level: Level, a: &[u8], b: &[u8], out: &mut [u8]) {
    assert!(
        a.len() == out.len() && b.len() == out.len(),
        "slice lengths"
    );
    #[cfg(all(target_arch = "x86_64", not(miri)))]
    if level > Level::Scalar {
        return x86_64::average_floor(level, a, b, out);
    }
    let _ = level;
    scalar::average_floor(a, b, out);
}

/// Reference implementations: always compiled, used where no SIMD
/// version applies, for the tails SIMD loops leave, and to check the SIMD
/// versions.
mod scalar {
    pub(super) fn expand_plane(plane: &[u8], bit: u32, out: &mut [u8]) {
        for (pixels, &byte) in out.chunks_mut(8).zip(plane) {
            for (i, pixel) in pixels.iter_mut().enumerate() {
                *pixel |= (byte >> (7 - i) & 1) << bit;
            }
        }
    }

    pub(super) fn palette_to_rgb(indices: &[u8], table: &[u32; 256], out: &mut [u8]) {
        for (rgb, &index) in out.as_chunks_mut::<3>().0.iter_mut().zip(indices) {
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

/// Runs every primitive at every supported level on inputs carved from
/// `data` and panics if any result differs from the scalar reference.
/// Shared by the unit tests and the `simd` fuzz target.
#[cfg(any(test, fuzzing))]
pub fn check_levels(data: &[u8]) {
    use alloc::vec::Vec;

    let (&control, data) = data.split_first().unwrap_or((&0, &[]));
    // Start offsets of up to 3 make the slices unaligned.
    let skip = usize::from(control & 3).min(data.len());
    let data = &data[skip..];
    let bit = u32::from(control >> 2 & 7);
    let half = data.len() / 2;
    let (first, second) = data.split_at(half);

    let mut table = [0u32; 256];
    for (i, entry) in table.iter_mut().enumerate() {
        *entry = (i as u32).wrapping_mul(0x9e37_79b9) ^ u32::from(control);
    }

    let pixels = (first.len() * 8).saturating_sub(usize::from(control >> 5));
    let start: Vec<u8> = second.iter().cycle().take(pixels).copied().collect();
    let mut expected = start.clone();
    scalar::expand_plane(first, bit, &mut expected);
    let mut expected_rgb = alloc::vec![0; data.len() * 3];
    scalar::palette_to_rgb(data, &table, &mut expected_rgb);
    let mut expected_avg = alloc::vec![0; half];
    scalar::average_floor(first, &second[..half], &mut expected_avg);

    for level in supported() {
        let mut out = start.clone();
        expand_plane_at(level, first, bit, &mut out);
        assert_eq!(out, expected, "expand_plane at {level:?}");
        let mut rgb = alloc::vec![0; data.len() * 3];
        palette_to_rgb_at(level, data, &table, &mut rgb);
        assert_eq!(rgb, expected_rgb, "palette_to_rgb at {level:?}");
        let mut avg = alloc::vec![0; half];
        average_floor_at(level, first, &second[..half], &mut avg);
        assert_eq!(avg, expected_avg, "average_floor at {level:?}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    /// xorshift64: deterministic pseudo-random bytes.
    fn random_bytes(seed: u64, len: usize) -> Vec<u8> {
        let mut state = seed | 1;
        (0..len)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                (state >> 24) as u8
            })
            .collect()
    }

    #[test]
    fn scalar_reference_examples() {
        let mut out = [0x10; 10];
        scalar::expand_plane(&[0b1000_0001, 0b0100_0000], 2, &mut out);
        assert_eq!(
            out,
            [0x14, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x14, 0x10, 0x14]
        );
        let mut rgb = [0; 6];
        let mut table = [0; 256];
        table[255] = 0xff12_3456;
        scalar::palette_to_rgb(&[255, 0], &table, &mut rgb);
        assert_eq!(rgb, [0x12, 0x34, 0x56, 0, 0, 0]);
        let mut avg = [0; 3];
        scalar::average_floor(&[255, 1, 0], &[255, 2, 0], &mut avg);
        assert_eq!(avg, [255, 1, 0]);
    }

    #[test]
    fn check_levels_handles_short_input() {
        for len in 0..40 {
            check_levels(&random_bytes(len as u64, len));
        }
    }

    // Under Miri only the scalar level exists: nothing to compare.
    #[test]
    #[cfg_attr(miri, ignore)]
    fn every_level_matches_scalar_on_random_input() {
        for len in 0..=200 {
            for seed in 0..8 {
                check_levels(&random_bytes(len as u64 * 31 + seed, len));
            }
        }
        for len in [255, 256, 257, 1023, 4097] {
            check_levels(&random_bytes(len as u64, len));
        }
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn every_level_matches_scalar_on_edge_values() {
        let all: Vec<u8> = (0..=255).collect();
        for control in 0..=255u8 {
            let mut data = alloc::vec![control];
            data.extend_from_slice(&all);
            data.extend(all.iter().rev());
            check_levels(&data);
            check_levels(&[control, 0xff, 0xff, 0xff, 0xff]);
            check_levels(&[control; 133]);
        }
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn every_bit_and_tail_length_of_expand_plane() {
        let plane = random_bytes(7, 40);
        for skip in 0..4 {
            for len in 0..=(plane.len() - skip) * 8 {
                for bit in 0..8 {
                    let start = random_bytes(len as u64, len);
                    let mut expected = start.clone();
                    scalar::expand_plane(&plane[skip..], bit, &mut expected);
                    for level in supported() {
                        let mut out = start.clone();
                        expand_plane_at(level, &plane[skip..], bit, &mut out);
                        assert_eq!(out, expected, "{level:?} len {len} bit {bit}");
                    }
                }
            }
        }
    }

    #[test]
    fn detection_is_consistent() {
        assert_eq!(detected(), detected());
        assert!(supported().count() >= 1);
        #[cfg(all(target_arch = "x86_64", not(miri)))]
        assert!(detected() >= Level::Sse2);
    }

    #[test]
    #[should_panic(expected = "plane bit")]
    fn expand_plane_rejects_bit_8() {
        expand_plane(&[0], 8, &mut [0]);
    }

    #[test]
    #[should_panic]
    fn expand_plane_rejects_short_plane() {
        expand_plane(&[0], 0, &mut [0; 9]);
    }
}
