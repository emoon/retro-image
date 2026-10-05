//! One decoder at a time, so the fuzzer's effort isn't spread over every
//! format for each input.
//!
//! Input layout: `[format index (2 bytes, LE)] [companion split] [file...]`.
//! The byte after the index picks where the rest is cut: the part before the
//! cut is the main file, the part after is offered as every companion file.
//! Seeds in this layout come from `cargo run --example fuzz_seeds`.

#![no_main]

use libfuzzer_sys::fuzz_target;
use retro_image::{Companions, Format};

struct SameCompanion<'a>(&'a [u8]);

impl Companions for SameCompanion<'_> {
    fn get(&self, _extension: &str) -> Option<Vec<u8>> {
        (!self.0.is_empty()).then(|| self.0.to_vec())
    }

    fn get_named(&self, _file_name: &str) -> Option<Vec<u8>> {
        (!self.0.is_empty()).then(|| self.0.to_vec())
    }
}

fuzz_target!(|data: &[u8]| {
    let [lo, hi, split, rest @ ..] = data else {
        return;
    };
    let formats: Vec<&Format> = retro_image::formats().collect();
    let format = formats[usize::from(u16::from_le_bytes([*lo, *hi])) % formats.len()];
    // 0 means "no companion": the whole rest is the main file.
    let cut = if *split == 0 {
        rest.len()
    } else {
        rest.len() * usize::from(*split) / 256
    };
    let (main, companion) = rest.split_at(cut);
    if let Ok(image) = format.decode_with(main, &SameCompanion(companion)) {
        let pixels = image.width() as usize * image.height() as usize;
        assert_eq!(
            image.rgb().len(),
            pixels * 3,
            "{}: pixel buffer doesn't match the dimensions",
            format.name
        );
        // `rgba` stops at the shorter plane, so a short alpha plane fails here.
        assert_eq!(
            image.rgba().len(),
            pixels * 4,
            "{}: alpha plane doesn't match the dimensions",
            format.name
        );
    }
});
