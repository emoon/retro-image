use alloc::vec::Vec;

use crate::{DecodeError, Image, platform};

/// One supported file format.
#[derive(Debug)]
pub struct Format {
    pub platform: &'static str,
    pub name: &'static str,
    /// Lower-case extensions without the dot.
    pub extensions: &'static [&'static str],
    decoder: Decoder,
    signature: bool,
}

#[derive(Debug, Clone, Copy)]
enum Decoder {
    Single(fn(&[u8]) -> Result<Image, DecodeError>),
    WithCompanions(fn(&[u8], &dyn Companions) -> Result<Image, DecodeError>),
}

/// Files that accompany the main one, such as a palette file next to a
/// picture (`PIC.COL` next to `PIC.MIC`).
///
/// Decoders must still decode the main file alone where the format allows,
/// e.g. with a default palette, because callers may not have the companions.
pub trait Companions {
    /// The file named like the main file, with `extension` (without the dot,
    /// case-insensitive) in place of the main file's extension, if present.
    fn get(&self, extension: &str) -> Option<Vec<u8>>;
}

/// No companion files.
pub struct NoCompanions;

impl Companions for NoCompanions {
    fn get(&self, _extension: &str) -> Option<Vec<u8>> {
        None
    }
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
            decoder: Decoder::Single(decoder),
            signature: false,
        }
    }

    /// A format whose decoder can use companion files.
    #[expect(dead_code, reason = "used once platforms adopt companion files")]
    pub(crate) const fn with_companions(
        platform: &'static str,
        name: &'static str,
        extensions: &'static [&'static str],
        decoder: fn(&[u8], &dyn Companions) -> Result<Image, DecodeError>,
    ) -> Self {
        Self {
            platform,
            name,
            extensions,
            decoder: Decoder::WithCompanions(decoder),
            signature: false,
        }
    }

    /// Marks the decoder as checking a reliable signature (magic bytes or a
    /// strictly validated header), so it is also tried on files whose
    /// extension doesn't match. Never mark headerless memory dumps.
    pub(crate) const fn signature(mut self) -> Self {
        self.signature = true;
        self
    }

    /// Whether the format is recognised by content as well as by extension.
    pub fn has_signature(&self) -> bool {
        self.signature
    }

    /// Whether the decoder reads companion files when they are available.
    pub fn uses_companions(&self) -> bool {
        matches!(self.decoder, Decoder::WithCompanions(_))
    }

    /// Decodes `data` alone.
    pub fn decode(&self, data: &[u8]) -> Result<Image, DecodeError> {
        self.decode_with(data, &NoCompanions)
    }

    /// Decodes `data`, reading companion files from `companions` if needed.
    pub fn decode_with(
        &self,
        data: &[u8],
        companions: &dyn Companions,
    ) -> Result<Image, DecodeError> {
        match self.decoder {
            Decoder::Single(decode) => decode(data),
            Decoder::WithCompanions(decode) => decode(data, companions),
        }
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

/// The formats [`decode`](crate::decode) tries for `filename`, in order:
/// those matching its extension, then the other formats with a signature.
pub fn candidates(filename: &str) -> impl Iterator<Item = &'static Format> + '_ {
    let by_extension = formats().filter(move |f| f.matches_filename(filename));
    let by_content = formats().filter(move |f| f.signature && !f.matches_filename(filename));
    by_extension.chain(by_content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidates_put_extension_matches_before_signature_formats() {
        let ordered: Vec<&Format> = candidates("picture.iff").collect();
        let first_signature_only = ordered
            .iter()
            .position(|f| !f.matches_filename("picture.iff"))
            .unwrap_or(ordered.len());
        assert!(first_signature_only > 0, "IFF matches by extension");
        assert!(
            ordered[first_signature_only..]
                .iter()
                .all(|f| f.has_signature() && !f.matches_filename("picture.iff")),
            "after the extension matches come only signature formats"
        );
        let by_content: Vec<&str> = candidates("picture.xyz").map(|f| f.name).collect();
        assert!(by_content.contains(&"Interchange File Format"));
        assert!(candidates("picture.xyz").all(Format::has_signature));
    }

    #[test]
    fn signature_formats_decode_files_with_any_extension() {
        // 1x1 single-plane ILBM: BMHD, CMAP (2 colours), BODY (one word).
        let mut ilbm = Vec::new();
        ilbm.extend_from_slice(b"FORM\0\0\0\x3eILBM");
        ilbm.extend_from_slice(
            b"BMHD\0\0\0\x14\0\x01\0\x01\0\0\0\0\x01\0\0\0\0\0\x01\x01\0\x01\0\x01",
        );
        ilbm.extend_from_slice(b"CMAP\0\0\0\x06\0\0\0\xff\xff\xff");
        ilbm.extend_from_slice(b"BODY\0\0\0\x02\x80\0");
        let as_iff = crate::decode("x.iff", &ilbm).unwrap();
        let renamed = crate::decode("x.dat", &ilbm).unwrap();
        assert_eq!(as_iff, renamed);
        assert_eq!(renamed.rgb(), &[0xff, 0xff, 0xff]);
        assert_eq!(
            crate::decode("x.dat", b"not a picture"),
            Err(DecodeError::UnknownFormat)
        );
    }
}
