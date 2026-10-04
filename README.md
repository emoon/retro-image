# retro-image

[![CI](https://github.com/emoon/retro-image/actions/workflows/ci.yml/badge.svg)](https://github.com/emoon/retro-image/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/retro-image.svg)](https://crates.io/crates/retro-image)
[![Documentation](https://docs.rs/retro-image/badge.svg)](https://docs.rs/retro-image)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/emoon/retro-image/blob/main/LICENSE)

retro-image opens pictures from old computers, so you can view them on a modern machine. It
reads 571 formats from 52 platforms: Atari 8-bit and ST, Amiga, Commodore, ZX Spectrum,
Amstrad CPC, MSX, PC-98 and a long tail of rarer machines. Every format is listed in
[docs/formats.md](https://github.com/emoon/retro-image/blob/main/docs/formats.md).

[RECOIL](https://recoil.sourceforge.net) inspired the project, and the tests compare the
output against it. None of its code was read or used. The decoders are written from public
format documentation, permissively licensed code, and reverse engineering of sample files.
[docs/sources.md](https://github.com/emoon/retro-image/blob/main/docs/sources.md) lists the
sources for each format.

## Library

The crate is `no_std` (it needs `alloc`) and has no dependencies. You give it bytes and it
gives you pixels. It never touches the filesystem.

```toml
[dependencies]
retro-image = "0.0.1"
```

```rust
let data = std::fs::read("PICTURE.PI1")?;
let image = retro_image::decode("PICTURE.PI1", &data)?;
let (width, height) = (image.width(), image.height());
let rgb: &[u8] = image.rgb(); // 3 bytes per pixel, row by row
```

The file name tells it which formats to try. Many formats also have a reliable signature,
so those still decode if the extension is wrong. A few formats keep their colours in a
second file, such as a `.SCR` with a `.PAL`. For those, call `decode_with` and pass the
extra file in.

## Command line

```sh
cargo install retro-image-cli
retro-image PICTURE.PI1                  # writes PICTURE.PI1.png
retro-image --list-formats
```

It can also generate thumbnails for file managers that use freedesktop thumbnailers:

```sh
retro-image --mime-xml    > ~/.local/share/mime/packages/retro-image.xml
retro-image --thumbnailer > ~/.local/share/thumbnailers/retro-image.thumbnailer
update-mime-database ~/.local/share/mime
```

## Documentation

- [Supported formats](https://github.com/emoon/retro-image/blob/main/docs/formats.md)
- [Sources](https://github.com/emoon/retro-image/blob/main/docs/sources.md)
- [Development](https://github.com/emoon/retro-image/blob/main/docs/development.md): building, testing and adding a format

## License

MIT, see [LICENSE](https://github.com/emoon/retro-image/blob/main/LICENSE).
