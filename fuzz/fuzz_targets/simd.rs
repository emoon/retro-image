//! Every SIMD level of every pixel primitive must match the scalar
//! reference on arbitrary input.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    retro_image::fuzz_check_simd_levels(data);
});
