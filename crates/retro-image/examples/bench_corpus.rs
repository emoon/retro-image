//! Times every decoder over the sample corpus, to find where time goes:
//! `cargo run --release --example bench_corpus -- [corpus dir] [top N] [runs]`
//!
//! Each file is decoded by the first format that accepts it (companions
//! taken from sibling files) and timed as the best of several runs.
//! Prints the formats and files with the highest total time and the
//! highest cost per pixel.
//!
//! With `DIGEST=<file>` set, also writes a hash of every decoded picture,
//! so two builds can be compared with `diff` to prove they decode alike.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use retro_image::Companions;

struct Siblings(Vec<PathBuf>);

impl Companions for Siblings {
    fn get(&self, extension: &str) -> Option<Vec<u8>> {
        let path = self.0.iter().find(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case(extension))
        })?;
        std::fs::read(path).ok()
    }
}

#[derive(Default)]
struct Totals {
    files: usize,
    time: Duration,
    pixels: u64,
    bytes: u64,
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, out);
        } else {
            out.push(path);
        }
    }
}

fn best_of(
    format: &retro_image::Format,
    data: &[u8],
    companions: &Siblings,
    runs: usize,
) -> Duration {
    let mut best = Duration::MAX;
    let budget = Instant::now();
    for _ in 0..runs {
        let start = Instant::now();
        let image = format.decode_with(data, companions);
        std::hint::black_box(&image);
        best = best.min(start.elapsed());
        if budget.elapsed() > Duration::from_millis(100) {
            break;
        }
    }
    best
}

/// Like `best_of`, but through `decode_with`, which tries every candidate
/// format in turn until one accepts the file.
fn best_of_dispatch(name: &str, data: &[u8], companions: &Siblings, runs: usize) -> Duration {
    let mut best = Duration::MAX;
    for _ in 0..runs.min(5) {
        let start = Instant::now();
        std::hint::black_box(retro_image::decode_with(name, data, companions).ok());
        best = best.min(start.elapsed());
    }
    best
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = args.first().map_or("corpus", String::as_str);
    let top: usize = args.get(1).and_then(|n| n.parse().ok()).unwrap_or(25);
    let runs: usize = args.get(2).and_then(|n| n.parse().ok()).unwrap_or(50);
    let mut paths = Vec::new();
    walk(Path::new(root), &mut paths);
    paths.sort();

    let mut formats: HashMap<String, Totals> = HashMap::new();
    let want_digest = std::env::var_os("DIGEST").is_some();
    let mut digest = String::new();
    let mut dispatch_total = Duration::ZERO;
    let mut slow_dispatch = Vec::new();
    let mut files: Vec<(Duration, u64, String)> = Vec::new();
    for path in &paths {
        let Ok(data) = std::fs::read(path) else {
            continue;
        };
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let stem = path.file_stem().map(|s| s.to_ascii_lowercase());
        let siblings = Siblings(
            std::fs::read_dir(path.parent().unwrap())
                .unwrap()
                .flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.is_file()
                        && p != path
                        && p.file_stem().map(|s| s.to_ascii_lowercase()) == stem
                })
                .collect(),
        );
        let found = retro_image::candidates(&name)
            .find_map(|f| f.decode_with(&data, &siblings).ok().map(|i| (f, i)));
        let Some((format, image)) = found else {
            continue;
        };
        if want_digest {
            use std::hash::{DefaultHasher, Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            (image.width(), image.height(), image.rgb()).hash(&mut hasher);
            digest += &format!("{} {:016x}\n", path.display(), hasher.finish());
        }
        let time = best_of(format, &data, &siblings, runs);
        let pixels = u64::from(image.width()) * u64::from(image.height());
        let t = formats
            .entry(format!("{} / {}", format.platform, format.name))
            .or_default();
        t.files += 1;
        t.time += time;
        t.pixels += pixels;
        t.bytes += data.len() as u64;
        let dispatch = best_of_dispatch(&name, &data, &siblings, runs);
        dispatch_total += dispatch;
        files.push((time, pixels, name));
        if dispatch > time * 2 + Duration::from_micros(50) {
            slow_dispatch.push((
                dispatch - time,
                format.name,
                files.last().unwrap().2.clone(),
            ));
        }
    }

    if let Some(file) = std::env::var_os("DIGEST") {
        std::fs::write(file, &digest).unwrap();
    }
    let total: Duration = formats.values().map(|t| t.time).sum();
    println!(
        "decoded {} files, total {:?}; through decode(): {:?}\n",
        files.len(),
        total,
        dispatch_total
    );
    slow_dispatch.sort_by_key(|d| std::cmp::Reverse(d.0));
    println!("== files where trying other formats first costs the most");
    for (extra, format, name) in slow_dispatch.iter().take(top) {
        println!("{extra:>10.2?} extra  {name} ({format})");
    }
    println!();

    let mut by_time: Vec<_> = formats.iter().collect();
    by_time.sort_by_key(|(_, t)| std::cmp::Reverse(t.time));
    println!("== formats by total time");
    for (name, t) in by_time.iter().take(top) {
        println!(
            "{:>10.2?} {:>6} files {:>8.2} ns/px  {name}",
            t.time,
            t.files,
            t.time.as_nanos() as f64 / t.pixels.max(1) as f64
        );
    }

    let mut by_px: Vec<_> = formats.iter().filter(|(_, t)| t.pixels > 20_000).collect();
    by_px.sort_by(|a, b| {
        let c = |t: &Totals| t.time.as_nanos() as f64 / t.pixels as f64;
        c(b.1).total_cmp(&c(a.1))
    });
    println!("\n== formats by ns/pixel");
    for (name, t) in by_px.iter().take(top) {
        println!(
            "{:>8.2} ns/px {:>10.2?} {:>6} files  {name}",
            t.time.as_nanos() as f64 / t.pixels as f64,
            t.time,
            t.files
        );
    }

    files.sort_by_key(|f| std::cmp::Reverse(f.0));
    println!("\n== slowest files");
    for (time, pixels, name) in files.iter().take(top) {
        println!("{time:>10.2?} {pixels:>9} px  {name}");
    }
}
