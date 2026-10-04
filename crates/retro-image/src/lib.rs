//! Decoder for image formats of retro computers.
//!
//! `no_std` (needs `alloc`): decoders work on in-memory byte slices only.
//!
//! This file holds no external format knowledge: it is the public API.
//! Each decoder module cites the documents its layouts come from; the
//! platform surveys are in `docs/research/`.
//!
//! ```
//! fn to_rgb(filename: &str, data: &[u8]) -> Result<Vec<u8>, retro_image::DecodeError> {
//!     Ok(retro_image::decode(filename, data)?.into_rgb())
//! }
//! ```

#![no_std]
#![warn(missing_docs)]
// Only `simd::x86_64` may use `unsafe`.
#![deny(unsafe_code)]

extern crate alloc;

mod bytes;
mod codec;
mod error;
mod format;
mod image;
mod json;
mod platform;
mod simd;

pub use error::DecodeError;
pub use format::{Companions, Format, NoCompanions, candidates, formats};
pub(crate) use image::BitOrder;
pub use image::Image;
/// For the `simd` fuzz target: compares every SIMD level with the scalar
/// reference.
#[cfg(fuzzing)]
#[doc(hidden)]
pub use simd::check_levels as fuzz_check_simd_levels;

/// Decodes `data` on its own. See [`decode_with`].
pub fn decode(filename: &str, data: &[u8]) -> Result<Image, DecodeError> {
    decode_with(filename, data, &NoCompanions)
}

/// Decodes `data`, choosing the format from `filename` and the content.
///
/// The formats from [`candidates`] are tried in turn (extension matches
/// first, then formats recognised by signature) and the first one that
/// accepts the data wins. Formats that use companion files read them from
/// `companions`.
pub fn decode_with(
    filename: &str,
    data: &[u8],
    companions: &dyn Companions,
) -> Result<Image, DecodeError> {
    candidates(filename)
        .find_map(|format| format.decode_with(data, companions).ok())
        .ok_or_else(|| {
            if formats().any(|f| f.matches_filename(filename)) {
                DecodeError::Unrecognized
            } else {
                DecodeError::UnknownFormat
            }
        })
}
