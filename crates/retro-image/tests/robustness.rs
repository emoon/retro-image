//! Decoders must never panic, whatever the input: they return
//! `DecodeError` instead. Cheap, deterministic stand-in for fuzzing that runs
//! on stable with `cargo test`; deeper runs use `fuzz/` (cargo-fuzz).
//!
//! - Synthetic buffers go to every format (no corpus needed).
//! - Corpus files (see `common/mod.rs`) go to every format whole, then
//!   truncated and mutated to the formats that claim their extension or
//!   accept the unmodified file by signature. Half the mutations hit the
//!   header, where dimensions and offsets live. Formats that read companion
//!   files also get truncated and mutated copies of those.
//! - A decode that succeeds must return a well-formed, non-empty image, and
//!   one that runs for more than `HANG_SECONDS` aborts the test naming its
//!   input.
//!
//! Kept small so `cargo test` stays quick; cargo-fuzz does the deep search.

mod common;

use std::panic::{self, AssertUnwindSafe};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use retro_image::Format;

const MUTATIONS_PER_FILE: usize = 32;
/// Every prefix up to this length is tried; longer files are sampled.
const ALL_PREFIXES_UP_TO: usize = 256;
const SAMPLED_PREFIXES: usize = 256;
/// Mutations aimed at the first bytes of a file, where the header is.
const HEADER_BYTES: usize = 256;
/// Values that most often break size arithmetic.
const EDGE_BYTES: [u8; 4] = [0x00, 0xff, 0x80, 0x7f];
const HANG_SECONDS: u64 = 60;
/// Same cap as `image::check_size`.
const MAX_PIXELS: usize = 1 << 26;

/// What each worker is decoding and since when, for the watchdog.
static RUNNING: Mutex<Vec<(usize, Instant, String)>> = Mutex::new(Vec::new());

/// Aborts the process, naming the input, if a decode runs for too long. An
/// infinite loop would otherwise hang CI with no hint of where.
fn spawn_watchdog() {
    std::thread::spawn(|| {
        loop {
            std::thread::sleep(Duration::from_secs(1));
            for (_, since, input) in RUNNING.lock().unwrap().iter() {
                if since.elapsed() > Duration::from_secs(HANG_SECONDS) {
                    eprintln!("hang: still decoding {input} after {HANG_SECONDS}s");
                    std::process::abort();
                }
            }
        }
    });
}

/// Marks this thread as decoding `input` until the guard drops.
struct Running(usize);

impl Running {
    fn new(input: &str) -> Self {
        let id = thread_id();
        let mut running = RUNNING.lock().unwrap();
        running.retain(|(t, ..)| *t != id);
        running.push((id, Instant::now(), input.to_owned()));
        Self(id)
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        RUNNING.lock().unwrap().retain(|(t, ..)| *t != self.0);
    }
}

fn thread_id() -> usize {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::thread::current().id().hash(&mut hasher);
    hasher.finish() as usize
}

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
    spawn_watchdog();
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

    // Formats that claim the extension, and those that take the unmodified
    // file by signature (e.g. under a name no format claims).
    let candidates: Vec<&Format> = retro_image::candidates(&sample.name)
        .filter(|f| f.matches_filename(&sample.name) || f.decode(&data).is_ok())
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
        let mutated = mutate(&data, &mut rng, i);
        check(
            candidates.iter().copied(),
            &mutated,
            &format!("{name} mutation {i}"),
            &mut failures,
        );
    }
    check_companions(sample, &candidates, &data, &mut failures);
    failures
}

/// Copy of `data` with 1-9 bytes changed. Even-numbered rounds aim at the
/// header; every other change is an edge value instead of a random byte.
fn mutate(data: &[u8], rng: &mut Rng, round: usize) -> Vec<u8> {
    let mut mutated = data.to_vec();
    if mutated.is_empty() {
        return mutated;
    }
    let reach = if round.is_multiple_of(2) {
        mutated.len().min(HEADER_BYTES)
    } else {
        mutated.len()
    };
    for n in 0..=rng.below(8) {
        let pos = rng.below(reach);
        mutated[pos] = if n % 2 == 0 {
            rng.byte()
        } else {
            EDGE_BYTES[rng.below(EDGE_BYTES.len())]
        };
    }
    mutated
}

/// Companion files served from memory, by extension.
struct MemoryCompanions(Vec<(String, Vec<u8>)>);

impl retro_image::Companions for MemoryCompanions {
    fn get(&self, extension: &str) -> Option<Vec<u8>> {
        self.0
            .iter()
            .find(|(e, _)| e.eq_ignore_ascii_case(extension))
            .map(|(_, data)| data.clone())
    }

    fn get_named(&self, _file_name: &str) -> Option<Vec<u8>> {
        None
    }
}

/// Decodes the unmodified main file with truncated and mutated copies of its
/// sibling files, for the formats that read companions.
fn check_companions(
    sample: &common::Sample,
    candidates: &[&Format],
    data: &[u8],
    failures: &mut Vec<String>,
) {
    let readers: Vec<&Format> = candidates
        .iter()
        .copied()
        .filter(|f| f.uses_companions())
        .collect();
    if readers.is_empty() {
        return;
    }
    let mut rng = Rng::seeded(&format!("{} companions", sample.id));
    for sibling in sample.siblings() {
        let (Some(ext), Ok(bytes)) = (
            sibling.extension().and_then(|e| e.to_str()),
            std::fs::read(&sibling),
        ) else {
            continue;
        };
        let variants = (0..8)
            .map(|i| mutate(&bytes, &mut rng, i))
            .chain([bytes[..bytes.len() / 2].to_vec(), Vec::new()]);
        for (i, variant) in variants.enumerate() {
            let companions = MemoryCompanions(vec![(ext.to_owned(), variant)]);
            let input = format!("{} with damaged .{ext} #{i}", sample.id);
            let _running = Running::new(&input);
            for format in &readers {
                let outcome =
                    panic::catch_unwind(AssertUnwindSafe(|| format.decode_with(data, &companions)));
                record(format, &input, outcome, failures);
            }
        }
    }
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
    let _running = Running::new(input);
    for format in formats {
        let outcome = panic::catch_unwind(AssertUnwindSafe(|| format.decode(data)));
        record(format, input, outcome, failures);
    }
}

type Outcome = std::thread::Result<Result<retro_image::Image, retro_image::DecodeError>>;

/// Records a panic, or a successful decode that breaks the `Image` contract.
fn record(format: &Format, input: &str, outcome: Outcome, failures: &mut Vec<String>) {
    let who = format!("{} / {}", format.platform, format.name);
    match outcome {
        Err(payload) => {
            let message = payload
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_default();
            failures.push(format!("{who}: {input}: {message}"));
        }
        Ok(Ok(image)) => {
            let pixels = image.width() as usize * image.height() as usize;
            if pixels == 0 || pixels > MAX_PIXELS || image.rgb().len() != pixels * 3 {
                failures.push(format!(
                    "{who}: {input}: malformed image {}x{} with {} bytes",
                    image.width(),
                    image.height(),
                    image.rgb().len()
                ));
            }
        }
        Ok(Err(_)) => {}
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
