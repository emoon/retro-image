//! Converts retro computer images to PNG.
//!
//! ```text
//! retro-image INPUT [-o OUTPUT]
//! retro-image -i INPUT -o OUTPUT [-s SIZE] [--ext EXT]   (thumbnailer mode)
//! retro-image --list-formats
//! retro-image --mime-xml | --thumbnailer
//! ```
//!
//! On failure nothing is written and the exit code is non-zero, as
//! thumbnailer hosts expect.
//!
//! Sources: the thumbnailer conventions this follows (`-i %i -o %o -s %s`,
//! a PNG written to the output path, a non-zero exit status on failure so
//! the host records the failure instead of a thumbnail) are those of
//! `.thumbnailer` entries, described in the Xfce Tumbler documentation,
//! <https://docs.xfce.org/xfce/tumbler/available_plugins>; failure handling
//! by the host is in the Thumbnail Managing Standard, "Thumbnail Creation
//! Failures", <https://specifications.freedesktop.org/thumbnail/latest/>.
//! See `freedesktop.rs` for the generated entry.

mod freedesktop;
mod thumbnail;

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Inputs larger than this are refused before reading: no supported format
/// comes close, so such a file is not a retro image.
const MAX_INPUT_LEN: u64 = 32 << 20;

const USAGE: &str = "usage: retro-image INPUT [-o OUTPUT]
       retro-image -i INPUT -o OUTPUT [-s SIZE] [--ext EXT]
       retro-image --list-formats | --mime-xml | --thumbnailer";

struct Convert {
    input: PathBuf,
    output: PathBuf,
    /// Fit the output within `size` x `size` pixels.
    size: Option<u32>,
    /// Extension to choose the format by, instead of the input's own (the
    /// input may be a symlink target without one).
    ext: Option<String>,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [flag] if flag == "--list-formats" => {
            print!("{}", freedesktop::format_list());
            return ExitCode::SUCCESS;
        }
        [flag] if flag == "--mime-xml" => {
            print!("{}", freedesktop::mime_xml());
            return ExitCode::SUCCESS;
        }
        [flag] if flag == "--thumbnailer" => {
            print!("{}", freedesktop::thumbnailer());
            return ExitCode::SUCCESS;
        }
        _ => {}
    }
    let Some(convert) = parse(&args) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    match run(&convert) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{}: {e}", convert.input.display());
            ExitCode::FAILURE
        }
    }
}

fn parse(args: &[String]) -> Option<Convert> {
    let mut input = None;
    let mut output = None;
    let mut size = None;
    let mut ext = None;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-i" => input = Some(PathBuf::from(args.next()?)),
            "-o" => output = Some(PathBuf::from(args.next()?)),
            "-s" => size = Some(args.next()?.parse().ok().filter(|&s| s > 0)?),
            "--ext" => ext = Some(args.next()?.trim_start_matches('.').to_owned()),
            positional if !positional.starts_with('-') && input.is_none() => {
                input = Some(PathBuf::from(positional));
            }
            _ => return None,
        }
    }
    let input = input?;
    let output = output.unwrap_or_else(|| {
        let mut name = input.clone().into_os_string();
        name.push(".png");
        PathBuf::from(name)
    });
    Some(Convert {
        input,
        output,
        size,
        ext,
    })
}

fn run(convert: &Convert) -> Result<(), Box<dyn Error>> {
    if std::fs::metadata(&convert.input)?.len() > MAX_INPUT_LEN {
        return Err("file too large for a retro image".into());
    }
    let data = std::fs::read(&convert.input)?;
    let filename = match &convert.ext {
        Some(ext) => format!("input.{ext}"),
        None => convert
            .input
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_owned(),
    };
    let image = retro_image::decode_with(&filename, &data, &SiblingFiles(&convert.input))?;
    let (width, height, rgb) = match convert.size {
        Some(size) => thumbnail::fit(&image, size),
        None => (image.width(), image.height(), image.rgb().to_vec()),
    };
    // Encode fully before creating the output, so failures leave no file.
    let png = encode_png(width, height, &rgb)?;
    std::fs::write(&convert.output, png)?;
    Ok(())
}

fn encode_png(width: u32, height: u32, rgb: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut png = Vec::new();
    let mut encoder = png::Encoder::new(&mut png, width, height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(rgb)?;
    Ok(png)
}

/// Companion files next to the input: same name, other extension, matched
/// case-insensitively (`PIC.MIC` finds `pic.col`).
struct SiblingFiles<'a>(&'a Path);

impl retro_image::Companions for SiblingFiles<'_> {
    fn get(&self, extension: &str) -> Option<Vec<u8>> {
        let stem = self.0.file_stem()?.to_str()?;
        let wanted = format!("{stem}.{extension}");
        let dir = self.0.parent().filter(|d| !d.as_os_str().is_empty());
        std::fs::read_dir(dir.unwrap_or(Path::new(".")))
            .ok()?
            .filter_map(Result::ok)
            .find(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .is_some_and(|n| n.eq_ignore_ascii_case(&wanted))
            })
            .and_then(|entry| std::fs::read(entry.path()).ok())
    }
}
