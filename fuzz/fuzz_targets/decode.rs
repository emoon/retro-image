//! Every decoder must reject arbitrary input without panicking.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    for format in retro_image::formats() {
        let _ = format.decode(data);
    }
});
