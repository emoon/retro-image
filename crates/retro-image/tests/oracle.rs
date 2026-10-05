//! Compares our output with `recoil2png` (used as a black box) on every
//! corpus file we can decode.
//!
//! - Corpus: see `common/mod.rs`.
//! - Reference decoder: `$RECOIL2PNG`, default `recoil2png` on `PATH`.
//! - `$RETRO_IMAGE_PLATFORMS`: optional comma-separated `Format::platform`
//!   names; only formats of those platforms are tried.
//!
//! Skips (passes) when either is missing, unless `RETRO_IMAGE_REQUIRE_ORACLE=1`
//! makes that a failure. RECOIL renders each file on its own,
//! without its companion files. Reference PNGs are cached under the cargo
//! target dir; delete it after upgrading RECOIL.
//!
//! RECOIL is the baseline, not the definition of correct. Files where we
//! deliberately differ (RECOIL crashes, rejects a valid file, or renders it
//! wrongly) are listed in `tests/divergences/*.tsv` with the evidence and the
//! fingerprint of our reviewed output, which is checked instead. Failure
//! messages print our fingerprint so a reviewed output can be recorded.
//!
//! Two more checks keep the test from passing by doing less. A recorded
//! divergence whose corpus file exists must be reached, and a file whose
//! extension some format claims must not be rejected by all of them while
//! RECOIL renders it (known gaps are listed in `tests/gaps.tsv`).

mod common;

use retro_image::Format;

use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

#[test]
fn matches_recoil_on_corpus() {
    let recoil = std::env::var_os("RECOIL2PNG").unwrap_or_else(|| "recoil2png".into());
    let Some(samples) = common::samples() else {
        return;
    };
    if Command::new(&recoil).arg("--help").output().is_err() {
        common::skip(&format!("cannot run {}", recoil.to_string_lossy()));
        return;
    }
    let platforms = PlatformFilter::from_env();
    let cache = Path::new(env!("CARGO_TARGET_TMPDIR")).join("oracle-single-file");
    std::fs::create_dir_all(&cache).unwrap();
    let mut oracle = Oracle {
        recoil,
        cache,
        divergences: load_divergences(),
        gaps: load_gaps(),
        used: HashSet::new(),
        undecoded: Vec::new(),
        matched: 0,
        diverged: 0,
        failures: Vec::new(),
    };

    for sample in &samples {
        let Ok(data) = std::fs::read(&sample.path) else {
            continue; // removed while the test ran (e.g. by a sample collector)
        };
        // Same order as `retro_image::decode`: extension, then signature.
        let candidates: Vec<&Format> = retro_image::candidates(&sample.name)
            .filter(|f| platforms.selects(f.platform))
            .collect();
        let alone = candidates
            .iter()
            .find_map(|f| f.decode(&data).ok().map(|image| (*f, image)));
        if let Some((_, ours)) = &alone {
            oracle.check(&sample.id, ours, &[sample.path.as_path()]);
        }

        // Formats with companion files are also checked with them present,
        // against RECOIL given the same files. That includes formats whose
        // main file doesn't decode alone (e.g. a picture without its colours).
        let siblings = sample.siblings();
        let companions = common::SiblingFiles::new(&siblings, sample.path.parent().unwrap());
        let decoded_alone = alone.is_some();
        let with_companions = match alone {
            Some((format, _)) => Some(format).filter(|f| f.uses_companions()),
            None => candidates
                .iter()
                .copied()
                .find(|f| f.uses_companions() && f.decode_with(&data, &companions).is_ok()),
        };
        let Some(format) = with_companions else {
            // Not decodable: fine unless the extension is claimed and RECOIL renders it.
            if !decoded_alone
                && candidates.iter().any(|f| f.matches_filename(&sample.name))
                && oracle
                    .reference_png(&sample.id, &[sample.path.as_path()])
                    .is_some()
            {
                oracle.undecoded.push(sample.id.clone());
            }
            continue;
        };
        let id = format!("{} +companions", sample.id);
        match format.decode_with(&data, &companions) {
            Ok(ours) => {
                // Files a list names (e.g. VSC) count as companions too.
                let named = companions.named_files();
                if siblings.is_empty() && named.is_empty() {
                    continue;
                }
                let mut inputs = vec![sample.path.as_path()];
                inputs.extend(siblings.iter().map(PathBuf::as_path));
                inputs.extend(named.iter().map(PathBuf::as_path));
                oracle.check(&id, &ours, &inputs);
            }
            Err(_) => oracle
                .failures
                .push(format!("{id}: rejected with companions")),
        }
    }
    oracle.finish(&samples, &platforms);
    eprintln!(
        "oracle: {} matched, {} recorded divergences, {} failed, {} corpus files",
        oracle.matched,
        oracle.diverged,
        oracle.failures.len(),
        samples.len()
    );
    assert!(
        oracle.failures.is_empty(),
        "mismatches:\n{}",
        oracle.failures.join("\n")
    );
}

struct Oracle {
    recoil: OsString,
    cache: PathBuf,
    divergences: HashMap<String, String>,
    /// Corpus ids RECOIL renders but every extension-claiming format rejects,
    /// with the evidence for each.
    gaps: HashMap<String, String>,
    /// Ids of the divergences and gaps this run reached.
    used: HashSet<String>,
    undecoded: Vec<String>,
    matched: usize,
    diverged: usize,
    failures: Vec<String>,
}

impl Oracle {
    /// Checks our decode of `inputs` (main file first, then companions)
    /// against the recorded divergence for `id`, or else against RECOIL.
    fn check(&mut self, id: &str, ours: &retro_image::Image, inputs: &[&Path]) {
        let fingerprint = fingerprint(ours);
        self.used.insert(id.to_owned());
        if let Some(expected) = self.divergences.get(id) {
            if fingerprint == *expected {
                self.diverged += 1;
            } else {
                self.failures.push(format!(
                    "{id}: differs from recorded divergence {expected} (ours: {fingerprint})"
                ));
            }
            return;
        }
        let Some(reference) = self.reference_png(id, inputs) else {
            self.failures.push(format!(
                "{id}: we decode it but recoil2png rejects it (ours: {fingerprint})"
            ));
            return;
        };
        match compare(ours, &read_png(&reference)) {
            Ok(()) => self.matched += 1,
            Err(why) => self
                .failures
                .push(format!("{id}: {why} (ours: {fingerprint})")),
        }
    }

    /// Adds the failures only visible after the whole corpus ran: undecoded
    /// files that are not known gaps, and recorded rows that were never reached.
    fn finish(&mut self, samples: &[common::Sample], platforms: &PlatformFilter) {
        for id in std::mem::take(&mut self.undecoded) {
            if self.gaps.contains_key(&id) {
                self.used.insert(id);
            } else {
                self.failures
                    .push(format!("{id}: recoil2png renders it but we reject it"));
            }
        }
        if platforms.0.is_some() {
            return; // a filtered run reaches only some rows
        }
        let ids: HashSet<&str> = samples.iter().map(|s| s.id.as_str()).collect();
        let rows = self.divergences.keys().chain(self.gaps.keys());
        for id in rows {
            let file = id.strip_suffix(" +companions").unwrap_or(id);
            if ids.contains(file) && !self.used.contains(id) {
                self.failures
                    .push(format!("{id}: recorded row was never reached"));
            }
        }
    }

    /// Renders `inputs[0]` with recoil2png, caching the PNG under `id`.
    ///
    /// recoil2png also reads companion files next to its input (e.g. `.S15`
    /// next to `.SC5`), so it runs on copies of exactly `inputs` in an
    /// otherwise empty directory. Returns `None` if RECOIL rejects the file.
    fn reference_png(&self, id: &str, inputs: &[&Path]) -> Option<PathBuf> {
        // Keyed by content too, so a replaced file or a changed set of
        // companions is rendered afresh.
        let hash = inputs.iter().fold(FNV_OFFSET, |h, input| {
            std::fs::read(input)
                .unwrap_or_default()
                .iter()
                .fold(h, |h, &b| fnv_step(h, b))
        });
        let png = self.cache.join(format!(
            "{}-{hash:016x}.png",
            id.replace('/', "__").replace(' ', "_")
        ));
        if png.exists() {
            return Some(png);
        }
        let isolated = self.cache.join("isolated");
        let _ = std::fs::remove_dir_all(&isolated);
        std::fs::create_dir_all(&isolated).unwrap();
        for input in inputs {
            std::fs::copy(input, isolated.join(input.file_name().unwrap())).unwrap();
        }
        let main = isolated.join(inputs[0].file_name().unwrap());
        let mut child = Command::new(&self.recoil)
            .arg("-o")
            .arg(&png)
            .arg(&main)
            .spawn()
            .unwrap();
        let succeeded = wait_with_timeout(&mut child, RECOIL_TIMEOUT);
        std::fs::remove_dir_all(&isolated).unwrap();
        if !succeeded {
            let _ = std::fs::remove_file(&png); // don't cache partial output
        }
        succeeded.then_some(png)
    }
}

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;

fn fnv_step(hash: u64, byte: u8) -> u64 {
    (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
}

/// Size and FNV-1a hash of the pixels, as written in `divergences/*.tsv`:
/// the RGB bytes, then the alpha plane if the image has one.
fn fingerprint(image: &retro_image::Image) -> String {
    let mut hash = image.rgb().iter().fold(FNV_OFFSET, |h, &b| fnv_step(h, b));
    if image.has_alpha() {
        let rgba = image.rgba();
        hash = (rgba.as_chunks::<4>().0.iter()).fold(hash, |h, pixel| fnv_step(h, pixel[3]));
    }
    format!("{}x{} {hash:016x}", image.width(), image.height())
}

/// Corpus id -> expected fingerprint, from every `tests/divergences/*.tsv`.
/// Every entry must state its evidence.
fn load_divergences() -> HashMap<String, String> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/divergences");
    let mut divergences = HashMap::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "tsv") {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        for line in text.lines() {
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }
            let fields: Vec<&str> = line.split('\t').collect();
            let [id, size, hash, evidence] = fields[..] else {
                panic!(
                    "{}: expected 4 tab-separated fields: {line}",
                    path.display()
                );
            };
            assert!(
                !evidence.trim().is_empty(),
                "{id}: divergence needs evidence"
            );
            let previous = divergences.insert(id.to_owned(), format!("{size} {hash}"));
            assert!(previous.is_none(), "{id}: recorded twice");
        }
    }
    divergences
}

/// Corpus id -> evidence, from `tests/gaps.tsv`: files RECOIL
/// renders that no format of ours decodes yet.
fn load_gaps() -> HashMap<String, String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/gaps.tsv");
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let mut gaps = HashMap::new();
    for line in text
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
    {
        let Some((id, evidence)) = line.split_once('\t') else {
            panic!("{}: expected id<TAB>evidence: {line}", path.display());
        };
        assert!(!evidence.trim().is_empty(), "{id}: gap needs evidence");
        assert!(
            gaps.insert(id.to_owned(), evidence.to_owned()).is_none(),
            "{id}: recorded twice"
        );
    }
    gaps
}

/// RECOIL can misbehave on hostile input; treat a hang as a rejection.
const RECOIL_TIMEOUT: Duration = Duration::from_secs(30);

/// Whether `child` exited successfully within `timeout`; kills it otherwise.
fn wait_with_timeout(child: &mut Child, timeout: Duration) -> bool {
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status.success();
        }
        if start.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            return false;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

struct PlatformFilter(Option<Vec<String>>);

impl PlatformFilter {
    /// Panics on a name that is no platform: a typo would select nothing and
    /// the test would pass without checking anything.
    fn from_env() -> Self {
        let list: Option<Vec<String>> = std::env::var("RETRO_IMAGE_PLATFORMS")
            .ok()
            .map(|list| list.split(',').map(|p| p.trim().to_owned()).collect());
        for name in list.iter().flatten() {
            assert!(
                retro_image::formats().any(|f| f.platform == name),
                "RETRO_IMAGE_PLATFORMS: unknown platform {name:?}"
            );
        }
        Self(list)
    }

    fn selects(&self, platform: &str) -> bool {
        self.0
            .as_ref()
            .is_none_or(|list| list.iter().any(|p| p == platform))
    }
}

struct Reference {
    width: u32,
    height: u32,
    rgb: Vec<u8>,
}

fn read_png(path: &Path) -> Reference {
    let mut decoder = png::Decoder::new(BufReader::new(File::open(path).unwrap()));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    let pixels = &buf[..info.buffer_size()];
    let rgb = match info.color_type {
        png::ColorType::Rgb => pixels.to_vec(),
        png::ColorType::Rgba => pixels
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[1], p[2]])
            .collect(),
        png::ColorType::Grayscale => pixels.iter().flat_map(|&v| [v, v, v]).collect(),
        png::ColorType::GrayscaleAlpha => pixels
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[0], p[0]])
            .collect(),
        png::ColorType::Indexed => unreachable!("expanded by normalize_to_color8"),
    };
    Reference {
        width: info.width,
        height: info.height,
        rgb,
    }
}

fn compare(ours: &retro_image::Image, reference: &Reference) -> Result<(), String> {
    if (ours.width(), ours.height()) != (reference.width, reference.height) {
        return Err(format!(
            "size {}x{}, expected {}x{}",
            ours.width(),
            ours.height(),
            reference.width,
            reference.height
        ));
    }
    // RECOIL has no alpha, so the color under a transparent pixel is whatever
    // it drew there; only pixels that show are compared.
    let rgba = ours.rgba();
    let differing = ours
        .rgb()
        .as_chunks::<3>()
        .0
        .iter()
        .zip(rgba.as_chunks::<4>().0)
        .zip(reference.rgb.as_chunks::<3>().0)
        .filter(|((a, shown), b)| shown[3] != 0 && a != b)
        .count();
    match differing {
        0 => Ok(()),
        n => Err(format!("{n} pixels differ")),
    }
}
