//! FM Towns.
//!
//! PIC pictures saved on the FM Towns are decoded by `sharp_x68000::pic`, which
//! lists its sources.

use super::nec_pc::Machine;
use super::sharp_x68000::pic;
use crate::Format;

pub(super) static FORMATS: &[Format] = &[Format::new("FM Towns", "PIC", &["pic"], |d| {
    pic::decode_pic(d, Machine::FmTowns)
})];
