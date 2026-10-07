# retro-image

[![CI](https://github.com/emoon/retro-image/actions/workflows/ci.yml/badge.svg)](https://github.com/emoon/retro-image/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/retro-image.svg)](https://crates.io/crates/retro-image)
[![Documentation](https://docs.rs/retro-image/badge.svg)](https://docs.rs/retro-image)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/emoon/retro-image/blob/main/LICENSE)

retro-image opens pictures from old computers, so you can view them on a modern machine. It
reads 688 formats from 75 platforms: Atari 8-bit and ST, Amiga, Commodore, ZX Spectrum,
Amstrad CPC, MSX, PC-98, several game consoles and a long tail of rarer machines. Every
format is listed in
[docs/formats.md](https://github.com/emoon/retro-image/blob/main/docs/formats.md).

[RECOIL](https://recoil.sourceforge.net) inspired the project, and the tests compare the
output against it. None of its code was read or used. The decoders are written from public
format documentation, permissively licensed code, and reverse engineering of sample files.
[docs/sources.md](https://github.com/emoon/retro-image/blob/main/docs/sources.md) lists the
sources for each format.

Most of the code was written by AI coding agents. The tests decode real sample files,
compare the output with RECOIL where it supports the format, and feed truncated and corrupted
files to every decoder.

## Library

The crate is `no_std` (it needs `alloc`) and has no dependencies. You give it bytes and it
gives you pixels. It never touches the filesystem.

```toml
[dependencies]
retro-image = "1.0"
```

```rust
let data = std::fs::read("PICTURE.PI1")?;
let decoded = retro_image::decode("PICTURE.PI1", &data)?;
let image = decoded.image();
let (width, height) = (image.width(), image.height());
let rgb: &[u8] = image.rgb(); // 3 bytes per pixel, row by row
println!("{} / {}", decoded.format().platform(), decoded.format().name());
```

If a picture has transparency, `image.has_alpha()` is true. `rgb()` still returns only the color
channels, and they are black wherever a pixel is fully transparent. `rgba()` copies the pixels
with straight (not premultiplied) alpha, and `flattened([r, g, b])` returns a copy drawn over a
background color. For these pictures the command line writes a PNG with an alpha channel.

The file name tells it which formats to try. Many formats also have a reliable signature,
so those still decode if the extension is wrong. A few formats keep their colors in a
second file, such as a `.SCR` with a `.PAL`. For those, call `decode_with` and pass an
implementation of `Companions` that can fetch the extra file.

When nothing decodes the file, the error says why. `DecodeError::NoMatch` lists every format
that was tried and the reason it gave, and `DecodeError::UnknownFormat` means that no format
uses the extension and none recognized the content. The enum is `#[non_exhaustive]`, so keep a
wildcard arm.

Decoding never panics on a truncated or corrupt file, it returns an error. It also doesn't let a
header demand memory: a decoded picture may take at most 64 MiB by default (4 bytes per pixel,
about 16.8 million pixels), and a bigger one fails with `DecodeError::TooLarge` before anything
is allocated. Call `set_max_image_bytes(n)` once at start-up to change it. The limit applies to the whole process.

The minimum supported Rust version is 1.94. Raising it counts as a minor-version change, not a
patch.

## Command line

```sh
cargo install retro-image-cli
retro-image PICTURE.PI1                  # writes PICTURE.PI1.png
retro-image --list-formats
retro-image --max-image-mb 128 BIG.GIF   # raise the picture size limit (default 64)
```

It can also generate thumbnails for file managers that use freedesktop thumbnailers:

```sh
retro-image --mime-xml    > ~/.local/share/mime/packages/retro-image.xml
retro-image --thumbnailer > ~/.local/share/thumbnailers/retro-image.thumbnailer
update-mime-database ~/.local/share/mime
```

The package gives the formats' extensions the type `image/x-retro-image` at a low priority, so
a system type for `.pbm` or `.xpm` still wins. Names that ordinary files often carry (`.cnf`,
`.tpl`, `.srm` and a few more) are left out. Those formats still decode from the command line,
but file managers won't thumbnail them.

## Documentation

- [Supported formats](https://github.com/emoon/retro-image/blob/main/docs/formats.md)
- [Sources](https://github.com/emoon/retro-image/blob/main/docs/sources.md)
- [Development](https://github.com/emoon/retro-image/blob/main/docs/development.md): building, testing and adding a format

## License

MIT, see [LICENSE](https://github.com/emoon/retro-image/blob/main/LICENSE).
