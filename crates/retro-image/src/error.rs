use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    /// No format uses the file's extension, and none recognised its content.
    UnknownFormat,
    /// Formats with this extension exist, but none accepted the data.
    Unrecognized,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownFormat => f.write_str("unknown file format"),
            Self::Unrecognized => f.write_str("data does not match any format for this extension"),
        }
    }
}

impl core::error::Error for DecodeError {}
