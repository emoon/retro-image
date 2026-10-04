//! Writes the sample corpus as seeds for the `format` fuzz target
//! (see `fuzz/fuzz_targets/format.rs` for the layout):
//! `cargo run --release --example fuzz_seeds -- <corpus dir> <out dir>`
//!
//! Each file seeds every format that claims its extension, or, if none
//! does, the signature formats that decode it. For formats that read
//! companions, a second seed puts the first sibling file (same name, other
//! extension) after the main file, cut where the split byte says (to within
//! 1/256 of the size).

use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [corpus, out] = &args[..] else {
        eprintln!("usage: fuzz_seeds <corpus dir> <out dir>");
        std::process::exit(2);
    };
    std::fs::create_dir_all(out).unwrap();
    let mut count = 0;
    let mut dirs = vec![Path::new(corpus).to_path_buf()];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                dirs.push(path);
                continue;
            }
            let (Ok(data), Some(name)) = (std::fs::read(&path), path.file_name()) else {
                continue;
            };
            if data.len() > 1 << 20 {
                continue;
            }
            let name = name.to_string_lossy();
            let claimed: Vec<_> = retro_image::formats()
                .enumerate()
                .filter(|(_, f)| f.matches_filename(&name))
                .collect();
            let chosen = if claimed.is_empty() {
                retro_image::formats()
                    .enumerate()
                    .filter(|(_, f)| f.has_signature() && f.decode(&data).is_ok())
                    .collect()
            } else {
                claimed
            };
            let sibling = first_sibling(&path);
            for (i, format) in chosen {
                let mut seeds = vec![(0, data.clone())];
                if let (true, Some(extra)) = (format.uses_companions(), &sibling) {
                    let total = data.len() + extra.len();
                    let split = (data.len() * 256 / total.max(1)).clamp(1, 255) as u8;
                    seeds.push((split, [data.as_slice(), extra].concat()));
                }
                for (split, body) in seeds {
                    let mut seed = (i as u16).to_le_bytes().to_vec();
                    seed.push(split);
                    seed.extend_from_slice(&body);
                    count += 1;
                    std::fs::write(Path::new(out).join(format!("{i:03}-{count}")), seed).unwrap();
                }
            }
        }
    }
    eprintln!("{count} seeds");
}

/// The first other file in `path`'s directory with the same stem, if small.
fn first_sibling(path: &Path) -> Option<Vec<u8>> {
    let stem = path.file_stem()?.to_str()?;
    let mut siblings: Vec<_> = std::fs::read_dir(path.parent()?)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && p != path)
        .filter(|p| {
            p.file_stem()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s.eq_ignore_ascii_case(stem))
        })
        .collect();
    siblings.sort();
    siblings
        .into_iter()
        .find_map(|p| std::fs::read(p).ok().filter(|d| d.len() <= 1 << 20))
}
