//! `retro-image INPUT [-o OUTPUT]`: converts a retro computer image to PNG.

use std::error::Error;
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (input, output) = match args.as_slice() {
        [input] => (PathBuf::from(input), PathBuf::from(format!("{input}.png"))),
        [input, flag, output] if flag == "-o" => (PathBuf::from(input), PathBuf::from(output)),
        _ => {
            eprintln!("usage: retro-image INPUT [-o OUTPUT]");
            return ExitCode::from(2);
        }
    };
    match convert(&input, &output) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{}: {e}", input.display());
            ExitCode::FAILURE
        }
    }
}

fn convert(input: &Path, output: &Path) -> Result<(), Box<dyn Error>> {
    let data = std::fs::read(input)?;
    let filename = input
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    let image = retro_image::decode(filename, &data)?;
    write_png(output, &image)
}

fn write_png(path: &Path, image: &retro_image::Image) -> Result<(), Box<dyn Error>> {
    let mut encoder = png::Encoder::new(
        BufWriter::new(File::create(path)?),
        image.width(),
        image.height(),
    );
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(image.rgb())?;
    Ok(())
}
