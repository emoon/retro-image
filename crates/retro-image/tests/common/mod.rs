//! Sample corpus shared by the integration tests.
//!
//! `$RETRO_IMAGE_CORPUS`, default `<workspace>/corpus` (see CLEANROOM.md),
//! searched recursively: RECOIL's sample set at the top level, collected
//! samples under `extra/<platform group>/`.

// Each test binary uses a different subset of these helpers.
#![allow(dead_code)]

use std::cell::RefCell;
use std::path::{Path, PathBuf};

pub struct Sample {
    pub path: PathBuf,
    /// Path relative to the corpus root, `/`-separated; unique.
    pub id: String,
    /// File name only; decoders choose formats by its extension.
    pub name: String,
}

impl Sample {
    /// Other files in the same directory with the same name before the
    /// extension (case-insensitive): candidates for companion files.
    pub fn siblings(&self) -> Vec<PathBuf> {
        let Some(stem) = self.path.file_stem().and_then(|s| s.to_str()) else {
            return Vec::new();
        };
        let Ok(entries) = std::fs::read_dir(self.path.parent().unwrap()) else {
            return Vec::new();
        };
        let mut siblings: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.is_file() && *p != self.path)
            .filter(|p| {
                p.file_stem()
                    .and_then(|s| s.to_str())
                    .is_some_and(|s| s.eq_ignore_ascii_case(stem))
            })
            .collect();
        siblings.sort();
        siblings
    }
}

/// Companion files taken from a list of sibling paths, and named files from
/// the main file's directory. Remembers the named files it served.
pub struct SiblingFiles<'a> {
    siblings: &'a [PathBuf],
    directory: &'a Path,
    named: RefCell<Vec<PathBuf>>,
}

impl<'a> SiblingFiles<'a> {
    pub fn new(siblings: &'a [PathBuf], directory: &'a Path) -> Self {
        Self {
            siblings,
            directory,
            named: RefCell::new(Vec::new()),
        }
    }

    /// The files served by `get_named` so far, without repeats.
    pub fn named_files(&self) -> Vec<PathBuf> {
        self.named.borrow().clone()
    }
}

impl retro_image::Companions for SiblingFiles<'_> {
    fn get(&self, extension: &str) -> Option<std::borrow::Cow<'_, [u8]>> {
        let path = self.siblings.iter().find(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case(extension))
        })?;
        std::fs::read(path).ok().map(Into::into)
    }

    fn get_named(&self, file_name: &str) -> Option<std::borrow::Cow<'_, [u8]>> {
        let name = file_name.rsplit(['/', '\\']).next()?;
        if matches!(name, "" | "." | "..") {
            return None;
        }
        let path = self.directory.join(name);
        let data = std::fs::read(&path).ok()?;
        let mut named = self.named.borrow_mut();
        if !named.contains(&path) {
            named.push(path);
        }
        Some(data.into())
    }
}

/// Whether `$RETRO_IMAGE_REQUIRE_ORACLE=1`: a missing corpus or reference
/// decoder then fails the tests instead of skipping them.
pub fn strict() -> bool {
    std::env::var_os("RETRO_IMAGE_REQUIRE_ORACLE").is_some_and(|v| v == "1")
}

/// Skips the calling test with `why`, or fails it in strict mode.
pub fn skip(why: &str) {
    assert!(!strict(), "{why} (RETRO_IMAGE_REQUIRE_ORACLE=1)");
    eprintln!("skipping: {why}");
}

/// All corpus files, sorted by `id`, or `None` if there is no corpus.
pub fn samples() -> Option<Vec<Sample>> {
    let root = std::env::var_os("RETRO_IMAGE_CORPUS")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus"));
    if !root.is_dir() {
        skip(&format!("no corpus at {}", root.display()));
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
