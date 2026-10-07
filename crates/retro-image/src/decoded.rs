//! The result of detecting a format and decoding with it.
//!
//! No external format knowledge: the crate's own result type.

use crate::{Format, Image};

/// A decoded picture and the format that decoded it, as
/// [`decode`](crate::decode) returns them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoded {
    pub(crate) image: Image,
    pub(crate) format: &'static Format,
}

impl Decoded {
    /// The picture.
    #[must_use]
    pub fn image(&self) -> &Image {
        &self.image
    }

    /// The format that accepted the file: the first of the
    /// [candidates](crate::candidates) that decoded it.
    #[must_use]
    pub fn format(&self) -> &'static Format {
        self.format
    }

    /// The picture, dropping the format.
    #[must_use]
    pub fn into_image(self) -> Image {
        self.image
    }
}
