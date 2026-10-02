//! Decoders must never panic, whatever the input: they return
//! `DecodeError` instead. Cheap, deterministic stand-in for fuzzing that runs
//! on stable with `cargo test`; deeper runs use `fuzz/` (cargo-fuzz).
//!
//! - Synthetic buffers go to every format (no corpus needed).
//! - Corpus files (see `common/mod.rs`) go to every format whole, then
//!   truncated and mutated to the formats that claim their extension.
//!
//! Kept small so `cargo test` stays quick; cargo-fuzz does the deep search.

mod common;

use std::panic::{self, AssertUnwindSafe};

use retro_image::Format;

const MUTATIONS_PER_FILE: usize = 32;
/// Every prefix up to this length is tried; longer files are sampled.
const ALL_PREFIXES_UP_TO: usize = 256;
const SAMPLED_PREFIXES: usize = 256;

#[test]
fn synthetic_inputs_do_not_panic() {
    let mut failures = Vec::new();
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let sizes = (0..=64).chain([255, 256, 257, 1000, 4096, 6912, 10003, 32000, 32034, 65535]);
    for size in sizes {
        for fill in [0x00, 0xff, 0x55] {
            check(
                retro_image::formats(),
                &vec![fill; size],
                &format!("{size} x {fill:#04x}"),
                &mut failures,
            );
        }
        let random: Vec<u8> = (0..size).map(|_| rng.byte()).collect();
        check(
            retro_image::formats(),
            &random,
            &format!("{size} random bytes"),
            &mut failures,
        );
    }
    report(failures);
}

#[test]
fn mutated_corpus_files_do_not_panic() {
    let Some(samples) = common::samples() else {
        return;
    };
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    let chunk_len = samples.len().div_ceil(threads).max(1);
    let failures: Vec<String> = std::thread::scope(|scope| {
        let workers: Vec<_> = samples
            .chunks(chunk_len)
            .map(|chunk| scope.spawn(|| chunk.iter().flat_map(check_sample).collect::<Vec<_>>()))
            .collect();
        workers
            .into_iter()
            .flat_map(|w| w.join().unwrap())
            .collect()
    });
    report(failures);
}

/// Feeds one corpus file, whole, truncated and mutated, to the decoders.
fn check_sample(sample: &common::Sample) -> Vec<String> {
    let mut failures = Vec::new();
    let name = &sample.id;
    let Ok(data) = std::fs::read(&sample.path) else {
        return failures; // removed while the test ran (e.g. by a sample collector)
    };
    check(retro_image::formats(), &data, name, &mut failures);

    let candidates: Vec<&Format> = retro_image::formats()
        .filter(|f| f.matches_filename(&sample.name))
        .collect();
    if candidates.is_empty() {
        return failures;
    }
    for len in prefix_lengths(data.len()) {
        check(
            candidates.iter().copied(),
            &data[..len],
            &format!("{name} truncated to {len}"),
            &mut failures,
        );
    }
    // Seeded per file so results don't depend on thread scheduling.
    let mut rng = Rng::seeded(name);
    for i in 0..MUTATIONS_PER_FILE {
        let mut mutated = data.clone();
        if !mutated.is_empty() {
            for _ in 0..=rng.below(8) {
                let pos = rng.below(mutated.len());
                mutated[pos] = rng.byte();
            }
        }
        check(
            candidates.iter().copied(),
            &mutated,
            &format!("{name} mutation {i}"),
            &mut failures,
        );
    }
    failures
}

fn prefix_lengths(len: usize) -> impl Iterator<Item = usize> {
    let sampled_step = (len / SAMPLED_PREFIXES).max(1);
    (0..len.min(ALL_PREFIXES_UP_TO)).chain((ALL_PREFIXES_UP_TO..len).step_by(sampled_step))
}

/// Runs every format in `formats` on `data`, recording panics.
fn check<'a>(
    formats: impl Iterator<Item = &'a Format>,
    data: &[u8],
    input: &str,
    failures: &mut Vec<String>,
) {
    for format in formats {
        if let Err(payload) = panic::catch_unwind(AssertUnwindSafe(|| format.decode(data))) {
            let message = payload
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_default();
            failures.push(format!(
                "{} / {}: {input}: {message}",
                format.platform, format.name
            ));
        }
    }
}

/// Fails with a per-format count of panics, then the first few in full.
/// Each failure starts with "platform / format name: ".
fn report(failures: Vec<String>) {
    const SHOWN: usize = 50;
    if failures.is_empty() {
        return;
    }
    let mut per_format = std::collections::BTreeMap::<&str, usize>::new();
    for failure in &failures {
        let format = failure.split(": ").next().unwrap_or_default();
        *per_format.entry(format).or_default() += 1;
    }
    let summary: Vec<String> = per_format
        .iter()
        .map(|(format, count)| format!("{count:>6}  {format}"))
        .collect();
    let first: Vec<&str> = failures.iter().take(SHOWN).map(String::as_str).collect();
    panic!(
        "{} panics in {} formats:\n{}\nfirst {SHOWN}:\n{}",
        failures.len(),
        per_format.len(),
        summary.join("\n"),
        first.join("\n")
    );
}

/// xorshift64: deterministic, so failures reproduce.
struct Rng(u64);

impl Rng {
    /// FNV-1a of `text`; never zero, which xorshift can't leave.
    fn seeded(text: &str) -> Self {
        let hash = text.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
            (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
        });
        Self(hash | 1)
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn byte(&mut self) -> u8 {
        self.next() as u8
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}
