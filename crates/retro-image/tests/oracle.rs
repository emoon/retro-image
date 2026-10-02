//! Compares our output with `recoil2png` (used as a black box) on every
//! corpus file we can decode.
//!
//! - Corpus: see `common/mod.rs`.
//! - Reference decoder: `$RECOIL2PNG`, default `recoil2png` on `PATH`.
//! - `$RETRO_IMAGE_PLATFORMS`: optional comma-separated `Format::platform`
//!   names; only formats of those platforms are tried.
//!
//! Skips (passes) when either is missing. RECOIL renders each file on its own,
//! without its companion files. Reference PNGs are cached under the cargo
//! target dir; delete it after upgrading RECOIL.
//!
//! RECOIL is the baseline, not the definition of correct. Files where we
//! deliberately differ (RECOIL crashes, rejects a valid file, or renders it
//! wrongly) are listed in `tests/divergences.tsv` with the evidence and the
//! fingerprint of our reviewed output, which is checked instead. Failure
//! messages print our fingerprint so a reviewed output can be recorded.

mod common;

use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
fn matches_recoil_on_corpus() {
    let recoil = std::env::var_os("RECOIL2PNG").unwrap_or_else(|| "recoil2png".into());
    let Some(samples) = common::samples() else {
        return;
    };
    if Command::new(&recoil).arg("--help").output().is_err() {
        eprintln!("skipping: cannot run {}", recoil.to_string_lossy());
        return;
    }
    let platforms = PlatformFilter::from_env();
    let cache = Path::new(env!("CARGO_TARGET_TMPDIR")).join("oracle-single-file");
    std::fs::create_dir_all(cache.join("isolated")).unwrap();

    let divergences = load_divergences();
    let mut matched = 0;
    let mut diverged = 0;
    let mut failures = Vec::new();
    for sample in &samples {
        let id = &sample.id;
        let Ok(data) = std::fs::read(&sample.path) else {
            continue; // removed while the test ran (e.g. by a sample collector)
        };
        let Some(ours) = retro_image::formats()
            .filter(|f| f.matches_filename(&sample.name) && platforms.selects(f.platform))
            .find_map(|f| f.decode(&data).ok())
        else {
            continue; // not supported yet
        };
        let fingerprint = fingerprint(&ours);
        if let Some(expected) = divergences.get(id.as_str()) {
            if fingerprint == *expected {
                diverged += 1;
            } else {
                failures.push(format!(
                    "{id}: differs from recorded divergence {expected} (ours: {fingerprint})"
                ));
            }
            continue;
        }
        let Some(reference_path) = reference_png(&recoil, sample, &cache) else {
            failures.push(format!(
                "{id}: we decode it but recoil2png rejects it (ours: {fingerprint})"
            ));
            continue;
        };
        match compare(&ours, &read_png(&reference_path)) {
            Ok(()) => matched += 1,
            Err(why) => failures.push(format!("{id}: {why} (ours: {fingerprint})")),
        }
    }
    eprintln!(
        "oracle: {matched} matched, {diverged} recorded divergences, {} failed, {} corpus files",
        failures.len(),
        samples.len()
    );
    assert!(failures.is_empty(), "mismatches:\n{}", failures.join("\n"));
}

/// Size and FNV-1a hash of the pixels, as written in `divergences.tsv`.
fn fingerprint(image: &retro_image::Image) -> String {
    let hash = image.rgb().iter().fold(0xcbf2_9ce4_8422_2325_u64, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    });
    format!("{}x{} {hash:016x}", image.width(), image.height())
}

/// Corpus id -> expected fingerprint. Every entry must state its evidence.
fn load_divergences() -> HashMap<String, String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/divergences.tsv");
    let text = std::fs::read_to_string(&path).unwrap();
    text.lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .map(|line| {
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
            (id.to_owned(), format!("{size} {hash}"))
        })
        .collect()
}

/// Renders `sample` with recoil2png, caching the PNG in `cache`.
///
/// recoil2png also reads companion files next to its input (e.g. `.S15`
/// next to `.SC5`), which a single-buffer decoder can't see, so it runs on a
/// lone copy of the file in `cache/isolated`. Returns `None` if RECOIL
/// rejects the file.
fn reference_png(recoil: &OsStr, sample: &common::Sample, cache: &Path) -> Option<PathBuf> {
    let png = cache.join(format!("{}.png", sample.id.replace('/', "__")));
    if png.exists() {
        return Some(png);
    }
    let lone = cache.join("isolated").join(&sample.name);
    std::fs::copy(&sample.path, &lone).unwrap();
    let status = Command::new(recoil)
        .arg("-o")
        .arg(&png)
        .arg(&lone)
        .status()
        .unwrap();
    std::fs::remove_file(&lone).unwrap();
    status.success().then_some(png)
}

struct PlatformFilter(Option<Vec<String>>);

impl PlatformFilter {
    fn from_env() -> Self {
        Self(
            std::env::var("RETRO_IMAGE_PLATFORMS")
                .ok()
                .map(|list| list.split(',').map(|p| p.trim().to_owned()).collect()),
        )
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
            .chunks_exact(4)
            .flat_map(|p| [p[0], p[1], p[2]])
            .collect(),
        png::ColorType::Grayscale => pixels.iter().flat_map(|&v| [v, v, v]).collect(),
        png::ColorType::GrayscaleAlpha => pixels
            .chunks_exact(2)
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
    let differing = ours
        .rgb()
        .chunks_exact(3)
        .zip(reference.rgb.chunks_exact(3))
        .filter(|(a, b)| a != b)
        .count();
    match differing {
        0 => Ok(()),
        n => Err(format!("{n} pixels differ")),
    }
}
