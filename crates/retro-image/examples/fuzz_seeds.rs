//! Writes the sample corpus as seeds for the `format` fuzz target
//! (see `fuzz/fuzz_targets/format.rs` for the layout):
//! `cargo run --release --example fuzz_seeds -- <corpus dir> <out dir>`
//!
//! Each file seeds every format that claims its extension, or, if none
//! does, the formats with a signature.

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
            for (i, format) in retro_image::formats().enumerate() {
                if !format.matches_filename(&name) {
                    continue;
                }
                let mut seed = (i as u16).to_le_bytes().to_vec();
                seed.push(0);
                seed.extend_from_slice(&data);
                count += 1;
                std::fs::write(Path::new(out).join(format!("{i:03}-{count}")), seed).unwrap();
            }
        }
    }
    eprintln!("{count} seeds");
}
