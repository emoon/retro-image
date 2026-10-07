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
/// [`Limits`] documents the figure for callers.
pub(crate) const BYTES_PER_PIXEL: usize = 4;

/// The default limit on the memory of one decoded [`Image`](crate::Image):
/// 32 MiB, which is about 8.4 million pixels (a 2896 x 2896 picture).
///
/// This is the one place the default is defined.
pub const DEFAULT_MAX_IMAGE_BYTES: usize = 32 << 20;

static MAX_IMAGE_BYTES: AtomicUsize = AtomicUsize::new(DEFAULT_MAX_IMAGE_BYTES);

/// Limits on what the decoders may allocate.
///
/// A picture counts 4 bytes per pixel (red, green, blue and the alpha plane), whether or not it
/// has alpha. A decode that would produce a larger picture fails with
/// [`DecodeError::TooLarge`](crate::DecodeError::TooLarge) before the
/// picture is allocated. Decoders may also hold temporary buffers while they
/// work, so the peak memory of a decode is a small multiple of the limit.
///
/// The limit applies to the whole process, including decodes that run on
/// other threads, so set it once at start-up.
///
/// ```
/// use retro_image::Limits;
///
/// assert_eq!(Limits::current().max_image_bytes(), 32 << 20);
/// // A caller that only makes thumbnails can tighten the limit:
/// let before = Limits::current();
/// Limits::default().with_max_image_bytes(1 << 20).install();
/// assert_eq!(Limits::current().max_image_bytes(), 1 << 20);
/// before.install();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    max_image_bytes: usize,
}

impl Limits {
    /// The limits in force now.
    #[must_use]
    pub fn current() -> Self {
        Self {
            max_image_bytes: MAX_IMAGE_BYTES.load(Ordering::Relaxed),
        }
    }

    /// The most memory one decoded picture may take, in bytes.
    #[must_use]
    pub const fn max_image_bytes(&self) -> usize {
        self.max_image_bytes
    }

    /// These limits with a different picture memory limit.
    #[must_use]
    pub const fn with_max_image_bytes(mut self, bytes: usize) -> Self {
        self.max_image_bytes = bytes;
        self
    }

    /// Makes these limits the ones every later decode uses.
    pub fn install(self) {
        MAX_IMAGE_BYTES.store(self.max_image_bytes, Ordering::Relaxed);
    }
}

impl Default for Limits {
    /// The default limits: [`DEFAULT_MAX_IMAGE_BYTES`].
    fn default() -> Self {
        Self {
            max_image_bytes: DEFAULT_MAX_IMAGE_BYTES,
        }
    }
}

/// Most pixels a picture may have under the limits in force.
pub(crate) fn max_pixels() -> usize {
    Limits::current().max_image_bytes / BYTES_PER_PIXEL
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
    fn the_default_is_32_mib_of_four_byte_pixels() {
        assert_eq!(DEFAULT_MAX_IMAGE_BYTES, 32 * 1024 * 1024);
        assert_eq!(DEFAULT_MAX_IMAGE_BYTES / BYTES_PER_PIXEL, 8 * 1024 * 1024);
    }
}
