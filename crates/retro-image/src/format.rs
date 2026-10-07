//! The format registry: extensions, content detection and dispatch to
//! the platform decoders.
//!
//! No external format knowledge: which extensions and signatures each
//! format has is stated, with its sources, by the platform modules.

use alloc::borrow::Cow;
use core::fmt;
use core::hash::{Hash, Hasher};

use crate::{DecodeError, Image, platform};

/// One supported file format.
///
/// Formats are the entries of the registry, obtained from [`formats`] and
/// [`candidates`]. Two formats are equal when they have the same
/// [`FormatId`].
pub struct Format {
    platform: &'static str,
    name: &'static str,
    extensions: &'static [&'static str],
    decoder: Decoder,
    signature: bool,
    id: FormatId,
}

/// An identifier for a [`Format`] that stays the same between releases, so a
/// program can store it, for example to remember which format a user picked.
///
/// It is computed from the format's platform, name and extensions, so it
/// changes only if one of those does, and it does not depend on the order of
/// the registry or on which other formats exist. It is a hash: treat it as
/// an opaque key that can be compared, ordered, hashed and printed, and
/// don't pick it apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FormatId(u64);

impl FormatId {
    /// FNV-1a over the platform, the name and the extensions, each part
    /// ended by a byte that cannot occur in them.
    const fn of(
        platform: &'static str,
        name: &'static str,
        extensions: &'static [&'static str],
    ) -> Self {
        const fn mix(mut hash: u64, bytes: &[u8]) -> u64 {
            let mut i = 0;
            while i < bytes.len() {
                hash ^= bytes[i] as u64;
                hash = hash.wrapping_mul(0x0100_0000_01b3);
                i += 1;
            }
            hash ^= 0xff;
            hash.wrapping_mul(0x0100_0000_01b3)
        }
        let mut hash = mix(0xcbf2_9ce4_8422_2325, platform.as_bytes());
        hash = mix(hash, name.as_bytes());
        let mut i = 0;
        while i < extensions.len() {
            hash = mix(hash, extensions[i].as_bytes());
            i += 1;
        }
        Self(hash)
    }
}

impl fmt::Display for FormatId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

impl fmt::Debug for Format {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Format")
            .field("platform", &self.platform)
            .field("name", &self.name)
            .field("extensions", &self.extensions)
            .finish()
    }
}

impl PartialEq for Format {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Format {}

impl Hash for Format {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
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
///
/// Every method has a default that finds nothing, so the trait can gain
/// methods in a minor release without breaking implementations: implement
/// only the lookups you can answer. Companion files are untrusted input as
/// much as the main file is, and the names passed to an implementation come
/// from it: implementations that read from a file system must not follow
/// directory parts or `..` out of the main file's directory.
///
/// The methods return a [`Cow`] so that an implementation that holds the
/// files in memory can lend them, while one that reads them from disk hands
/// over what it read. Changing the return type of a published trait method
/// would break every implementation, so this is decided here once.
pub trait Companions {
    /// The file named like the main file, with `extension` (without the dot,
    /// case-insensitive) in place of the main file's extension, if present.
    fn get(&self, extension: &str) -> Option<Cow<'_, [u8]>> {
        let _ = extension;
        None
    }

    /// The file called `file_name` in the main file's directory, for formats
    /// whose main file lists the files it needs (e.g. a scroll list). Any
    /// directory part of the name (`/` or `\`) is ignored; the rest must match
    /// exactly. Callers without access to the directory return `None`.
    fn get_named(&self, file_name: &str) -> Option<Cow<'_, [u8]>> {
        let _ = file_name;
        None
    }
}

/// No companion files: every lookup finds nothing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NoCompanions;

impl Companions for NoCompanions {}

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
            id: FormatId::of(platform, name, extensions),
        }
    }

    /// A format whose decoder can use companion files.
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
            id: FormatId::of(platform, name, extensions),
        }
    }

    /// Marks the decoder as checking a reliable signature (magic bytes or a
    /// strictly validated header), so it is also tried on files whose
    /// extension doesn't match. Never mark headerless memory dumps.
    pub(crate) const fn signature(mut self) -> Self {
        self.signature = true;
        self
    }

    /// The machine the format belongs to (e.g. `"Atari ST"`), as named in
    /// `docs/formats.md`. Meant for display and grouping; the names are not a
    /// closed set, because new platforms are added.
    #[must_use]
    pub fn platform(&self) -> &'static str {
        self.platform
    }

    /// The name of the program or format, e.g. `"NEOchrome"`.
    #[must_use]
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// The file name extensions of the format, lower case and without the
    /// dot. Some formats are found by content only and have none.
    #[must_use]
    pub fn extensions(&self) -> &'static [&'static str] {
        self.extensions
    }

    /// An identifier for the format that stays the same between releases.
    #[must_use]
    pub fn id(&self) -> FormatId {
        self.id
    }

    /// Whether the format is recognised by content as well as by extension.
    #[must_use]
    pub fn has_signature(&self) -> bool {
        self.signature
    }

    /// Whether the decoder reads companion files when they are available.
    #[must_use]
    pub fn uses_companions(&self) -> bool {
        matches!(self.decoder, Decoder::WithCompanions(_))
    }

    /// Decodes `data` alone, as this format and no other.
    ///
    /// Unlike [`crate::decode`], which finds the format for a file, this
    /// asks one format to decode and says nothing about the file name.
    ///
    /// # Errors
    ///
    /// [`DecodeError::Invalid`] if the data is not valid for this format,
    /// including truncated data, and [`DecodeError::TooLarge`] if the picture
    /// would exceed the [`Limits`](crate::Limits).
    ///
    /// # Examples
    ///
    /// ```
    /// use retro_image::DecodeError;
    ///
    /// let pam = retro_image::formats()
    ///     .find(|f| f.extensions().contains(&"pam"))
    ///     .unwrap();
    /// let file = b"P7\nWIDTH 1\nHEIGHT 1\nDEPTH 3\nMAXVAL 255\nTUPLTYPE RGB\nENDHDR\n\x01\x02\x03";
    /// assert_eq!(pam.decode(file)?.rgb(), [1, 2, 3]);
    /// assert_eq!(pam.decode(b"P7\n").unwrap_err(), DecodeError::Invalid);
    /// # Ok::<(), DecodeError>(())
    /// ```
    pub fn decode(&self, data: &[u8]) -> Result<Image, DecodeError> {
        self.decode_with(data, &NoCompanions)
    }

    /// Decodes `data`, reading companion files from `companions` if needed.
    /// Like [`Format::decode`], for one format only.
    ///
    /// # Errors
    ///
    /// The same as [`Format::decode`].
    pub fn decode_with(
        &self,
        data: &[u8],
        companions: &dyn Companions,
    ) -> Result<Image, DecodeError> {
        let image = match self.decoder {
            Decoder::Single(decode) => decode(data),
            Decoder::WithCompanions(decode) => decode(data, companions),
        }?;
        Ok(image.normalized())
    }

    /// Whether `filename`'s extension is one of this format's, case-insensitively.
    #[must_use]
    pub fn matches_filename(&self, filename: &str) -> bool {
        let ext = filename.rsplit_once('.').map_or("", |(_, ext)| ext);
        self.extensions.iter().any(|e| e.eq_ignore_ascii_case(ext))
    }
}

/// Every supported format.
///
/// ```
/// let count = retro_image::formats().count();
/// assert!(count > 600);
/// assert!(retro_image::formats().all(|f| !f.name().is_empty()));
/// ```
pub fn formats() -> impl Iterator<Item = &'static Format> {
    platform::ALL.iter().flat_map(|formats| formats.iter())
}

/// The formats [`decode`](crate::decode) tries for `filename`, in order:
/// those matching its extension, then the other formats with a signature.
/// `filename` is only looked at for its extension.
///
/// ```
/// let first = retro_image::candidates("PICTURE.PI1").next().unwrap();
/// assert!(first.extensions().contains(&"pi1"));
/// ```
pub fn candidates(filename: &str) -> impl Iterator<Item = &'static Format> + '_ {
    let by_extension = formats().filter(move |f| f.matches_filename(filename));
    let by_content = formats().filter(move |f| f.signature && !f.matches_filename(filename));
    by_extension.chain(by_content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

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
    fn every_format_has_its_own_id() {
        let mut ids: Vec<FormatId> = formats().map(Format::id).collect();
        let count = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), count, "two formats share an id");
        let first = formats().next().unwrap();
        assert_eq!(first, first);
        assert_ne!(first, formats().nth(1).unwrap());
    }

    #[test]
    fn signature_formats_decode_files_with_any_extension() {
        // 1x1 single-plane ILBM: BMHD, CMAP (2 colors), BODY (one word).
        let mut ilbm = Vec::new();
        ilbm.extend_from_slice(b"FORM\0\0\0\x3eILBM");
        ilbm.extend_from_slice(
            b"BMHD\0\0\0\x14\0\x01\0\x01\0\0\0\0\x01\0\0\0\0\0\x01\x01\0\x01\0\x01",
        );
        ilbm.extend_from_slice(b"CMAP\0\0\0\x06\0\0\0\xff\xff\xff");
        ilbm.extend_from_slice(b"BODY\0\0\0\x02\x80\0");
        let as_iff = crate::decode("x.iff", &ilbm).unwrap();
        let renamed = crate::decode("x.xyz", &ilbm).unwrap();
        assert_eq!(as_iff, renamed);
        assert_eq!(renamed.image().rgb(), &[0xff, 0xff, 0xff]);
        assert_eq!(renamed.format().name(), "Interchange File Format");
        assert_eq!(
            crate::decode("x.xyz", b"not a picture"),
            Err(DecodeError::UnknownFormat)
        );
    }
}
