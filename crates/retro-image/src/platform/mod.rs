//! Decoders, one module per platform.

mod zx_spectrum;

use crate::Format;

pub(crate) static FORMATS: &[Format] = &[zx_spectrum::SCR];
