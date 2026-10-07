//! One decoder at a time, so the fuzzer's effort isn't spread over every
//! format for each input.
//!
//! Input layout: `[format index (2 bytes, LE)] [companion split] [file...]`.
//! The byte after the index picks where the rest is cut: the part before the
//! cut is the main file, the part after is offered as every companion file.
//! Seeds in this layout come from `cargo run --example fuzz_seeds`.

#![no_main]

use libfuzzer_sys::fuzz_target;
use std::borrow::Cow;
use std::time::{Duration, Instant};

use retro_image::{Companions, Format};

/// A decode slower than this is a bug. libfuzzer's own `-timeout` is
/// whole-process and in whole seconds, and it let a 4.3 s decode through.
const SLOW: Duration = Duration::from_secs(2);

struct SameCompanion<'a>(&'a [u8]);

impl Companions for SameCompanion<'_> {
    fn get(&self, _extension: &str) -> Option<Cow<'_, [u8]>> {
        (!self.0.is_empty()).then(|| Cow::Borrowed(self.0))
    }

    fn get_named(&self, _file_name: &str) -> Option<Cow<'_, [u8]>> {
        (!self.0.is_empty()).then(|| Cow::Borrowed(self.0))
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
    let started = Instant::now();
    let decoded = format.decode_with(main, &SameCompanion(companion));
    let took = started.elapsed();
    assert!(
        took < SLOW,
        "{}: decoding {} bytes took {took:?}",
        format.name(),
        data.len()
    );
    if let Ok(image) = decoded {
        let pixels = image.width() as usize * image.height() as usize;
        assert_eq!(
            image.rgb().len(),
            pixels * 3,
            "{}: pixel buffer doesn't match the dimensions",
            format.name()
        );
        // `rgba` stops at the shorter plane, so a short alpha plane fails here.
        assert_eq!(
            image.rgba().len(),
            pixels * 4,
            "{}: alpha plane doesn't match the dimensions",
            format.name()
        );
    }
});
