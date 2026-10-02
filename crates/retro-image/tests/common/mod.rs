//! Sample corpus shared by the integration tests.
//!
//! `$RETRO_IMAGE_CORPUS`, default `<workspace>/corpus` (see CLEANROOM.md),
//! searched recursively: RECOIL's sample set at the top level, collected
//! samples under `extra/<platform group>/`.

use std::path::{Path, PathBuf};

pub struct Sample {
    pub path: PathBuf,
    /// Path relative to the corpus root, `/`-separated; unique.
    pub id: String,
    /// File name only; decoders choose formats by its extension.
    pub name: String,
}

/// All corpus files, sorted by `id`, or `None` if there is no corpus.
pub fn samples() -> Option<Vec<Sample>> {
    let root = std::env::var_os("RETRO_IMAGE_CORPUS")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus"));
    if !root.is_dir() {
        eprintln!("skipping: no corpus at {}", root.display());
        return None;
    }
    let mut samples = Vec::new();
    let mut dirs = vec![root.clone()];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.is_file() {
                let id = path
                    .strip_prefix(&root)
                    .unwrap()
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/");
                let name = path.file_name().unwrap().to_string_lossy().into_owned();
                samples.push(Sample { path, id, name });
            }
        }
    }
    samples.sort_by(|a, b| a.id.cmp(&b.id));
    Some(samples)
}
