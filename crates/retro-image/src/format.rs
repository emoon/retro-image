use crate::{DecodeError, Image, platform};

/// One supported file format.
#[derive(Debug)]
pub struct Format {
    pub platform: &'static str,
    pub name: &'static str,
    /// Lower-case extensions without the dot.
    pub extensions: &'static [&'static str],
    decoder: fn(&[u8]) -> Result<Image, DecodeError>,
}

impl Format {
    pub(crate) const fn new(
        platform: &'static str,
        name: &'static str,
        extensions: &'static [&'static str],
        decoder: fn(&[u8]) -> Result<Image, DecodeError>,
    ) -> Self {
        Self {
            platform,
            name,
            extensions,
            decoder,
        }
    }

    pub fn decode(&self, data: &[u8]) -> Result<Image, DecodeError> {
        (self.decoder)(data)
    }
}

/// Every supported format.
pub fn formats() -> &'static [Format] {
    platform::FORMATS
}

/// Formats whose extensions match `filename`'s, case-insensitively.
pub(crate) fn by_filename(filename: &str) -> impl Iterator<Item = &'static Format> + '_ {
    let ext = filename.rsplit_once('.').map_or("", |(_, ext)| ext);
    formats()
        .iter()
        .filter(move |f| f.extensions.iter().any(|e| e.eq_ignore_ascii_case(ext)))
}
