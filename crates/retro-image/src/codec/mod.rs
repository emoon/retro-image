//! Decompressors shared by several platforms.
//!
//! No external format knowledge: a module list; each codec cites its own
//! sources.

pub(crate) mod crunch_mania;
pub(crate) mod flf;
pub(crate) mod imploder;
pub(crate) mod inflate;
mod lz;
pub(crate) mod pack_ice;
pub(crate) mod packbits;
pub(crate) mod powerpacker;
pub(crate) mod rnc;
pub(crate) mod stos_pictbank;
pub(crate) mod xpk;
