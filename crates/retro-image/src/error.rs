use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    /// No supported format uses the file's extension.
    UnknownExtension,
    /// Formats with this extension exist, but none accepted the data.
    Unrecognized,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownExtension => f.write_str("unknown file extension"),
            Self::Unrecognized => f.write_str("data does not match any format for this extension"),
        }
    }
}

impl core::error::Error for DecodeError {}
