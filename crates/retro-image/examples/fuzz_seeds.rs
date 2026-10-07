//! Writes the sample corpus as seeds for the `format` fuzz target
//! (see `fuzz/fuzz_targets/format.rs` for the layout):
//! `cargo run --release --example fuzz_seeds -- <corpus dir> <out dir>`
//!
//! Each file seeds every format that claims its extension, or, if none
//! does, the signature formats that decode it. For formats that read
//! companions, a second seed puts the first sibling file (same name, other
//! extension) after the main file, cut where the split byte says (to within
//! 1/256 of the size).
//!
//! It also writes seeds that no corpus file and no random mutation reaches:
//! files built to make a decoder repeat work, such as thousands of 6-byte
//! FLIC chunks or hundreds of overlapping Utah RLE runs, where the time a
//! decode takes grows with how the input is arranged rather than its size.

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
                    count += 1;
                    write_seed(out, i, split, &body, count);
                }
            }
        }
    }
    for (name, split, body) in structured_seeds() {
        let (i, _) = retro_image::formats()
            .enumerate()
            .find(|(_, f)| f.name() == name.0 && f.matches_filename(&format!("seed.{}", name.1)))
            .unwrap_or_else(|| panic!("no format {name:?}"));
        count += 1;
        write_seed(out, i, split, &body, count);
    }
    eprintln!("{count} seeds");
}

fn write_seed(out: &str, format: usize, split: u8, body: &[u8], number: usize) {
    let mut seed = (format as u16).to_le_bytes().to_vec();
    seed.push(split);
    seed.extend_from_slice(body);
    std::fs::write(Path::new(out).join(format!("{format:03}-{number}")), seed).unwrap();
}

/// Largest file the `format` target is run with (`-max_len` in the README),
/// minus its 3 bytes of layout.
const MAX_BODY: usize = 262_144 - 3;

/// A seed: `((format name, an extension it uses), split byte, body)`.
type Structured = ((&'static str, &'static str), u8, Vec<u8>);

/// Seeds for decoders whose work depends on how the input is arranged.
fn structured_seeds() -> Vec<Structured> {
    let fli = ("Autodesk Animator FLI and FLC", "flc");
    let mut seeds = Vec::new();
    // A big screen and as many 6-byte full-frame chunks as fit.
    let chunks = (MAX_BODY - 128 - 16) / 6;
    let black = flic_chunk(13, &[]);
    let copy = flic_chunk(16, &[]);
    for (magic, depth) in [(0xaf12, 8), (0xaf11, 8), (0xaf44, 16)] {
        seeds.push((fli, 0, flic(magic, depth, &black.repeat(chunks), chunks)));
    }
    let mixed = [copy.clone(), black].concat().repeat(chunks / 2);
    seeds.push((fli, 0, flic(0xaf12, 8, &mixed, chunks / 2 * 2)));
    seeds.push((fli, 0, flic(0xaf12, 8, &copy.repeat(chunks), chunks)));

    // A full-row run, over and over: SET_COLOR 0, then RUN (long form) of 60000.
    let mut rle = vec![0x52, 0xcc, 0, 0, 0, 0];
    rle.extend_from_slice(&60_000u16.to_le_bytes());
    rle.extend_from_slice(&1u16.to_le_bytes());
    rle.extend_from_slice(&[0, 1, 8, 0, 8, 0]);
    let run = [2, 0, 0x46, 0, 0x5f, 0xea, 7, 0];
    rle.extend(run.repeat((MAX_BODY - rle.len()) / run.len()));
    seeds.push((("Utah RLE", "rle"), 0, rle));

    // One color segment whose areas are all the whole screen.
    let mut kt4 = vec![2, 1, 2, 3, 4, 5, 6];
    let fill = [0x00, 0, 0, 159, 99];
    kt4.extend(fill.repeat((MAX_BODY - 10) / fill.len()));
    kt4.extend_from_slice(&[0xff, 0xff, 0xff]);
    seeds.push((("Kitty", "kt4"), 0, kt4));

    // A KiSS set that draws one big cel (the companion) in every entry.
    let (width, height) = (600usize, 400usize);
    let mut cel = (width as u16).to_le_bytes().to_vec();
    cel.extend_from_slice(&(height as u16).to_le_bytes());
    cel.resize(4 + width.div_ceil(2) * height, 0x11);
    let mut set = b"(448,320)\n".to_vec();
    for _ in 0..4096 {
        set.extend_from_slice(b"#0 a.cel\n");
    }
    set.extend_from_slice(b"$0 0,0\n");
    let (split, set) = with_companion(set, cel.len());
    seeds.push((("Set", "cnf"), split, [set, cel].concat()));
    seeds
}

/// A FLIC file of a 2048x2048 screen with one frame of `count` chunks.
fn flic(magic: u16, depth: u16, chunks: &[u8], count: usize) -> Vec<u8> {
    let mut file = vec![0; 128];
    file[4..6].copy_from_slice(&magic.to_le_bytes());
    file[8..10].copy_from_slice(&2048u16.to_le_bytes());
    file[10..12].copy_from_slice(&2048u16.to_le_bytes());
    file[12..14].copy_from_slice(&depth.to_le_bytes());
    let mut frame = flic_chunk(0xf1fa, &[]);
    frame.extend_from_slice(&[0; 10]);
    frame[6..8].copy_from_slice(&(count.min(65535) as u16).to_le_bytes());
    frame.extend_from_slice(chunks);
    let size = frame.len() as u32;
    frame[..4].copy_from_slice(&size.to_le_bytes());
    file.extend(frame);
    let size = file.len() as u32;
    file[..4].copy_from_slice(&size.to_le_bytes());
    file
}

fn flic_chunk(kind: u16, body: &[u8]) -> Vec<u8> {
    let mut chunk = (body.len() as u32 + 6).to_le_bytes().to_vec();
    chunk.extend_from_slice(&kind.to_le_bytes());
    chunk.extend_from_slice(body);
    chunk
}

/// The split byte that cuts a file of `main` followed by `companion_len`
/// bytes exactly after `main`, and `main` padded with blank lines to the
/// length that needs.
fn with_companion(mut main: Vec<u8>, companion_len: usize) -> (u8, Vec<u8>) {
    for split in 1..=255usize {
        let len = (companion_len * split).div_ceil(256 - split);
        if len >= main.len() && (len + companion_len) * split / 256 == len {
            main.resize(len, b'\n');
            return (split as u8, main);
        }
    }
    panic!("no split byte cuts the companion off exactly");
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
