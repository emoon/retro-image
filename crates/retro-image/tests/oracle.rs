//! Compares our output with `recoil2png` (used as a black box) on every
//! corpus file we can decode.
//!
//! - Corpus: `$RETRO_IMAGE_CORPUS`, default `<workspace>/corpus` (see CLEANROOM.md).
//! - Reference decoder: `$RECOIL2PNG`, default `recoil2png` on `PATH`.
//!
//! Skips (passes) when either is missing. Reference PNGs are cached under the
//! cargo target dir; delete it after upgrading RECOIL.

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
fn matches_recoil_on_corpus() {
    let corpus = std::env::var_os("RETRO_IMAGE_CORPUS")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus"));
    let recoil = std::env::var_os("RECOIL2PNG").unwrap_or_else(|| "recoil2png".into());
    let Ok(entries) = std::fs::read_dir(&corpus) else {
        eprintln!("skipping: no corpus at {}", corpus.display());
        return;
    };
    if Command::new(&recoil).arg("--help").output().is_err() {
        eprintln!("skipping: cannot run {}", recoil.to_string_lossy());
        return;
    }
    let cache = Path::new(env!("CARGO_TARGET_TMPDIR")).join("oracle");
    std::fs::create_dir_all(&cache).unwrap();

    let mut paths: Vec<PathBuf> = entries
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_file())
        .collect();
    paths.sort();
    let mut matched = 0;
    let mut failures = Vec::new();
    for path in &paths {
        let name = path.file_name().unwrap().to_string_lossy();
        let Ok(ours) = retro_image::decode(&name, &std::fs::read(path).unwrap()) else {
            continue; // not supported yet
        };
        let reference_path = cache.join(format!("{name}.png"));
        if !reference_path.exists() {
            let status = Command::new(&recoil)
                .arg("-o")
                .arg(&reference_path)
                .arg(path)
                .status()
                .unwrap();
            if !status.success() {
                failures.push(format!("{name}: we decode it but recoil2png rejects it"));
                continue;
            }
        }
        match compare(&ours, &read_png(&reference_path)) {
            Ok(()) => matched += 1,
            Err(why) => failures.push(format!("{name}: {why}")),
        }
    }
    eprintln!(
        "oracle: {matched} matched, {} failed, {} corpus files",
        failures.len(),
        paths.len()
    );
    assert!(failures.is_empty(), "mismatches:\n{}", failures.join("\n"));
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
