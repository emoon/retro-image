//! Decode errors.
//!
//! No external format knowledge: the crate's error type.

use alloc::vec::Vec;
use core::fmt;

use crate::Format;

/// Why a file could not be decoded.
///
/// [`Format::decode`] fails with [`Invalid`](Self::Invalid) or
/// [`TooLarge`](Self::TooLarge): the reason that one format gave.
/// [`decode`](crate::decode) tries several formats and fails with
/// [`NoMatch`](Self::NoMatch), which lists what each one said, or with
/// [`UnknownFormat`](Self::UnknownFormat).
///
/// New reasons may be added in minor releases, so match with a wildcard arm.
/// An error is not `Copy`, because the variants that describe several
/// attempts own their data.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DecodeError {
    /// No format uses the file's extension, and none recognized its content.
    UnknownFormat,
    /// The data is not valid for the format, or uses a variant of it that is
    /// not supported. Truncated data is invalid.
    Invalid,
    /// The picture would be larger than the [`Limits`](crate::Limits) allow.
    TooLarge,
    /// Formats were tried and every one failed. A format that has the
    /// file's extension is tried first, so the attempts show whether a
    /// format that should have decoded the file refused it, and why.
    NoMatch {
        /// Every format that was tried, in order, with why it failed.
        attempts: Vec<Attempt>,
    },
}

/// One format that [`decode`](crate::decode) tried and that failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attempt {
    format: &'static Format,
    error: DecodeError,
}

impl Attempt {
    pub(crate) fn new(format: &'static Format, error: DecodeError) -> Self {
        Self { format, error }
    }

    /// The format that was tried.
    #[must_use]
    pub fn format(&self) -> &'static Format {
        self.format
    }

    /// Why the format failed: [`DecodeError::Invalid`] or
    /// [`DecodeError::TooLarge`].
    #[must_use]
    pub fn error(&self) -> &DecodeError {
        &self.error
    }
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownFormat => f.write_str("unknown file format"),
            Self::Invalid => f.write_str("data is not valid for this format"),
            Self::TooLarge => f.write_str("picture is larger than the size limit"),
            Self::NoMatch { attempts } => {
                let too_large = attempts
                    .iter()
                    .filter(|a| a.error == Self::TooLarge)
                    .count();
                write!(
                    f,
                    "data does not match any of the {} formats tried",
                    attempts.len()
                )?;
                if too_large > 0 {
                    write!(f, " ({too_large} rejected it as too large)")?;
                }
                Ok(())
            }
        }
    }
}

impl core::error::Error for DecodeError {}
