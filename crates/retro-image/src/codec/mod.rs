//! Decompressors shared by several platforms.
//!
//! No external format knowledge: a module list; each codec cites its own
//! sources.

pub(crate) mod flf;
pub(crate) mod gx;
pub(crate) mod inflate;
pub(crate) mod pack_ice;
pub(crate) mod packbits;
pub(crate) mod powerpacker;
pub(crate) mod stos_pictbank;
