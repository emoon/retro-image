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

mod common;

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

    let mut matched = 0;
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
        let Some(reference_path) = reference_png(&recoil, sample, &cache) else {
            failures.push(format!("{id}: we decode it but recoil2png rejects it"));
            continue;
        };
        match compare(&ours, &read_png(&reference_path)) {
            Ok(()) => matched += 1,
            Err(why) => failures.push(format!("{id}: {why}")),
        }
    }
    eprintln!(
        "oracle: {matched} matched, {} failed, {} corpus files",
        failures.len(),
        samples.len()
    );
    assert!(failures.is_empty(), "mismatches:\n{}", failures.join("\n"));
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
