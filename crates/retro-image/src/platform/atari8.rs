//! Atari 8-bit, including VBXE, and Atari Portfolio.
//!
//! Each submodule lists the sources of its formats. The palette is in
//! [`palette`]; ANTIC/GTIA bitmap rendering in [`antic`].

mod antic;
mod palette;
mod screen;

use crate::Format;

const ATARI8: &str = "Atari 8-bit";

pub(super) static FORMATS: &[Format] = &[
    Format::new(ATARI8, "Graphics 7", &["gr7"], screen::decode_gr7),
    Format::new(ATARI8, "Graphics 8", &["gr8"], screen::decode_gr8),
    Format::new(ATARI8, "Graphics 9", &["gr9"], screen::decode_gr9),
    Format::new(ATARI8, "Graphics 10", &["g10"], screen::decode_g10),
    Format::new(ATARI8, "Graphics 11", &["g11"], screen::decode_g11),
    Format::new(ATARI8, "Micro Illustrator", &["mic"], screen::decode_mic),
];
