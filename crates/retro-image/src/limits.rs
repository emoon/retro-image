//! The size limit on decoded pictures.
//!
//! No external format knowledge: this is the crate's own safety policy.
//! Picture dimensions come from untrusted headers, so a few bytes of file
//! must not be able to demand gigabytes. Every [`Image`](crate::Image)
//! constructor checks the limit before it allocates, so no decoder can skip
//! it.
//!
//! The limit is process-wide state rather than an argument of `decode`:
//! decoders are plain `fn(&[u8])` values deep inside the platform modules,
//! and `no_std` has no thread-local storage to scope a limit to one call.
//! Threading a parameter through every decoder would touch hundreds of
//! signatures for a value that a program sets once at start-up.

use core::sync::atomic::{AtomicUsize, Ordering};

/// Bytes the crate budgets for each pixel of a decoded picture: 3 color
/// channels and the alpha plane. Every picture is counted as if it had
/// alpha, so whether a picture fits does not depend on its transparency.
/// [`max_image_bytes`] documents the figure for callers.
pub(crate) const BYTES_PER_PIXEL: usize = 4;

/// The default limit on the memory of one decoded picture: 64 MiB, which is
/// about 16.8 million pixels (a 4096 x 4096 picture).
const DEFAULT_MAX_IMAGE_BYTES: usize = 64 << 20;

static MAX_IMAGE_BYTES: AtomicUsize = AtomicUsize::new(DEFAULT_MAX_IMAGE_BYTES);

/// The most memory one decoded picture may take, in bytes: 64 MiB until
/// [`set_max_image_bytes`] changes it.
///
/// A picture counts 4 bytes per pixel (red, green, blue and the alpha
/// plane), whether or not it has alpha. A decode that would produce a larger
/// picture fails with [`DecodeError::TooLarge`](crate::DecodeError::TooLarge)
/// before the picture is allocated. Decoders may also hold temporary buffers
/// while they work, so the peak memory of a decode is a small multiple of
/// the limit.
#[must_use]
pub fn max_image_bytes() -> usize {
    MAX_IMAGE_BYTES.load(Ordering::Relaxed)
}

/// Sets the most memory one decoded picture may take, in bytes. See
/// [`max_image_bytes`] for how pictures are counted.
///
/// The limit applies to the whole process, including decodes that run on
/// other threads, so set it once at start-up.
///
/// ```
/// // A program that only makes thumbnails can tighten the limit:
/// let before = retro_image::max_image_bytes();
/// retro_image::set_max_image_bytes(1 << 20);
/// assert_eq!(retro_image::max_image_bytes(), 1 << 20);
/// retro_image::set_max_image_bytes(before);
/// ```
pub fn set_max_image_bytes(bytes: usize) {
    MAX_IMAGE_BYTES.store(bytes, Ordering::Relaxed);
}

/// Most pixels a picture may have under the limits in force.
pub(crate) fn max_pixels() -> usize {
    max_image_bytes() / BYTES_PER_PIXEL
}

/// Whether a `width` x `height` picture is within `max_pixels`.
pub(crate) fn fits(width: usize, height: usize, max_pixels: usize) -> bool {
    width.checked_mul(height).is_some_and(|px| px <= max_pixels)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fits_rejects_products_that_overflow() {
        assert!(fits(10, 10, 100));
        assert!(!fits(10, 11, 100));
        assert!(!fits(usize::MAX, 2, usize::MAX));
        assert!(fits(0, usize::MAX, 0));
    }

    #[test]
    fn the_default_is_64_mib_of_four_byte_pixels() {
        assert_eq!(DEFAULT_MAX_IMAGE_BYTES, 64 * 1024 * 1024);
        assert_eq!(DEFAULT_MAX_IMAGE_BYTES / BYTES_PER_PIXEL, 16 * 1024 * 1024);
    }
}
