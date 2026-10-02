//! Decoder for image formats of retro computers.
//!
//! `no_std` (needs `alloc`): decoders work on in-memory byte slices only.
//!
//! ```
//! fn to_rgb(filename: &str, data: &[u8]) -> Result<Vec<u8>, retro_image::DecodeError> {
//!     Ok(retro_image::decode(filename, data)?.into_rgb())
//! }
//! ```

#![no_std]

extern crate alloc;

mod error;
mod format;
mod image;
mod platform;

pub use error::DecodeError;
pub use format::{Format, formats};
pub use image::Image;

/// Decodes `data`, choosing the format from the extension of `filename`.
///
/// Several formats can share an extension; each candidate is tried in turn
/// and the first one that accepts the data wins.
pub fn decode(filename: &str, data: &[u8]) -> Result<Image, DecodeError> {
    let mut candidates = format::by_filename(filename).peekable();
    if candidates.peek().is_none() {
        return Err(DecodeError::UnknownExtension);
    }
    candidates
        .find_map(|format| format.decode(data).ok())
        .ok_or(DecodeError::Unrecognized)
}
