//! Decoder for image formats of retro computers.
//!
//! Give [`decode`] a file name and the bytes of a file and it returns the
//! picture as 8-bit RGB, plus an alpha plane if the picture has
//! transparency. The file name chooses which of the supported formats to
//! try (see [`formats`]); many formats are also recognized by their content,
//! so a file with the wrong extension still decodes.
//!
//! ```
//! // A 1x1 PAM picture, held in memory: the library never touches files.
//! let file = b"P7\nWIDTH 1\nHEIGHT 1\nDEPTH 3\nMAXVAL 255\nTUPLTYPE RGB\nENDHDR\n\xff\x80\x00";
//!
//! let decoded = retro_image::decode("dot.pam", file)?;
//! let image = decoded.image();
//! assert_eq!((image.width(), image.height()), (1, 1));
//! assert_eq!(image.rgb(), [0xff, 0x80, 0x00]); // 3 bytes per pixel, row by row
//! assert!(!image.has_alpha());
//! assert_eq!(decoded.format().platform(), "Unix");
//! # Ok::<(), retro_image::DecodeError>(())
//! ```
//!
//! # Guarantees
//!
//! - **Decoding never panics on malformed input.** Truncated, corrupt and
//!   hostile files make a decoder return a [`DecodeError`]; a panic would be
//!   a bug. Every decoder is tested against truncated and mutated real files
//!   and fuzzed for this.
//! - **Memory is bounded.** A picture may take at most
//!   [`max_image_bytes`] (64 MiB, 4 bytes per pixel) of memory
//!   unless the program calls [`set_max_image_bytes`]. Dimensions come from
//!   untrusted headers, so the limit is checked before anything is
//!   allocated for them; a bigger picture fails with
//!   [`DecodeError::TooLarge`]. A decoder holds some temporary buffers
//!   while it works, so the peak memory of a decode is a small multiple of
//!   the limit, not of the file size.
//! - **Types are thread-safe.** [`Image`], [`Format`] and [`DecodeError`] are
//!   `Send` and `Sync`.
//! - **Errors can grow.** [`DecodeError`] is `#[non_exhaustive]`, so new
//!   reasons are not breaking changes.
//!
//! The crate is `no_std` (it needs `alloc`) and has no dependencies.
//! This file holds no external format knowledge: it is the public API.
//! Each decoder module cites the documents its layouts come from; the
//! platform surveys are in `docs/research/`.

#![no_std]
#![warn(missing_docs)]
// Only `simd::x86_64` may use `unsafe`.
#![deny(unsafe_code)]

extern crate alloc;

use alloc::vec::Vec;

mod bytes;
mod codec;
mod decoded;
mod error;
mod format;
mod image;
mod json;
mod limits;
mod macbinary;
mod morton;
mod platform;
mod sheet;
mod simd;
mod tiles;

pub use decoded::Decoded;
pub use error::{Attempt, DecodeError};
pub use format::{Companions, Format, FormatId, NoCompanions, candidates, formats};
pub(crate) use image::BitOrder;
pub use image::Image;
pub use limits::{max_image_bytes, set_max_image_bytes};
/// For the `simd` fuzz target: compares every SIMD level with the scalar
/// reference.
#[cfg(fuzzing)]
#[doc(hidden)]
pub use simd::check_levels as fuzz_check_simd_levels;

// What 1.0 promises about thread safety: pictures, formats and errors can be
// sent to and shared between threads.
const _: () = {
    const fn is_send_sync<T: Send + Sync>() {}
    is_send_sync::<Image>();
    is_send_sync::<Decoded>();
    is_send_sync::<Format>();
    is_send_sync::<DecodeError>();
};

/// Decodes `data` on its own: the picture, and the format that accepted it.
/// See [`decode_with`] for how the format is chosen.
///
/// # Errors
///
/// Fails like [`decode_with`]: with [`DecodeError::UnknownFormat`] or
/// [`DecodeError::NoMatch`].
///
/// # Examples
///
/// ```
/// use retro_image::DecodeError;
///
/// let file = b"P7\nWIDTH 1\nHEIGHT 1\nDEPTH 3\nMAXVAL 255\nTUPLTYPE RGB\nENDHDR\n\x10\x20\x30";
/// let image = retro_image::decode("dot.pam", file)?.into_image();
/// assert_eq!(image.rgb(), [0x10, 0x20, 0x30]);
///
/// // A file that is not a picture of the kind its extension says. The error
/// // lists the formats that were tried and why each refused the data.
/// let Err(DecodeError::NoMatch { attempts }) = retro_image::decode("dot.pam", b"hello") else {
///     panic!("expected the data to be refused");
/// };
/// assert!(attempts.iter().any(|a| a.format().name().contains("PAM")));
/// # Ok::<(), DecodeError>(())
/// ```
pub fn decode(filename: &str, data: &[u8]) -> Result<Decoded, DecodeError> {
    decode_with(filename, data, &NoCompanions)
}

/// Decodes `data`, choosing the format from `filename` and the content.
///
/// The formats from [`candidates`] are tried in turn (extension matches
/// first, then formats recognized by signature) and the first one that
/// accepts the data wins; [`Decoded::format`] says which. Formats that use
/// companion files read them from `companions`. `filename` is only looked at
/// for its extension: it is never opened, and a path or a name with no
/// extension is fine.
///
/// # Errors
///
/// - [`DecodeError::NoMatch`] if some format uses the file's extension but
///   every format tried refused the data. It lists each attempt with its
///   reason, so a picture that is merely over the size limit can be told
///   from one that is corrupt.
/// - [`DecodeError::UnknownFormat`] if no format uses the extension and no
///   format recognized the content.
///
/// # Examples
///
/// A format that takes a palette from a file next to the picture asks the
/// [`Companions`] for it; an implementation answers from wherever it keeps
/// the files, here a list in memory. (A PAM picture has no use for a
/// palette, so this one is not asked.)
///
/// ```
/// use std::borrow::Cow;
/// use retro_image::Companions;
///
/// struct Beside(Vec<(String, Vec<u8>)>);
///
/// impl Companions for Beside {
///     // The file called `<stem>.<extension>` next to the picture.
///     fn get(&self, extension: &str) -> Option<Cow<'_, [u8]>> {
///         let (_, data) = self.0.iter().find(|(name, _)| name.ends_with(extension))?;
///         Some(Cow::Borrowed(data))
///     }
/// }
///
/// let file = b"P7\nWIDTH 1\nHEIGHT 1\nDEPTH 3\nMAXVAL 255\nTUPLTYPE RGB\nENDHDR\n\x01\x02\x03";
/// let beside = Beside(vec![("dot.pal".to_string(), vec![0; 48])]);
/// let decoded = retro_image::decode_with("dot.pam", file, &beside)?;
/// assert_eq!(decoded.image().rgb(), [1, 2, 3]);
/// # Ok::<(), retro_image::DecodeError>(())
/// ```
pub fn decode_with(
    filename: &str,
    data: &[u8],
    companions: &dyn Companions,
) -> Result<Decoded, DecodeError> {
    let mut attempts = Vec::new();
    for format in candidates(filename) {
        match format.decode_with(data, companions) {
            Ok(image) => return Ok(Decoded { image, format }),
            Err(error) => attempts.push(Attempt::new(format, error)),
        }
    }
    if formats().any(|f| f.matches_filename(filename)) {
        Err(DecodeError::NoMatch { attempts })
    } else {
        Err(DecodeError::UnknownFormat)
    }
}
