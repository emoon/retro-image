//! x86_64 versions of the primitives and the runtime CPU feature check.
//!
//! Each entry point clamps the requested level to [`detected`], so a
//! `#[target_feature]` function only ever runs on a CPU that has the
//! feature. Kernels handle whole blocks; the scalar reference finishes
//! the tail.

use core::arch::x86_64::*;
use core::sync::atomic::{AtomicU8, Ordering};

use super::{Level, scalar};

/// `detected()` as `Level as u8 + 1`; 0 until the first check.
static DETECTED: AtomicU8 = AtomicU8::new(0);

pub(super) fn detected() -> Level {
    match DETECTED.load(Ordering::Relaxed) {
        0 => {
            let level = probe();
            DETECTED.store(level as u8 + 1, Ordering::Relaxed);
            level
        }
        n => Level::ALL[usize::from(n - 1)],
    }
}

/// Checks CPUID and, for AVX levels, that the OS saves the wider
/// registers (CPUID.1:ECX.OSXSAVE, then XCR0 via XGETBV).
fn probe() -> Level {
    // Built with these features enabled (e.g. `-C target-cpu=native`):
    // every CPU the binary runs on has them.
    if cfg!(all(target_feature = "avx512f", target_feature = "avx512bw")) {
        return Level::Avx512;
    }
    if cfg!(target_feature = "avx2") {
        return Level::Avx2;
    }
    const SSSE3: u32 = 1 << 9;
    const SSE41: u32 = 1 << 19;
    const OSXSAVE: u32 = 1 << 27;
    const AVX: u32 = 1 << 28;
    const AVX2: u32 = 1 << 5;
    const AVX512F: u32 = 1 << 16;
    const AVX512BW: u32 = 1 << 30;
    // XCR0: SSE (bit 1) and AVX (bit 2) state; AVX-512 adds opmask,
    // ZMM0-15 upper halves and ZMM16-31 (bits 5-7).
    const XCR0_AVX: u64 = 0b110;
    const XCR0_AVX512: u64 = 0b1110_0110;

    let max_leaf = __cpuid(0).eax;
    let leaf1 = __cpuid(1);
    if leaf1.ecx & (SSSE3 | SSE41) != SSSE3 | SSE41 {
        return Level::Sse2;
    }
    if max_leaf < 7 || leaf1.ecx & (OSXSAVE | AVX) != OSXSAVE | AVX {
        return Level::Sse41;
    }
    // SAFETY: CPUID.1:ECX.OSXSAVE is set, so XGETBV is available.
    let xcr0 = unsafe { xcr0() };
    let leaf7 = __cpuid_count(7, 0);
    if xcr0 & XCR0_AVX != XCR0_AVX || leaf7.ebx & AVX2 == 0 {
        return Level::Sse41;
    }
    if xcr0 & XCR0_AVX512 == XCR0_AVX512 && leaf7.ebx & (AVX512F | AVX512BW) == AVX512F | AVX512BW {
        return Level::Avx512;
    }
    Level::Avx2
}

#[target_feature(enable = "xsave")]
fn xcr0() -> u64 {
    // SAFETY: XCR0 (register 0) always exists when XGETBV does.
    unsafe { _xgetbv(0) }
}

pub(super) fn expand_plane(level: Level, plane: &[u8], bit: u32, out: &mut [u8]) {
    let done = match level.min(detected()) {
        // SAFETY: the CPU has AVX-512 F and BW (clamped above).
        Level::Avx512 => unsafe { expand_plane_avx512(plane, bit, out) },
        // SAFETY: the CPU has AVX2 (clamped above).
        Level::Avx2 => unsafe { expand_plane_avx2(plane, bit, out) },
        // SAFETY: SSE2 is part of the x86_64 baseline.
        _ => unsafe { expand_plane_sse2(plane, bit, out) },
    };
    scalar::expand_plane(&plane[done / 8..], bit, &mut out[done..]);
}

// Each kernel repeats every plane byte across 8 output bytes, keeps the
// bytes whose own bit (`masks`: 0x80, 0x40, ..., 0x01) is set and ORs
// `1 << bit` into them. Returns the number of pixels done.

#[target_feature(enable = "sse2")]
fn expand_plane_sse2(plane: &[u8], bit: u32, out: &mut [u8]) -> usize {
    let masks = _mm_set1_epi64x(i64::from_le_bytes([128, 64, 32, 16, 8, 4, 2, 1]));
    let value = _mm_set1_epi8(1 << bit);
    let (blocks, _) = out.as_chunks_mut::<16>();
    let (bytes, _) = plane.as_chunks::<2>();
    for (pixels, bytes) in blocks.iter_mut().zip(bytes) {
        let v = _mm_cvtsi32_si128(i32::from(u16::from_le_bytes(*bytes)));
        let v = _mm_unpacklo_epi8(v, v);
        let v = _mm_unpacklo_epi16(v, v);
        let v = _mm_unpacklo_epi32(v, v);
        let set = _mm_cmpeq_epi8(_mm_and_si128(v, masks), masks);
        let bits = _mm_or_si128(load16(pixels), _mm_and_si128(set, value));
        store16(pixels, bits);
    }
    blocks.len() * 16
}

#[target_feature(enable = "avx2")]
fn expand_plane_avx2(plane: &[u8], bit: u32, out: &mut [u8]) -> usize {
    let masks = _mm256_set1_epi64x(i64::from_le_bytes([128, 64, 32, 16, 8, 4, 2, 1]));
    // The shuffle stays within 128-bit lanes: the low lane spreads bytes
    // 0 and 1, the high lane bytes 2 and 3 of its copy.
    let spread = _mm256_setr_epi64x(
        0,
        0x0101_0101_0101_0101,
        0x0202_0202_0202_0202,
        0x0303_0303_0303_0303,
    );
    let value = _mm256_set1_epi8(1 << bit);
    let (blocks, _) = out.as_chunks_mut::<32>();
    let (bytes, _) = plane.as_chunks::<4>();
    for (pixels, bytes) in blocks.iter_mut().zip(bytes) {
        let v = _mm256_set1_epi32(i32::from_le_bytes(*bytes));
        let v = _mm256_shuffle_epi8(v, spread);
        let set = _mm256_cmpeq_epi8(_mm256_and_si256(v, masks), masks);
        let bits = _mm256_or_si256(load32(pixels), _mm256_and_si256(set, value));
        store32(pixels, bits);
    }
    blocks.len() * 32
}

#[target_feature(enable = "avx512f,avx512bw")]
fn expand_plane_avx512(plane: &[u8], bit: u32, out: &mut [u8]) -> usize {
    let masks = _mm512_set1_epi64(i64::from_le_bytes([128, 64, 32, 16, 8, 4, 2, 1]));
    // 128-bit lane j spreads bytes 2j and 2j + 1.
    let spread = _mm512_set_epi64(
        0x0707_0707_0707_0707,
        0x0606_0606_0606_0606,
        0x0505_0505_0505_0505,
        0x0404_0404_0404_0404,
        0x0303_0303_0303_0303,
        0x0202_0202_0202_0202,
        0x0101_0101_0101_0101,
        0,
    );
    let value = _mm512_set1_epi8(1 << bit);
    let (blocks, _) = out.as_chunks_mut::<64>();
    let (bytes, _) = plane.as_chunks::<8>();
    for (pixels, bytes) in blocks.iter_mut().zip(bytes) {
        let v = _mm512_set1_epi64(i64::from_le_bytes(*bytes));
        let set = _mm512_test_epi8_mask(_mm512_shuffle_epi8(v, spread), masks);
        let old = load64(pixels);
        store64(
            pixels,
            _mm512_mask_blend_epi8(set, old, _mm512_or_si512(old, value)),
        );
    }
    blocks.len() * 64
}

pub(super) fn palette_to_rgb(level: Level, indices: &[u8], table: &[u32; 256], out: &mut [u8]) {
    let done = if level.min(detected()) >= Level::Sse41 {
        // SAFETY: the CPU has SSSE3 and SSE4.1 (clamped above).
        unsafe { palette_to_rgb_sse41(indices, table, out) }
    } else {
        0
    };
    scalar::palette_to_rgb(&indices[done..], table, &mut out[done * 3..]);
}

/// Returns the number of pixels done (a multiple of 4).
#[target_feature(enable = "ssse3,sse4.1")]
fn palette_to_rgb_sse41(indices: &[u8], table: &[u32; 256], out: &mut [u8]) -> usize {
    // An entry is B, G, R, 0 in memory: pack R, G, B of four entries into
    // 12 bytes, written as 8 + 4.
    let rgb = _mm_setr_epi8(2, 1, 0, 6, 5, 4, 10, 9, 8, 14, 13, 12, -1, -1, -1, -1);
    let (blocks, _) = out.as_chunks_mut::<12>();
    let (groups, _) = indices.as_chunks::<4>();
    for (pixels, group) in blocks.iter_mut().zip(groups) {
        let [a, b, c, d] = group.map(|i| table[usize::from(i)] as i32);
        let packed = _mm_shuffle_epi8(_mm_setr_epi32(a, b, c, d), rgb);
        let (low, high) = pixels.split_at_mut(8);
        low.copy_from_slice(&_mm_cvtsi128_si64(packed).to_le_bytes());
        high.copy_from_slice(&_mm_extract_epi32::<2>(packed).to_le_bytes());
    }
    blocks.len() * 4
}

// Unaligned loads and stores of whole arrays: the array type proves the
// bytes are there, and `loadu`/`storeu` need no alignment.

#[inline]
#[target_feature(enable = "sse2")]
fn load16(bytes: &[u8; 16]) -> __m128i {
    // SAFETY: 16 readable bytes; no alignment needed.
    unsafe { _mm_loadu_si128(bytes.as_ptr().cast()) }
}

#[inline]
#[target_feature(enable = "sse2")]
fn store16(bytes: &mut [u8; 16], v: __m128i) {
    // SAFETY: 16 writable bytes; no alignment needed.
    unsafe { _mm_storeu_si128(bytes.as_mut_ptr().cast(), v) }
}

#[inline]
#[target_feature(enable = "avx")]
fn load32(bytes: &[u8; 32]) -> __m256i {
    // SAFETY: 32 readable bytes; no alignment needed.
    unsafe { _mm256_loadu_si256(bytes.as_ptr().cast()) }
}

#[inline]
#[target_feature(enable = "avx")]
fn store32(bytes: &mut [u8; 32], v: __m256i) {
    // SAFETY: 32 writable bytes; no alignment needed.
    unsafe { _mm256_storeu_si256(bytes.as_mut_ptr().cast(), v) }
}

#[inline]
#[target_feature(enable = "avx512f")]
fn load64(bytes: &[u8; 64]) -> __m512i {
    // SAFETY: 64 readable bytes; no alignment needed.
    unsafe { _mm512_loadu_si512(bytes.as_ptr().cast()) }
}

#[inline]
#[target_feature(enable = "avx512f")]
fn store64(bytes: &mut [u8; 64], v: __m512i) {
    // SAFETY: 64 writable bytes; no alignment needed.
    unsafe { _mm512_storeu_si512(bytes.as_mut_ptr().cast(), v) }
}
