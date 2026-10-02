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

    /// Whether `filename`'s extension is one of this format's, case-insensitively.
    pub fn matches_filename(&self, filename: &str) -> bool {
        let ext = filename.rsplit_once('.').map_or("", |(_, ext)| ext);
        self.extensions.iter().any(|e| e.eq_ignore_ascii_case(ext))
    }
}

/// Every supported format.
pub fn formats() -> impl Iterator<Item = &'static Format> {
    platform::ALL.iter().flat_map(|formats| formats.iter())
}

/// Formats whose extensions match `filename`'s.
pub(crate) fn by_filename(filename: &str) -> impl Iterator<Item = &'static Format> + '_ {
    formats().filter(move |f| f.matches_filename(filename))
}
