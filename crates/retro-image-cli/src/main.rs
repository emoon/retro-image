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
    let data = read_input(&convert.input)?;
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
    let raster = match convert.size {
        Some(size) => thumbnail::fit(&image, size),
        None => Raster::of(&image),
    };
    // Encode fully before creating the output, so failures leave no file.
    let png = encode_png(&raster)?;
    std::fs::write(&convert.output, png)?;
    Ok(())
}

/// Reads a regular file of at most [`MAX_INPUT_LEN`] bytes. Other kinds
/// (FIFOs, devices) could block or never end, and companions named by an
/// untrusted file get the same limit as the input itself.
fn read_input(path: &Path) -> std::io::Result<Vec<u8>> {
    use std::io::{Error, ErrorKind, Read};
    // Checked before opening: opening a FIFO blocks.
    if !std::fs::metadata(path)?.is_file() {
        return Err(Error::new(ErrorKind::InvalidInput, "not a regular file"));
    }
    let file = std::fs::File::open(path)?;
    let mut data = Vec::new();
    file.take(MAX_INPUT_LEN + 1).read_to_end(&mut data)?;
    if data.len() as u64 > MAX_INPUT_LEN {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "file too large for a retro image",
        ));
    }
    Ok(data)
}

/// Pixels ready for a PNG: RGBA (4 bytes per pixel) for a picture with
/// transparency, RGB (3 bytes) otherwise.
struct Raster {
    width: u32,
    height: u32,
    alpha: bool,
    data: Vec<u8>,
}

impl Raster {
    /// All of `image` at its own size.
    fn of(image: &retro_image::Image) -> Self {
        let alpha = image.has_alpha();
        Self {
            width: image.width(),
            height: image.height(),
            alpha,
            data: if alpha {
                image.rgba()
            } else {
                image.rgb().to_vec()
            },
        }
    }
}

fn encode_png(raster: &Raster) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut png = Vec::new();
    let mut encoder = png::Encoder::new(&mut png, raster.width, raster.height);
    encoder.set_color(if raster.alpha {
        png::ColorType::Rgba
    } else {
        png::ColorType::Rgb
    });
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(&raster.data)?;
    Ok(png)
}

/// Companion files next to the input: same name, other extension, matched
/// case-insensitively (`PIC.MIC` finds `pic.col`), or files looked up by name.
struct SiblingFiles<'a>(&'a Path);

impl SiblingFiles<'_> {
    fn directory(&self) -> &Path {
        self.0
            .parent()
            .filter(|d| !d.as_os_str().is_empty())
            .unwrap_or(Path::new("."))
    }
}

impl retro_image::Companions for SiblingFiles<'_> {
    fn get_named(&self, file_name: &str) -> Option<std::borrow::Cow<'_, [u8]>> {
        let name = file_name.rsplit(['/', '\\']).next()?;
        if matches!(name, "" | "." | "..") {
            return None;
        }
        read_input(&self.directory().join(name))
            .ok()
            .map(Into::into)
    }

    fn get(&self, extension: &str) -> Option<std::borrow::Cow<'_, [u8]>> {
        let stem = self.0.file_stem()?.to_str()?;
        let wanted = format!("{stem}.{extension}");
        std::fs::read_dir(self.directory())
            .ok()?
            .filter_map(Result::ok)
            .find(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .is_some_and(|n| n.eq_ignore_ascii_case(&wanted))
            })
            .and_then(|entry| read_input(&entry.path()).ok())
            .map(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `width` x 1 image from straight RGBA samples, decoded through a PAM
    /// (a format that keeps alpha) like any user file.
    pub(crate) fn pam_image(width: u32, rgba: &[u8]) -> retro_image::Image {
        let mut file = format!(
            "P7\nWIDTH {width}\nHEIGHT 1\nDEPTH 4\nMAXVAL 255\nTUPLTYPE RGB_ALPHA\nENDHDR\n"
        )
        .into_bytes();
        file.extend_from_slice(rgba);
        retro_image::decode("picture.pam", &file).unwrap()
    }

    fn color_type(png_data: &[u8]) -> png::ColorType {
        let decoder = png::Decoder::new(std::io::Cursor::new(png_data));
        decoder.read_info().unwrap().info().color_type
    }

    #[test]
    fn png_has_alpha_only_when_the_picture_has() {
        let clear = pam_image(2, &[10, 20, 30, 255, 1, 2, 3, 0]);
        let png = encode_png(&Raster::of(&clear)).unwrap();
        assert_eq!(color_type(&png), png::ColorType::Rgba);
        let solid = pam_image(2, &[10, 20, 30, 255, 1, 2, 3, 255]);
        let png = encode_png(&Raster::of(&solid)).unwrap();
        assert_eq!(color_type(&png), png::ColorType::Rgb);
    }
}
