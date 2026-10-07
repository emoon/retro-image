//! Converts retro computer images to PNG.
//!
//! ```text
//! retro-image INPUT [-o OUTPUT]
//! retro-image -i INPUT -o OUTPUT [-s SIZE] [--ext EXT]   (thumbnailer mode)
//! retro-image ... [--max-image-mb MB]   (size limit, library default)
//! retro-image --list-formats
//! retro-image --mime-xml | --thumbnailer
//! retro-image --help | --version
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

use std::cell::{Cell, OnceCell};
use std::error::Error;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Inputs larger than this are refused before reading: no supported format
/// comes close, so such a file is not a retro image.
const MAX_INPUT_LEN: u64 = 32 << 20;

fn usage() -> String {
    format!(
        "usage: retro-image INPUT [-o OUTPUT]
       retro-image -i INPUT -o OUTPUT [-s SIZE] [--ext EXT]
       retro-image ... [--max-image-mb MB]   (default {})
       retro-image --list-formats | --mime-xml | --thumbnailer
       retro-image --help | --version",
        retro_image::max_image_bytes() >> 20
    )
}

struct Convert {
    input: PathBuf,
    output: PathBuf,
    /// Fit the output within `size` x `size` pixels.
    size: Option<u32>,
    /// Extension to choose the format by, instead of the input's own (the
    /// input may be a symlink target without one).
    ext: Option<String>,
    /// Largest decoded picture, in MiB; the library's default if not given.
    max_image_mb: Option<usize>,
}

fn main() -> ExitCode {
    // OsString: a file name need not be UTF-8, and `args()` would panic.
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    if let [flag] = args.as_slice() {
        match flag.to_str() {
            Some("--help" | "-h") => return print_stdout(&format!("{}\n", usage())),
            Some("--version" | "-V") => {
                return print_stdout(&format!("retro-image {}\n", env!("CARGO_PKG_VERSION")));
            }
            Some("--list-formats") => return print_stdout(&freedesktop::format_list()),
            Some("--mime-xml") => return print_stdout(&freedesktop::mime_xml()),
            Some("--thumbnailer") => return print_stdout(&freedesktop::thumbnailer()),
            _ => {}
        }
    }
    let Some(convert) = parse(&args) else {
        eprintln!("{}", usage());
        return ExitCode::from(2);
    };
    match run(&convert) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{}: {}", convert.input.display(), describe(e.as_ref()));
            ExitCode::FAILURE
        }
    }
}

/// Writes `text` to standard output. A reader that went away (`| head`) is
/// not an error: the output is simply no longer wanted.
fn print_stdout(text: &str) -> ExitCode {
    use std::io::Write;
    let mut out = std::io::stdout().lock();
    match out.write_all(text.as_bytes()).and_then(|()| out.flush()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("retro-image: cannot write to standard output: {e}");
            ExitCode::FAILURE
        }
    }
}

/// The message for a failure. A decode failure lists each format that was
/// tried and why it refused the file, so "unknown format" is not all a user
/// sees.
fn describe(error: &(dyn Error + 'static)) -> String {
    use retro_image::DecodeError;
    let Some(DecodeError::NoMatch { attempts }) = error.downcast_ref::<DecodeError>() else {
        return error.to_string();
    };
    let mut text = error.to_string();
    // The formats with the file's extension come first; a long tail of
    // content probes after them is noise.
    const SHOWN: usize = 5;
    for attempt in attempts.iter().take(SHOWN) {
        text.push_str(&format!(
            "\n  {} {}: {}",
            attempt.format().platform(),
            attempt.format().name(),
            attempt.error()
        ));
    }
    if attempts.len() > SHOWN {
        text.push_str(&format!("\n  ... and {} more", attempts.len() - SHOWN));
    }
    text
}

fn parse(args: &[OsString]) -> Option<Convert> {
    let mut input = None;
    let mut output = None;
    let mut size = None;
    let mut ext = None;
    let mut max_image_mb = None;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        // An argument that is not UTF-8 can only be a path.
        match arg.to_str() {
            Some("-i") => input = Some(PathBuf::from(args.next()?)),
            Some("-o") => output = Some(PathBuf::from(args.next()?)),
            Some("-s") => size = Some(args.next()?.to_str()?.parse().ok().filter(|&s| s > 0)?),
            Some("--max-image-mb") => {
                max_image_mb = Some(args.next()?.to_str()?.parse().ok().filter(|&mb| mb > 0)?);
            }
            Some("--ext") => {
                ext = Some(args.next()?.to_str()?.trim_start_matches('.').to_owned());
            }
            Some(flag) if flag.starts_with('-') => return None,
            _ if input.is_none() => input = Some(PathBuf::from(arg)),
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
        max_image_mb,
    })
}

fn run(convert: &Convert) -> Result<(), Box<dyn Error>> {
    if let Some(mb) = convert.max_image_mb {
        let bytes = mb
            .checked_mul(1 << 20)
            .ok_or("--max-image-mb is too large")?;
        retro_image::set_max_image_bytes(bytes);
    }
    let data = read_input(&convert.input)?;
    let filename = match &convert.ext {
        Some(ext) => format!("input.{ext}"),
        None => convert
            .input
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
    };
    let image = retro_image::decode_with(&filename, &data, &SiblingFiles::new(&convert.input))?
        .into_image();
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
    read_limited(path, MAX_INPUT_LEN)
}

/// Like [`read_input`], with a smaller `limit`.
fn read_limited(path: &Path, limit: u64) -> std::io::Result<Vec<u8>> {
    use std::io::{Error, ErrorKind, Read};
    // Checked before opening: opening a FIFO blocks.
    if !std::fs::metadata(path)?.is_file() {
        return Err(Error::new(ErrorKind::InvalidInput, "not a regular file"));
    }
    let file = std::fs::File::open(path)?;
    let mut data = Vec::new();
    file.take(limit + 1).read_to_end(&mut data)?;
    if data.len() as u64 > limit {
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
///
/// The input's file names are `OsStr`, so a name that is not UTF-8 works.
/// The directory is listed once per run, and all companions together may
/// read at most [`COMPANION_BUDGET`] bytes: a file can name other files (a
/// KiSS set lists thousands of cells) and must not make a run read
/// gigabytes.
struct SiblingFiles<'a> {
    input: &'a Path,
    listing: OnceCell<Vec<OsString>>,
    /// Bytes companions may still read.
    budget: Cell<u64>,
}

/// Most bytes all companions of one input may read in total.
const COMPANION_BUDGET: u64 = 256 << 20;

impl<'a> SiblingFiles<'a> {
    fn new(input: &'a Path) -> Self {
        Self {
            input,
            listing: OnceCell::new(),
            budget: Cell::new(COMPANION_BUDGET),
        }
    }

    fn directory(&self) -> &Path {
        self.input
            .parent()
            .filter(|d| !d.as_os_str().is_empty())
            .unwrap_or(Path::new("."))
    }

    /// File names in the directory, read on first use.
    fn names(&self) -> &[OsString] {
        self.listing.get_or_init(|| {
            std::fs::read_dir(self.directory())
                .into_iter()
                .flatten()
                .filter_map(Result::ok)
                .map(|entry| entry.file_name())
                .collect()
        })
    }

    /// A companion's contents, charged to the budget.
    fn read(&self, path: &Path) -> Option<std::borrow::Cow<'static, [u8]>> {
        let limit = MAX_INPUT_LEN.min(self.budget.get());
        let data = read_limited(path, limit).ok()?;
        self.budget.set(self.budget.get() - data.len() as u64);
        Some(data.into())
    }
}

impl retro_image::Companions for SiblingFiles<'_> {
    fn get_named(&self, file_name: &str) -> Option<std::borrow::Cow<'_, [u8]>> {
        let name = file_name.rsplit(['/', '\\']).next()?;
        if matches!(name, "" | "." | "..") {
            return None;
        }
        self.read(&self.directory().join(name))
    }

    fn get(&self, extension: &str) -> Option<std::borrow::Cow<'_, [u8]>> {
        let mut wanted = self.input.file_stem()?.to_owned();
        wanted.push(".");
        wanted.push(extension);
        let found = self
            .names()
            .iter()
            .find(|name| name.eq_ignore_ascii_case(&wanted))?;
        self.read(&self.directory().join(found))
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
        retro_image::decode("picture.pam", &file)
            .unwrap()
            .into_image()
    }

    #[cfg(unix)]
    #[test]
    fn companions_work_for_non_utf8_names_and_stop_at_the_budget() {
        use retro_image::Companions;
        use std::os::unix::ffi::OsStrExt;
        let dir = std::env::temp_dir().join(format!("retro-image-cli-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let input = dir.join(std::ffi::OsStr::from_bytes(b"p\xFFic.mic"));
        let companion = dir.join(std::ffi::OsStr::from_bytes(b"P\xFFIC.col"));
        std::fs::write(&companion, [7u8; 1000]).unwrap();

        let siblings = SiblingFiles::new(&input);
        siblings.budget.set(1500);
        assert_eq!(siblings.get("col").unwrap().len(), 1000);
        // 500 bytes left: the same file no longer fits.
        assert!(siblings.get("col").is_none());

        std::fs::remove_dir_all(&dir).unwrap();
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
